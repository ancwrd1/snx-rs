use std::net::Ipv4Addr;

use ipnet::Ipv4Net;
use isakmp::model::{EspAuthAlgorithm, EspCryptMaterial, TransformId};
use rand::random;
use tracing::{debug, trace};

use crate::{
    model::{IpsecSession, params::TunnelParams},
    platform::{self, IpsecConfigurator, NetworkInterface, RoutingConfigurator},
    util,
};

async fn iproute2(args: &[&str]) -> anyhow::Result<String> {
    util::run_command("ip", args).await
}

struct XfrmLink<'a> {
    name: &'a str,
    if_id: u32,
    address: Ipv4Net,
}

impl XfrmLink<'_> {
    async fn add(&self) -> anyhow::Result<()> {
        let _ = self.delete().await;

        iproute2(&[
            "link",
            "add",
            self.name,
            "type",
            "xfrm",
            "if_id",
            &self.if_id.to_string(),
        ])
        .await?;

        let _ = platform::new_network_interface().configure_device(self.name).await;

        let opt = format!("net.ipv4.conf.{}.disable_policy=1", self.name);
        util::run_command("sysctl", ["-qw", &opt]).await?;

        let opt = format!("net.ipv4.conf.{}.rp_filter=0", self.name);
        util::run_command("sysctl", ["-qw", &opt]).await?;

        let opt = format!("net.ipv4.conf.{}.forwarding=1", self.name);
        util::run_command("sysctl", ["-qw", &opt]).await?;

        iproute2(&[
            "link",
            "set",
            self.name,
            "mtu",
            &TunnelParams::DEFAULT_MTU.to_string(),
            "up",
        ])
        .await?;

        self.add_address().await?;

        Ok(())
    }

    async fn add_address(&self) -> anyhow::Result<()> {
        iproute2(&["addr", "add", &self.address.to_string(), "dev", self.name]).await?;

        Ok(())
    }

    async fn delete_address(&self, address: Ipv4Net) -> anyhow::Result<()> {
        iproute2(&["addr", "del", &address.to_string(), "dev", self.name]).await?;

        Ok(())
    }

    async fn delete(&self) -> anyhow::Result<()> {
        iproute2(&["link", "del", "name", self.name]).await?;
        Ok(())
    }
}

struct XfrmState<'a> {
    src: Ipv4Addr,
    dst: Ipv4Addr,
    src_port: u16,
    dest_port: u16,
    if_id: u32,
    params: &'a EspCryptMaterial,
}

impl XfrmState<'_> {
    fn auth_alg_as_xfrm_name(&self) -> &'static str {
        match self.params.auth_algorithm {
            EspAuthAlgorithm::HmacSha96 | EspAuthAlgorithm::HmacSha160 => "hmac(sha1)",
            EspAuthAlgorithm::HmacSha256 | EspAuthAlgorithm::HmacSha256v2 => "hmac(sha256)",
            EspAuthAlgorithm::Other(_) => "",
        }
    }
    fn enc_alg_as_xfrm_name(&self) -> &'static str {
        match self.params.transform_id {
            TransformId::EspAesCbc => "cbc(aes)",
            TransformId::Esp3Des => "cbc(des3_ede)",
            _ => "",
        }
    }

    async fn add(&self) -> anyhow::Result<()> {
        let authkey = format!("0x{}", hex::encode(&self.params.sk_a));
        let enckey = format!("0x{}", hex::encode(&self.params.sk_e));
        let trunc_len = (self.params.auth_algorithm.hash_len() * 8).to_string();

        let spi = format!("0x{:x}", self.params.spi);
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
            self.auth_alg_as_xfrm_name(),
            &authkey,
            &trunc_len,
            "enc",
            self.enc_alg_as_xfrm_name(),
            &enckey,
            "if_id",
            &self.if_id.to_string(),
            "encap",
            "espinudp",
            &self.src_port.to_string(),
            &self.dest_port.to_string(),
            "0.0.0.0",
        ])
        .await?;

        Ok(())
    }

    async fn delete(&self) -> anyhow::Result<()> {
        let src = self.src.to_string();
        let dst = self.dst.to_string();
        let spi = format!("0x{:x}", self.params.spi);

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
    if_id: u32,
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
            "if_id",
            &self.if_id.to_string(),
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
            "if_id",
            &self.if_id.to_string(),
            "src",
            "0.0.0.0/0",
            "dst",
            "0.0.0.0/0",
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
    fn as_str(self) -> &'static str {
        match self {
            PolicyDir::In => "in",
            PolicyDir::Out => "out",
        }
    }
}

pub struct XfrmConfigurator {
    name: String,
    ipsec_session: IpsecSession,
    source_ip: Ipv4Addr,
    if_id: u32,
    src_port: u16,
    dest_ip: Ipv4Addr,
    dest_port: u16,
}

impl XfrmConfigurator {
    pub fn new(
        name: &str,
        ipsec_session: IpsecSession,
        src_port: u16,
        dest_ip: Ipv4Addr,
        dest_port: u16,
    ) -> anyhow::Result<Self> {
        let if_id = random();

        Ok(Self {
            name: name.to_owned(),
            ipsec_session,
            source_ip: Ipv4Addr::new(0, 0, 0, 0),
            dest_ip,
            if_id,
            src_port,
            dest_port,
        })
    }

    fn new_xfrm_link(&self) -> XfrmLink {
        XfrmLink {
            name: &self.name,
            if_id: self.if_id,
            address: Ipv4Net::with_netmask(self.ipsec_session.address, self.ipsec_session.netmask)
                .unwrap_or_else(|_| Ipv4Net::from(self.ipsec_session.address)),
        }
    }

    async fn setup_xfrm_link(&self) -> anyhow::Result<()> {
        self.new_xfrm_link().add().await
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
            dest_port: self.dest_port,
            if_id: self.if_id,
            params,
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
            if_id: self.if_id,
        };
        match command {
            CommandType::Add => policy.add().await?,
            CommandType::Delete => policy.delete().await?,
        }

        Ok(())
    }

    async fn setup_xfrm_state_and_policies(&self) -> anyhow::Result<()> {
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
}

#[async_trait::async_trait]
impl IpsecConfigurator for XfrmConfigurator {
    async fn configure(&mut self) -> anyhow::Result<()> {
        self.source_ip = platform::new_network_interface().get_default_ip().await?;
        debug!("Source IP: {}", self.source_ip);
        debug!("Target IP: {}", self.dest_ip);

        self.cleanup().await;
        self.setup_xfrm_link().await?;
        self.setup_xfrm_state_and_policies().await?;

        Ok(())
    }

    async fn rekey(&mut self, session: &IpsecSession) -> anyhow::Result<()> {
        trace!(
            "Rekeying XFRM state with new session: IN: {:?}, OUT: {:?}",
            session.esp_in, session.esp_out
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

        let old_address = self.new_xfrm_link().address;

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

        let link = self.new_xfrm_link();

        if old_address != link.address {
            link.add_address().await?;
            link.delete_address(old_address).await?;
        }

        Ok(())
    }

    async fn cleanup(&mut self) {
        let configurator = platform::new_routing_configurator(&self.name, self.ipsec_session.address);

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

        let _ = self.new_xfrm_link().delete().await;

        let _ = configurator.remove_keepalive_route(self.dest_ip).await;
        let _ = configurator.remove_default_route(self.dest_ip).await;
    }
}
