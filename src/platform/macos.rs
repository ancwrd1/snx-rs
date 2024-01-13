use std::time::Duration;

use tokio::net::UdpSocket;

use crate::{
    platform::{UdpEncap, UdpSocketExt},
    prompt::SecurePrompt,
};

pub mod ipsec;
pub mod net;

#[async_trait::async_trait]
impl UdpSocketExt for UdpSocket {
    fn set_encap(&self, _encap: UdpEncap) -> anyhow::Result<()> {
        Ok(())
    }

    fn set_no_check(&self, _flag: bool) -> anyhow::Result<()> {
        Ok(())
    }

    async fn send_receive(&self, data: &[u8], timeout: Duration) -> anyhow::Result<Vec<u8>> {
        super::udp_send_receive(self, data, timeout).await
    }
}

pub async fn acquire_password(user_name: &str, prompt: SecurePrompt) -> anyhow::Result<String> {
    Ok(prompt
        .get_secure_input(&format!("Enter password for {}: ", user_name))?
        .trim()
        .to_owned())
}
