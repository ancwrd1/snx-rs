#![allow(unsafe_code)]

use std::{
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    time::Duration,
};

use tokio::net::UdpSocket;
use uuid::Uuid;

use crate::{
    model::{IPsecSession, params::TunnelType},
    platform::{
        DeviceConfig, IPsecConfigurator, Keychain, NetworkInterface, PlatformAccess, PlatformFeatures,
        ResolverConfigurator, RoutingConfigurator, SingleInstance, UdpEncapType, UdpSocketExt,
    },
};

mod ipsec_stub;
mod keychain;
mod machine_uuid;
mod net;
mod resolver;
mod routing;
mod single_instance;
mod stats;

// macOS has no in-kernel ESP-in-UDP encapsulation; the userspace ESP data path
// (forced by ipsec_native = false) drives the raw socket itself, so these are no-ops.
impl UdpSocketExt for UdpSocket {
    fn set_encapsulation(&self, _encap: UdpEncapType) -> anyhow::Result<()> {
        Ok(())
    }

    fn set_no_check(&self, _flag: bool) -> anyhow::Result<()> {
        Ok(())
    }

    fn bind_to_tunnel(&self, _device: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn send_receive(&self, data: &[u8], timeout: Duration, target: SocketAddr) -> anyhow::Result<Vec<u8>> {
        super::udp_send_receive(self, data, timeout, target).await
    }
}

pub struct MacosPlatformAccess;

impl PlatformAccess for MacosPlatformAccess {
    async fn get_features(&self) -> PlatformFeatures {
        PlatformFeatures {
            ipsec_native: false,
            // The tunnel stays up without an app-level keepalive, so leave it off.
            ipsec_keepalive: false,
            split_dns: true,
        }
    }

    fn new_resolver_configurator<S: AsRef<str>>(
        &self,
        device: S,
    ) -> anyhow::Result<Box<dyn ResolverConfigurator + Send + Sync>> {
        resolver::new_resolver_configurator(device)
    }

    fn new_keychain(&self) -> impl Keychain + Send + Sync {
        keychain::MacosKeychain::new()
    }

    fn get_machine_uuid(&self) -> anyhow::Result<Uuid> {
        machine_uuid::get_machine_uuid()
    }

    fn is_root(&self) -> bool {
        nix::unistd::Uid::effective().is_root()
    }

    fn init(&self) {}

    fn new_ipsec_configurator(
        &self,
        device_config: DeviceConfig,
        ipsec_session: IPsecSession,
        src_ip: Ipv4Addr,
        src_port: u16,
        dest_ip: Ipv4Addr,
        dest_port: u16,
    ) -> impl IPsecConfigurator + use<> + Send + Sync {
        ipsec_stub::MacosIPsecStub::new(device_config, ipsec_session, src_ip, src_port, dest_ip, dest_port)
    }

    async fn new_routing_configurator<S: AsRef<str> + Send>(
        &self,
        device: S,
        tunnel_type: TunnelType,
    ) -> anyhow::Result<Box<dyn RoutingConfigurator + Send + Sync>> {
        Ok(Box::new(routing::MacosRoutingConfigurator::new(device, tunnel_type)))
    }

    fn new_network_interface(&self) -> impl NetworkInterface + Send + Sync + 'static {
        net::MacosNetworkInterface::new()
    }

    fn new_single_instance<S: AsRef<str>>(&self, name: S) -> anyhow::Result<impl SingleInstance + 'static> {
        single_instance::MacosSingleInstance::new(name)
    }

    fn data_dir(&self) -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_owned());
        PathBuf::from(home).join("Library/Application Support/snx-rs")
    }
}
