use std::{collections::HashMap, os::fd::AsRawFd, time::Duration};

use anyhow::anyhow;
use secret_service::{EncryptionType, SecretService};
use tokio::net::UdpSocket;

use crate::{
    platform::{UdpEncap, UdpSocketExt},
    prompt,
};

pub mod net;
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

pub async fn acquire_password(user_name: &str) -> anyhow::Result<String> {
    let ss = SecretService::connect(EncryptionType::Dh).await?;

    let props = HashMap::from([("snx-rs.password", user_name)]);

    let search_items = ss.search_items(props.clone()).await?;
    if let Some(item) = search_items.unlocked.get(0) {
        if let Ok(secret) = item.get_secret().await {
            return Ok(String::from_utf8_lossy(&secret).into_owned());
        }
    }

    let password = prompt::get_input_from_tty(&format!("Enter password for {} (echo is off): ", user_name))?;

    let collection = ss.get_default_collection().await?;

    collection
        .create_item("snx-rs user password", props, password.as_bytes(), true, "text/plain")
        .await?;

    Ok(password)
}
