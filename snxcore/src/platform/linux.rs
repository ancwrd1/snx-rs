use std::{
    fs,
    net::SocketAddr,
    os::{
        fd::AsRawFd,
        fd::{AsFd, OwnedFd},
    },
    time::Duration,
};

use anyhow::anyhow;
use cached::proc_macro::cached;
pub use keychain::SecretServiceKeychain as KeychainImpl;
pub use net::LinuxNetworkInterface as NetworkInterfaceImpl;
use nix::{
    fcntl::{self, FcntlArg, OFlag},
    sys::stat::Mode,
    unistd,
};
pub use resolver::new_resolver_configurator;
pub use routing::LinuxRoutingConfigurator as RoutingImpl;
use tokio::net::UdpSocket;
use tracing::debug;
use uuid::Uuid;
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
        use std::sync::OnceLock;

        use openssl::provider::Provider;

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

    async fn send_receive(&self, data: &[u8], timeout: Duration) -> anyhow::Result<Vec<u8>> {
        super::udp_send_receive(self, data, timeout).await
    }

    async fn send_receive_to(&self, data: &[u8], timeout: Duration, target: SocketAddr) -> anyhow::Result<Vec<u8>> {
        super::udp_send_receive_to(self, data, timeout, target).await
    }
}

pub struct SingleInstance {
    name: String,
    handle: Option<OwnedFd>,
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

        let fl = libc::flock {
            l_type: libc::F_WRLCK as _,
            l_whence: libc::SEEK_SET as _,
            l_start: 0,
            l_len: 0,
            l_pid: 0,
        };

        match fcntl::fcntl(fd.as_fd(), FcntlArg::F_SETLK(&fl)) {
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
            let _ = fs::remove_file(&self.name);
        }
    }
}

#[cached(result = true)]
pub fn get_machine_uuid() -> anyhow::Result<Uuid> {
    let data = fs::read_to_string("/etc/machine-id")?;
    Ok(Uuid::try_parse(data.trim())?)
}

#[cached]
async fn is_xfrm_available() -> bool {
    let _ = crate::util::run_command("modprobe", ["xfrm_user"]).await;

    let output = crate::util::run_command::<_, _, &str>("lsmod", [])
        .await
        .unwrap_or_default();

    let result = output.contains("xfrm_user");

    debug!("XFRM available: {}", result);

    result
}

pub async fn get_features() -> PlatformFeatures {
    PlatformFeatures {
        ipsec_native: is_xfrm_available().await,
        ipsec_keepalive: true,
        split_dns: true,
    }
}
