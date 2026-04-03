use std::net::{IpAddr, Ipv4Addr};

use ipnet::Ipv4Net;
use isakmp::model::{EspAuthAlgorithm, EspCryptMaterial, TransformId};
use netlink_packet_xfrm::{
    constants::{
        IPPROTO_ESP, UDP_ENCAP_ESPINUDP, XFRM_MODE_TUNNEL, XFRM_POLICY_IN, XFRM_POLICY_OUT, XFRM_STATE_AF_UNSPEC,
    },
    nlas::UserTemplate,
};
use rand::random;
use rtnetlink::{LinkMessageBuilder, LinkXfrm};
use sysctl::{Ctl, Sysctl};
use tracing::{debug, trace};

use crate::{
    model::IpsecSession,
    platform::{IpsecConfigurator, NetworkInterface, Platform, PlatformAccess},
};

fn new_xfrm_connection() -> anyhow::Result<xfrmnetlink::Handle> {
    let (connection, handle, _) = xfrmnetlink::new_connection()?;
    tokio::spawn(connection);
    Ok(handle)
}

struct XfrmLink<'a> {
    name: &'a str,
    if_id: u32,
    address: Ipv4Net,
    mtu: u16,
    handle: rtnetlink::Handle,
}

impl<'a> XfrmLink<'a> {
    fn new(name: &'a str, if_id: u32, address: Ipv4Net, mtu: u16) -> anyhow::Result<Self> {
        let handle = super::new_netlink_connection()?;

        Ok(Self {
            name,
            if_id,
            address,
            mtu,
            handle,
        })
    }

    async fn add(&self) -> anyhow::Result<()> {
        let _ = self.delete().await;

        let msg = LinkMessageBuilder::<LinkXfrm>::new(self.name)
            .if_id(self.if_id)
            .mtu(self.mtu as _)
            .up()
            .build();

        self.handle.link().add(msg).execute().await?;

        let _ = Platform::get()
            .new_network_interface()
            .configure_device(self.name)
            .await;

        let opt = format!("net.ipv4.conf.{}.disable_policy", self.name);
        Ctl::new(&opt)?.set_value_string("1")?;

        let opt = format!("net.ipv4.conf.{}.rp_filter", self.name);
        Ctl::new(&opt)?.set_value_string("0")?;

        let opt = format!("net.ipv4.conf.{}.forwarding", self.name);
        Ctl::new(&opt)?.set_value_string("1")?;

        let index = super::resolve_device_index(&self.handle, self.name).await?;
        self.handle
            .address()
            .add(index, self.address.addr().into(), self.address.prefix_len())
            .execute()
            .await?;
        Ok(())
    }

    async fn delete(&self) -> anyhow::Result<()> {
        if let Ok(index) = super::resolve_device_index(&self.handle, self.name).await {
            self.handle.link().del(index).execute().await?;
        }
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
        let handle = new_xfrm_connection()?;
        let src: IpAddr = self.src.into();
        let dst: IpAddr = self.dst.into();
        let trunc_len = (self.params.auth_algorithm.hash_len() * 8) as u32;

        handle
            .state()
            .add(src, dst)
            .protocol(IPPROTO_ESP)
            .spi(self.params.spi)
            .mode(XFRM_MODE_TUNNEL)
            .flags(XFRM_STATE_AF_UNSPEC)
            .authentication_trunc(self.auth_alg_as_xfrm_name(), &self.params.sk_a.to_vec(), trunc_len)?
            .encryption(self.enc_alg_as_xfrm_name(), &self.params.sk_e.to_vec())?
            .ifid(self.if_id)
            .encapsulation(
                UDP_ENCAP_ESPINUDP,
                self.src_port,
                self.dest_port,
                Ipv4Addr::UNSPECIFIED.into(),
            )
            .execute()
            .await?;

        Ok(())
    }

    async fn delete(&self) -> anyhow::Result<()> {
        let handle = new_xfrm_connection()?;
        let src: IpAddr = self.src.into();
        let dst: IpAddr = self.dst.into();

        handle
            .state()
            .delete(src, dst)
            .protocol(IPPROTO_ESP)
            .spi(self.params.spi)
            .execute()
            .await?;

        Ok(())
    }
}

struct XfrmPolicy {
    dir: u8,
    src: Ipv4Addr,
    dst: Ipv4Addr,
    if_id: u32,
}

impl XfrmPolicy {
    async fn add(&self) -> anyhow::Result<()> {
        let handle = new_xfrm_connection()?;
        let src: IpAddr = self.src.into();
        let dst: IpAddr = self.dst.into();

        let mut tmpl = UserTemplate::default();
        tmpl.source(&src);
        tmpl.destination(&dst);
        tmpl.protocol(IPPROTO_ESP);
        tmpl.mode(XFRM_MODE_TUNNEL);

        handle
            .policy()
            .add(Ipv4Addr::UNSPECIFIED.into(), 0, Ipv4Addr::UNSPECIFIED.into(), 0)
            .direction(self.dir)
            .ifid(self.if_id)
            .add_template(tmpl)
            .execute()
            .await?;

        Ok(())
    }

    async fn delete(&self) -> anyhow::Result<()> {
        let handle = new_xfrm_connection()?;

        handle
            .policy()
            .delete(Ipv4Addr::UNSPECIFIED.into(), 0, Ipv4Addr::UNSPECIFIED.into(), 0)
            .direction(self.dir)
            .ifid(self.if_id)
            .execute()
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

pub struct XfrmConfigurator {
    name: String,
    ipsec_session: IpsecSession,
    source_ip: Ipv4Addr,
    if_id: u32,
    src_port: u16,
    dest_ip: Ipv4Addr,
    dest_port: u16,
    mtu: u16,
}

impl XfrmConfigurator {
    pub fn new(
        name: &str,
        ipsec_session: IpsecSession,
        src_port: u16,
        dest_ip: Ipv4Addr,
        dest_port: u16,
        mtu: u16,
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
            mtu,
        })
    }

    fn new_xfrm_link(&self) -> anyhow::Result<XfrmLink<'_>> {
        XfrmLink::new(&self.name, self.if_id, self.ipsec_session.ipv4net_address(), self.mtu)
    }

    async fn setup_xfrm_link(&self) -> anyhow::Result<()> {
        self.new_xfrm_link()?.add().await
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
        dir: u8,
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

        self.configure_xfrm_policy(CommandType::Add, XFRM_POLICY_OUT, self.source_ip, self.dest_ip)
            .await?;
        self.configure_xfrm_policy(CommandType::Add, XFRM_POLICY_IN, self.dest_ip, self.source_ip)
            .await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl IpsecConfigurator for XfrmConfigurator {
    async fn configure(&mut self) -> anyhow::Result<()> {
        self.source_ip = Platform::get().new_network_interface().get_default_ip().await?;
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

        let old_address = self.ipsec_session.ipv4net_address();
        let new_address = session.ipv4net_address();

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

        if old_address != new_address {
            debug!(
                "IP address changed from {} to {}, replacing it for device {}",
                old_address, new_address, self.name
            );
            Platform::get()
                .new_network_interface()
                .replace_ip_address(&self.name, old_address, new_address)
                .await?;
        }

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
            .configure_xfrm_policy(CommandType::Delete, XFRM_POLICY_OUT, self.source_ip, self.dest_ip)
            .await;

        let _ = self
            .configure_xfrm_policy(CommandType::Delete, XFRM_POLICY_IN, self.dest_ip, self.source_ip)
            .await;

        if let Ok(link) = self.new_xfrm_link() {
            let _ = link.delete().await;
        };
    }
}
