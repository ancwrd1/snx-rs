use std::{fs, io::Write, path::PathBuf};

use async_trait::async_trait;
use tracing::debug;

use crate::platform::{ResolverConfig, ResolverConfigurator};

const RESOLV_CONF: &str = "/etc/resolv.conf";

#[derive(Clone, Copy, Debug, PartialEq)]
enum ResolverType {
    SystemdResolved,
    ResolvConf,
}

struct SystemdResolvedConfigurator {
    device: String,
}

#[async_trait]
impl ResolverConfigurator for SystemdResolvedConfigurator {
    async fn configure(&self, config: &ResolverConfig) -> anyhow::Result<()> {
        let mut args = vec!["domain", &self.device];

        let search_domains = config.search_domains.iter().map(|s| s.trim()).collect::<Vec<_>>();

        args.extend(search_domains);

        crate::util::run_command("resolvectl", args).await?;
        crate::util::run_command("resolvectl", ["default-route", &self.device, "false"]).await?;

        let mut args = vec!["dns", &self.device];

        let servers = config.dns_servers.iter().map(|s| s.trim()).collect::<Vec<_>>();

        args.extend(servers);

        crate::util::run_command("resolvectl", args).await?;

        Ok(())
    }

    async fn cleanup(&self, _config: &ResolverConfig) -> anyhow::Result<()> {
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
    let mut resolver_type = ResolverType::ResolvConf;

    if fs::symlink_metadata(RESOLV_CONF)?.is_symlink() {
        if let Ok(conf_link) = fs::read_link(RESOLV_CONF) {
            for component in conf_link.components() {
                if let Some("systemd") = component.as_os_str().to_str() {
                    resolver_type = ResolverType::SystemdResolved;
                    break;
                }
            }
        }
    }

    debug!("Detected resolver: {:?}", resolver_type);

    Ok(resolver_type)
}

struct ResolvConfConfigurator {
    config_path: PathBuf,
}

impl ResolvConfConfigurator {
    fn configure_or_cleanup(&self, config: &ResolverConfig, configure: bool) -> anyhow::Result<()> {
        let conf = fs::read_to_string(&self.config_path)?;

        let existing_nameservers = conf
            .lines()
            .filter(|line| line.starts_with("nameserver") && !config.dns_servers.iter().any(|s| line.contains(s)))
            .collect::<Vec<_>>();

        let other_lines = conf
            .lines()
            .filter(|line| !line.starts_with("nameserver") && !line.starts_with("search"))
            .collect::<Vec<_>>();

        let new_nameservers = config
            .dns_servers
            .iter()
            .map(|s| format!("nameserver {}", s))
            .collect::<Vec<_>>();

        let mut search = conf
            .lines()
            .filter(|line| line.starts_with("search"))
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();

        let search_domains = config.search_domains.join(" ");

        if configure {
            if search.is_empty() {
                search.push(format!("search {}", search_domains));
            } else if !search.iter().any(|s| s.contains(&search_domains)) {
                search[0] = format!("{} {}", search[0], search_domains);
            }
        } else {
            search = search
                .into_iter()
                .map(|s| s.replace(&search_domains, "").trim().to_owned())
                .filter(|s| s != "search")
                .collect::<Vec<_>>();
        }

        let mut file = fs::File::create(&self.config_path)?;

        writeln!(file, "{}", other_lines.join("\n"))?;
        writeln!(file, "{}", search.join("\n"))?;
        if configure {
            writeln!(file, "{}", new_nameservers.join("\n"))?;
        }
        writeln!(file, "{}", existing_nameservers.join("\n"))?;

        Ok(())
    }
}

#[async_trait]
impl ResolverConfigurator for ResolvConfConfigurator {
    async fn configure(&self, config: &ResolverConfig) -> anyhow::Result<()> {
        Ok(self.configure_or_cleanup(config, true)?)
    }

    async fn cleanup(&self, config: &ResolverConfig) -> anyhow::Result<()> {
        Ok(self.configure_or_cleanup(config, false)?)
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

        let config = ResolverConfig {
            search_domains: vec!["dom1.com".to_owned(), "dom2.net".to_owned()],
            dns_servers: vec!["192.168.1.1".to_owned(), "192.168.1.2".to_owned()],
        };
        cut.configure(&config).await.unwrap();

        let new_conf = fs::read_to_string(&conf).unwrap();
        assert_eq!(new_conf, "# comment\nsearch acme.com dom1.com dom2.net\nnameserver 192.168.1.1\nnameserver 192.168.1.2\nnameserver 10.0.0.1\n");
    }

    #[tokio::test]
    async fn test_resolv_conf_configurator_cleanup() {
        let conf = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        fs::write(&conf, "# comment\nnameserver 10.0.0.1\nsearch acme.com dom1.com dom2.net\nnameserver 192.168.1.1\nnameserver 192.168.1.2\n").unwrap();

        let cut = ResolvConfConfigurator {
            config_path: conf.to_owned(),
        };

        let config = ResolverConfig {
            search_domains: vec!["dom1.com".to_owned(), "dom2.net".to_owned()],
            dns_servers: vec!["192.168.1.1".to_owned(), "192.168.1.2".to_owned()],
        };

        cut.cleanup(&config).await.unwrap();

        let new_conf = fs::read_to_string(&conf).unwrap();
        assert_eq!(new_conf, "# comment\nsearch acme.com\nnameserver 10.0.0.1\n");
    }
}
