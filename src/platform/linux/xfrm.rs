use std::{
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
    time::Duration,
};

use anyhow::anyhow;
use tracing::debug;

use crate::{
    model::{
        params::TunnelParams,
        snx::{ClientSettingsResponseData, IpsecResponseData, IpsecSA},
    },
    platform::{IpsecConfigurator, UdpSocketExt},
    util,
};

const VTI_KEY: &str = "1000";
const VTI_NAME: &str = "snx-vti";
const MAX_ISAKMP_PROBES: usize = 5;

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

pub struct XfrmConfigurator {
    tunnel_params: Arc<TunnelParams>,
    ipsec_params: IpsecResponseData,
    client_settings: ClientSettingsResponseData,
    source_ip: Ipv4Addr,
    dest_ip: Ipv4Addr,
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
        }
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
        let udp = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
        udp.connect(format!("{}:4500", self.dest_ip)).await?;

        let data = vec![0u8; 32];

        let result = udp.send_receive(&data, Duration::from_secs(5)).await;

        match result {
            Ok(reply) if reply.len() == 32 => {
                let srcport: [u8; 4] = reply[8..12].try_into().unwrap();
                let dstport: [u8; 4] = reply[12..16].try_into().unwrap();
                debug!(
                    "Received isakmp reply from {}: srcport: {}, dstport: {}, hash: {}",
                    self.dest_ip,
                    u32::from_be_bytes(srcport),
                    u32::from_be_bytes(dstport),
                    hex::encode(&reply[reply.len() - 16..reply.len()])
                );
                Ok(())
            }
            _ => Err(anyhow!("No isakmp reply!")),
        }
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

        let _ = util::run_command("nmcli", ["device", "set", VTI_NAME, "managed", "no"]).await;

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
        params: &IpsecSA,
    ) -> anyhow::Result<()> {
        let spi = format!("0x{:x}", params.spi);

        match command {
            CommandType::Add =>
            // out state
            {
                let authkey = format!("0x{}", params.authkey.0);
                let enckey = format!("0x{}", params.enckey.0);
                let trunc_len = self.ipsec_params.authalg.trunc_length().to_string();

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
                    self.ipsec_params.authalg.as_xfrm_name(),
                    &authkey,
                    &trunc_len,
                    "enc",
                    self.ipsec_params.encalg.as_xfrm_name(),
                    &enckey,
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
                let _ = crate::platform::add_default_route(VTI_NAME, addr).await;
            } else {
                for range in &self.client_settings.updated_policies.range.settings {
                    crate::platform::add_route(range, VTI_NAME, addr).await?;
                }
            }
        }

        let port = TunnelParams::IPSEC_KEEPALIVE_PORT.to_string();
        let dst = self.dest_ip.to_string();

        // set up routing correctly so that keepalive packets are not wrapped into ESP
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
            debug!("Adding acquired DNS suffixes: {}", self.ipsec_params.om_domain_name.0);
            debug!("Adding provided DNS suffixes: {:?}", self.tunnel_params.search_domains);
            let suffixes = self
                .ipsec_params
                .om_domain_name
                .0
                .split(',')
                .chain(self.tunnel_params.search_domains.iter().map(|s| s.as_ref()));
            let _ = crate::platform::add_dns_suffixes(suffixes, VTI_NAME).await;

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

            let _ = crate::platform::add_dns_servers(servers, VTI_NAME).await;
        }
        Ok(())
    }

    async fn setup_iptables(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn cleanup_iptables(&self) {}
}

#[async_trait::async_trait]
impl IpsecConfigurator for XfrmConfigurator {
    async fn configure(&mut self) -> anyhow::Result<()> {
        self.source_ip = crate::platform::get_default_ip().await?.parse()?;
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

        Ok(())
    }

    async fn cleanup(&mut self) {
        let _ = self.iproute2(&["xfrm", "state", "flush"]).await;
        let _ = self.iproute2(&["xfrm", "policy", "flush"]).await;
        let _ = self.iproute2(&["link", "del", "name", VTI_NAME]).await;

        let dst = self.dest_ip.to_string();
        let port = TunnelParams::IPSEC_KEEPALIVE_PORT.to_string();

        let _ = self
            .iproute2(&[
                "rule", "del", "to", &dst, "ipproto", "udp", "dport", &port, "table", &port,
            ])
            .await;

        self.cleanup_iptables().await;
    }
}
