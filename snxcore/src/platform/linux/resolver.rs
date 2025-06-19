use std::{fs, io::Write, path::PathBuf};

use anyhow::Context;
use async_trait::async_trait;
use cached::proc_macro::cached;
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

        let search_domains = config
            .search_domains
            .iter()
            .map(|s| s.trim_matches(|c: char| c.is_whitespace() || c == '.'))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();

        args.extend(search_domains);

        crate::util::run_command("resolvectl", args).await?;
        crate::util::run_command("resolvectl", ["default-route", &self.device, "false"]).await?;

        let mut args = vec!["dns".to_owned(), self.device.clone()];

        let servers = config.dns_servers.iter().map(|s| s.to_string()).collect::<Vec<_>>();

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
    match detect_resolver(RESOLV_CONF.into())? {
        ResolverType::SystemdResolved => Ok(Box::new(SystemdResolvedConfigurator {
            device: device.as_ref().to_owned(),
        })),
        ResolverType::ResolvConf(path) => Ok(Box::new(ResolvConfConfigurator { config_path: path })),
    }
}

// In some distros (NixOS, for example), /etc/resolv.conf is doubly linked.
// So, we must follow symbolic links until we find a real file.
// But we'll stop following after 10 hoops because we don't want to fall into
// a circular reference loop.
fn read_symlinks(path: PathBuf, depth: u8) -> anyhow::Result<PathBuf> {
    if depth == 0 {
        Err(anyhow::anyhow!(
            "Cannot resolve symlink '{}', possible loop",
            path.display()
        ))
    } else {
        let metadata = fs::symlink_metadata(&path)
            .with_context(|| format!("Failed to get symlink metadata of '{}'", path.display()))?;
        if metadata.is_symlink() {
            let link_target =
                fs::read_link(&path).with_context(|| format!("Failed to read symlink target '{}'", path.display()))?;

            let absolute_target = match path.parent() {
                Some(parent) => parent.join(link_target),
                None => link_target,
            };

            read_symlinks(absolute_target, depth - 1)
                .with_context(|| format!("Failed to resolve symlink '{}'", path.display()))
        } else {
            Ok(path)
        }
    }
}

#[cached(result = true)]
fn detect_resolver(path: PathBuf) -> anyhow::Result<ResolverType> {
    let resolve_conf_path = read_symlinks(path, 10)?;

    let result = if resolve_conf_path
        .canonicalize()?
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
            .filter(|line| {
                line.starts_with("nameserver") && !config.dns_servers.iter().any(|s| line.contains(&s.to_string()))
            })
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

        // resolv.conf has no concept of routing domains
        let search_domains = config
            .search_domains
            .iter()
            .map(|s| s.trim_matches(|c: char| c.is_whitespace() || c == '.' || c == '~'))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

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
    use std::io::{Error, ErrorKind};

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

    #[test]
    fn test_detect_resolver_relative_symlink() {
        // <dir>/
        //    etc/
        //       resolv.conf -> ../run/systemd/resolve/stub-resolv.conf
        //    run/
        //       systemd/
        //          resolve/
        //             stub-resolv.conf
        let dir = tempfile::TempDir::new().unwrap();

        let etc = dir.path().join("etc");
        fs::create_dir(&etc).unwrap();

        let run_systemd_resolve = dir.path().join("run").join("systemd").join("resolve");
        fs::create_dir_all(&run_systemd_resolve).unwrap();

        let stub_resolv_conf = run_systemd_resolve.join("stub-resolv.conf");
        fs::write(&stub_resolv_conf, "").unwrap();

        let symlink = etc.join("resolv.conf");
        let relative_target = Path::new("../run/systemd/resolve/stub-resolv.conf");
        std::os::unix::fs::symlink(relative_target, &symlink).unwrap();

        let resolver = detect_resolver(symlink).expect("Failed to detect resolver");
        assert_eq!(resolver, ResolverType::SystemdResolved);
    }

    #[test]
    fn test_detect_resolver_invalid_symlink() {
        // <dir>/
        //    etc/
        //       resolv.conf -> ../nonexistent.conf
        let dir = tempfile::TempDir::new().unwrap();

        let etc = dir.path().join("etc");
        fs::create_dir(&etc).unwrap();

        let symlink = etc.join("resolv.conf");
        let relative_target = Path::new("../nonexistent.conf");
        std::os::unix::fs::symlink(relative_target, &symlink).unwrap();

        let error = detect_resolver(symlink.clone()).expect_err("Invalid symlink should trigger error");

        println!("{:#}", error);

        assert_eq!(
            format!("{}", error),
            format!("Failed to resolve symlink '{}'", symlink.display())
        );
        assert_eq!(
            format!("{}", error.source().unwrap()),
            format!(
                "Failed to get symlink metadata of '{}/../nonexistent.conf'",
                etc.display()
            )
        );

        let cause = error
            .root_cause()
            .downcast_ref::<Error>()
            .expect("Root cause should be an IO error");

        assert_eq!(cause.kind(), ErrorKind::NotFound)
    }

    #[test]
    fn test_detect_resolver_circular_symlink() {
        // <dir>/
        //    etc/
        //       resolv.conf -> resolv2.conf
        //       resolv2.conf -> resolv.conf
        let dir = tempfile::TempDir::new().unwrap();

        let etc = dir.path().join("etc");
        fs::create_dir(&etc).unwrap();

        let symlink1 = etc.join("resolv.conf");
        let symlink2 = etc.join("resolv2.conf");
        std::os::unix::fs::symlink(&symlink1, &symlink2).unwrap();
        std::os::unix::fs::symlink(&symlink2, &symlink1).unwrap();

        let error = detect_resolver(symlink1.clone()).expect_err("Invalid symlink should trigger error");

        println!("{:#}", error);

        assert_eq!(
            format!("{}", error),
            format!("Failed to resolve symlink '{}'", symlink1.display())
        );
        assert_eq!(
            format!("{}", error.source().unwrap()),
            format!("Failed to resolve symlink '{}'", symlink2.display())
        );

        let root_cause = format!("{}", error.root_cause());
        assert!(
            root_cause.contains("possible loop"),
            "'{}' should contain 'possible loop'",
            root_cause
        );
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
            dns_servers: vec!["192.168.1.1".parse().unwrap(), "192.168.1.2".parse().unwrap()],
        };
        cut.configure(&config).await.unwrap();

        let new_conf = fs::read_to_string(&conf).unwrap();
        assert_eq!(
            new_conf,
            "# comment\nsearch acme.com dom1.com dom2.net\nnameserver 192.168.1.1\nnameserver 192.168.1.2\nnameserver 10.0.0.1\n"
        );
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
            dns_servers: vec!["192.168.1.1".parse().unwrap(), "192.168.1.2".parse().unwrap()],
        };

        cut.cleanup(&config).await.unwrap();

        let new_conf = fs::read_to_string(&conf).unwrap();
        assert_eq!(new_conf, "# comment\nsearch acme.com\nnameserver 10.0.0.1\n");
    }
}
