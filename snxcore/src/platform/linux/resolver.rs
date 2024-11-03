use std::{fs, path::PathBuf};

use async_trait::async_trait;
use tracing::debug;

const RESOLV_CONF: &str = "/etc/resolv.conf";

#[derive(Clone, Copy, Debug, PartialEq)]
enum ResolverType {
    SystemdResolved,
    ResolvConf,
}

#[async_trait]
pub trait ResolverConfigurator {
    async fn configure_interface(&self, device: &str) -> anyhow::Result<()> {
        let _ = crate::util::run_command("nmcli", ["device", "set", device, "managed", "no"]).await;
        Ok(())
    }

    async fn configure_dns_suffixes(&self, device: &str, suffixes: &[String]) -> anyhow::Result<()>;

    async fn configure_dns_servers(&self, device: &str, servers: &[String]) -> anyhow::Result<()>;
}

struct SystemdResolvedConfigurator;

#[async_trait]
impl ResolverConfigurator for SystemdResolvedConfigurator {
    async fn configure_dns_suffixes(&self, device: &str, suffixes: &[String]) -> anyhow::Result<()> {
        let mut args = vec!["domain", device];

        let suffixes = suffixes.iter().map(|s| s.trim()).collect::<Vec<_>>();

        args.extend(suffixes);

        crate::util::run_command("resolvectl", args).await?;
        crate::util::run_command("resolvectl", ["default-route", device, "false"]).await?;

        Ok(())
    }

    async fn configure_dns_servers(&self, device: &str, servers: &[String]) -> anyhow::Result<()> {
        let mut args = vec!["dns", device];

        let servers = servers.iter().map(|s| s.trim()).collect::<Vec<_>>();

        args.extend(servers);

        crate::util::run_command("resolvectl", args).await?;

        Ok(())
    }
}

pub fn new_resolver_configurator() -> anyhow::Result<Box<dyn ResolverConfigurator + Send + Sync>> {
    match detect_resolver()? {
        ResolverType::SystemdResolved => Ok(Box::new(SystemdResolvedConfigurator)),
        ResolverType::ResolvConf => Ok(Box::new(ResolvConfConfigurator(RESOLV_CONF.into()))),
    }
}

fn detect_resolver() -> anyhow::Result<ResolverType> {
    let conf_link = fs::read_link(RESOLV_CONF)?;
    let mut resolver_type = ResolverType::ResolvConf;

    for component in conf_link.components() {
        if let Some("systemd") = component.as_os_str().to_str() {
            resolver_type = ResolverType::SystemdResolved;
        }
    }

    debug!("Detected resolver: {:?}", resolver_type);

    Ok(resolver_type)
}

// Fallback resolver to modify resolv.conf directly
struct ResolvConfConfigurator(PathBuf);

#[async_trait]
impl ResolverConfigurator for ResolvConfConfigurator {
    async fn configure_dns_suffixes(&self, _device: &str, suffixes: &[String]) -> anyhow::Result<()> {
        let conf = fs::read_to_string(&self.0)?;
        let mut found = false;
        let mut lines = Vec::new();
        let suffixes = suffixes.join(" ");

        for line in conf.lines() {
            if line.starts_with("search ") && !line.contains(&suffixes) {
                lines.push(format!("{} {}", line, suffixes));
                found = true;
            } else {
                lines.push(line.to_owned());
            }
        }

        if !found {
            lines.push(format!("search {}", suffixes));
        }

        fs::write(&self.0, format!("{}\n", lines.join("\n")))?;

        Ok(())
    }

    async fn configure_dns_servers(&self, _device: &str, servers: &[String]) -> anyhow::Result<()> {
        let conf = fs::read_to_string(&self.0)?;
        let mut lines = Vec::new();

        let mut servers = servers.to_vec();

        for line in conf.lines() {
            if line.starts_with("nameserver ") {
                servers.retain(|s| !line.contains(s));
            }
            lines.push(line.to_owned());
        }

        for server in servers {
            lines.push(format!("nameserver {}", server));
        }

        fs::write(&self.0, format!("{}\n", lines.join("\n")))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_resolver() {
        let resolver = detect_resolver().expect("Failed to detect resolver");
        println!("{:?}", resolver);
    }

    #[tokio::test]
    async fn test_resolv_conf_configurator() {
        let conf = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        fs::write(&conf, "# comment\nnameserver 10.0.0.1\nsearch acme.com\n").unwrap();

        let cut = ResolvConfConfigurator(conf.to_owned());

        cut.configure_dns_servers("", &["192.168.1.1".to_owned(), "192.168.1.2".to_owned()])
            .await
            .unwrap();

        cut.configure_dns_suffixes("", &["dom1.com".to_owned(), "dom2.net".to_owned()])
            .await
            .unwrap();

        let new_conf = fs::read_to_string(&conf).unwrap();
        assert_eq!(new_conf, "# comment\nnameserver 10.0.0.1\nsearch acme.com dom1.com dom2.net\nnameserver 192.168.1.1\nnameserver 192.168.1.2\n");
    }
}
