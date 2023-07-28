#[cfg(target_os = "linux")]
pub use linux::{
    add_default_route, add_dns_servers, add_dns_suffixes, add_route, get_default_ip, is_online,
    start_network_state_monitoring,
};
#[cfg(target_os = "macos")]
pub use macos::{
    add_default_route, add_dns_servers, add_dns_suffixes, add_route, get_default_ip, is_online,
    start_network_state_monitoring,
};

#[cfg(target_os = "linux")]
mod linux {
    use std::sync::atomic::Ordering;
    use std::{net::Ipv4Addr, sync::atomic::AtomicBool};

    use anyhow::anyhow;
    use futures::StreamExt;
    use ipnet::Ipv4Subnets;
    use tracing::debug;
    use zbus::{dbus_proxy, Connection};

    use crate::model::snx::NetworkRange;

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
        #[dbus_proxy(signal)]
        fn state_changed(&self, state: u32) -> zbus::Result<()>;
    }

    pub async fn start_network_state_monitoring() -> anyhow::Result<()> {
        let connection = Connection::system().await?;
        let proxy = NetworkManagerProxy::new(&connection).await?;

        let mut stream = proxy.receive_state_changed().await?;
        tokio::spawn(async move {
            while let Some(signal) = stream.next().await {
                let state: NetworkManagerState = signal.args()?.state.into();
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

    pub async fn get_default_ip() -> anyhow::Result<String> {
        let result = crate::util::run_command("ip", ["route", "show", "default"]).await?;
        let mut parts = result.split_whitespace();
        while let Some(part) = parts.next() {
            if part == "src" {
                if let Some(ip) = parts.next() {
                    return Ok(ip.to_owned());
                }
            }
        }
        Err(anyhow!("Cannot determine default IP!"))
    }

    pub async fn add_route(range: &NetworkRange, device: &str, ipaddr: Ipv4Addr) -> anyhow::Result<()> {
        let subnets = Ipv4Subnets::new(range.from, range.to, 0);
        for subnet in subnets {
            if subnet.contains(&ipaddr) {
                let snet = subnet.to_string();
                debug!("Adding route: {} via {}", snet, device);
                crate::util::run_command("ip", ["route", "add", &snet, "dev", device]).await?;
            }
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

        let suffixes = suffixes
            .into_iter()
            .map(|s| format!("~{}", s.as_ref()))
            .collect::<Vec<_>>();

        args.extend(suffixes.iter().map(|s| s.as_str()));

        crate::util::run_command("resolvectl", args).await?;

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
}

#[cfg(target_os = "macos")]
mod macos {
    use std::net::Ipv4Addr;

    use anyhow::anyhow;
    use ipnet::Ipv4Subnets;
    use tracing::debug;

    use crate::model::snx::NetworkRange;

    pub async fn start_network_state_monitoring() -> anyhow::Result<()> {
        Ok(())
    }

    pub fn is_online() -> bool {
        true
    }

    pub async fn add_route(range: &NetworkRange, _device: &str, ipaddr: Ipv4Addr) -> anyhow::Result<()> {
        let ip_str = ipaddr.to_string();

        let subnets = Ipv4Subnets::new(range.from, range.to, 0);
        for subnet in subnets {
            if subnet.contains(&ipaddr) {
                let snet = subnet.to_string();
                debug!("Adding route: {} via {}", snet, ip_str);
                crate::util::run_command("route", ["add", "-net", &snet, &ip_str]).await?;
            }
        }

        Ok(())
    }

    pub async fn add_default_route(_device: &str, ipaddr: Ipv4Addr) -> anyhow::Result<()> {
        let ip_str = ipaddr.to_string();
        crate::util::run_command("route", ["add", "-net", "default", &ip_str]).await?;

        Ok(())
    }

    pub async fn add_dns_suffixes<I, T>(suffixes: I, device: &str) -> anyhow::Result<()>
    where
        I: IntoIterator<Item = T>,
        T: AsRef<str>,
    {
        let mut args = vec!["-setsearchdomains", device];

        let suffixes = suffixes.into_iter().map(|s| s.as_ref().to_owned()).collect::<Vec<_>>();

        args.extend(suffixes.iter().map(|s| s.as_str()));

        crate::util::run_command("networksetup", args).await?;

        Ok(())
    }

    pub async fn add_dns_servers<I, T>(servers: I, device: &str) -> anyhow::Result<()>
    where
        I: IntoIterator<Item = T>,
        T: AsRef<str>,
    {
        let mut args = vec!["-setdnsservers", device];

        let servers = servers.into_iter().map(|s| s.as_ref().to_owned()).collect::<Vec<_>>();

        args.extend(servers.iter().map(|s| s.as_str()));

        crate::util::run_command("networksetup", args).await?;

        Ok(())
    }

    pub async fn get_default_ip() -> anyhow::Result<String> {
        Err(anyhow!("Cannot determine default IP!"))
    }
}
