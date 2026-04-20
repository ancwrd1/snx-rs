use std::{
    fs,
    net::{Ipv4Addr, SocketAddr},
    os::fd::{AsFd, OwnedFd},
    time::Duration,
};

use anyhow::anyhow;
use cached::proc_macro::cached;
use nix::{
    fcntl::{self, FcntlArg, OFlag},
    getsockopt_impl, setsockopt_impl, sockopt_impl,
    sys::{socket, stat::Mode},
    unistd,
};
use tokio::net::UdpSocket;
use tracing::debug;
use uuid::Uuid;

use crate::{
    model::{IPsecSession, params::TunnelType},
    platform::{
        DeviceConfig, IPsecConfigurator, Keychain, NetworkInterface, PlatformAccess, PlatformFeatures,
        ResolverConfigurator, RoutingConfigurator, SingleInstance, UdpEncapType, UdpSocketExt,
    },
};

mod keychain;
pub mod net;
pub mod resolver;
mod routing;
pub mod xfrm;

// nix does not provide these socket options yet, so we implement them here using the convenient macros.
sockopt_impl!(UdpEncap, Both, libc::SOL_UDP, libc::UDP_ENCAP, UdpEncapType);
sockopt_impl!(NoCheck, Both, libc::SOL_SOCKET, libc::SO_NO_CHECK, bool);

#[async_trait::async_trait]
impl UdpSocketExt for UdpSocket {
    fn set_encapsulation(&self, encap_type: UdpEncapType) -> anyhow::Result<()> {
        socket::setsockopt(self, UdpEncap, &encap_type)
            .map_err(|e| anyhow!(i18n::tr!("error-udp-encap-failed", code = e)))
    }

    fn set_no_check(&self, flag: bool) -> anyhow::Result<()> {
        socket::setsockopt(self, NoCheck, &flag).map_err(|e| anyhow!(i18n::tr!("error-so-no-check-failed", code = e)))
    }

    async fn send_receive(&self, data: &[u8], timeout: Duration, target: SocketAddr) -> anyhow::Result<Vec<u8>> {
        super::udp_send_receive(self, data, timeout, target).await
    }
}

pub struct UnixSingleInstance {
    name: String,
    handle: Option<OwnedFd>,
}

impl UnixSingleInstance {
    pub fn new<N: AsRef<str>>(name: N) -> anyhow::Result<Self> {
        let fd = fcntl::open(
            name.as_ref(),
            OFlag::O_RDWR | OFlag::O_CREAT,
            Mode::from_bits_truncate(0o600),
        )?;

        let fl = libc::flock {
            l_type: libc::F_WRLCK as _,
            l_whence: libc::SEEK_SET as _,
            l_start: 0,
            l_len: 0,
            l_pid: 0,
        };

        match fcntl::fcntl(fd.as_fd(), FcntlArg::F_SETLK(&fl)) {
            Ok(_) => Ok(UnixSingleInstance {
                name: name.as_ref().to_owned(),
                handle: Some(fd),
            }),
            Err(_) => {
                let _ = unistd::close(fd);
                Ok(UnixSingleInstance {
                    name: name.as_ref().to_owned(),
                    handle: None,
                })
            }
        }
    }
}

impl Drop for UnixSingleInstance {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = unistd::close(handle);
            let _ = fs::remove_file(&self.name);
        }
    }
}

impl SingleInstance for UnixSingleInstance {
    fn is_single(&self) -> bool {
        self.handle.is_some()
    }
}

#[cached(result = true)]
pub fn get_machine_uuid() -> anyhow::Result<Uuid> {
    let data = fs::read_to_string("/etc/machine-id")?;
    Ok(Uuid::try_parse(data.trim())?)
}

fn is_wsl2() -> bool {
    fs::read_dir("/proc/sys/fs/binfmt_misc").is_ok_and(|dir| {
        dir.flatten()
            .any(|entry| entry.file_name().to_string_lossy().starts_with("WSLInterop"))
    })
}

async fn check_for_xfrm_state() -> bool {
    let Ok(handle) = xfrmnetlink::new_connection().map(|(conn, handle, _)| {
        tokio::spawn(conn);
        handle
    }) else {
        return false;
    };
    handle.state().get_sadinfo().execute().await.is_ok()
}

#[cached]
async fn is_xfrm_available() -> bool {
    if is_wsl2() {
        debug!("WSL2 detected, xfrm not available");
        return false;
    }

    let result = check_for_xfrm_state().await;

    debug!("Kernel xfrm available: {}", result);

    result
}

pub(crate) fn new_netlink_connection() -> anyhow::Result<rtnetlink::Handle> {
    let (connection, handle, _) = rtnetlink::new_connection()?;
    tokio::spawn(connection);
    Ok(handle)
}

pub(crate) async fn resolve_device_index(handle: &rtnetlink::Handle, device: &str) -> anyhow::Result<u32> {
    use futures::StreamExt;

    let mut links = handle.link().get().match_name(device.to_string()).execute();
    if let Some(Ok(link)) = links.next().await {
        Ok(link.header.index)
    } else {
        Err(anyhow!(i18n::tr!("error-device-not-found", device = device)))
    }
}

pub struct LinuxPlatformAccess;

impl PlatformAccess for LinuxPlatformAccess {
    async fn get_features(&self) -> PlatformFeatures {
        PlatformFeatures {
            ipsec_native: is_xfrm_available().await,
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
        keychain::SecretServiceKeychain::new()
    }

    fn get_machine_uuid(&self) -> anyhow::Result<Uuid> {
        get_machine_uuid()
    }

    fn init(&self) {
        #[cfg(openssl3)]
        {
            use std::sync::OnceLock;

            use openssl::provider::Provider;

            static LEGACY_PROVIDER: OnceLock<Provider> = OnceLock::new();

            if let Ok(provider) = Provider::try_load(None, "legacy", true) {
                let _ = LEGACY_PROVIDER.set(provider);
            }
        }
    }

    fn new_ipsec_configurator(
        &self,
        device_config: DeviceConfig,
        ipsec_session: IPsecSession,
        src_port: u16,
        dest_ip: Ipv4Addr,
        dest_port: u16,
    ) -> anyhow::Result<impl IPsecConfigurator + use<> + Send + Sync> {
        xfrm::XfrmConfigurator::new(device_config, ipsec_session, src_port, dest_ip, dest_port)
    }

    fn new_routing_configurator<S: AsRef<str>>(
        &self,
        device: S,
        tunnel_type: TunnelType,
    ) -> impl RoutingConfigurator + Send + Sync {
        routing::LinuxRoutingConfigurator::new(device, tunnel_type)
    }

    fn new_network_interface(&self) -> impl NetworkInterface + Send + Sync {
        net::LinuxNetworkInterface::new()
    }

    fn new_single_instance<S: AsRef<str>>(&self, name: S) -> anyhow::Result<impl SingleInstance> {
        UnixSingleInstance::new(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_xfrm_check() {
        println!("{}", is_xfrm_available().await);
    }
}
