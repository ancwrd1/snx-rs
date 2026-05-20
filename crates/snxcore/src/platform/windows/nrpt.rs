use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use tracing::warn;
use uuid::Uuid;
use winreg::{
    RegKey,
    enums::{HKEY_LOCAL_MACHINE, KEY_WRITE, REG_OPTION_NON_VOLATILE},
};

use crate::platform::{Platform, PlatformAccess, SearchDomain};

const POLICY_BASE: &str = r"SYSTEM\CurrentControlSet\Services\Dnscache\Parameters\DnsPolicyConfig";

#[link(name = "dnsapi")]
unsafe extern "system" {
    fn DnsFlushResolverCache() -> u32;
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    rules: Vec<String>,
}

pub struct Nrpt {
    state_path: PathBuf,
}

impl Nrpt {
    pub fn new() -> Self {
        Self {
            state_path: Platform::get().data_dir().join("nrpt-rules.json"),
        }
    }

    pub fn purge_previous(&self) {
        let Ok(state) = self.load_state() else { return };

        for rule in &state.rules {
            if let Err(e) = delete_rule(rule) {
                warn!("Failed to delete leftover NRPT rule {rule}: {e}");
            }
        }
        if let Err(e) = self.clear_state() {
            warn!("Failed to clear NRPT state file: {e}");
        }
    }

    pub fn install(&self, routing_domains: &[&SearchDomain], dns_servers: &[String]) -> anyhow::Result<()> {
        if routing_domains.is_empty() || dns_servers.is_empty() {
            return Ok(());
        }

        let rule = format!("{{{}}}", Uuid::new_v4());
        create_rule(&rule, routing_domains, dns_servers)?;

        let mut state = self.load_state().unwrap_or_default();
        state.rules.push(rule);
        self.save_state(&state)?;

        flush_dns_cache();
        Ok(())
    }

    pub fn uninstall(&self) {
        self.purge_previous();
        flush_dns_cache();
    }

    fn load_state(&self) -> anyhow::Result<State> {
        let bytes = fs::read(&self.state_path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    fn save_state(&self, state: &State) -> anyhow::Result<()> {
        if let Some(parent) = self.state_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.state_path, serde_json::to_vec_pretty(state)?)?;
        Ok(())
    }

    fn clear_state(&self) -> anyhow::Result<()> {
        fs::remove_file(&self.state_path)?;
        Ok(())
    }
}

fn flush_dns_cache() {
    let rc = unsafe { DnsFlushResolverCache() };
    if rc == 0 {
        warn!("DnsFlushResolverCache failed");
    }
}

fn create_rule(rule_name: &str, domains: &[&SearchDomain], dns_servers: &[String]) -> anyhow::Result<()> {
    let subkey = format!(r"{POLICY_BASE}\{rule_name}");

    let (hkey, _) = RegKey::predef(HKEY_LOCAL_MACHINE).create_subkey_with_options_flags(
        &subkey,
        REG_OPTION_NON_VOLATILE,
        KEY_WRITE,
    )?;

    let result = (|| -> anyhow::Result<()> {
        // NRPT suffix-match entries begin with a dot.
        let names: Vec<String> = domains
            .iter()
            .map(|d| format!(".{}", d.name.trim_matches('.')))
            .collect();
        hkey.set_value("Name", &names)?;
        hkey.set_value("GenericDNSServers", &dns_servers.join(";"))?;
        // Version = 2 (NRPT v2 schema), ConfigOptions = 0x8 (DnsServers).
        hkey.set_value("Version", &2u32)?;
        hkey.set_value("ConfigOptions", &0x8u32)?;
        Ok(())
    })();

    // Close the handle before delete_rule runs on failure — delete_subkey_all
    // on the same path can fail or race while the key is still open.
    drop(hkey);

    if result.is_err() {
        let _ = delete_rule(rule_name);
    }

    result
}

fn delete_rule(rule_name: &str) -> anyhow::Result<()> {
    let subkey = format!(r"{POLICY_BASE}\{rule_name}");
    Ok(RegKey::predef(HKEY_LOCAL_MACHINE).delete_subkey_all(&subkey)?)
}
