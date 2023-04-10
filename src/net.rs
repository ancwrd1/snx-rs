use std::process::ExitStatus;

use anyhow::anyhow;
use tracing::warn;

#[cfg(target_os = "linux")]
pub use linux::{add_dns_servers, add_dns_suffixes, add_route};

#[cfg(target_os = "macos")]
pub use macos::{add_dns_servers, add_dns_suffixes, add_route};

fn status_or_error(status: ExitStatus) -> anyhow::Result<ExitStatus> {
    if status.success() {
        Ok(status)
    } else {
        warn!("Command failed: {}", status);
        Err(anyhow!("Command failed: {}", status))
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::net::Ipv4Addr;

    use crate::net::status_or_error;

    pub async fn add_route(
        target_net: &str,
        device: &str,
        _ipaddr: &Ipv4Addr,
    ) -> anyhow::Result<()> {
        let status = tokio::process::Command::new("ip")
            .args(["route", "add", target_net, "dev", device])
            .status()
            .await?;

        status_or_error(status)?;

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

        let status = tokio::process::Command::new("resolvectl")
            .args(args)
            .status()
            .await?;

        status_or_error(status)?;

        Ok(())
    }

    pub async fn add_dns_servers<I, T>(servers: I, device: &str) -> anyhow::Result<()>
    where
        I: IntoIterator<Item = T>,
        T: AsRef<str>,
    {
        let mut args = vec!["dns", device];

        let servers = servers
            .into_iter()
            .map(|s| s.as_ref().to_owned())
            .collect::<Vec<_>>();

        args.extend(servers.iter().map(|s| s.as_str()));

        let status = tokio::process::Command::new("resolvectl")
            .args(args)
            .status()
            .await?;

        status_or_error(status)?;

        Ok(())
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::net::Ipv4Addr;

    use crate::net::status_or_error;

    pub async fn add_route(
        target_net: &str,
        _device: &str,
        ipaddr: &Ipv4Addr,
    ) -> anyhow::Result<()> {
        let ip_str = ipaddr.to_string();

        let status = tokio::process::Command::new("route")
            .args(["add", "-net", target_net, ip_str.as_str()])
            .status()
            .await?;

        status_or_error(status)?;

        Ok(())
    }

    pub async fn add_dns_suffixes<I, T>(suffixes: I, device: &str) -> anyhow::Result<()>
    where
        I: IntoIterator<Item = T>,
        T: AsRef<str>,
    {
        let mut args = vec!["-setsearchdomains", device];

        let suffixes = suffixes
            .into_iter()
            .map(|s| s.as_ref().to_owned())
            .collect::<Vec<_>>();

        args.extend(suffixes.iter().map(|s| s.as_str()));

        let status = tokio::process::Command::new("networksetup")
            .args(args)
            .status()
            .await?;

        status_or_error(status)?;

        Ok(())
    }

    pub async fn add_dns_servers<I, T>(servers: I, device: &str) -> anyhow::Result<()>
    where
        I: IntoIterator<Item = T>,
        T: AsRef<str>,
    {
        let mut args = vec!["-setdnsservers", device];

        let servers = servers
            .into_iter()
            .map(|s| s.as_ref().to_owned())
            .collect::<Vec<_>>();

        args.extend(servers.iter().map(|s| s.as_str()));

        let status = tokio::process::Command::new("networksetup")
            .args(args)
            .status()
            .await?;

        status_or_error(status)?;

        Ok(())
    }
}
