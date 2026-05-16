use anyhow::{Context, anyhow};
use secrecy::{ExposeSecret, SecretString};
use tracing::debug;
use uuid::Uuid;
use windows::{
    Win32::Security::Credentials::{
        CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredDeleteW, CredFree, CredReadW, CredWriteW,
    },
    core::{HSTRING, PWSTR},
};

use crate::platform::Keychain;

#[derive(Default)]
pub struct WindowsKeychain;

impl WindowsKeychain {
    pub fn new() -> Self {
        Self
    }
}

fn target_name(profile_id: Uuid) -> String {
    format!("snx-rs/{profile_id}")
}

impl Keychain for WindowsKeychain {
    async fn acquire_password(&self, profile_id: Uuid) -> anyhow::Result<String> {
        debug!("Reading password from Credential Manager for profile {profile_id}");

        let name = HSTRING::from(target_name(profile_id));

        let mut cred_ptr: *mut CREDENTIALW = std::ptr::null_mut();

        unsafe { CredReadW(&name, CRED_TYPE_GENERIC, None, &mut cred_ptr) }
            .map_err(|e| anyhow!("CredReadW failed for {profile_id}: {e}"))?;

        let result = unsafe {
            let cred = &*cred_ptr;
            let len = cred.CredentialBlobSize as usize;
            let bytes = if len == 0 || cred.CredentialBlob.is_null() {
                Vec::new()
            } else {
                std::slice::from_raw_parts(cred.CredentialBlob, len).to_vec()
            };
            String::from_utf8(bytes).context("credential blob is not valid UTF-8")
        };
        unsafe { CredFree(cred_ptr as *const _) };
        result
    }

    async fn store_password(&self, profile_id: Uuid, password: &SecretString) -> anyhow::Result<()> {
        debug!("Storing password in Credential Manager for profile {profile_id}");
        let mut name_w: Vec<u16> = target_name(profile_id)
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let mut user_w: Vec<u16> = "snx-rs".encode_utf16().chain(std::iter::once(0)).collect();
        let mut blob = password.expose_secret().as_bytes().to_vec();

        let cred = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            TargetName: PWSTR(name_w.as_mut_ptr()),
            UserName: PWSTR(user_w.as_mut_ptr()),
            CredentialBlobSize: blob.len() as u32,
            CredentialBlob: blob.as_mut_ptr(),
            ..Default::default()
        };

        unsafe { CredWriteW(&cred, 0) }.map_err(|e| anyhow!("CredWriteW failed for {profile_id}: {e}"))
    }

    async fn delete_password(&self, profile_id: Uuid) -> anyhow::Result<()> {
        debug!("Deleting password from Credential Manager for profile {profile_id}");
        let name = HSTRING::from(target_name(profile_id));
        unsafe { CredDeleteW(&name, CRED_TYPE_GENERIC, None) }
            .map_err(|e| anyhow!("CredDeleteW failed for {profile_id}: {e}"))
    }
}
