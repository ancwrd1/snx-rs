#![allow(unsafe_code)]

use std::{
    net::{Ipv4Addr, SocketAddr},
    os::windows::io::AsRawSocket,
    path::PathBuf,
    time::Duration,
};

use anyhow::anyhow;
use tokio::net::UdpSocket;
use uuid::Uuid;
use windows::{
    Win32::{
        NetworkManagement::{
            IpHelper::{ConvertInterfaceAliasToLuid, ConvertInterfaceLuidToGuid, ConvertInterfaceLuidToIndex},
            Ndis::{IF_MAX_STRING_SIZE, NET_LUID_LH},
        },
        Networking::WinSock::{AF_INET, IP_UNICAST_IF, IPPROTO_IP, SOCKADDR_INET, SOCKET, WSAGetLastError, setsockopt},
    },
    core::{GUID, PCWSTR},
};

use crate::{
    model::{IPsecSession, params::TunnelType},
    platform::{
        DeviceConfig, IPsecConfigurator, Keychain, NetworkInterface, PlatformAccess, PlatformFeatures,
        ResolverConfigurator, RoutingConfigurator, SingleInstance, UdpEncapType, UdpSocketExt,
    },
};

mod firewall;
mod ipsec_stub;
mod keychain;
mod machine_uuid;
mod net;
mod nrpt;
mod resolver;
mod routing;
mod single_instance;
mod stats;

#[macro_export]
macro_rules! utf16z {
    ($str: expr) => {
        $str.encode_utf16().chain([0]).collect::<Vec<_>>()
    };
}

fn alias_to_luid(alias: &str) -> anyhow::Result<NET_LUID_LH> {
    let wide = utf16z!(alias);
    if wide.len() > IF_MAX_STRING_SIZE as usize {
        return Err(anyhow::anyhow!("interface alias too long: {alias}"));
    }
    let mut luid = NET_LUID_LH::default();
    unsafe { ConvertInterfaceAliasToLuid(PCWSTR(wide.as_ptr()), &mut luid) }
        .ok()
        .map_err(|e| anyhow::anyhow!("ConvertInterfaceAliasToLuid({alias}) failed: {e}"))?;
    Ok(luid)
}

fn alias_to_guid(alias: &str) -> anyhow::Result<GUID> {
    let luid = alias_to_luid(alias)?;
    let mut guid = GUID::default();

    unsafe { ConvertInterfaceLuidToGuid(&luid, &mut guid) }
        .ok()
        .map_err(|e| anyhow!("ConvertInterfaceLuidToGuid failed: {e}"))?;

    Ok(guid)
}

fn luid_to_index(luid: &NET_LUID_LH) -> anyhow::Result<u32> {
    let mut index: u32 = 0;
    unsafe { ConvertInterfaceLuidToIndex(luid, &mut index) }.ok()?;
    Ok(index)
}

fn sockaddr_ipv4(addr: Ipv4Addr) -> SOCKADDR_INET {
    let mut sa = SOCKADDR_INET::default();
    let ipv4 = unsafe { &mut sa.Ipv4 };
    ipv4.sin_family = AF_INET;
    ipv4.sin_addr.S_un.S_addr = u32::from_ne_bytes(addr.octets());
    sa
}

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

    fn bind_to_tunnel(&self, device: &str) -> anyhow::Result<()> {
        let luid = alias_to_luid(device)?;
        let index = luid_to_index(&luid)?;

        // IP_UNICAST_IF takes the interface index in network byte order — a
        // documented quirk of this specific setsockopt.
        let value_be = index.to_be();
        let bytes = value_be.to_ne_bytes();
        let rc = unsafe {
            setsockopt(
                SOCKET(self.as_raw_socket() as _),
                IPPROTO_IP.0,
                IP_UNICAST_IF,
                Some(&bytes),
            )
        };
        if rc != 0 {
            let err = unsafe { WSAGetLastError() };
            anyhow::bail!("IP_UNICAST_IF({device}, idx={index}) failed: {err:?}");
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
    ) -> anyhow::Result<Box<dyn RoutingConfigurator + Send + Sync>> {
        Ok(Box::new(
            routing::WindowsRoutingConfigurator::new(device, tunnel_type).await?,
        ))
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
