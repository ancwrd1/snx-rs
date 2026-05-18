#![allow(unsafe_code)]

use std::path::PathBuf;
use std::{
    net::{Ipv4Addr, SocketAddr},
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
mod nrpt;
mod resolver;
mod routing;
mod single_instance;
mod stats;

impl UdpSocketExt for UdpSocket {
    fn set_encapsulation(&self, _encap_type: UdpEncapType) -> anyhow::Result<()> {
        Ok(())
    }

    fn set_no_check(&self, flag: bool) -> anyhow::Result<()> {
        use std::os::windows::io::AsRawSocket;

        use windows::Win32::Networking::WinSock::{IPPROTO_UDP, SOCKET, UDP_NOCHECKSUM, WSAGetLastError, setsockopt};

        let val: i32 = if flag { 1 } else { 0 };
        let bytes = val.to_ne_bytes();
        let rc = unsafe {
            setsockopt(
                SOCKET(self.as_raw_socket() as _),
                IPPROTO_UDP.0,
                UDP_NOCHECKSUM,
                Some(&bytes),
            )
        };
        if rc != 0 {
            let err = unsafe { WSAGetLastError() };
            anyhow::bail!("UDP_NOCHECKSUM failed: {err:?}");
        }
        Ok(())
    }

    async fn send_receive(&self, data: &[u8], timeout: Duration, target: SocketAddr) -> anyhow::Result<Vec<u8>> {
        super::udp_send_receive(self, data, timeout, target).await
    }
}

fn is_elevated() -> bool {
    use windows::Win32::{
        Security::{AllocateAndInitializeSid, CheckTokenMembership, FreeSid, PSID, SECURITY_NT_AUTHORITY},
        System::SystemServices::{DOMAIN_ALIAS_RID_ADMINS, SECURITY_BUILTIN_DOMAIN_RID},
    };

    let mut admins_sid = PSID::default();

    if unsafe {
        AllocateAndInitializeSid(
            &SECURITY_NT_AUTHORITY,
            2,
            SECURITY_BUILTIN_DOMAIN_RID as u32,
            DOMAIN_ALIAS_RID_ADMINS as u32,
            0,
            0,
            0,
            0,
            0,
            0,
            &mut admins_sid,
        )
    }
    .is_err()
    {
        return false;
    }

    let mut is_member = windows::core::BOOL(0);
    let check_ok = unsafe { CheckTokenMembership(None, admins_sid, &mut is_member) };
    unsafe { FreeSid(admins_sid) };

    check_ok.is_ok() && is_member.as_bool()
}

pub struct WindowsPlatformAccess;

impl PlatformAccess for WindowsPlatformAccess {
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
        keychain::WindowsKeychain::new()
    }

    fn get_machine_uuid(&self) -> anyhow::Result<Uuid> {
        machine_uuid::get_machine_uuid()
    }

    fn is_root(&self) -> bool {
        is_elevated()
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
        ipsec_stub::WindowsIPsecStub::new(device_config, ipsec_session, src_ip, src_port, dest_ip, dest_port)
    }

    async fn new_routing_configurator<S: AsRef<str> + Send>(
        &self,
        device: S,
        tunnel_type: TunnelType,
    ) -> anyhow::Result<impl RoutingConfigurator + Send + Sync + 'static> {
        routing::WindowsRoutingConfigurator::new(device, tunnel_type).await
    }

    fn new_network_interface(&self) -> impl NetworkInterface + Send + Sync + 'static {
        net::WindowsNetworkInterface::new()
    }

    fn new_single_instance<S: AsRef<str>>(&self, name: S) -> anyhow::Result<impl SingleInstance + 'static> {
        single_instance::WindowsSingleInstance::new(name)
    }

    fn data_dir(&self) -> PathBuf {
        PathBuf::from(std::env::var("PROGRAMDATA").unwrap_or_else(|_| "C:\\ProgramData".to_owned())).join("snx-rs")
    }
}
