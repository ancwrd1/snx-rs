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
    async fn configure_interface(&self) -> anyhow::Result<()>;
    async fn configure_dns_suffixes(&self, suffixes: &[String], cleanup: bool) -> anyhow::Result<()>;

    async fn configure_dns_servers(&self, servers: &[String], cleanup: bool) -> anyhow::Result<()>;
}

struct SystemdResolvedConfigurator {
    device: String,
}

#[async_trait]
impl ResolverConfigurator for SystemdResolvedConfigurator {
    async fn configure_interface(&self) -> anyhow::Result<()> {
        let _ = crate::util::run_command("nmcli", ["device", "set", &self.device, "managed", "no"]).await;
        Ok(())
    }

    async fn configure_dns_suffixes(&self, suffixes: &[String], cleanup: bool) -> anyhow::Result<()> {
        if !cleanup {
            let mut args = vec!["domain", &self.device];

            let suffixes = suffixes.iter().map(|s| s.trim()).collect::<Vec<_>>();

            args.extend(suffixes);

            crate::util::run_command("resolvectl", args).await?;
            crate::util::run_command("resolvectl", ["default-route", &self.device, "false"]).await?;
        }

        Ok(())
    }

    async fn configure_dns_servers(&self, servers: &[String], cleanup: bool) -> anyhow::Result<()> {
        if !cleanup {
            let mut args = vec!["dns", &self.device];

            let servers = servers.iter().map(|s| s.trim()).collect::<Vec<_>>();

            args.extend(servers);

            crate::util::run_command("resolvectl", args).await?;
        }

        Ok(())
    }
}

pub fn new_resolver_configurator<S>(device: S) -> anyhow::Result<Box<dyn ResolverConfigurator + Send + Sync>>
where
    S: AsRef<str>,
{
    match detect_resolver()? {
        ResolverType::SystemdResolved => Ok(Box::new(SystemdResolvedConfigurator {
            device: device.as_ref().to_owned(),
        })),
        ResolverType::ResolvConf => Ok(Box::new(ResolvConfConfigurator {
            config_path: RESOLV_CONF.into(),
        })),
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
struct ResolvConfConfigurator {
    config_path: PathBuf,
}

#[async_trait]
impl ResolverConfigurator for ResolvConfConfigurator {
    async fn configure_interface(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn configure_dns_suffixes(&self, suffixes: &[String], cleanup: bool) -> anyhow::Result<()> {
        let conf = fs::read_to_string(&self.config_path)?;
        let mut found = false;
        let mut lines = Vec::new();
        let suffixes = suffixes.join(" ");

        for line in conf.lines() {
            if line.starts_with("search ") {
                if cleanup {
                    let trimmed = line.replace(&suffixes, "").trim().to_owned();
                    if trimmed != "search" {
                        lines.push(trimmed);
                    }
                } else if !line.contains(&suffixes) {
                    lines.push(format!("{} {}", line, suffixes));
                    found = true;
                } else {
                    lines.push(line.to_owned());
                }
            } else {
                lines.push(line.to_owned());
            }
        }

        if !found && !cleanup {
            lines.push(format!("search {}", suffixes));
        }

        fs::write(&self.config_path, format!("{}\n", lines.join("\n")))?;

        Ok(())
    }

    async fn configure_dns_servers(&self, servers: &[String], cleanup: bool) -> anyhow::Result<()> {
        let conf = fs::read_to_string(&self.config_path)?;
        let mut lines = Vec::new();

        let mut servers = servers.to_vec();

        for line in conf.lines() {
            if cleanup {
                if line.starts_with("nameserver ") && servers.iter().any(|s| line.contains(s)) {
                    continue;
                }
            } else if line.starts_with("nameserver ") {
                servers.retain(|s| !line.contains(s));
            }
            lines.push(line.to_owned());
        }

        if !cleanup {
            for server in servers {
                lines.push(format!("nameserver {}", server));
            }
        }

        fs::write(&self.config_path, format!("{}\n", lines.join("\n")))?;

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
    async fn test_resolv_conf_configurator_setup() {
        let conf = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        fs::write(&conf, "# comment\nnameserver 10.0.0.1\nsearch acme.com\n").unwrap();

        let cut = ResolvConfConfigurator {
            config_path: conf.to_owned(),
        };

        cut.configure_dns_servers(&["192.168.1.1".to_owned(), "192.168.1.2".to_owned()], false)
            .await
            .unwrap();

        cut.configure_dns_suffixes(&["dom1.com".to_owned(), "dom2.net".to_owned()], false)
            .await
            .unwrap();

        let new_conf = fs::read_to_string(&conf).unwrap();
        assert_eq!(new_conf, "# comment\nnameserver 10.0.0.1\nsearch acme.com dom1.com dom2.net\nnameserver 192.168.1.1\nnameserver 192.168.1.2\n");
    }

    #[tokio::test]
    async fn test_resolv_conf_configurator_cleanup() {
        let conf = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        fs::write(&conf, "# comment\nnameserver 10.0.0.1\nsearch acme.com dom1.com dom2.net\nnameserver 192.168.1.1\nnameserver 192.168.1.2\n").unwrap();

        let cut = ResolvConfConfigurator {
            config_path: conf.to_owned(),
        };

        cut.configure_dns_servers(&["192.168.1.1".to_owned(), "192.168.1.2".to_owned()], true)
            .await
            .unwrap();

        cut.configure_dns_suffixes(&["dom1.com".to_owned(), "dom2.net".to_owned()], true)
            .await
            .unwrap();

        let new_conf = fs::read_to_string(&conf).unwrap();
        assert_eq!(new_conf, "# comment\nnameserver 10.0.0.1\nsearch acme.com\n");
    }
}
