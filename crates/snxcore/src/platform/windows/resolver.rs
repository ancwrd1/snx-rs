use anyhow::anyhow;
use async_trait::async_trait;
use tracing::debug;
use windows::{
    Win32::{
        Foundation::NO_ERROR,
        NetworkManagement::{
            IpHelper::{
                ConvertInterfaceAliasToLuid, ConvertInterfaceLuidToGuid, DNS_INTERFACE_SETTINGS,
                DNS_SETTING_NAMESERVER, DNS_SETTING_SEARCHLIST, SetInterfaceDnsSettings,
            },
            Ndis::NET_LUID_LH,
        },
    },
    core::{GUID, PCWSTR, PWSTR},
};

use super::nrpt::Nrpt;
use crate::platform::{ResolverConfig, ResolverConfigurator, SearchDomain};

const DNS_INTERFACE_SETTINGS_VERSION1: u32 = 1;

pub struct WindowsResolverConfigurator {
    device: String,
    nrpt: Nrpt,
}

impl WindowsResolverConfigurator {
    pub fn new<S: AsRef<str>>(device: S) -> Self {
        Self {
            device: device.as_ref().to_owned(),
            nrpt: Nrpt::new(),
        }
    }

    fn apply(&self, nameserver: PWSTR, searchlist: PWSTR) -> anyhow::Result<()> {
        let guid = alias_to_guid(&self.device)?;

        let settings = DNS_INTERFACE_SETTINGS {
            Version: DNS_INTERFACE_SETTINGS_VERSION1,
            Flags: (DNS_SETTING_NAMESERVER | DNS_SETTING_SEARCHLIST) as u64,
            NameServer: nameserver,
            SearchList: searchlist,
            ..Default::default()
        };

        let rc = unsafe { SetInterfaceDnsSettings(guid, &settings) };
        if rc == NO_ERROR {
            Ok(())
        } else {
            Err(anyhow!("SetInterfaceDnsSettings failed: {:?}", rc))
        }
    }
}

fn pwstr_from(buf: &[u16]) -> PWSTR {
    PWSTR(buf.as_ptr() as *mut u16)
}

#[async_trait]
impl ResolverConfigurator for WindowsResolverConfigurator {
    async fn configure(&self, config: &ResolverConfig) -> anyhow::Result<()> {
        debug!(
            "Configuring DNS on {}: servers={:?}, search={:?}, split_dns={}",
            self.device, config.dns_servers, config.search_domains, config.split_dns
        );

        self.nrpt.purge_previous();

        let nameserver = wide_csv(config.dns_servers.iter().map(|s| s.to_string()));

        let searchlist = wide_csv(
            config
                .search_domains
                .iter()
                .filter(|d: &&SearchDomain| !d.is_routing)
                .map(|d| d.name.clone()),
        );

        self.apply(pwstr_from(&nameserver), pwstr_from(&searchlist))?;

        if config.split_dns {
            let routing: Vec<&SearchDomain> = config.search_domains.iter().filter(|d| d.is_routing).collect();
            let servers: Vec<String> = config.dns_servers.iter().map(|s| s.to_string()).collect();
            self.nrpt.install(&routing, &servers)?;
        }

        Ok(())
    }

    async fn cleanup(&self, _config: &ResolverConfig) -> anyhow::Result<()> {
        debug!("Cleaning up DNS on {}", self.device);
        self.nrpt.uninstall();
        self.apply(PWSTR::null(), PWSTR::null())
    }
}

pub fn new_resolver_configurator<S>(device: S) -> anyhow::Result<Box<dyn ResolverConfigurator + Send + Sync>>
where
    S: AsRef<str>,
{
    Ok(Box::new(WindowsResolverConfigurator::new(device)))
}

fn to_wide_nul(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn wide_csv<I, S>(parts: I) -> Vec<u16>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let joined = parts
        .into_iter()
        .map(|p| p.as_ref().to_owned())
        .collect::<Vec<_>>()
        .join(",");

    to_wide_nul(&joined)
}

fn alias_to_guid(alias: &str) -> anyhow::Result<GUID> {
    let wide = to_wide_nul(alias);
    let mut luid = NET_LUID_LH::default();

    unsafe { ConvertInterfaceAliasToLuid(PCWSTR(wide.as_ptr()), &mut luid) }
        .ok()
        .map_err(|e| anyhow!("ConvertInterfaceAliasToLuid({alias}) failed: {e}"))?;

    let mut guid = GUID::default();

    unsafe { ConvertInterfaceLuidToGuid(&luid, &mut guid) }
        .ok()
        .map_err(|e| anyhow!("ConvertInterfaceLuidToGuid failed: {e}"))?;

    Ok(guid)
}
