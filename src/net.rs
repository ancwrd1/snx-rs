#[cfg(target_os = "linux")]
pub use linux::{add_default_route, add_dns_servers, add_dns_suffixes, add_route, get_default_ip};
#[cfg(target_os = "macos")]
pub use macos::{add_default_route, add_dns_servers, add_dns_suffixes, add_route, get_default_ip};

#[cfg(target_os = "linux")]
mod linux {
    use std::net::Ipv4Addr;

    use anyhow::anyhow;
    use ipnet::Ipv4Subnets;
    use tracing::debug;

    use crate::model::NetworkRange;

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

    use crate::model::NetworkRange;

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
