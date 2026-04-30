use std::{
    net::{IpAddr, Ipv4Addr},
    sync::atomic::{AtomicBool, Ordering},
};

use anyhow::anyhow;
use async_trait::async_trait;
use futures::StreamExt;
use ipnet::Ipv4Net;
use rtnetlink::{
    AddressMessageBuilder, LinkMessageBuilder, LinkUnspec, RouteMessageBuilder,
    packet_route::{
        AddressFamily,
        address::AddressAttribute,
        route::{RouteAddress, RouteAttribute, RouteScope, RouteType},
    },
};
use sysctl::{Ctl, Sysctl};
use tracing::debug;
use zbus::Connection;

use crate::platform::{DeviceConfig, NetworkInterface, StatsPoller, linux::stats::LinuxStatsPoller};

static ONLINE_STATE: AtomicBool = AtomicBool::new(true);

// Setting sysctl values can be flaky for new interfaces because they can acquire default values asynchronously.
async fn sysctl_set(name: String, value: &'static str) -> anyhow::Result<()> {
    const POST_SET_DELAYS_MS: &[u64] = &[50, 100, 150];

    tokio::task::spawn_blocking(move || {
        let ctl = Ctl::new(&name)?;

        for &delay_ms in POST_SET_DELAYS_MS {
            debug!("Setting sysctl {name} = {value}");
            ctl.set_value_string(value)?;

            std::thread::sleep(std::time::Duration::from_millis(delay_ms));

            if ctl.value_string()? == value {
                return Ok(());
            }
        }
        anyhow::bail!(i18n::tr!(
            "error-sysctl-not-converged",
            entry = format!("{name} = {value}")
        ))
    })
    .await?
}

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

    fn get_device_by_ip_iface(&self, iface: &str) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
}

#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager.Device",
    default_service = "org.freedesktop.NetworkManager"
)]
pub trait NetworkManagerDevice {
    #[zbus(property)]
    fn set_managed(&self, managed: bool) -> zbus::Result<()>;
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

    async fn get_default_ipv4(&self) -> anyhow::Result<Ipv4Addr> {
        let handle = super::new_netlink_connection()?;

        let mut routes = handle
            .route()
            .get(RouteMessageBuilder::<Ipv4Addr>::new().build())
            .execute();

        while let Some(route) = routes.next().await {
            if let Ok(route) = route
                && route.header.scope == RouteScope::Universe
                && route.header.kind == RouteType::Unicast
                && route.header.address_family == AddressFamily::Inet
                && route.header.destination_prefix_length == 0
                && route.attributes.iter().any(|a| matches!(a, RouteAttribute::Gateway(_)))
            {
                // Try PrefSource first (src field in routing table)
                if let Some(ip) = route.attributes.iter().find_map(|a| match a {
                    RouteAttribute::PrefSource(RouteAddress::Inet(ip)) => Some(*ip),
                    _ => None,
                }) {
                    debug!("Found default route with preferred source {}", ip);
                    return Ok(ip);
                }

                // PrefSource may not be set; fall back to querying the interface address
                if let Some(oif) = route.attributes.iter().find_map(|a| match a {
                    RouteAttribute::Oif(id) => Some(*id),
                    _ => None,
                }) {
                    let mut addrs = handle.address().get().set_link_index_filter(oif).execute();
                    while let Some(addr) = addrs.next().await {
                        if let Ok(addr) = addr
                            && let Some(ip) = addr.attributes.iter().find_map(|a| match a {
                                AddressAttribute::Address(IpAddr::V4(ip)) => Some(*ip),
                                _ => None,
                            })
                        {
                            debug!("Found default route via interface {} with address {}", oif, ip);
                            return Ok(ip);
                        }
                    }
                }
            }
        }

        Err(anyhow!(i18n::tr!("error-cannot-determine-ip")))
    }

    async fn delete_device(&self, device_name: &str) -> anyhow::Result<()> {
        let handle = super::new_netlink_connection()?;
        if let Ok(index) = super::resolve_device_index(&handle, device_name).await {
            super::run_netlink_op(handle.link().del(index).execute(), libc::ENOENT).await?;
        }
        Ok(())
    }

    async fn configure_device(&self, device_config: &DeviceConfig) -> anyhow::Result<()> {
        debug!("Configuring device: {:?}", device_config);

        if let Ok(connection) = Connection::system().await
            && let Ok(nm_proxy) = NetworkManagerProxy::new(&connection).await
            && let Ok(device_path) = nm_proxy.get_device_by_ip_iface(&device_config.name).await
            && let Ok(device_proxy) = NetworkManagerDeviceProxy::builder(&connection)
                .path(device_path)?
                .build()
                .await
        {
            debug!("NM: setting device {} to unmanaged", device_config.name);
            device_proxy.set_managed(false).await?;
        }

        let handle = super::new_netlink_connection()?;

        let index = super::resolve_device_index(&handle, &device_config.name).await?;
        let msg = LinkMessageBuilder::<LinkUnspec>::default()
            .index(index)
            .mtu(device_config.mtu as u32)
            .build();
        handle.link().set(msg).execute().await?;

        handle
            .address()
            .add(
                index,
                device_config.address.addr().into(),
                device_config.address.prefix_len(),
            )
            .execute()
            .await?;

        let forwarding = if device_config.allow_forwarding { "1" } else { "0" };

        let prefix = format!("net.ipv4.conf.{}", device_config.name);
        let key_rp = format!("{prefix}.rp_filter");
        let key_promote = format!("{prefix}.promote_secondaries");
        let key_forwarding = format!("{prefix}.forwarding");
        let key_disable_policy = format!("{prefix}.disable_policy");

        tokio::try_join!(
            // reverse path filter must be disabled (0) or loose (2) for IPsec tunnel
            sysctl_set(key_rp, "2"),
            // promote secondaries must be enabled to allow IP address changes
            sysctl_set(key_promote, "1"),
            // forwarding depends on the user setting
            sysctl_set(key_forwarding, forwarding),
            // when forwarding is enabled, disable IPsec policy enforcement
            sysctl_set(key_disable_policy, forwarding),
        )?;

        Ok(())
    }

    async fn replace_ip_address(
        &self,
        device_name: &str,
        old_address: Ipv4Net,
        new_address: Ipv4Net,
    ) -> anyhow::Result<()> {
        let handle = super::new_netlink_connection()?;
        let index = super::resolve_device_index(&handle, device_name).await?;

        handle
            .address()
            .add(index, new_address.addr().into(), new_address.prefix_len())
            .execute()
            .await?;

        handle
            .address()
            .del(
                AddressMessageBuilder::<Ipv4Addr>::new()
                    .index(index)
                    .address(old_address.addr(), old_address.prefix_len())
                    .build(),
            )
            .execute()
            .await?;

        Ok(())
    }

    fn new_stats_poller(
        &self,
        device_name: &str,
    ) -> impl Future<Output = anyhow::Result<impl StatsPoller + Send + Sync + 'static>> + Send {
        let device_name = device_name.to_owned();
        async move { LinuxStatsPoller::new(&device_name).await }
    }

    fn is_online(&self) -> bool {
        ONLINE_STATE.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_default_ip() {
        let ip = LinuxNetworkInterface.get_default_ipv4().await;
        println!("{ip:?}");
    }
}
