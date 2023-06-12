use std::{
    net::{IpAddr, Ipv4Addr},
    os::fd::AsRawFd,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::anyhow;
use tracing::{debug, trace};

use crate::{
    model::{ClientSettingsResponseData, IpsecResponseData},
    params::TunnelParams,
    util,
};

const VTI_KEY: &str = "1000";
const VTI_NAME: &str = "snx-vti";
const MAX_ISAKMP_PROBES: usize = 5;
const UDP_ENCAP_ESPINUDP: libc::c_int = 2; // from /usr/include/linux/udp.h
const KEEPALIVE_PORT: u16 = 18234;

// picked from wireshark logs
fn make_keepalive_packet() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend(0x11u32.to_be_bytes());
    data.extend(0x00010002u32.to_be_bytes());
    data.extend((SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64).to_be_bytes());
    data.extend((0..68).map(|_| 0u8));
    data
}

pub struct IpsecConfigurator {
    tunnel_params: TunnelParams,
    ipsec_params: IpsecResponseData,
    client_settings: ClientSettingsResponseData,
    source_ip: Ipv4Addr,
    dest_ip: Ipv4Addr,
}

impl IpsecConfigurator {
    pub fn new(
        tunnel_params: TunnelParams,
        ipsec_params: IpsecResponseData,
        client_settings: ClientSettingsResponseData,
    ) -> Self {
        Self {
            tunnel_params,
            ipsec_params,
            client_settings,
            source_ip: Ipv4Addr::new(0, 0, 0, 0),
            dest_ip: Ipv4Addr::new(0, 0, 0, 0),
        }
    }

    pub async fn configure(&mut self) -> anyhow::Result<()> {
        self.source_ip = crate::net::get_default_ip().await?.parse()?;
        debug!("Source IP: {}", self.source_ip);

        self.dest_ip = self.client_settings.gw_internal_ip.parse()?;
        debug!("Target IP: {}", self.dest_ip);

        self.cleanup().await;
        self.send_isakmp_probes().await?;
        self.setup_vti().await?;
        self.setup_iptables().await?;
        self.setup_xfrm().await?;
        self.setup_routing().await?;
        self.setup_dns().await?;
        self.start_udp_listener().await?;
        self.start_keepalive_task().await?;
        Ok(())
    }

    async fn send_isakmp_probes(&self) -> anyhow::Result<()> {
        for _ in 0..MAX_ISAKMP_PROBES {
            if self.send_isakmp_probe().await.is_ok() {
                return Ok(());
            }
        }
        Err(anyhow!("Probing failed, server is not reachable via ESPinUDP tunnel!"))
    }

    async fn send_isakmp_probe(&self) -> anyhow::Result<()> {
        debug!("Sending isakmp probe to {}", self.dest_ip);
        let udp = tokio::net::UdpSocket::bind("0.0.0.0:4500").await?;
        let data = vec![0u8; 32];

        let mut buf = [0u8; 256];

        let send_fut = udp.send_to(&data, (self.dest_ip.to_string(), 4500));
        let receive_fut = tokio::time::timeout(Duration::from_secs(5), udp.recv_from(&mut buf));

        let result = futures::future::join(send_fut, receive_fut).await;

        if let (Ok(_), Ok(Ok((size, _)))) = result {
            if size == 32 {
                let srcport: [u8; 4] = buf[8..12].try_into().unwrap();
                let dstport: [u8; 4] = buf[12..16].try_into().unwrap();
                debug!(
                    "Received isakmp reply from {}: srcport: {}, dstport: {}, hash: {}",
                    self.dest_ip,
                    u32::from_be_bytes(srcport),
                    u32::from_be_bytes(dstport),
                    hex::encode(&buf[size - 16..size])
                );
                Ok(())
            } else {
                Err(anyhow!("Invalid isakmp reply!"))
            }
        } else {
            Err(anyhow!("No isakmp reply!"))
        }
    }

    pub async fn cleanup(&self) {
        let _ = self.iproute2(&["xfrm", "state", "flush"]).await;
        let _ = self.iproute2(&["xfrm", "policy", "flush"]).await;
        let _ = self.iproute2(&["link", "del", "name", VTI_NAME]).await;

        let dst = self.dest_ip.to_string();
        let port = KEEPALIVE_PORT.to_string();

        let _ = self
            .iproute2(&[
                "rule", "del", "to", &dst, "ipproto", "udp", "dport", &port, "table", &port,
            ])
            .await;
        self.cleanup_iptables().await;
    }

