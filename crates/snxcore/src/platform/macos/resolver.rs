use anyhow::anyhow;
use async_trait::async_trait;
use system_configuration::{
    core_foundation::{array::CFArray, base::TCFType, dictionary::CFDictionary, string::CFString},
    dynamic_store::SCDynamicStoreBuilder,
};
use tracing::{debug, warn};

use crate::platform::{ResolverConfig, ResolverConfigurator};

// SCDynamicStore DNS dictionary keys: literal values of the documented kSCPropNetDNS* constants.
const KEY_SERVER_ADDRESSES: &str = "ServerAddresses";
const KEY_SEARCH_DOMAINS: &str = "SearchDomains";
const KEY_SUPPLEMENTAL_MATCH_DOMAINS: &str = "SupplementalMatchDomains";

const STORE_NAME: &str = "snx-rs";

pub struct MacosResolverConfigurator {
    store_key: String,
}

fn cf_string_array<I: IntoIterator<Item = String>>(values: I) -> CFArray<CFString> {
    let items = values.into_iter().map(|s| CFString::new(&s)).collect::<Vec<_>>();
    CFArray::from_CFTypes(&items)
}

#[async_trait]
impl ResolverConfigurator for MacosResolverConfigurator {
    async fn configure(&self, config: &ResolverConfig) -> anyhow::Result<()> {
        debug!(
            "Configuring DNS at {}: servers={:?}, search={:?}, split_dns={}",
            self.store_key, config.dns_servers, config.search_domains, config.split_dns
        );

        let store = SCDynamicStoreBuilder::new(STORE_NAME)
            .build()
            .ok_or_else(|| anyhow!(i18n::tr!("error-no-service-connection")))?;

        let servers = cf_string_array(config.dns_servers.iter().map(ToString::to_string));
        let domains = cf_string_array(config.search_domains.iter().map(|d| d.name.clone()));

        // Split DNS publishes a supplemental scoped resolver so only the pushed domains resolve
        // through the tunnel; otherwise the domains become a global search list.
        let domain_key = if config.split_dns {
            KEY_SUPPLEMENTAL_MATCH_DOMAINS
        } else {
            KEY_SEARCH_DOMAINS
        };

        let dns_dict = CFDictionary::from_CFType_pairs(&[
            (CFString::new(KEY_SERVER_ADDRESSES), servers.into_CFType()),
            (CFString::new(domain_key), domains.into_CFType()),
        ])
        .into_untyped();

        if store.set(self.store_key.as_str(), dns_dict) {
            Ok(())
        } else {
            warn!("Failed to write DNS configuration to {}", self.store_key);
            Err(anyhow!(i18n::tr!("error-no-service-connection")))
        }
    }

    async fn cleanup(&self, _config: &ResolverConfig) -> anyhow::Result<()> {
        debug!("Cleaning up DNS at {}", self.store_key);

        let store = SCDynamicStoreBuilder::new(STORE_NAME)
            .build()
            .ok_or_else(|| anyhow!(i18n::tr!("error-no-service-connection")))?;

        // Cleanup is idempotent: a false result means the key was already gone.
        if !store.remove(self.store_key.as_str()) {
            debug!("DNS key {} was already absent", self.store_key);
        }
        Ok(())
    }
}

pub fn new_resolver_configurator<S>(device: S) -> anyhow::Result<Box<dyn ResolverConfigurator + Send + Sync>>
where
    S: AsRef<str>,
{
    Ok(Box::new(MacosResolverConfigurator {
        store_key: format!("State:/Network/Service/snx-rs-{}/DNS", device.as_ref()),
    }))
}

// Remove DNS keys left by a previous instance that exited before Drop ran (a panic aborts, launchd
// restarts us); otherwise a stale supplemental resolver keeps split-DNS broken. Store resets on reboot.
pub(super) fn cleanup_stale_dns() {
    let Some(store) = SCDynamicStoreBuilder::new(STORE_NAME).build() else {
        return;
    };
    let Some(keys) = store.get_keys(format!("State:/Network/Service/{STORE_NAME}-[^/]+/DNS").as_str()) else {
        return;
    };
    for key in keys.iter() {
        let key = key.to_string();
        if store.remove(key.as_str()) {
            debug!("Removed stale DNS configuration {key}");
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn cf_string_array_length_matches_input() {
        assert_eq!(super::cf_string_array(Vec::<String>::new()).len(), 0);
        assert_eq!(
            super::cf_string_array(vec![
                "a.example".to_string(),
                "b.example".to_string(),
                "c.example".to_string()
            ])
            .len(),
            3
        );
    }
}
