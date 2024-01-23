use std::{
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
    time::Duration,
};

use anyhow::anyhow;
use tokio::net::UdpSocket;
use tracing::debug;

use crate::{
    model::{
        params::TunnelParams,
        proto::{ClientSettingsResponse, IpsecSA, KeyManagementResponse},
        wrappers::HexKey,
        AuthenticationAlgorithm, EncryptionAlgorithm,
    },
    platform::{self, IpsecConfigurator, UdpSocketExt},
    util,
};

const MAX_ISAKMP_PROBES: usize = 5;

async fn iproute2(args: &[&str]) -> anyhow::Result<String> {
    util::run_command("ip", args).await
}

struct VtiDevice {
    name: String,
    key: u32,
    address: Ipv4Addr,
    prefix: u8,
    local: Ipv4Addr,
    remote: Ipv4Addr,
}

impl VtiDevice {
    async fn add(&self) -> anyhow::Result<()> {
        let _ = self.delete().await;

        iproute2(&[
            "tunnel",
            "add",
            &self.name,
            "mode",
            "vti",
            "key",
            &self.key.to_string(),
            "local",
            &self.local.to_string(),
            "remote",
            &self.remote.to_string(),
        ])
        .await?;

        let _ = util::run_command("nmcli", ["device", "set", &self.name, "managed", "no"]).await;

        let opt = format!("net.ipv4.conf.{}.disable_policy=1", &self.name);
        util::run_command("sysctl", ["-qw", &opt]).await?;

        let opt = format!("net.ipv4.conf.{}.rp_filter=0", &self.name);
        util::run_command("sysctl", ["-qw", &opt]).await?;

        let opt = format!("net.ipv4.conf.{}.forwarding=1", &self.name);
        util::run_command("sysctl", ["-qw", &opt]).await?;

        iproute2(&["link", "set", &self.name, "up"]).await?;

        iproute2(&[
            "addr",
            "add",
            &format!("{}/{}", self.address, self.prefix),
            "dev",
            &self.name,
        ])
        .await?;

        Ok(())
    }

    async fn delete(&self) -> anyhow::Result<()> {
        iproute2(&["link", "del", "name", &self.name]).await?;
        Ok(())
    }
}

struct XfrmState {
    src: Ipv4Addr,
    dst: Ipv4Addr,
    src_port: u16,
    dst_port: u16,
    spi: u32,
    auth_alg: AuthenticationAlgorithm,
    auth_key: HexKey,
    enc_alg: EncryptionAlgorithm,
    enc_key: HexKey,
}

impl XfrmState {
    async fn add(&self) -> anyhow::Result<()> {
        let authkey = format!("0x{}", self.auth_key.0);
        let enckey = format!("0x{}", self.enc_key.0);
        let trunc_len = self.auth_alg.trunc_length().to_string();

        let spi = format!("0x{:x}", self.spi);
        let src = self.src.to_string();
        let dst = self.dst.to_string();

        iproute2(&[
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
            &spi,
            "mode",
            "tunnel",
            "flag",
            "af-unspec",
            "auth-trunc",
            self.auth_alg.as_xfrm_name(),
            &authkey,
            &trunc_len,
            "enc",
            self.enc_alg.as_xfrm_name(),
            &enckey,
            "encap",
            "espinudp",
            &self.src_port.to_string(),
            &self.dst_port.to_string(),
            "0.0.0.0",
        ])
        .await?;

        Ok(())
    }

    async fn delete(&self) -> anyhow::Result<()> {
        let src = self.src.to_string();
        let dst = self.dst.to_string();
        let spi = format!("0x{:x}", self.spi);

        iproute2(&[
            "xfrm", "state", "del", "src", &src, "dst", &dst, "proto", "esp", "spi", &spi,
        ])
        .await?;

        Ok(())
    }
}

struct XfrmPolicy {
    dir: PolicyDir,
    src: Ipv4Addr,
    dst: Ipv4Addr,
    mark: u32,
    index: u32,
}

impl XfrmPolicy {
    async fn add(&self) -> anyhow::Result<()> {
        iproute2(&[
            "xfrm",
            "policy",
            "add",
            "dir",
            self.dir.as_str(),
            "tmpl",
            "src",
            &self.src.to_string(),
            "dst",
            &self.dst.to_string(),
            "proto",
            "esp",
            "mode",
            "tunnel",
            "mark",
            &self.mark.to_string(),
            "index",
            &self.index.to_string(),
        ])
        .await?;

        Ok(())
    }