    async fn iproute2(&self, args: &[&str]) -> anyhow::Result<String> {
        util::run_command("ip", args).await
    }

    async fn setup_vti(&self) -> anyhow::Result<()> {
        let src = self.source_ip.to_string();
        let dst = self.dest_ip.to_string();

        let _ = self.iproute2(&["tunnel", "del", "name", VTI_NAME]).await;

        self.iproute2(&[
            "tunnel", "add", VTI_NAME, "mode", "vti", "key", VTI_KEY, "local", &src, "remote", &dst,
        ])
        .await?;

        let opt = format!("net.ipv4.conf.{}.disable_policy=1", VTI_NAME);
        util::run_command("sysctl", ["-qw", &opt]).await?;

        let opt = format!("net.ipv4.conf.{}.rp_filter=0", VTI_NAME);
        util::run_command("sysctl", ["-qw", &opt]).await?;

        let opt = format!("net.ipv4.conf.{}.forwarding=1", VTI_NAME);
        util::run_command("sysctl", ["-qw", &opt]).await?;

        self.iproute2(&["link", "set", VTI_NAME, "up"]).await?;

        let addr: Ipv4Addr = self.ipsec_params.om_addr.into();
        let prefix = ipnet::ip_mask_to_prefix(IpAddr::from(self.ipsec_params.om_subnet_mask.to_be_bytes()))?;
        let addr = format!("{}/{}", addr, prefix);

        debug!("Tunnel address: {}", addr);

        self.iproute2(&["addr", "add", &addr, "dev", VTI_NAME]).await?;
        Ok(())
    }

    async fn setup_xfrm(&self) -> anyhow::Result<()> {
        let src = self.source_ip.to_string();
        let dst = self.dest_ip.to_string();

        let enc_params = self.ipsec_params.client_encsa.decode();
        let dec_params = self.ipsec_params.client_decsa.decode();

        let enc_spi = format!("0x{:x}", enc_params.spi);
        let dec_spi = format!("0x{:x}", dec_params.spi);

        // out state
        self.iproute2(&[
            "xfrm",
            "state",
            "add",
            "src",
            &src,
            "dst",
            &dst,
            "proto",
            "esp",
            "spi",
            &enc_spi,
            "mode",
            "tunnel",
            "flag",
            "af-unspec",
            "auth-trunc",
            "sha256",
            &enc_params.authkey,
            "128",
            "enc",
            "aes",
            &enc_params.enckey,
            "encap",
            "espinudp",
            "4500",
            "4500",
            "0.0.0.0",
        ])
        .await?;

        // in state
        self.iproute2(&[
            "xfrm",
            "state",
            "add",
            "src",
            &dst,
            "dst",
            &src,
            "proto",
            "esp",
            "spi",
            &dec_spi,
            "mode",
            "tunnel",
            "flag",
            "af-unspec",
            "auth-trunc",
            "sha256",
            &dec_params.authkey,
            "128",
            "enc",
            "aes",
            &dec_params.enckey,
            "encap",
            "espinudp",
            "4500",
            "4500",
            "0.0.0.0",
        ])
        .await?;

        self.iproute2(&[
            "xfrm", "policy", "add", "dir", "out", "tmpl", "src", &src, "dst", &dst, "proto", "esp", "mode", "tunnel",
            "mark", VTI_KEY,
        ])
        .await?;

        self.iproute2(&[
            "xfrm", "policy", "add", "dir", "in", "tmpl", "src", &dst, "dst", &src, "proto", "esp", "mode", "tunnel",
            "mark", VTI_KEY,
        ])
        .await?;

        Ok(())
    }

    async fn setup_routing(&self) -> anyhow::Result<()> {
        if !self.tunnel_params.no_routing {
            let addr: Ipv4Addr = self.ipsec_params.om_addr.into();
            if self.tunnel_params.default_route {
                let _ = crate::net::add_default_route(VTI_NAME, addr).await;
            } else {
                for range in &self.client_settings.updated_policies.range.settings {
                    crate::net::add_route(range, VTI_NAME, addr).await?;
                }
            }
        }

        let dst = self.dest_ip.to_string();
        let port = KEEPALIVE_PORT.to_string();

        // for tunnel keepalive
        self.iproute2(&["route", "add", "table", &port, &dst, "dev", "snx-vti"])
            .await?;
        self.iproute2(&[
            "rule", "add", "to", &dst, "ipproto", "udp", "dport", &port, "table", &port,
        ])
        .await?;

        Ok(())
    }

