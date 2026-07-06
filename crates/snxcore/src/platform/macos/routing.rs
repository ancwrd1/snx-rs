use std::{
    ffi::CString,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    os::fd::AsFd,
    sync::Mutex,
};

use anyhow::anyhow;
use async_trait::async_trait;
use ipnet::{Ipv4Net, Ipv6Net};
use libc::c_int;
use tracing::{debug, warn};

use crate::{
    model::params::TunnelType,
    platform::{
        RoutingConfig, RoutingConfigurator,
        macos::net::{
            build_route_message, next_seq, route_get, route_socket, send_route_message, sockaddr_dl, sockaddr_in,
            sockaddr_in6, struct_bytes,
        },
    },
};

struct TrackedRoute {
    dest: IpAddr,
    netmask: Option<IpAddr>,
}

pub struct MacosRoutingConfigurator {
    device: String,
    _tunnel_type: TunnelType,
    added: Mutex<Vec<TrackedRoute>>,
}

impl MacosRoutingConfigurator {
    pub fn new<S: AsRef<str>>(device: S, tunnel_type: TunnelType) -> Self {
        Self {
            device: device.as_ref().to_owned(),
            _tunnel_type: tunnel_type,
            added: Mutex::new(Vec::new()),
        }
    }

    fn tunnel_ifindex(&self) -> anyhow::Result<u16> {
        let name = CString::new(self.device.as_bytes())?;
        let index = unsafe { libc::if_nametoindex(name.as_ptr()) };
        if index == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(index as u16)
    }

    fn add_route_via_tunnel(&self, prefix: Ipv4Net) -> anyhow::Result<()> {
        debug!("Adding route {prefix} via {}", self.device);
        let dst = sockaddr_in(prefix.network());
        let gateway = sockaddr_dl(self.tunnel_ifindex()?);
        let netmask = sockaddr_in(prefix.netmask());

        let msg = build_route_message(
            libc::RTM_ADD as u8,
            libc::RTF_UP | libc::RTF_STATIC,
            next_seq(),
            &[
                (libc::RTA_DST, struct_bytes(&dst)),
                (libc::RTA_GATEWAY, struct_bytes(&gateway)),
                (libc::RTA_NETMASK, struct_bytes(&netmask)),
            ],
        );

        let sock = route_socket()?;
        match send_route_message(sock.as_fd(), &msg) {
            // Track only routes this process created; a pre-existing route (EEXIST) must survive
            // cleanup so we do not delete something the user already had.
            Ok(()) => self.added.lock().unwrap_or_else(|e| e.into_inner()).push(TrackedRoute {
                dest: prefix.network().into(),
                netmask: Some(prefix.netmask().into()),
            }),
            Err(e) if e.raw_os_error() == Some(libc::EEXIST) => debug!("Route {prefix} already exists, not tracking"),
            Err(e) => return Err(e.into()),
        }
        Ok(())
    }

    fn add_host_exclusion(&self, destination: Ipv4Addr, require_gateway: bool) -> anyhow::Result<()> {
        let sock = route_socket()?;
        let reply = route_get(sock.as_fd(), destination)?;

        if reply.ifindex == self.tunnel_ifindex()? {
            debug!("{destination} already resolves via the tunnel; skipping exclusion");
            return Ok(());
        }

        let Some(gateway) = reply.gateway.filter(|g| g.len() >= 2) else {
            if require_gateway {
                // Full-route mode covers the default with 0.0.0.0/1 + 128.0.0.0/1; without pinning the
                // gateway outside the tunnel, ESP-to-gateway would route back in, forming an encap loop.
                return Err(anyhow!(i18n::tr!("error-cannot-determine-ip")));
            }
            debug!("No original gateway for {destination}; skipping exclusion");
            return Ok(());
        };

        let dst = sockaddr_in(destination);
        let mut flags = libc::RTF_UP | libc::RTF_STATIC | libc::RTF_HOST;
        if gateway[1] as c_int == libc::AF_INET {
            flags |= libc::RTF_GATEWAY;
        }

        let msg = build_route_message(
            libc::RTM_ADD as u8,
            flags,
            next_seq(),
            &[
                (libc::RTA_DST, struct_bytes(&dst)),
                (libc::RTA_GATEWAY, gateway.as_slice()),
            ],
        );

        match send_route_message(sock.as_fd(), &msg) {
            Ok(()) => self.added.lock().unwrap_or_else(|e| e.into_inner()).push(TrackedRoute {
                dest: destination.into(),
                netmask: None,
            }),
            Err(e) if e.raw_os_error() == Some(libc::EEXIST) => {
                debug!("Host route {destination} already exists, not tracking")
            }
            Err(e) => return Err(e.into()),
        }
        Ok(())
    }

