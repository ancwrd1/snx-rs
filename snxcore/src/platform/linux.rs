use std::{
    fs,
    net::{Ipv4Addr, SocketAddr},
    os::fd::{AsFd, AsRawFd, OwnedFd},
    sync::mpsc,
    time::Duration,
};

use anyhow::anyhow;
use cached::proc_macro::cached;
use nix::{
    fcntl::{self, FcntlArg, OFlag},
    sys::stat::Mode,
    unistd,
};
use tokio::net::UdpSocket;
use tracing::debug;
use uuid::Uuid;

use crate::{
    model::IpsecSession,
    platform::{
        IpsecConfigurator, Keychain, NetworkInterface, PlatformAccess, PlatformFeatures, ResolverConfigurator,
        RoutingConfigurator, SingleInstance, UdpEncap, UdpSocketExt,
    },
    util,
};

mod keychain;
pub mod net;
pub mod resolver;
mod routing;
pub mod xfrm;

const UDP_ENCAP_ESPINUDP: libc::c_int = 2; // from /usr/include/linux/udp.h

#[async_trait::async_trait]
impl UdpSocketExt for UdpSocket {
    fn set_encap(&self, encap: UdpEncap) -> anyhow::Result<()> {
        let stype: libc::c_int = match encap {
            UdpEncap::EspInUdp => UDP_ENCAP_ESPINUDP,
        };

        unsafe {
            let rc = libc::setsockopt(
                self.as_raw_fd(),
                libc::SOL_UDP,
                libc::UDP_ENCAP,
                &stype as *const libc::c_int as _,
                size_of::<libc::c_int>() as _,
            );
            if rc != 0 {
                Err(anyhow!(i18n::tr!("error-udp-encap-failed", code = rc)))
            } else {
                Ok(())
            }
        }
    }

    fn set_no_check(&self, flag: bool) -> anyhow::Result<()> {
        let disable: libc::c_int = flag.into();
        unsafe {
            let rc = libc::setsockopt(
                self.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_NO_CHECK,
                &disable as *const libc::c_int as _,
                size_of::<libc::c_int>() as _,
            );
            if rc != 0 {
                Err(anyhow!(i18n::tr!("error-so-no-check-failed", code = rc)))
            } else {
                Ok(())
            }
        }
    }

    async fn send_receive(&self, data: &[u8], timeout: Duration, target: SocketAddr) -> anyhow::Result<Vec<u8>> {
        super::udp_send_receive(self, data, timeout, target).await
    }
}

pub struct UnixSingleInstance {
    name: String,
    handle: Option<OwnedFd>,
}

unsafe impl Send for UnixSingleInstance {}
unsafe impl Sync for UnixSingleInstance {}

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

#[cached]
fn is_xfrm_available() -> bool {
    let (tx, rx) = mpsc::channel();

    tokio::spawn(async move {
        let _ = util::run_command("modprobe", ["xfrm_interface"]).await;
        let modules = tokio::fs::read_to_string("/proc/modules").await.unwrap_or_default();
        let result = modules.lines().any(|line| line.starts_with("xfrm_"));
        tx.send(result)
    });

    let result = rx.recv().unwrap_or(false);

    debug!("Kernel xfrm available: {}", result);

    result
}

pub struct LinuxPlatformAccess;

impl PlatformAccess for LinuxPlatformAccess {
    fn get_features(&self) -> PlatformFeatures {
        PlatformFeatures {
            ipsec_native: is_xfrm_available(),
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
        name: &str,
        ipsec_session: IpsecSession,
        src_port: u16,
        dest_ip: Ipv4Addr,
        dest_port: u16,
    ) -> anyhow::Result<impl IpsecConfigurator + use<> + Send + Sync> {
        xfrm::XfrmConfigurator::new(name, ipsec_session, src_port, dest_ip, dest_port)
    }

    fn new_routing_configurator<S: AsRef<str>>(
        &self,
        device: S,
        address: Ipv4Addr,
    ) -> impl RoutingConfigurator + Send + Sync {
        routing::LinuxRoutingConfigurator::new(device, address)
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

    #[test]
    fn test_xfrm_check() {
        assert!(is_xfrm_available());
    }
}
