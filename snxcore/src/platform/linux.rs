use std::{fs, os::fd::AsRawFd, time::Duration};

use anyhow::anyhow;
use cached::proc_macro::cached;
use nix::{
    fcntl::{self, FcntlArg, OFlag},
    sys::stat::Mode,
    unistd,
};
use tokio::net::UdpSocket;
use uuid::Uuid;

pub use keychain::SecretServiceKeychain as KeychainImpl;
pub use net::LinuxNetworkInterface as NetworkInterfaceImpl;
pub use resolver::new_resolver_configurator;
pub use routing::LinuxRoutingConfigurator as RoutingImpl;
pub use xfrm::XfrmConfigurator as IpsecImpl;

use crate::platform::{PlatformFeatures, UdpEncap, UdpSocketExt};

mod keychain;
pub mod net;
pub mod resolver;
mod routing;
pub mod xfrm;

const UDP_ENCAP_ESPINUDP: libc::c_int = 2; // from /usr/include/linux/udp.h

pub fn init() {
    #[cfg(openssl3)]
    {
        use openssl::provider::Provider;
        use std::sync::OnceLock;

        static LEGACY_PROVIDER: OnceLock<Provider> = OnceLock::new();

        if let Ok(provider) = Provider::try_load(None, "legacy", true) {
            let _ = LEGACY_PROVIDER.set(provider);
        }
    }
}

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
                std::mem::size_of::<libc::c_int>() as _,
            );
            if rc != 0 {
                Err(anyhow!("Cannot set UDP_ENCAP socket option, error code: {}", rc))
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
                std::mem::size_of::<libc::c_int>() as _,
            );
            if rc != 0 {
                Err(anyhow!("Cannot set SO_NO_CHECK socket option, error code: {}", rc))
            } else {
                Ok(())
            }
        }
    }

    async fn send_receive(&self, data: &[u8], timeout: Duration) -> anyhow::Result<Vec<u8>> {
        super::udp_send_receive(self, data, timeout).await
    }
}

pub struct SingleInstance {
    name: String,
    handle: Option<nix::libc::c_int>,
}

unsafe impl Send for SingleInstance {}
unsafe impl Sync for SingleInstance {}

impl SingleInstance {
    pub fn new<N: AsRef<str>>(name: N) -> anyhow::Result<Self> {
        let fd = fcntl::open(
            name.as_ref(),
            OFlag::O_RDWR | OFlag::O_CREAT,
            Mode::from_bits_truncate(0o600),
        )?;

        let fl = nix::libc::flock {
            l_type: nix::libc::F_WRLCK as _,
            l_whence: nix::libc::SEEK_SET as _,
            l_start: 0,
            l_len: 0,
            l_pid: 0,
        };

        match fcntl::fcntl(fd, FcntlArg::F_SETLK(&fl)) {
            Ok(_) => Ok(SingleInstance {
                name: name.as_ref().to_owned(),
                handle: Some(fd),
            }),
            Err(_) => {
                let _ = unistd::close(fd);
                Ok(SingleInstance {
                    name: name.as_ref().to_owned(),
                    handle: None,
                })
            }
        }
    }

    pub fn is_single(&self) -> bool {
        self.handle.is_some()
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = unistd::close(handle);
            let _ = std::fs::remove_file(&self.name);
        }
    }
}

#[cached(result = true)]
pub fn get_machine_uuid() -> anyhow::Result<Uuid> {
    let data = fs::read_to_string("/etc/machine-id")?;
    Ok(Uuid::try_parse(data.trim())?)
}

pub fn get_features() -> PlatformFeatures {
    PlatformFeatures {
        ipsec_native: true,
        ipsec_keepalive: true,
        split_dns: true,
    }
}
