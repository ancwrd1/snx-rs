use std::{fs, io::Write, path::Path, path::PathBuf};

use async_trait::async_trait;
use tracing::debug;

use crate::platform::{ResolverConfig, ResolverConfigurator};

const RESOLV_CONF: &str = "/etc/resolv.conf";

#[derive(Clone, Debug, PartialEq)]
enum ResolverType {
    SystemdResolved,
    ResolvConf(PathBuf),
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
    match detect_resolver(RESOLV_CONF)? {
        ResolverType::SystemdResolved => Ok(Box::new(SystemdResolvedConfigurator {
            device: device.as_ref().to_owned(),
        })),
        ResolverType::ResolvConf(path) => Ok(Box::new(ResolvConfConfigurator { config_path: path })),
    }
}

fn detect_resolver<P>(path: P) -> anyhow::Result<ResolverType>
where
    P: AsRef<Path>,
{
    // In some distros (NixOS, for example), /etc/resolv.conf is doubly linked.
    // So, we must follow symbolic links until we find a real file.
    // But we'll stop following after 10 hoops, because we don't want to fall into
    // a circular reference loop.

    let mut resolve_conf_path = path.as_ref().to_owned();
    let mut count_links = 0;

    while count_links < 10 && fs::symlink_metadata(&resolve_conf_path)?.is_symlink() {
        resolve_conf_path = fs::read_link(&resolve_conf_path)?;
        count_links += 1;
    }

    let result = if resolve_conf_path
        .components()
        .any(|component| component.as_os_str().to_str() == Some("systemd"))
    {
        ResolverType::SystemdResolved
    } else {
        ResolverType::ResolvConf(resolve_conf_path)
    };

    debug!("Detected resolver: {:?}", result);

    Ok(result)
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
    fn test_detect_resolver_systemd() {
        // <dir>/
        //    systemd/
        //       resolv.conf
        //    resolv-stub.conf -> systemd/resolv.conf
        //    resolf.conf -> resolv-stub.conf
        let dir = tempfile::TempDir::new().unwrap();

        let subdir = dir.path().join("systemd");
        fs::create_dir(&subdir).unwrap();

        let conf_path = subdir.join("resolv.conf");
        fs::write(&conf_path, "").unwrap();

        let symlink1 = dir.path().join("resolv-stub.conf");
        std::os::unix::fs::symlink(&conf_path, &symlink1).unwrap();

        let symlink2 = dir.path().join("resolv.conf");
        std::os::unix::fs::symlink(&symlink1, &symlink2).unwrap();

        let resolver = detect_resolver(symlink2).expect("Failed to detect resolver");
        assert_eq!(resolver, ResolverType::SystemdResolved);
    }

    #[test]
    fn test_detect_resolver_resolvconf() {
        let dir = tempfile::TempDir::new().unwrap();

        let subdir = dir.path().join("NetworkManager");
        fs::create_dir(&subdir).unwrap();

        let conf_path = subdir.join("resolv.conf");
        fs::write(&conf_path, "").unwrap();

        let symlink = dir.path().join("resolv.conf");
        std::os::unix::fs::symlink(&conf_path, &symlink).unwrap();

        let resolver = detect_resolver(symlink).expect("Failed to detect resolver");
        assert_eq!(resolver, ResolverType::ResolvConf(conf_path));
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
