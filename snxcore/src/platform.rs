use std::{
    marker::PhantomData,
    net::{Ipv4Addr, SocketAddr},
    time::Duration,
};

use anyhow::anyhow;
use async_trait::async_trait;
use ipnet::Ipv4Net;
#[cfg(target_os = "linux")]
use linux::LinuxPlatformAccess as PlatformAccessImpl;
use tokio::net::UdpSocket;
use uuid::Uuid;

use crate::model::IpsecSession;

#[cfg(target_os = "linux")]
mod linux;

#[derive(Debug, Clone, PartialEq)]
pub struct PlatformFeatures {
    pub ipsec_native: bool,
    pub ipsec_keepalive: bool,
    pub split_dns: bool,
}

#[async_trait]
pub trait IpsecConfigurator {
    async fn configure(&mut self) -> anyhow::Result<()>;
    async fn rekey(&mut self, session: &IpsecSession) -> anyhow::Result<()>;
    async fn cleanup(&mut self);
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub enum UdpEncap {
    EspInUdp,
}

#[async_trait]
pub trait UdpSocketExt {
    fn set_encap(&self, encap: UdpEncap) -> anyhow::Result<()>;
    fn set_no_check(&self, flag: bool) -> anyhow::Result<()>;
    async fn send_receive(&self, data: &[u8], timeout: Duration, target: SocketAddr) -> anyhow::Result<Vec<u8>>;
}

async fn udp_send_receive(
    socket: &UdpSocket,
    data: &[u8],
    timeout: Duration,
    target: SocketAddr,
) -> anyhow::Result<Vec<u8>> {
    let mut buf = vec![0u8; 65536];

    let send_fut = socket.send_to(data, target);
    let recv_fut = tokio::time::timeout(timeout, socket.recv_from(&mut buf));

    let result = futures::future::join(send_fut, recv_fut).await;

    if let (Ok(_), Ok(Ok((size, _)))) = result {
        Ok(buf[0..size].to_vec())
    } else {
        Err(anyhow!(i18n::tr!("error-udp-request-failed")))
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResolverConfig {
    pub search_domains: Vec<String>,
    pub dns_servers: Vec<Ipv4Addr>,
}

#[async_trait]
pub trait ResolverConfigurator {
    async fn configure(&self, config: &ResolverConfig) -> anyhow::Result<()>;
    async fn cleanup(&self, config: &ResolverConfig) -> anyhow::Result<()>;
}

#[async_trait]
pub trait Keychain {
    async fn acquire_password(&self, username: &str) -> anyhow::Result<String>;
    async fn store_password(&self, username: &str, password: &str) -> anyhow::Result<()>;
}

#[async_trait]
pub trait RoutingConfigurator {
    async fn add_routes(&self, routes: &[Ipv4Net], ignore_routes: &[Ipv4Net]) -> anyhow::Result<()>;
    async fn setup_default_route(&self, destination: Ipv4Addr) -> anyhow::Result<()>;
    async fn setup_keepalive_route(&self, destination: Ipv4Addr, with_table: bool) -> anyhow::Result<()>;
    async fn remove_default_route(&self, destination: Ipv4Addr) -> anyhow::Result<()>;
    async fn remove_keepalive_route(&self, destination: Ipv4Addr) -> anyhow::Result<()>;
}

#[async_trait]
pub trait NetworkInterface {
    async fn start_network_state_monitoring(&self) -> anyhow::Result<()>;
    async fn get_default_ip(&self) -> anyhow::Result<Ipv4Addr>;
    async fn delete_device(&self, device_name: &str) -> anyhow::Result<()>;
    async fn configure_device(&self, device_name: &str) -> anyhow::Result<()>;
    async fn replace_ip_address(
        &self,
        device_name: &str,
        old_address: Ipv4Net,
        new_address: Ipv4Net,
    ) -> anyhow::Result<()>;

    fn is_online(&self) -> bool;
    fn poll_online(&self);
}

pub trait SingleInstance {
    fn is_single(&self) -> bool;
}

pub struct Platform(PhantomData<()>);

impl Platform {
    pub fn get() -> impl PlatformAccess {
        PlatformAccessImpl
    }
}

pub trait PlatformAccess {
    fn get_features(&self) -> PlatformFeatures;
    fn new_resolver_configurator<S: AsRef<str>>(
        &self,
        device: S,
    ) -> anyhow::Result<Box<dyn ResolverConfigurator + Send + Sync>>;
    fn new_keychain(&self) -> impl Keychain + Send + Sync;
    fn get_machine_uuid(&self) -> anyhow::Result<Uuid>;
    fn init(&self);
    fn new_ipsec_configurator(
        &self,
        name: &str,
        ipsec_session: IpsecSession,
        src_port: u16,
        dest_ip: Ipv4Addr,
        dest_port: u16,
    ) -> anyhow::Result<impl IpsecConfigurator + use<Self> + Send + Sync>;
    fn new_routing_configurator<S: AsRef<str>>(
        &self,
        device: S,
        address: Ipv4Addr,
    ) -> impl RoutingConfigurator + Send + Sync;
    fn new_network_interface(&self) -> impl NetworkInterface + Send + Sync;
    fn new_single_instance<S: AsRef<str>>(&self, name: S) -> anyhow::Result<impl SingleInstance>;
}
