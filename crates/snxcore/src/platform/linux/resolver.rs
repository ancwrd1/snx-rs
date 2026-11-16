use std::{
    fs,
    io::{ErrorKind, Write},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

use anyhow::{Context, anyhow};
use async_trait::async_trait;
use cached::cached;
use i18n::tr;
use itertools::Itertools;
use tracing::{debug, warn};
use zbus::{Connection, fdo::DBusProxy, names::BusName};

use crate::{
    model::params::DnsResolver,
    platform::{ResolverConfig, ResolverConfigurator},
};

const RESOLV_CONF: &str = "/etc/resolv.conf";
const RESOLVED_BUS_NAME: &str = "org.freedesktop.resolve1";
const AF_INET: i32 = 2;
const UNKNOWN_METHOD_ERROR: &str = "org.freedesktop.DBus.Error.UnknownMethod";

#[derive(Clone, Debug, PartialEq)]
enum ResolverType {
    SystemdResolved,
    ResolvConf(PathBuf),
}

#[zbus::proxy(
    interface = "org.freedesktop.resolve1.Manager",
    default_service = "org.freedesktop.resolve1",
    default_path = "/org/freedesktop/resolve1"
)]
pub trait Resolve1Manager {
    #[zbus(name = "SetLinkDNS")]
    fn set_link_dns(&self, ifindex: i32, addresses: Vec<(i32, Vec<u8>)>) -> zbus::Result<()>;
    fn set_link_domains(&self, ifindex: i32, domains: Vec<(String, bool)>) -> zbus::Result<()>;
    fn set_link_default_route(&self, ifindex: i32, enable: bool) -> zbus::Result<()>;
    #[zbus(name = "SetLinkMulticastDNS")]
    fn set_link_multicast_dns(&self, ifindex: i32, mode: &str) -> zbus::Result<()>;
    #[zbus(name = "SetLinkLLMNR")]
    fn set_link_llmnr(&self, ifindex: i32, mode: &str) -> zbus::Result<()>;
    #[zbus(name = "SetLinkDNSOverTLS")]
    fn set_link_dns_over_tls(&self, ifindex: i32, mode: &str) -> zbus::Result<()>;
}

struct SystemdResolvedConfigurator {
    device: String,
}

#[async_trait]
impl ResolverConfigurator for SystemdResolvedConfigurator {
    async fn configure(&self, config: &ResolverConfig) -> anyhow::Result<()> {
        let handle = super::new_netlink_connection()?;
        let ifindex = super::resolve_device_index(&handle, &self.device).await? as i32;

        let connection = Connection::system().await?;
        let proxy = Resolve1ManagerProxy::new(&connection).await?;

        let domains: Vec<(String, bool)> = config
            .search_domains
            .iter()
            .map(|s| (s.name.clone(), s.is_routing))
            .collect();

        proxy.set_link_domains(ifindex, domains).await?;

        // SetLinkDefaultRoute is only available since systemd 240 (e.g. missing on RHEL 8)
        match proxy.set_link_default_route(ifindex, !config.split_dns).await {
            Err(zbus::Error::MethodError(name, _, _)) if name.as_str() == UNKNOWN_METHOD_ERROR => {
                warn!("systemd-resolved does not support SetLinkDefaultRoute, skipping");
            }
            other => other?,
        }

        proxy.set_link_multicast_dns(ifindex, "no").await?;
        proxy.set_link_llmnr(ifindex, "no").await?;
        proxy.set_link_dns_over_tls(ifindex, "no").await?;

        let addresses: Vec<(i32, Vec<u8>)> = config
            .dns_servers
            .iter()
            .map(|ip| (AF_INET, ip.octets().to_vec()))
            .collect();

        proxy.set_link_dns(ifindex, addresses).await?;

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
    Ok(Box::new(LinuxResolverConfigurator {
        device: device.as_ref().to_owned(),
    }))
}

struct LinuxResolverConfigurator {
    device: String,
}

impl LinuxResolverConfigurator {
    async fn select(&self, resolver: DnsResolver) -> anyhow::Result<Box<dyn ResolverConfigurator + Send + Sync>> {
        let resolver_type = match resolver {
            DnsResolver::AutoDetect => autodetect_resolver().await?,
            DnsResolver::SystemdResolved => ResolverType::SystemdResolved,
            DnsResolver::ResolvConf => ResolverType::ResolvConf(read_symlinks(RESOLV_CONF.into(), 10)?),
        };

        debug!("Using resolver: {:?}", resolver_type);

        Ok(match resolver_type {
            ResolverType::SystemdResolved => Box::new(SystemdResolvedConfigurator {
                device: self.device.clone(),
            }),
            ResolverType::ResolvConf(path) => Box::new(ResolvConfConfigurator { config_path: path }),
        })
    }
}

#[async_trait]
impl ResolverConfigurator for LinuxResolverConfigurator {
    async fn configure(&self, config: &ResolverConfig) -> anyhow::Result<()> {
        self.select(config.resolver).await?.configure(config).await
    }