    // Blackhole an IPv6 prefix through the routing socket to prevent v6 leaks past the IPv4 tunnel.
    fn add_blackhole_v6(&self, prefix: Ipv6Net) -> anyhow::Result<()> {
        debug!("Adding IPv6 blackhole route {prefix}");
        let dst = sockaddr_in6(prefix.network());
        // RTF_BLACKHOLE drops matching packets; the gateway is unused but the entry still needs one.
        let gateway = sockaddr_in6(Ipv6Addr::LOCALHOST);
        let netmask = sockaddr_in6(prefix.netmask());

        let msg = build_route_message(
            libc::RTM_ADD as u8,
            libc::RTF_UP | libc::RTF_STATIC | libc::RTF_GATEWAY | libc::RTF_BLACKHOLE,
            next_seq(),
            &[
                (libc::RTA_DST, struct_bytes(&dst)),
                (libc::RTA_GATEWAY, struct_bytes(&gateway)),
                (libc::RTA_NETMASK, struct_bytes(&netmask)),
            ],
        );

        let sock = route_socket()?;
        match send_route_message(sock.as_fd(), &msg) {
            Ok(()) => self.added.lock().unwrap_or_else(|e| e.into_inner()).push(TrackedRoute {
                dest: prefix.network().into(),
                netmask: Some(prefix.netmask().into()),
            }),
            Err(e) if e.raw_os_error() == Some(libc::EEXIST) => {
                debug!("IPv6 blackhole route {prefix} already exists, not tracking")
            }
            Err(e) => return Err(e.into()),
        }
        Ok(())
    }

    fn delete_all(&self) {
        let routes = std::mem::take(&mut *self.added.lock().unwrap_or_else(|e| e.into_inner()));
        if routes.is_empty() {
            return;
        }

        let sock = match route_socket() {
            Ok(sock) => sock,
            Err(e) => {
                warn!("Cannot open route socket for cleanup: {e}");
                return;
            }
        };

        for route in routes {
            let mut flags = libc::RTF_UP | libc::RTF_STATIC;
            let msg = match route.dest {
                IpAddr::V4(dest) => {
                    let dst = sockaddr_in(dest);
                    let mask = match route.netmask {
                        Some(IpAddr::V4(m)) => Some(sockaddr_in(m)),
                        _ => None,
                    };
                    let mut parts: Vec<(c_int, &[u8])> = vec![(libc::RTA_DST, struct_bytes(&dst))];
                    match &mask {
                        Some(mask) => parts.push((libc::RTA_NETMASK, struct_bytes(mask))),
                        None => flags |= libc::RTF_HOST,
                    }
                    build_route_message(libc::RTM_DELETE as u8, flags, next_seq(), &parts)
                }
                IpAddr::V6(dest) => {
                    let dst = sockaddr_in6(dest);
                    let mask = match route.netmask {
                        Some(IpAddr::V6(m)) => Some(sockaddr_in6(m)),
                        _ => None,
                    };
                    let mut parts: Vec<(c_int, &[u8])> = vec![(libc::RTA_DST, struct_bytes(&dst))];
                    match &mask {
                        Some(mask) => parts.push((libc::RTA_NETMASK, struct_bytes(mask))),
                        None => flags |= libc::RTF_HOST,
                    }
                    build_route_message(libc::RTM_DELETE as u8, flags, next_seq(), &parts)
                }
            };

            match send_route_message(sock.as_fd(), &msg) {
                Ok(()) => {}
                Err(e) if matches!(e.raw_os_error(), Some(libc::ESRCH) | Some(libc::ENOENT)) => {}
                Err(e) => warn!("Failed to delete route {}: {e}", route.dest),
            }
        }
    }
}

