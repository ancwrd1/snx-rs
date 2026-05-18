use std::{
    fmt,
    marker::PhantomData,
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use anyhow::anyhow;
use async_trait::async_trait;
use ipnet::Ipv4Net;
#[cfg(target_os = "linux")]
use linux::LinuxPlatformAccess as PlatformAccessImpl;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;
use uuid::Uuid;
#[cfg(target_os = "windows")]
use windows::WindowsPlatformAccess as PlatformAccessImpl;

use crate::model::{
    IPsecSession, LiveStats,
    params::{TunnelParams, TunnelType},
};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

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
pub trait IPsecConfigurator {
    async fn configure(&self) -> anyhow::Result<()>;
    async fn rekey(&mut self, session: &IPsecSession) -> anyhow::Result<()>;
    async fn cleanup(&self);
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
#[repr(i32)]
pub enum UdpEncapType {
    EspInUdp = 2,
}

pub trait UdpSocketExt {
    fn set_encapsulation(&self, encap: UdpEncapType) -> anyhow::Result<()>;
    fn set_no_check(&self, flag: bool) -> anyhow::Result<()>;
    fn send_receive(
        &self,
        data: &[u8],
        timeout: Duration,
        target: SocketAddr,
    ) -> impl Future<Output = anyhow::Result<Vec<u8>>> + Send;
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
    pub split_dns: bool,
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
                SearchDomain::new(d, false)
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
            split_dns: self.features.split_dns && !self.params.no_split_dns,
            search_domains: self.search_domains,
            dns_servers: self.dns_servers,
        }
    }
}

#[derive(Debug)]
pub enum RoutingConfig {
    Full {
        destination: Ipv4Addr,
        disable_ipv6: bool,
    },
    Split {
        destination: Ipv4Addr,
        routes: Vec<Ipv4Net>,
    },
    Cleanup {
        destination: Ipv4Addr,
        enable_ipv6: bool,
    },
}

#[async_trait]
pub trait ResolverConfigurator {
    async fn configure(&self, config: &ResolverConfig) -> anyhow::Result<()>;
    async fn cleanup(&self, config: &ResolverConfig) -> anyhow::Result<()>;
}

pub trait Keychain {
    fn acquire_password(&self, profile_id: Uuid) -> impl Future<Output = anyhow::Result<String>> + Send;
    fn store_password(
        &self,
        profile_id: Uuid,
        password: &SecretString,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
    fn delete_password(&self, profile_id: Uuid) -> impl Future<Output = anyhow::Result<()>> + Send;
}

pub trait RoutingConfigurator {
    fn configure(&self, config: &RoutingConfig) -> impl Future<Output = anyhow::Result<()>> + Send;
}

#[async_trait]
pub trait StatsPoller {
    async fn poll(&self) -> anyhow::Result<LiveStats>;
}

pub trait NetworkInterface {
    fn start_network_state_monitoring(&self) -> impl Future<Output = anyhow::Result<()>> + Send;
    fn get_default_ipv4(&self) -> impl Future<Output = anyhow::Result<Ipv4Addr>> + Send;
    fn delete_device(&self, device_name: &str) -> impl Future<Output = anyhow::Result<()>> + Send;
    fn configure_device(&self, device_config: &DeviceConfig) -> impl Future<Output = anyhow::Result<()>> + Send;
    fn replace_ip_address(
        &self,
        device_name: &str,
        old_address: Ipv4Net,
        new_address: Ipv4Net,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    fn new_stats_poller(
        &self,
        device_name: &str,
    ) -> impl Future<Output = anyhow::Result<impl StatsPoller + Send + Sync + 'static>> + Send;

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
    fn is_root(&self) -> bool;
    fn init(&self);
    fn new_ipsec_configurator(
        &self,
        device_config: DeviceConfig,
        ipsec_session: IPsecSession,
        src_ip: Ipv4Addr,
        src_port: u16,
        dest_ip: Ipv4Addr,
        dest_port: u16,
    ) -> impl IPsecConfigurator + use<Self> + Send + Sync;
    fn new_routing_configurator<S: AsRef<str> + Send>(
        &self,
        device: S,
        tunnel_type: TunnelType,
    ) -> impl Future<Output = anyhow::Result<impl RoutingConfigurator + Send + Sync + 'static>> + Send;
    fn new_network_interface(&self) -> impl NetworkInterface + Send + Sync + 'static;
    fn new_single_instance<S: AsRef<str>>(&self, name: S) -> anyhow::Result<impl SingleInstance + 'static>;
    fn data_dir(&self) -> PathBuf;
}