    async fn cleanup(&self, config: &ResolverConfig) -> anyhow::Result<()> {
        self.select(config.resolver).await?.cleanup(config).await
    }
}

// systemd-resolved can only manage links in its own network namespace. From another one (`ip netns exec`)
// it must not be used: our ifindex means a different link there, and its stub listener is unreachable.
async fn autodetect_resolver() -> anyhow::Result<ResolverType> {
    let detected = detect_resolver(RESOLV_CONF.into())?;
    if detected != ResolverType::SystemdResolved {
        return Ok(detected);
    }

    let pid = match resolved_pid().await {
        Ok(pid) => pid,
        Err(e) => {
            warn!("Cannot query systemd-resolved process: {e}, assuming the same network namespace");
            return Ok(detected);
        }
    };

    let proc_dir = PathBuf::from(format!("/proc/{pid}"));

    match is_same_file("/proc/self/ns/net", proc_dir.join("ns/net")) {
        Ok(true) => return Ok(detected),
        Ok(false) => debug!("systemd-resolved runs in another network namespace"),
        Err(e) => {
            warn!("Cannot compare network namespaces: {e}, assuming the same network namespace");
            return Ok(detected);
        }
    }

    // Writing is only safe when the namespace has its own copy of the file, such as the one
    // `ip netns exec` bind-mounts from /etc/netns/<name>/resolv.conf; otherwise it is the host's.
    let conf_path = read_symlinks(RESOLV_CONF.into(), 10)?.canonicalize()?;
    let resolved_view = proc_dir
        .join("root")
        .join(conf_path.strip_prefix("/").unwrap_or(&conf_path));

    match is_same_file(&conf_path, &resolved_view) {
        Ok(false) => Ok(ResolverType::ResolvConf(conf_path)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(ResolverType::ResolvConf(conf_path)),
        _ => Err(anyhow!(tr!("error-netns-shared-resolv-conf"))),
    }
}

async fn resolved_pid() -> anyhow::Result<u32> {
    let connection = Connection::system().await?;
    let proxy = DBusProxy::new(&connection).await?;
    Ok(proxy
        .get_connection_unix_process_id(BusName::try_from(RESOLVED_BUS_NAME)?)
        .await?)
}

fn is_same_file<P1: AsRef<Path>, P2: AsRef<Path>>(path1: P1, path2: P2) -> std::io::Result<bool> {
    let meta1 = fs::metadata(path1)?;
    let meta2 = fs::metadata(path2)?;
    Ok(meta1.dev() == meta2.dev() && meta1.ino() == meta2.ino())
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

#[cached]
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
            .map(|s| format!("nameserver {s}"))
            .collect::<Vec<_>>();

        let mut search = conf
            .lines()
            .filter(|line| line.starts_with("search"))
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();

        let search_domains = config.search_domains.iter().map(|s| &s.name).join(" ");

        if configure {
            if search.is_empty() {
                search.push(format!("search {search_domains}"));
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
    use std::{
        io::{Error, ErrorKind},
        path::Path,
    };

    use super::*;
    use crate::platform::SearchDomain;

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

        println!("{error:#}");

        assert_eq!(
            format!("{error}"),
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

        println!("{error:#}");

        assert_eq!(
            format!("{error}"),
            format!("Failed to resolve symlink '{}'", symlink1.display())
        );
        assert_eq!(
            format!("{}", error.source().unwrap()),
            format!("Failed to resolve symlink '{}'", symlink2.display())
        );

        let root_cause = format!("{}", error.root_cause());
        assert!(
            root_cause.contains("possible loop"),
            "'{root_cause}' should contain 'possible loop'"
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
            split_dns: false,
            search_domains: vec![
                SearchDomain::new("dom1.com", true),
                SearchDomain::new("dom2.net", false),
            ],
            dns_servers: vec!["192.168.1.1".parse().unwrap(), "192.168.1.2".parse().unwrap()],
            ..Default::default()
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
            split_dns: false,
            search_domains: vec![
                SearchDomain::new("dom1.com", true),
                SearchDomain::new("dom2.net", false),
            ],
            dns_servers: vec!["192.168.1.1".parse().unwrap(), "192.168.1.2".parse().unwrap()],
            ..Default::default()
        };

        cut.cleanup(&config).await.unwrap();

        let new_conf = fs::read_to_string(&conf).unwrap();
        assert_eq!(new_conf, "# comment\nsearch acme.com\nnameserver 10.0.0.1\n");
    }

    #[tokio::test]
    #[ignore = "queries the host's systemd-resolved; run as root with --ignored, optionally under `ip netns exec`"]
    async fn autodetect_resolver_in_current_namespace() {
        println!("Detected resolver: {:?}", autodetect_resolver().await);
    }
}