    async fn setup_dns(&self) -> anyhow::Result<()> {
        if !self.tunnel_params.no_dns {
            debug!("Adding acquired DNS suffixes: {}", self.ipsec_params.om_domain_name);
            debug!("Adding provided DNS suffixes: {:?}", self.tunnel_params.search_domains);
            let suffixes = self
                .ipsec_params
                .om_domain_name
                .trim_matches('"')
                .split(',')
                .chain(self.tunnel_params.search_domains.iter().map(|s| s.as_ref()));
            let _ = crate::net::add_dns_suffixes(suffixes, VTI_NAME).await;

            let dns_servers = [
                self.ipsec_params.om_dns0,
                self.ipsec_params.om_dns1,
                self.ipsec_params.om_dns2,
            ];

            let servers = dns_servers.into_iter().filter_map(|server| {
                if server != 0 {
                    let addr: Ipv4Addr = server.into();
                    debug!("Adding DNS server: {}", addr);
                    Some(addr.to_string())
                } else {
                    None
                }
            });

            let _ = crate::net::add_dns_servers(servers, VTI_NAME).await;
        }
        Ok(())
    }

    async fn setup_iptables(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn cleanup_iptables(&self) {}

    // without this listener automatic IPSec decapsulation from UDP 4500 does not work
    async fn start_udp_listener(&self) -> anyhow::Result<()> {
        let udp = tokio::net::UdpSocket::bind("0.0.0.0:4500").await?;
        let stype: libc::c_int = UDP_ENCAP_ESPINUDP;
        unsafe {
            let rc = libc::setsockopt(
                udp.as_raw_fd(),
                libc::SOL_UDP,
                libc::UDP_ENCAP,
                &stype as *const libc::c_int as _,
                std::mem::size_of::<libc::c_int>() as _,
            );
            if rc != 0 {
                return Err(anyhow!("Cannot set UDP_ENCAP socket option!"));
            }
        }

        tokio::spawn(async move {
            debug!("Listening for NAT-T packets on port 4500");
            let mut buf = [0u8; 1024];
            while let Ok((size, from)) = udp.recv_from(&mut buf).await {
                debug!("Received NON-ESP data from {}, length: {}", from, size);
            }
        });
        Ok(())
    }

    async fn start_keepalive_task(&self) -> anyhow::Result<()> {
        let src: Ipv4Addr = self.ipsec_params.om_addr.into();
        let dst = self.dest_ip;
        let udp = tokio::net::UdpSocket::bind((src, KEEPALIVE_PORT)).await?;

        // disable UDP checksum validation for incoming packets.
        // Checkpoint gateway doesn't set it correctly.
        let disable: libc::c_int = 1;
        unsafe {
            let rc = libc::setsockopt(
                udp.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_NO_CHECK,
                &disable as *const libc::c_int as _,
                std::mem::size_of::<libc::c_int>() as _,
            );
            if rc != 0 {
                return Err(anyhow!("Cannot set SO_NO_CHECK socket option!"));
            }
        }

        tokio::spawn(async move {
            loop {
                trace!("Sending keepalive to {}", dst);

                let data = make_keepalive_packet();

                let send_fut = udp.send_to(&data, (dst, KEEPALIVE_PORT));

                let mut buf = [0u8; 1024];
                let recv_fut = tokio::time::timeout(Duration::from_secs(5), udp.recv_from(&mut buf));

                let result = futures::future::join(send_fut, recv_fut).await;

                if let (Ok(_), Ok(Ok((size, _)))) = result {
                    trace!("Received keepalive response from {}, size: {}", dst, size);
                } else {
                    // warn!("Keepalive failed, exiting");
                    // unsafe {
                    //     libc::kill(
                    //         libc::getpid(),
                    //         tokio::signal::unix::SignalKind::terminate().as_raw_value(),
                    //     );
                    //     break;
                    // }
                }

                tokio::time::sleep(Duration::from_secs(20)).await;
            }
        });

        Ok(())
    }
}
