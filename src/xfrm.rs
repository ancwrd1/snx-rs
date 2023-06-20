use std::{
    net::{IpAddr, Ipv4Addr},
    os::fd::AsRawFd,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::anyhow;
use tokio::sync::oneshot;
use tracing::{debug, trace};

use crate::{
    model::{ClientSettingsResponseData, IpsecKey, IpsecResponseData},
    params::TunnelParams,
    util,
};

const VTI_KEY: &str = "1000";
const VTI_NAME: &str = "snx-vti";
const MAX_ISAKMP_PROBES: usize = 5;
const UDP_ENCAP_ESPINUDP: libc::c_int = 2; // from /usr/include/linux/udp.h

const KEEPALIVE_PORT: u16 = 18234;
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(20);
const KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(5);
const KEEPALIVE_MAX_RETRIES: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum CommandType {
    Add,
    Delete,
}

impl CommandType {
    fn as_str(&self) -> &'static str {
        match self {
            CommandType::Add => "add",
            CommandType::Delete => "del",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PolicyDir {
    In,
    Out,
}

impl PolicyDir {
    fn as_str(&self) -> &'static str {
        match self {
            PolicyDir::In => "in",
            PolicyDir::Out => "out",
        }
    }
}

// picked from wireshark logs
fn make_keepalive_packet() -> [u8; 84] {
    let mut data = [0u8; 84];

    // 0x00000011 looks like a packet type, KEEPALIVE in this case
    data[0..4].copy_from_slice(&0x00000011u32.to_be_bytes());

    // 0x0001 is probably a direction: request or response. We get 0x0002 as a response back.
    data[4..6].copy_from_slice(&0x0001u16.to_be_bytes());

    // this looks like a content type, probably means TIMESTAMP
    data[6..8].copy_from_slice(&0x0002u16.to_be_bytes());

    // timestamp
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
    data[8..16].copy_from_slice(&timestamp.to_be_bytes());
    data
}

pub struct XfrmConfigurator {
    tunnel_params: Arc<TunnelParams>,
    ipsec_params: IpsecResponseData,
    client_settings: ClientSettingsResponseData,
    source_ip: Ipv4Addr,
    dest_ip: Ipv4Addr,
    stopper: Option<oneshot::Sender<()>>,
}

impl XfrmConfigurator {
    pub fn new(
        tunnel_params: Arc<TunnelParams>,
        ipsec_params: IpsecResponseData,
        client_settings: ClientSettingsResponseData,
    ) -> Self {
        Self {
            tunnel_params,
            ipsec_params,
            client_settings,
            source_ip: Ipv4Addr::new(0, 0, 0, 0),
            dest_ip: Ipv4Addr::new(0, 0, 0, 0),
            stopper: None,
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

    pub async fn cleanup(&mut self) {
        if let Some(stopper) = self.stopper.take() {
            let _ = stopper.send(());
        }
        let dst = self.dest_ip.to_string();

        let _ = self.iproute2(&["xfrm", "state", "flush"]).await;
        let _ = self.iproute2(&["xfrm", "policy", "flush"]).await;

        let _ = self.iproute2(&["link", "del", "name", VTI_NAME]).await;

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

    async fn configure_xfrm_state(
        &self,
        command: CommandType,
        src: &str,
        dst: &str,
        params: &IpsecKey,
    ) -> anyhow::Result<()> {
        let spi = format!("0x{:x}", params.spi);

        match command {
            CommandType::Add =>
            // out state
            {
                self.iproute2(&[
                    "xfrm",
                    "state",
                    command.as_str(),
                    "src",
                    src,
                    "dst",
                    dst,
                    "proto",
                    "esp",
                    "spi",
                    &spi,
                    "mode",
                    "tunnel",
                    "flag",
                    "af-unspec",
                    "auth-trunc",
                    "sha256",
                    &params.authkey,
                    "128",
                    "enc",
                    "aes",
                    &params.enckey,
                    "encap",
                    "espinudp",
                    "4500",
                    "4500",
                    "0.0.0.0",
                ])
                .await?;
            }
            CommandType::Delete => {
                self.iproute2(&["xfrm", "state", command.as_str(), "spi", &spi]).await?;
            }
        }

        Ok(())
    }

    async fn configure_xfrm_policy(
        &self,
        command: CommandType,
        dir: PolicyDir,
        src: &str,
        dst: &str,
    ) -> anyhow::Result<()> {
        match command {
            CommandType::Add => {
                self.iproute2(&[
                    "xfrm",
                    "policy",
                    command.as_str(),
                    "dir",
                    dir.as_str(),
                    "tmpl",
                    "src",
                    src,
                    "dst",
                    dst,
                    "proto",
                    "esp",
                    "mode",
                    "tunnel",
                    "mark",
                    VTI_KEY,
                ])
                .await?;
            }
            CommandType::Delete => {}
        }

        Ok(())
    }

    async fn setup_xfrm(&self) -> anyhow::Result<()> {
        let src = self.source_ip.to_string();
        let dst = self.dest_ip.to_string();

        self.configure_xfrm_state(CommandType::Add, &src, &dst, &self.ipsec_params.client_encsa)
            .await?;
        self.configure_xfrm_state(CommandType::Add, &dst, &src, &self.ipsec_params.client_decsa)
            .await?;

        self.configure_xfrm_policy(CommandType::Add, PolicyDir::Out, &src, &dst)
            .await?;
        self.configure_xfrm_policy(CommandType::Add, PolicyDir::In, &dst, &src)
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
    async fn start_udp_listener(&mut self) -> anyhow::Result<()> {
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

        let (tx, mut rx) = oneshot::channel();
        self.stopper = Some(tx);

        tokio::spawn(async move {
            debug!("Listening for NAT-T packets on port 4500");
            let mut buf = [0u8; 1024];

            loop {
                tokio::select! {
                    result = udp.recv_from(&mut buf) => {
                        if let Ok((size, from)) = result {
                            debug!("Received NON-ESP data from {}, length: {}", from, size);
                        }
                    }
                    _ = &mut rx => {
                        break;
                    }
                }
            }
        });
        Ok(())
    }

    pub async fn run_keepalive(&self) -> anyhow::Result<()> {
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

        let mut num_failures = 0;

        loop {
            trace!("Sending keepalive to {}", dst);

            let data = make_keepalive_packet();

            let send_fut = udp.send_to(&data, (dst, KEEPALIVE_PORT));

            let mut buf = [0u8; 128];
            let recv_fut = tokio::time::timeout(KEEPALIVE_TIMEOUT, udp.recv_from(&mut buf));

            let result = futures::future::join(send_fut, recv_fut).await;

            if let (Ok(_), Ok(Ok((size, _)))) = result {
                trace!("Received keepalive response from {}, size: {}", dst, size);
            } else {
                num_failures += 1;
                if num_failures >= KEEPALIVE_MAX_RETRIES {
                    break;
                }
            }

            tokio::time::sleep(KEEPALIVE_INTERVAL).await;
        }

        debug!("Keepalive failed!");

        Err(anyhow!("Keepalive failed!"))
    }
}
