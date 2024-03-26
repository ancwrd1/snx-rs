use std::net::Ipv4Addr;
use std::{sync::Arc, time::Duration};

use anyhow::anyhow;
use ipnet::Ipv4Net;
use tokio::net::UdpSocket;

#[cfg(target_os = "linux")]
use linux as platform_impl;
pub use platform_impl::{
    acquire_password, init_theme_monitoring,
    net::{
        add_default_route, add_dns_servers, add_dns_suffixes, add_route, add_routes, get_default_ip, is_online,
        poll_online, start_network_state_monitoring,
    },
    new_tun_config, send_notification, store_password, system_color_theme, IpsecImpl, SingleInstance,
};

use crate::model::{params::TunnelParams, IpsecSession};

#[cfg(target_os = "linux")]
mod linux;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SystemColorTheme {
    #[default]
    NoPreference,
    Light,
    Dark,
}

impl TryFrom<u32> for SystemColorTheme {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(SystemColorTheme::NoPreference),
            1 => Ok(SystemColorTheme::Dark),
            2 => Ok(SystemColorTheme::Light),
            _ => Err(anyhow!("Unknown color-scheme value")),
        }
    }
}

#[async_trait::async_trait]
pub trait IpsecConfigurator {
    async fn configure(&mut self) -> anyhow::Result<()>;
    async fn re_key(&mut self, session: &IpsecSession) -> anyhow::Result<()>;
    async fn cleanup(&mut self);
}

pub async fn new_ipsec_configurator(
    tunnel_params: Arc<TunnelParams>,
    ipsec_session: IpsecSession,
    src_port: u16,
    dest_ip: Ipv4Addr,
    subnets: Vec<Ipv4Net>,
) -> anyhow::Result<impl IpsecConfigurator> {
    IpsecImpl::new(tunnel_params, ipsec_session, src_port, dest_ip, subnets).await
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub enum UdpEncap {
    EspInUdp,
}

#[async_trait::async_trait]
pub trait UdpSocketExt {
    fn set_encap(&self, encap: UdpEncap) -> anyhow::Result<()>;
    fn set_no_check(&self, flag: bool) -> anyhow::Result<()>;
    async fn send_receive(&self, data: &[u8], timeout: Duration) -> anyhow::Result<Vec<u8>>;
}

async fn udp_send_receive(socket: &UdpSocket, data: &[u8], timeout: Duration) -> anyhow::Result<Vec<u8>> {
    let mut buf = [0u8; 65536];

    let send_fut = socket.send(data);
    let recv_fut = tokio::time::timeout(timeout, socket.recv_from(&mut buf));

    let result = futures::future::join(send_fut, recv_fut).await;

    if let (Ok(_), Ok(Ok((size, _)))) = result {
        Ok(buf[0..size].to_vec())
    } else {
        Err(anyhow!("Error sending UDP request!"))
    }
}
