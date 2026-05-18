use std::{fs, path::PathBuf};

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use tracing::warn;
use uuid::Uuid;
use windows::{
    Win32::{
        Foundation::{ERROR_FILE_NOT_FOUND, NO_ERROR},
        System::Registry::{
            HKEY, HKEY_LOCAL_MACHINE, KEY_WRITE, REG_DWORD, REG_MULTI_SZ, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
            RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW,
        },
    },
    core::{HSTRING, PCWSTR},
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
    let subkey_w = HSTRING::from(&subkey);

    let mut hkey = HKEY::default();
    let rc = unsafe {
        RegCreateKeyExW(
            HKEY_LOCAL_MACHINE,
            &subkey_w,
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut hkey,
            None,
        )
    };
    if rc != NO_ERROR {
        return Err(anyhow!("RegCreateKeyExW({subkey}) failed: {:?}", rc));
    }

    let result = (|| -> anyhow::Result<()> {
        // NRPT suffix-match entries begin with a dot.
        let names: Vec<String> = domains
            .iter()
            .map(|d| format!(".{}", d.name.trim_matches('.')))
            .collect();
        set_multi_sz(hkey, "Name", &names)?;
        set_string(hkey, "GenericDNSServers", &dns_servers.join(";"))?;
        // Version = 2 (NRPT v2 schema), ConfigOptions = 0x8 (DnsServers).
        set_dword(hkey, "Version", 2)?;
        set_dword(hkey, "ConfigOptions", 0x8)?;
        Ok(())
    })();

    let _ = unsafe { RegCloseKey(hkey) };

    if result.is_err() {
        let _ = delete_rule(rule_name);
    }

    result
}

fn delete_rule(rule_name: &str) -> anyhow::Result<()> {
    let subkey = format!(r"{POLICY_BASE}\{rule_name}");
    let subkey_w = HSTRING::from(&subkey);
    let rc = unsafe { RegDeleteTreeW(HKEY_LOCAL_MACHINE, &subkey_w) };
    if rc == NO_ERROR || rc == ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        Err(anyhow!("RegDeleteTreeW({subkey}) failed: {:?}", rc))
    }
}

fn set_string(hkey: HKEY, name: &str, value: &str) -> anyhow::Result<()> {
    let bytes = wide_bytes_nul(value);
    let rc = unsafe { RegSetValueExW(hkey, &HSTRING::from(name), None, REG_SZ, Some(&bytes)) };
    if rc != NO_ERROR {
        return Err(anyhow!("RegSetValueExW({name}) failed: {:?}", rc));
    }
    Ok(())
}

fn set_multi_sz(hkey: HKEY, name: &str, items: &[String]) -> anyhow::Result<()> {
    let bytes = multi_sz_bytes(items);
    let rc = unsafe { RegSetValueExW(hkey, &HSTRING::from(name), None, REG_MULTI_SZ, Some(&bytes)) };
    if rc != NO_ERROR {
        return Err(anyhow!("RegSetValueExW({name}) failed: {:?}", rc));
    }
    Ok(())
}

fn set_dword(hkey: HKEY, name: &str, value: u32) -> anyhow::Result<()> {
    let rc = unsafe { RegSetValueExW(hkey, &HSTRING::from(name), None, REG_DWORD, Some(&value.to_le_bytes())) };
    if rc != NO_ERROR {
        return Err(anyhow!("RegSetValueExW({name}) failed: {:?}", rc));
    }
    Ok(())
}

fn wide_bytes_nul(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity((s.len() + 1) * 2);
    for u in s.encode_utf16() {
        out.extend_from_slice(&u.to_le_bytes());
    }
    out.extend_from_slice(&[0u8, 0u8]);
    out
}

fn multi_sz_bytes(items: &[String]) -> Vec<u8> {
    let mut out = Vec::new();
    for item in items {
        for u in item.encode_utf16() {
            out.extend_from_slice(&u.to_le_bytes());
        }
        out.extend_from_slice(&[0u8, 0u8]);
    }
    // Block terminator (extra NUL after the last string).
    out.extend_from_slice(&[0u8, 0u8]);
    out
}
