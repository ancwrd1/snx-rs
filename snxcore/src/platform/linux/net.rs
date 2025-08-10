use std::{
    net::Ipv4Addr,
    sync::atomic::{AtomicBool, Ordering},
};

use anyhow::anyhow;
use async_trait::async_trait;
use futures::StreamExt;
use ipnet::Ipv4Net;
use tracing::debug;
use zbus::Connection;

use crate::{platform::NetworkInterface, util};

static ONLINE_STATE: AtomicBool = AtomicBool::new(true);

#[derive(Debug, Copy, Clone, PartialEq)]
enum NetworkManagerState {
    Unknown,
    Asleep,
    Disconnected,
    Disconnecting,
    Connecting,
    ConnectedLocal,
    ConnectedSite,
    ConnectedGlobal,
}

impl From<u32> for NetworkManagerState {
    fn from(value: u32) -> Self {
        match value {
            10 => Self::Asleep,
            20 => Self::Disconnected,
            30 => Self::Disconnecting,
            40 => Self::Connecting,
            50 => Self::ConnectedLocal,
            60 => Self::ConnectedSite,
            70 => Self::ConnectedGlobal,
            _ => Self::Unknown,
        }
    }
}

impl NetworkManagerState {
    fn is_online(self) -> bool {
        matches!(self, Self::ConnectedGlobal)
    }
}

#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
pub trait NetworkManager {
    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;
}

#[derive(Default)]
pub struct LinuxNetworkInterface;

impl LinuxNetworkInterface {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NetworkInterface for LinuxNetworkInterface {
    async fn start_network_state_monitoring(&self) -> anyhow::Result<()> {
        let connection = Connection::system().await?;
        let proxy = NetworkManagerProxy::new(&connection).await?;

        let mut stream = proxy.receive_state_changed().await;
        tokio::spawn(async move {
            while let Some(signal) = stream.next().await {
                let state: NetworkManagerState = signal.get().await?.into();
                debug!("NetworkManager state changed to {:?}", state);
                ONLINE_STATE.store(state.is_online(), Ordering::SeqCst);
            }

            Ok::<_, zbus::Error>(())
        });

        Ok(())
    }

    async fn get_default_ip(&self) -> anyhow::Result<Ipv4Addr> {
        let default_route = util::run_command("ip", ["-4", "route", "show", "default"]).await?;
        let mut parts = default_route.split_whitespace();
        while let Some(part) = parts.next() {
            if part == "dev"
                && let Some(dev) = parts.next()
            {
                let addr = util::run_command("ip", ["-4", "-o", "addr", "show", "dev", dev]).await?;
                let mut parts = addr.split_whitespace();
                while let Some(part) = parts.next() {
                    if part == "inet"
                        && let Some(ip) = parts.next()
                    {
                        return Ok(ip
                            .split_once('/')
                            .map_or(ip, |(before, _)| before)
                            .to_string()
                            .parse()?);
                    }
                }
            }
        }
        Err(anyhow!(i18n::tr!("error-cannot-determine-ip")))
    }

    async fn delete_device(&self, device_name: &str) -> anyhow::Result<()> {
        util::run_command("ip", ["link", "del", "name", device_name]).await?;
        Ok(())
    }

    async fn configure_device(&self, device_name: &str) -> anyhow::Result<()> {
        util::run_command("nmcli", ["device", "set", device_name, "managed", "no"]).await?;
        let opt = format!("net.ipv4.conf.{device_name}.promote_secondaries");
        super::sysctl(opt, "1")?;
        Ok(())
    }

    async fn replace_ip_address(
        &self,
        device_name: &str,
        old_address: Ipv4Net,
        new_address: Ipv4Net,
    ) -> anyhow::Result<()> {
        util::run_command("ip", &["addr", "add", &new_address.to_string(), "dev", device_name]).await?;
        util::run_command("ip", &["addr", "del", &old_address.to_string(), "dev", device_name]).await?;
        Ok(())
    }

    fn is_online(&self) -> bool {
        ONLINE_STATE.load(Ordering::SeqCst)
    }

    fn poll_online(&self) {
        tokio::spawn(async move {
            let connection = Connection::system().await?;
            let proxy = NetworkManagerProxy::new(&connection).await?;
            let state = proxy.state().await?;
            let state: NetworkManagerState = state.into();
            debug!("Acquired network state via polling: {:?}", state);
            ONLINE_STATE.store(state.is_online(), Ordering::SeqCst);
            Ok::<_, anyhow::Error>(())
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_default_ip() {
        let ip = LinuxNetworkInterface.get_default_ip().await.unwrap();
        println!("{ip}");
    }
}
