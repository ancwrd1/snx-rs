use anyhow::anyhow;
use async_trait::async_trait;
use std::fs;
use tracing::debug;

#[derive(Clone, Copy, Debug, PartialEq)]
enum ResolverType {
    SystemdResolved,
    NetworkManager,
}

#[async_trait]
pub trait ResolverConfigurator {
    async fn configure_device(&self, device: &str) -> anyhow::Result<()>;
    async fn configure_dns_suffixes(&self, device: &str, suffixes: &[String]) -> anyhow::Result<()>;

    async fn configure_dns_servers(&self, device: &str, servers: &[String]) -> anyhow::Result<()>;
}

struct SystemdResolvedConfigurator;

#[async_trait]
impl ResolverConfigurator for SystemdResolvedConfigurator {
    async fn configure_device(&self, device: &str) -> anyhow::Result<()> {
        crate::util::run_command("nmcli", ["device", "set", device, "managed", "no"]).await?;
        Ok(())
    }

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
        other => Err(anyhow!("Resolver {:?} is not supported yet", other)),
    }
}

fn detect_resolver() -> anyhow::Result<ResolverType> {
    let conf_link = fs::read_link("/etc/resolv.conf")?;
    let resolver_type = conf_link
        .components()
        .find_map(|component| match component.as_os_str().to_str() {
            Some("systemd") => Some(ResolverType::SystemdResolved),
            Some("NetworkManager") => Some(ResolverType::NetworkManager),
            _ => None,
        });

    debug!("Detected resolver: {:?}", resolver_type);

    resolver_type.ok_or_else(|| anyhow!("No supported resolver found"))
}
