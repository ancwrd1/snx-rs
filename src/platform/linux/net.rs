use std::{
    net::Ipv4Addr,
    sync::{atomic::AtomicBool, atomic::Ordering},
};

use anyhow::anyhow;
use futures::StreamExt;
use ipnet::Ipv4Net;
use tracing::debug;
use zbus::{dbus_proxy, Connection};

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
    fn is_online(&self) -> bool {
        matches!(self, Self::ConnectedGlobal)
    }
}

#[dbus_proxy(
    interface = "org.freedesktop.NetworkManager",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
pub trait NetworkManager {
    #[dbus_proxy(property)]
    fn state(&self) -> zbus::Result<u32>;
}

pub async fn start_network_state_monitoring() -> anyhow::Result<()> {
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

pub fn is_online() -> bool {
    ONLINE_STATE.load(Ordering::SeqCst)
}

pub fn poll_online() {
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

pub async fn get_default_ip() -> anyhow::Result<String> {
    let default_route = crate::util::run_command("ip", ["-4", "route", "show", "default"]).await?;
    let mut parts = default_route.split_whitespace();
    while let Some(part) = parts.next() {
        if part == "dev" {
            if let Some(dev) = parts.next() {
                let addr = crate::util::run_command("ip", ["-4", "-o", "addr", "show", "dev", dev]).await?;
                let mut parts = addr.split_whitespace();
                while let Some(part) = parts.next() {
                    if part == "inet" {
                        if let Some(ip) = parts.next() {
                            if let Some((ip, _)) = ip.split_once('/') {
                                return Ok(ip.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    Err(anyhow!("Cannot determine default IP!"))
}

pub async fn add_route(route: Ipv4Net, device: &str, _ipaddr: Ipv4Addr) -> anyhow::Result<()> {
    debug!("Adding route: {} via {}", route, device);
    crate::util::run_command("ip", ["route", "add", &route.to_string(), "dev", device]).await?;
    Ok(())
}

fn subnet_overlaps(index: usize, subnet: Ipv4Net, other: &[Ipv4Net]) -> bool {
    other
        .iter()
        .enumerate()
        .any(|(i, s)| i != index && (*s == subnet || s.contains(&subnet) || subnet.contains(s)))
}

pub async fn add_routes(routes: &[Ipv4Net], device: &str, ipaddr: Ipv4Addr) -> anyhow::Result<()> {
    debug!("Routes to add: {:?}", routes);
    for (_, subnet) in routes
        .iter()
        .enumerate()
        .filter(|(i, s)| !subnet_overlaps(*i, **s, routes))
    {
        let _ = add_route(*subnet, device, ipaddr).await;
    }

    Ok(())
}

pub async fn add_default_route(device: &str, _ipaddr: Ipv4Addr) -> anyhow::Result<()> {
    debug!("Adding default route for {}", device);
    let _ = crate::util::run_command("ip", ["route", "add", "default", "dev", device]).await?;

    Ok(())
}

pub async fn add_dns_suffixes<I, T>(suffixes: I, device: &str) -> anyhow::Result<()>
where
    I: IntoIterator<Item = T>,
    T: AsRef<str>,
{
    let mut args = vec!["domain", device];

    let suffixes = suffixes.into_iter().map(|s| s.as_ref().trim().to_owned()).collect::<Vec<_>>();

    args.extend(suffixes.iter().map(|s| s.as_str()));

    crate::util::run_command("resolvectl", args).await?;

    crate::util::run_command("resolvectl", ["default-route", device, "false"]).await?;

    Ok(())
}

pub async fn add_dns_servers<I, T>(servers: I, device: &str) -> anyhow::Result<()>
where
    I: IntoIterator<Item = T>,
    T: AsRef<str>,
{
    let mut args = vec!["dns", device];

    let servers = servers.into_iter().map(|s| s.as_ref().to_owned()).collect::<Vec<_>>();

    args.extend(servers.iter().map(|s| s.as_str()));

    crate::util::run_command("resolvectl", args).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_default_ip() {
        let ip = get_default_ip().await.unwrap();
        println!("{}", ip);
    }
}
