use secrecy::{ExposeSecret, SecretString};
use security_framework::passwords::{delete_generic_password, get_generic_password, set_generic_password};
use tracing::debug;
use uuid::Uuid;

use crate::platform::Keychain;

const SERVICE: &str = "snx-rs";

// Security/SecBase.h errSecItemNotFound: SecItemDelete returns it when the entry is already gone.
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;

#[derive(Default)]
pub struct MacosKeychain;

impl MacosKeychain {
    pub fn new() -> Self {
        Self
    }
}

impl Keychain for MacosKeychain {
    async fn acquire_password(&self, profile_id: Uuid) -> anyhow::Result<String> {
        debug!("Reading password from the keychain for profile {profile_id}");
        // The daemon runs as root, where the login keychain is out of reach: the lookup fails with
        // errSecInteractionNotAllowed instead of returning data. Propagate it so the caller prompts.
        let bytes = get_generic_password(SERVICE, &profile_id.to_string())?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    async fn store_password(&self, profile_id: Uuid, password: &SecretString) -> anyhow::Result<()> {
        debug!("Storing password in the keychain for profile {profile_id}");
        set_generic_password(SERVICE, &profile_id.to_string(), password.expose_secret().as_bytes())?;
        Ok(())
    }

    async fn delete_password(&self, profile_id: Uuid) -> anyhow::Result<()> {
        debug!("Deleting password from the keychain for profile {profile_id}");
        match delete_generic_password(SERVICE, &profile_id.to_string()) {
            Ok(()) => Ok(()),
            Err(e) if e.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use secrecy::SecretString;

    use super::*;

    const TEST_PROFILE: Uuid = Uuid::from_u128(0x736e_782d_7273_2d74_6573_745f_726f_756e);

    #[tokio::test]
    async fn delete_missing_password_is_ok() {
        MacosKeychain::new()
            .delete_password(Uuid::from_u128(0xffff_ffff_dead_beef_ffff_ffff_dead_beef))
            .await
            .expect("deleting an absent entry must be a no-op");
    }

    #[tokio::test]
    #[ignore = "touches the real login keychain; run with --ignored"]
    async fn store_acquire_delete_roundtrip() {
        let kc = MacosKeychain::new();
        let secret: SecretString = "correct horse battery staple".to_string().into();
        kc.store_password(TEST_PROFILE, &secret)
            .await
            .expect("store must succeed");
        let got = kc
            .acquire_password(TEST_PROFILE)
            .await
            .expect("stored password reads back");
        assert_eq!(got, "correct horse battery staple");
        kc.delete_password(TEST_PROFILE).await.expect("delete must succeed");
        assert!(
            kc.acquire_password(TEST_PROFILE).await.is_err(),
            "entry must be gone after delete"
        );
    }
}
