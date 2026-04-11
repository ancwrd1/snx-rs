use core::fmt;
use std::{
    marker::PhantomData,
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use anyhow::anyhow;
use async_trait::async_trait;
use ipnet::Ipv4Net;
#[cfg(target_os = "linux")]
use linux::LinuxPlatformAccess as PlatformAccessImpl;
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;
use uuid::Uuid;

use crate::model::{IpsecSession, params::TunnelParams};

#[cfg(target_os = "linux")]
mod linux;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SearchDomain {
    pub name: String,
    pub is_routing: bool,
}

impl SearchDomain {
    pub fn new<S: AsRef<str>>(name: S, is_routing: bool) -> Self {
        Self {
            name: name.as_ref().to_owned(),
            is_routing,
        }
    }
}

impl fmt::Display for SearchDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", if self.is_routing { "~" } else { "" }, self.name)
    }
}

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
#[repr(i32)]
pub enum UdpEncapType {
    EspInUdp = 2,
}

#[async_trait]
pub trait UdpSocketExt {
    fn set_encapsulation(&self, encap: UdpEncapType) -> anyhow::Result<()>;
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
pub struct DeviceConfig {
    pub name: String,
    pub mtu: u16,
    pub address: Ipv4Net,
    pub allow_forwarding: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResolverConfig {
    pub search_domains: Vec<SearchDomain>,
    pub dns_servers: Vec<Ipv4Addr>,
}

impl ResolverConfig {
    pub fn builder(params: Arc<TunnelParams>, features: PlatformFeatures) -> ResolverConfigBuilder {
        ResolverConfigBuilder {
            params,
            features,
            search_domains: Vec::new(),
            dns_servers: Vec::new(),
        }
    }
}

pub struct ResolverConfigBuilder {
    params: Arc<TunnelParams>,
    features: PlatformFeatures,
    search_domains: Vec<SearchDomain>,
    dns_servers: Vec<Ipv4Addr>,
}

impl ResolverConfigBuilder {
    // add search domains acquired from the tunnel
    pub fn search_domains<I, S>(mut self, domains: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.search_domains.extend(domains.into_iter().filter_map(|s| {
            let trimmed = s.as_ref().trim_matches(|c: char| c.is_whitespace() || c == '.');
            if trimmed.is_empty() {
                None
            } else {
                Some(SearchDomain::new(
                    trimmed,
                    self.params.set_routing_domains && self.features.split_dns,
                ))
            }
        }));
        self
    }

    // add DNS servers acquired from the tunnel
    pub fn dns_servers<I>(mut self, servers: I) -> Self
    where
        I: IntoIterator<Item = Ipv4Addr>,
    {
        self.dns_servers.extend(servers);

        self
    }

    pub fn build(mut self) -> ResolverConfig {
        // add manual search domains
        self.search_domains.extend(self.params.search_domains.iter().map(|d| {
            if let Some(s) = d.strip_prefix("~") {
                SearchDomain::new(s, self.features.split_dns)
            } else {
                SearchDomain::new(d, self.params.set_routing_domains && self.features.split_dns)
            }
        }));

        // remove ignored domains
        self.search_domains.retain(|domain| {
            !self
                .params
                .ignore_search_domains
                .iter()
                .any(|d| d.eq_ignore_ascii_case(&domain.name))
        });

        self.dns_servers.extend(&self.params.dns_servers);
        self.dns_servers.retain(|s| !self.params.ignore_dns_servers.contains(s));

        ResolverConfig {
            search_domains: self.search_domains,
            dns_servers: self.dns_servers,
        }
    }
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
    async fn setup_default_route(&self, destination: Ipv4Addr, disable_ipv6: bool) -> anyhow::Result<()>;
    async fn setup_keepalive_route(&self, destination: Ipv4Addr, with_table: bool) -> anyhow::Result<()>;
    async fn remove_default_route(&self, destination: Ipv4Addr, enable_ipv6: bool) -> anyhow::Result<()>;
    async fn remove_keepalive_route(&self, destination: Ipv4Addr) -> anyhow::Result<()>;
}

#[async_trait]
pub trait NetworkInterface {
    async fn start_network_state_monitoring(&self) -> anyhow::Result<()>;
    async fn get_default_ipv4(&self) -> anyhow::Result<Ipv4Addr>;
    async fn delete_device(&self, device_name: &str) -> anyhow::Result<()>;
    async fn configure_device(&self, device_config: &DeviceConfig) -> anyhow::Result<()>;
    async fn replace_ip_address(
        &self,
        device_name: &str,
        old_address: Ipv4Net,
        new_address: Ipv4Net,
    ) -> anyhow::Result<()>;

    fn is_online(&self) -> bool;
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
    fn get_features(&self) -> impl Future<Output = PlatformFeatures> + Send;
    fn new_resolver_configurator<S: AsRef<str>>(
        &self,
        device: S,
    ) -> anyhow::Result<Box<dyn ResolverConfigurator + Send + Sync>>;
    fn new_keychain(&self) -> impl Keychain + Send + Sync;
    fn get_machine_uuid(&self) -> anyhow::Result<Uuid>;
    fn init(&self);
    fn new_ipsec_configurator(
        &self,
        device_config: DeviceConfig,
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
