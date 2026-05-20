use std::{net::Ipv4Addr, sync::Mutex};

use anyhow::anyhow;
use ipnet::Ipv4Net;
use tracing::{debug, warn};
use windows::Win32::{
    Foundation::{ERROR_NOT_FOUND, ERROR_OBJECT_ALREADY_EXISTS, NO_ERROR},
    NetworkManagement::{
        IpHelper::{
            CreateIpForwardEntry2, DeleteIpForwardEntry2, GetBestRoute2, InitializeIpForwardEntry, MIB_IPFORWARD_ROW2,
        },
        Ndis::NET_LUID_LH,
    },
    Networking::WinSock::SOCKADDR_INET,
};

use crate::{
    model::params::TunnelType,
    platform::{RoutingConfig, RoutingConfigurator, windows::firewall::WfpIpv6Block},
};

const TUNNEL_ROUTE_METRIC: u32 = 1;

pub struct WindowsRoutingConfigurator {
    device: String,
    tunnel_luid: NET_LUID_LH,
    _tunnel_type: TunnelType,
    added_rows: Mutex<Vec<MIB_IPFORWARD_ROW2>>,
    ipv6_block: Mutex<Option<WfpIpv6Block>>,
}

impl WindowsRoutingConfigurator {
    pub async fn new<S: AsRef<str>>(device: S, tunnel_type: TunnelType) -> anyhow::Result<Self> {
        let device = device.as_ref().to_owned();
        let tunnel_luid = super::alias_to_luid(&device)?;
        Ok(Self {
            device,
            tunnel_luid,
            _tunnel_type: tunnel_type,
            added_rows: Mutex::new(Vec::new()),
            ipv6_block: Mutex::new(None),
        })
    }

    fn add_route_via_tunnel(&self, prefix: Ipv4Net) -> anyhow::Result<()> {
        let mut row = new_forward_row();
        row.InterfaceLuid = self.tunnel_luid;
        row.DestinationPrefix.Prefix = super::sockaddr_ipv4(prefix.network());
        row.DestinationPrefix.PrefixLength = prefix.prefix_len();
        // Point-to-point tunnel: leave NextHop as unspecified (on-link via adapter).
        row.NextHop = super::sockaddr_ipv4(Ipv4Addr::UNSPECIFIED);
        row.Metric = TUNNEL_ROUTE_METRIC;

        self.create_row(row, &format!("{prefix} via {}", self.device))
    }

    fn add_host_exclusion(&self, destination: Ipv4Addr) -> anyhow::Result<()> {
        let dest_sa = super::sockaddr_ipv4(destination);
        let mut best = MIB_IPFORWARD_ROW2::default();
        let mut best_src = SOCKADDR_INET::default();

        unsafe { GetBestRoute2(None, 0, None, &dest_sa, 0, &mut best, &mut best_src) }
            .ok()
            .map_err(|e| anyhow!("GetBestRoute2({destination}) failed: {:?}", e))?;

        if luids_equal(best.InterfaceLuid, self.tunnel_luid) {
            // Best route already points at the tunnel — nothing useful to pin.
            debug!("GetBestRoute2({destination}) already resolves via the tunnel adapter; skipping exclusion");
            return Ok(());
        }

        let mut row = new_forward_row();
        row.InterfaceLuid = best.InterfaceLuid;
        row.DestinationPrefix.Prefix = dest_sa;
        row.DestinationPrefix.PrefixLength = 32;
        row.NextHop = best.NextHop;
        row.Metric = TUNNEL_ROUTE_METRIC;

        self.create_row(row, &format!("{destination}/32 via original interface"))
    }

    fn create_row(&self, row: MIB_IPFORWARD_ROW2, label: &str) -> anyhow::Result<()> {
        debug!("Adding route: {label}");
        let rc = unsafe { CreateIpForwardEntry2(&row) };
        if rc == NO_ERROR {
            self.added_rows.lock().unwrap_or_else(|e| e.into_inner()).push(row);
            Ok(())
        } else if rc == ERROR_OBJECT_ALREADY_EXISTS {
            debug!("Route already exists, tracking for cleanup: {label}");
            self.added_rows.lock().unwrap_or_else(|e| e.into_inner()).push(row);
            Ok(())
        } else {
            Err(anyhow!("CreateIpForwardEntry2 failed for {label}: {:?}", rc))
        }
    }

    fn delete_all(&self) {
        let rows = std::mem::take(&mut *self.added_rows.lock().unwrap_or_else(|e| e.into_inner()));
        for row in rows {
            let rc = unsafe { DeleteIpForwardEntry2(&row) };
            if rc != NO_ERROR && rc != ERROR_NOT_FOUND {
                warn!("DeleteIpForwardEntry2 failed: {:?}", rc);
            }
        }
    }
}

#[async_trait::async_trait]
impl RoutingConfigurator for WindowsRoutingConfigurator {
    async fn configure(&self, config: &RoutingConfig) -> anyhow::Result<()> {
        match config {
            RoutingConfig::Full {
                destination,
                disable_ipv6,
            } => {
                debug!("Configuring full routing via {}", self.device);
                self.add_host_exclusion(*destination)?;
                // Split the IPv4 default into two /1 routes (0.0.0.0/1 and 128.0.0.0/1).
                // This avoids touching the existing 0.0.0.0/0 entry on the physical
                // adapter (Windows allows multiple defaults but picks by metric, and
                // we want our routes to win unambiguously by longest-prefix-match).
                self.add_route_via_tunnel(Ipv4Net::new(Ipv4Addr::new(0, 0, 0, 0), 1)?)?;
                self.add_route_via_tunnel(Ipv4Net::new(Ipv4Addr::new(128, 0, 0, 0), 1)?)?;

                if *disable_ipv6 {
                    // WFP block at the ALE connect/recv-accept v6 layers under a
                    // dynamic session — filters auto-vanish if the daemon dies.
                    // Loopback (::1) is permitted via a higher-weight rule.
                    match WfpIpv6Block::install() {
                        Ok(block) => {
                            *self.ipv6_block.lock().unwrap_or_else(|e| e.into_inner()) = Some(block);
                            debug!("IPv6 traffic blocked via WFP");
                        }
                        Err(e) => warn!("Failed to install WFP IPv6 block: {e:#}"),
                    }
                }
            }
            RoutingConfig::Split { destination, routes } => {
                debug!("Configuring split routing via {}", self.device);
                self.add_host_exclusion(*destination)?;
                for route in routes {
                    self.add_route_via_tunnel(*route)?;
                }
            }
            RoutingConfig::Cleanup { .. } => {
                debug!("Cleaning up routing rules for {}", self.device);
                *self.ipv6_block.lock().unwrap_or_else(|e| e.into_inner()) = None;
                self.delete_all();
            }
        }
        Ok(())
    }
}

fn new_forward_row() -> MIB_IPFORWARD_ROW2 {
    let mut row = MIB_IPFORWARD_ROW2::default();
    unsafe { InitializeIpForwardEntry(&mut row) };
    row
}

fn luids_equal(a: NET_LUID_LH, b: NET_LUID_LH) -> bool {
    unsafe { a.Value == b.Value }
}