    async fn delete(&self) -> anyhow::Result<()> {
        iproute2(&[
            "xfrm",
            "policy",
            "del",
            "dir",
            self.dir.as_str(),
            "index",
            &self.index.to_string(),
            "mark",
            &self.mark.to_string(),
        ])
        .await?;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum CommandType {
    Add,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PolicyDir {
    In,
    Out,
}

impl PolicyDir {
    fn as_u32(&self) -> u32 {
        match self {
            PolicyDir::In => 0,
            PolicyDir::Out => 1,
        }
    }
    fn as_str(&self) -> &'static str {
        match self {
            PolicyDir::In => "in",
            PolicyDir::Out => "out",
        }
    }
}

pub struct XfrmConfigurator {
    tunnel_params: Arc<TunnelParams>,
    ipsec_params: KeyManagementResponse,
    client_settings: ClientSettingsResponse,
    source_ip: Ipv4Addr,
    dest_ip: Ipv4Addr,
    key: u32,
    decap_socket: Arc<UdpSocket>,
}

impl XfrmConfigurator {
    pub async fn new(
        tunnel_params: Arc<TunnelParams>,
        ipsec_params: KeyManagementResponse,
        client_settings: ClientSettingsResponse,
        xfrm_key: u32,
    ) -> anyhow::Result<Self> {
        let udp = UdpSocket::bind("0.0.0.0:0").await?;

        Ok(Self {
            tunnel_params,
            ipsec_params,
            client_settings,
            source_ip: Ipv4Addr::new(0, 0, 0, 0),
            dest_ip: Ipv4Addr::new(0, 0, 0, 0),
            key: xfrm_key,
            decap_socket: Arc::new(udp),
        })
    }

    fn vti_name(&self) -> &str {
        self.tunnel_params
            .if_name
            .as_deref()
            .unwrap_or(TunnelParams::DEFAULT_IF_NAME)
    }

    fn new_vti_device(&self) -> VtiDevice {
        VtiDevice {
            name: self.vti_name().to_owned(),
            key: self.key,
            address: self.ipsec_params.om_addr.into(),
            prefix: ipnet::ip_mask_to_prefix(IpAddr::from(self.ipsec_params.om_subnet_mask.to_be_bytes()))
                .unwrap_or_default(),
            local: self.source_ip,
            remote: self.dest_ip,
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
        let udp = UdpSocket::bind("0.0.0.0:0").await?;
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

    async fn setup_vti(&self) -> anyhow::Result<()> {
        let device = self.new_vti_device();

        device.add().await
    }

    async fn configure_xfrm_state(
        &self,
        command: CommandType,
        src: Ipv4Addr,
        dst: Ipv4Addr,
        params: &IpsecSA,
    ) -> anyhow::Result<()> {
        let state = XfrmState {
            src,
            dst,
            src_port: self.decap_socket.local_addr()?.port(),
            dst_port: 4500,
            spi: params.spi,
            auth_alg: self.ipsec_params.authalg,
            auth_key: params.authkey.clone(),
            enc_alg: self.ipsec_params.encalg,
            enc_key: params.enckey.clone(),
        };
        match command {
            CommandType::Add => state.add().await?,
            CommandType::Delete => state.delete().await?,
        }

        Ok(())
    }

    async fn configure_xfrm_policy(
        &self,
        command: CommandType,
        dir: PolicyDir,
        src: Ipv4Addr,
        dst: Ipv4Addr,
    ) -> anyhow::Result<()> {
        let policy = XfrmPolicy {
            dir,
            src,
            dst,
            mark: self.key,
            // Weird undocumented Linux xfrm stuff: the lowest byte of the index must be equal to direction (0 = in, 1 = out)
            index: (self.key << 8) | dir.as_u32(),
        };
        match command {
            CommandType::Add => policy.add().await?,
            CommandType::Delete => policy.delete().await?,
        }

        Ok(())
    }

    async fn setup_xfrm(&self) -> anyhow::Result<()> {
        self.configure_xfrm_state(
            CommandType::Add,
            self.source_ip,
            self.dest_ip,
            &self.ipsec_params.client_encsa,
        )
        .await?;
        self.configure_xfrm_state(
            CommandType::Add,
            self.dest_ip,
            self.source_ip,
            &self.ipsec_params.client_decsa,
        )
        .await?;

        self.configure_xfrm_policy(CommandType::Add, PolicyDir::Out, self.source_ip, self.dest_ip)
            .await?;
        self.configure_xfrm_policy(CommandType::Add, PolicyDir::In, self.dest_ip, self.source_ip)
            .await?;

        Ok(())
    }

    async fn setup_routing(&self) -> anyhow::Result<()> {
        if !self.tunnel_params.no_routing {
            let addr: Ipv4Addr = self.ipsec_params.om_addr.into();
            if self.tunnel_params.default_route {
                let _ = platform::add_default_route(self.vti_name(), addr).await;
            } else {
                debug!("Ignoring acquired routes to {}", self.dest_ip);
                let subnets = util::ranges_to_subnets(&self.client_settings.updated_policies.range.settings)
                    .chain(self.tunnel_params.add_routes.clone())
                    .filter(|s| s.addr() != self.dest_ip)
                    .collect::<Vec<_>>();

                let _ = platform::add_routes(&subnets, self.vti_name(), addr).await;
            }
        }

        let port = TunnelParams::IPSEC_KEEPALIVE_PORT.to_string();
        let dst = self.dest_ip.to_string();

        // set up routing correctly so that keepalive packets are not wrapped into ESP
        iproute2(&["route", "add", "table", &port, &dst, "dev", self.vti_name()]).await?;

        iproute2(&[
            "rule", "add", "to", &dst, "ipproto", "udp", "dport", &port, "table", &port,
        ])
        .await?;

        Ok(())
    }

    async fn setup_dns(&self) -> anyhow::Result<()> {
        if !self.tunnel_params.no_dns {
            debug!("Adding acquired DNS suffixes: {:?}", self.ipsec_params.om_domain_name);
            debug!("Adding provided DNS suffixes: {:?}", self.tunnel_params.search_domains);
            let suffixes = self
                .ipsec_params
                .om_domain_name
                .as_ref()
                .map(|s| s.0.as_str())
                .unwrap_or_default()
                .split(',')
                .chain(self.tunnel_params.search_domains.iter().map(|s| s.as_ref()))
                .filter(|&s| {
                    !self
                        .tunnel_params
                        .ignore_search_domains
                        .iter()
                        .any(|d| d.to_lowercase() == s.to_lowercase())
                });
            let _ = crate::platform::add_dns_suffixes(suffixes, self.vti_name()).await;

            let dns_servers = [
                self.ipsec_params.om_dns0,
                self.ipsec_params.om_dns1,
                self.ipsec_params.om_dns2,
            ];

            let servers = dns_servers.into_iter().flatten().filter_map(|server| {
                if server != 0 {
                    let addr: Ipv4Addr = server.into();
                    debug!("Adding DNS server: {}", addr);
                    Some(addr.to_string())
                } else {
                    None
                }
            });

            let _ = crate::platform::add_dns_servers(servers, self.vti_name()).await;
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
        let _ = self
            .configure_xfrm_state(
                CommandType::Delete,
                self.source_ip,
                self.dest_ip,
                &self.ipsec_params.client_encsa,
            )
            .await;

        let _ = self
            .configure_xfrm_state(
                CommandType::Delete,
                self.dest_ip,
                self.source_ip,
                &self.ipsec_params.client_decsa,
            )
            .await;

        let _ = self
            .configure_xfrm_policy(CommandType::Delete, PolicyDir::Out, self.source_ip, self.dest_ip)
            .await;

        let _ = self
            .configure_xfrm_policy(CommandType::Delete, PolicyDir::In, self.dest_ip, self.source_ip)
            .await;

        let device = self.new_vti_device();

        let _ = device.delete().await;

        let dst = self.dest_ip.to_string();
        let port = TunnelParams::IPSEC_KEEPALIVE_PORT.to_string();

        let _ = iproute2(&[
            "rule", "del", "to", &dst, "ipproto", "udp", "dport", &port, "table", &port,
        ])
        .await;

        self.cleanup_iptables().await;
    }

    fn decap_socket(&self) -> Arc<UdpSocket> {
        self.decap_socket.clone()
    }
}