// Delete IPv6 blackholes a previous instance may have left: unlike the utun-scoped /1 overrides the
// kernel drops with the device, these are gatewayed via ::1 and outlive it, black-holing all IPv6.
// The ::/1 + 8000::/1 pair is our signature; deletion is idempotent, table resets on reboot.
pub(super) fn cleanup_stale_blackholes() {
    let sock = match route_socket() {
        Ok(sock) => sock,
        Err(e) => {
            warn!("Cannot open route socket for blackhole cleanup: {e}");
            return;
        }
    };

    let prefixes = [
        Ipv6Net::new(Ipv6Addr::UNSPECIFIED, 1),
        Ipv6Net::new(Ipv6Addr::new(0x8000, 0, 0, 0, 0, 0, 0, 0), 1),
    ];
    for prefix in prefixes.into_iter().flatten() {
        let dst = sockaddr_in6(prefix.network());
        let netmask = sockaddr_in6(prefix.netmask());
        let msg = build_route_message(
            libc::RTM_DELETE as u8,
            libc::RTF_UP | libc::RTF_STATIC,
            next_seq(),
            &[
                (libc::RTA_DST, struct_bytes(&dst)),
                (libc::RTA_NETMASK, struct_bytes(&netmask)),
            ],
        );
        match send_route_message(sock.as_fd(), &msg) {
            Ok(()) => debug!("Removed stale IPv6 blackhole {prefix}"),
            Err(e) if matches!(e.raw_os_error(), Some(libc::ESRCH) | Some(libc::ENOENT)) => {}
            Err(e) => warn!("Failed to delete stale IPv6 blackhole {prefix}: {e}"),
        }
    }
}

impl Drop for MacosRoutingConfigurator {
    fn drop(&mut self) {
        // If configure() fails partway, the caller never stashes us for a later Cleanup, so undo
        // any routes we installed here. delete_all() is idempotent and tolerates already-gone routes.
        self.delete_all();
    }
}

#[async_trait]
impl RoutingConfigurator for MacosRoutingConfigurator {
    async fn configure(&self, config: &RoutingConfig) -> anyhow::Result<()> {
        match config {
            RoutingConfig::Full {
                destination,
                disable_ipv6,
            } => {
                debug!("Configuring full routing via {}", self.device);
                self.add_host_exclusion(*destination, true)?;
                // Override the default without deleting it by covering it with two more-specific halves.
                self.add_route_via_tunnel(Ipv4Net::new(Ipv4Addr::new(0, 0, 0, 0), 1)?)?;
                self.add_route_via_tunnel(Ipv4Net::new(Ipv4Addr::new(128, 0, 0, 0), 1)?)?;

                if *disable_ipv6 {
                    // Blackhole all IPv6 to prevent leaks past the IPv4 tunnel. Like the /1 halves
                    // above, two /1 blackholes win over any existing ::/0 default by longest-prefix
                    // match, while more-specific ::1/128 and fe80::/10 routes keep loopback and
                    // link-local working.
                    self.add_blackhole_v6(Ipv6Net::new(Ipv6Addr::UNSPECIFIED, 1)?)?;
                    self.add_blackhole_v6(Ipv6Net::new(Ipv6Addr::new(0x8000, 0, 0, 0, 0, 0, 0, 0), 1)?)?;
                }
            }
            RoutingConfig::Split { destination, routes } => {
                debug!("Configuring split routing via {}", self.device);
                self.add_host_exclusion(*destination, false)?;
                for route in routes {
                    self.add_route_via_tunnel(*route)?;
                }
            }
            RoutingConfig::Cleanup { .. } => {
                debug!("Cleaning up routing rules for {}", self.device);
                self.delete_all();
            }
        }
        Ok(())
    }
}
