use std::{sync::Arc, time::Duration};

use anyhow::anyhow;
use tokio::net::UdpSocket;

#[cfg(target_os = "linux")]
use linux as platform_impl;

pub use platform_impl::{
    acquire_password,
    net::{
        add_default_route, add_dns_servers, add_dns_suffixes, add_route, get_default_ip, is_online, poll_online,
        start_network_state_monitoring,
    },
    IpsecImpl,
};

use crate::model::{
    params::TunnelParams,
    proto::{ClientSettingsResponse, KeyManagementResponse},
};

#[cfg(target_os = "linux")]
pub mod linux;

#[async_trait::async_trait]
pub trait IpsecConfigurator {
    async fn configure(&mut self) -> anyhow::Result<()>;
    async fn cleanup(&mut self);
    fn decap_socket(&self) -> Arc<UdpSocket>;
}

pub async fn new_ipsec_configurator(
    tunnel_params: Arc<TunnelParams>,
    ipsec_params: KeyManagementResponse,
    client_settings: ClientSettingsResponse,
    vti_name: &str,
    key: u32,
) -> anyhow::Result<impl IpsecConfigurator> {
    IpsecImpl::new(tunnel_params, ipsec_params, client_settings, vti_name, key).await
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
