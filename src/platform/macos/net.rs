use std::net::Ipv4Addr;

use anyhow::anyhow;
use tracing::debug;

pub async fn start_network_state_monitoring() -> anyhow::Result<()> {
    Ok(())
}

pub fn is_online() -> bool {
    true
}

pub async fn add_route(route: &str, _device: &str, ipaddr: Ipv4Addr) -> anyhow::Result<()> {
    let ip_str = ipaddr.to_string();

    debug!("Adding route: {} via {}", route, ip_str);
    crate::util::run_command("route", ["add", "-net", route, &ip_str]).await?;

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
