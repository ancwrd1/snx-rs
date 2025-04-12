use std::{
    collections::HashSet,
    net::Ipv4Addr,
    sync::{atomic::AtomicBool, atomic::Ordering},
};

use crate::model::params::TunnelParams;
use anyhow::anyhow;
use futures::StreamExt;
use ipnet::Ipv4Net;
use tracing::debug;
use zbus::Connection;

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
                            return Ok(ip.split_once('/').map_or(ip, |(before, _)| before).to_string());
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

pub async fn add_routes(
    routes: &[Ipv4Net],
    device: &str,
    ipaddr: Ipv4Addr,
    ignore_routes: &[Ipv4Net],
) -> anyhow::Result<()> {
    let routes = routes.iter().collect::<HashSet<_>>();
    debug!("Routes to add: {:?}", routes);

    for route in routes {
        if ignore_routes.iter().any(|ignore| ignore == route) {
            debug!("Ignoring route: {}", route);
            continue;
        }
        let _ = add_route(*route, device, ipaddr).await;
    }

    Ok(())
}

pub async fn setup_default_route(device: &str, ipaddr: Ipv4Addr) -> anyhow::Result<()> {
    debug!("Setting up default route through {device}");

    let port = TunnelParams::IPSEC_KEEPALIVE_PORT.to_string();
    let dst = ipaddr.to_string();

    crate::util::run_command("ip", ["route", "add", "table", &port, "default", "dev", device]).await?;
    crate::util::run_command("ip", ["rule", "add", "not", "to", &dst, "table", &port]).await?;

    Ok(())
}

pub async fn setup_keepalive_route(device: &str, ipaddr: Ipv4Addr, with_table: bool) -> anyhow::Result<()> {
    debug!("Setting up keepalive route through {device}");

    let port = TunnelParams::IPSEC_KEEPALIVE_PORT.to_string();
    let dst = ipaddr.to_string();

    if with_table {
        crate::util::run_command("ip", &["route", "add", "table", &port, &dst, "dev", device]).await?;
    }

    crate::util::run_command(
        "ip",
        &[
            "rule", "add", "to", &dst, "ipproto", "udp", "dport", &port, "table", &port,
        ],
    )
    .await?;

    Ok(())
}

pub async fn remove_default_route(ipaddr: Ipv4Addr) -> anyhow::Result<()> {
    let port = TunnelParams::IPSEC_KEEPALIVE_PORT.to_string();
    let dst = ipaddr.to_string();

    crate::util::run_command("ip", ["rule", "del", "not", "to", &dst, "table", &port]).await?;

    Ok(())
}

pub async fn remove_keepalive_route(ipaddr: Ipv4Addr) -> anyhow::Result<()> {
    let port = TunnelParams::IPSEC_KEEPALIVE_PORT.to_string();
    let dst = ipaddr.to_string();

    crate::util::run_command(
        "ip",
        &[
            "rule", "del", "to", &dst, "ipproto", "udp", "dport", &port, "table", &port,
        ],
    )
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_default_ip() {
        let ip = get_default_ip().await.unwrap();
        println!("{ip}");
    }
}
