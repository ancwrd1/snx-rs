#![allow(unsafe_code)]

use std::{
    net::{Ipv4Addr, SocketAddr},
    os::raw::c_int,
    path::PathBuf,
    time::Duration,
};

use anyhow::anyhow;
use nix::{getsockopt_impl, setsockopt_impl, sockopt_impl, sys::socket};
use tokio::net::UdpSocket;
use uuid::Uuid;

use crate::{
    model::{IPsecSession, params::TunnelType},
    platform::{
        DeviceConfig, IPsecConfigurator, Keychain, NetworkInterface, PlatformAccess, PlatformFeatures,
        ResolverConfigurator, RoutingConfigurator, SingleInstance, UdpEncapType, UdpSocketExt,
    },
};

mod command_socket;
mod ipsec_stub;
mod keychain;
mod machine_uuid;
mod net;
mod resolver;
mod routing;
mod single_instance;
mod stats;

const UDP_NOCKSUM: c_int = 0x01;

sockopt_impl!(NoCheck, Both, libc::IPPROTO_UDP, UDP_NOCKSUM, bool);
sockopt_impl!(IpBoundIf, Both, libc::IPPROTO_IP, libc::IP_BOUND_IF, u32);

impl UdpSocketExt for UdpSocket {
    // macOS has no in-kernel ESP-in-UDP encapsulation; the userspace ESP data path
    // (forced by ipsec_native = false) drives the raw socket itself.
    fn set_encapsulation(&self, _encap: UdpEncapType) -> anyhow::Result<()> {
        Ok(())
    }

    fn set_no_check(&self, flag: bool) -> anyhow::Result<()> {
        socket::setsockopt(self, NoCheck, &flag).map_err(|e| anyhow!(i18n::tr!("error-so-no-check-failed", code = e)))
    }

    fn bind_to_tunnel(&self, device: &str) -> anyhow::Result<()> {
        let index =
            nix::net::if_::if_nametoindex(device).map_err(|e| anyhow!("if_nametoindex({device}) failed: {e}"))?;
        socket::setsockopt(self, IpBoundIf, &index)
            .map_err(|e| anyhow!("IP_BOUND_IF({device}, idx={index}) failed: {e}"))
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
            ipsec_keepalive: true,
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

    fn init(&self) {
        // Reconcile state a previous instance left after an unclean exit (panic aborts before Drop,
        // launchd restarts us): stale DNS keys, IPv6 blackholes, IP forwarding. All boot-lifetime.
        resolver::cleanup_stale_dns();
        net::restore_forwarding_from_marker();
        routing::cleanup_stale_blackholes();
    }

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
    ) -> anyhow::Result<impl RoutingConfigurator + Send + Sync + 'static> {
        Ok(routing::MacosRoutingConfigurator::new(device, tunnel_type))
    }

    fn new_network_interface(&self) -> impl NetworkInterface + Send + Sync + 'static {
        net::MacosNetworkInterface::new()
    }

    fn new_single_instance<S: AsRef<str>>(&self, name: S) -> anyhow::Result<impl SingleInstance + 'static> {
        single_instance::MacosSingleInstance::new(name)
    }

    fn data_dir(&self) -> PathBuf {
        // Root's home rather than world-readable /tmp: the session store may hold secrets.
        let home = std::env::var("HOME").unwrap_or_else(|_| "/var/root".to_owned());
        PathBuf::from(home).join("Library/Application Support/snx-rs")
    }

    fn command_socket_name(&self, name: &str) -> std::io::Result<interprocess::local_socket::Name<'static>> {
        command_socket::name(name)
    }

    fn secure_command_socket(&self, name: &str) -> std::io::Result<()> {
        command_socket::secure(name)
    }

    fn authorize_socket_peer(&self, stream: &interprocess::local_socket::tokio::Stream) -> bool {
        command_socket::authorize_peer(stream)
    }
}
