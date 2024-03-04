use std::{net::Ipv4Addr, sync::Arc};

use bytes::Bytes;
use ipnet::Ipv4Net;
use isakmp::session::EspCryptMaterial;
use tracing::{debug, trace};

use crate::{
    model::{
        params::TunnelParams,
        proto::{AuthenticationAlgorithm, EncryptionAlgorithm},
        IpsecSession,
    },
    platform::{self, IpsecConfigurator},
    util,
};

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
    auth_key: Bytes,
    enc_alg: EncryptionAlgorithm,
    enc_key: Bytes,
}

impl XfrmState {
    async fn add(&self) -> anyhow::Result<()> {
        let authkey = format!("0x{}", hex::encode(&self.auth_key));
        let enckey = format!("0x{}", hex::encode(&self.enc_key));
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
    ipsec_session: IpsecSession,
    source_ip: Ipv4Addr,
    key: u32,
    src_port: u16,
    dest_ip: Ipv4Addr,
    subnets: Vec<Ipv4Net>,
}

impl XfrmConfigurator {
    pub async fn new(
        tunnel_params: Arc<TunnelParams>,
        ipsec_session: IpsecSession,
        xfrm_key: u32,
        src_port: u16,
        dest_ip: Ipv4Addr,
        subnets: Vec<Ipv4Net>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            tunnel_params,
            ipsec_session,
            source_ip: Ipv4Addr::new(0, 0, 0, 0),
            dest_ip,
            key: xfrm_key,
            src_port,
            subnets,
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
            address: self.ipsec_session.address,
            prefix: ipnet::ipv4_mask_to_prefix(self.ipsec_session.netmask).unwrap_or_default(),
            local: self.source_ip,
            remote: self.dest_ip,
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
        params: &EspCryptMaterial,
    ) -> anyhow::Result<()> {
        let state = XfrmState {
            src,
            dst,
            src_port: self.src_port,
            dst_port: 4500,
            spi: params.spi,
            auth_alg: AuthenticationAlgorithm::HmacSha256,
            auth_key: params.sk_a.clone(),
            enc_alg: EncryptionAlgorithm::Aes256Cbc,
            enc_key: params.sk_e.clone(),
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
            &self.ipsec_session.esp_out,
        )
        .await?;
        self.configure_xfrm_state(
            CommandType::Add,
            self.dest_ip,
            self.source_ip,
            &self.ipsec_session.esp_in,
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
            if self.tunnel_params.default_route {
                let _ = platform::add_default_route(self.vti_name(), self.ipsec_session.address).await;
            } else {
                let subnets = self
                    .subnets
                    .iter()
                    .chain(&self.tunnel_params.add_routes)
                    .filter(|s| !s.contains(&self.dest_ip))
                    .cloned()
                    .collect::<Vec<_>>();

                let _ = platform::add_routes(&subnets, self.vti_name(), self.ipsec_session.address).await;
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
            debug!("Adding acquired DNS suffixes: {:?}", self.ipsec_session.domains);
            debug!("Adding provided DNS suffixes: {:?}", self.tunnel_params.search_domains);
            let suffixes = self
                .ipsec_session
                .domains
                .iter()
                .map(|s| s.as_str())
                .chain(self.tunnel_params.search_domains.iter().map(|s| s.as_ref()))
                .filter(|&s| {
                    !self
                        .tunnel_params
                        .ignore_search_domains
                        .iter()
                        .any(|d| d.to_lowercase() == s.to_lowercase())
                });
            let _ = platform::add_dns_suffixes(suffixes, self.vti_name()).await;

            let servers = self.ipsec_session.dns.iter().map(|server| server.to_string());
            let _ = platform::add_dns_servers(servers, self.vti_name()).await;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl IpsecConfigurator for XfrmConfigurator {
    async fn configure(&mut self) -> anyhow::Result<()> {
        self.source_ip = platform::get_default_ip().await?.parse()?;
        debug!("Source IP: {}", self.source_ip);
        debug!("Target IP: {}", self.dest_ip);

        self.cleanup().await;
        self.setup_vti().await?;
        self.setup_xfrm().await?;
        self.setup_routing().await?;
        self.setup_dns().await?;

        Ok(())
    }

    async fn re_key(&mut self, session: &IpsecSession) -> anyhow::Result<()> {
        trace!(
            "Re-keying XFRM state with new session: IN: {:?}, OUT: {:?}",
            session.esp_in,
            session.esp_out
        );
        let _ = self
            .configure_xfrm_state(
                CommandType::Delete,
                self.source_ip,
                self.dest_ip,
                &self.ipsec_session.esp_out,
            )
            .await;

        let _ = self
            .configure_xfrm_state(
                CommandType::Delete,
                self.dest_ip,
                self.source_ip,
                &self.ipsec_session.esp_in,
            )
            .await;

        self.ipsec_session = session.clone();

        self.configure_xfrm_state(
            CommandType::Add,
            self.source_ip,
            self.dest_ip,
            &self.ipsec_session.esp_out,
        )
        .await?;
        self.configure_xfrm_state(
            CommandType::Add,
            self.dest_ip,
            self.source_ip,
            &self.ipsec_session.esp_in,
        )
        .await?;

        Ok(())
    }

    async fn cleanup(&mut self) {
        let _ = self
            .configure_xfrm_state(
                CommandType::Delete,
                self.source_ip,
                self.dest_ip,
                &self.ipsec_session.esp_out,
            )
            .await;

        let _ = self
            .configure_xfrm_state(
                CommandType::Delete,
                self.dest_ip,
                self.source_ip,
                &self.ipsec_session.esp_in,
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
    }
}
