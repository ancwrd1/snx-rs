use std::collections::HashMap;

use anyhow::Context;
use secret_service::{EncryptionType, SecretService};
use tracing::debug;

use crate::platform::Keychain;

#[derive(Default)]
pub struct SecretServiceKeychain;

impl SecretServiceKeychain {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Keychain for SecretServiceKeychain {
    async fn acquire_password(&self, username: &str) -> anyhow::Result<String> {
        let props = HashMap::from([("snx-rs.username", username)]);

        debug!("Attempting to acquire password from the keychain");

        let ss = SecretService::connect(EncryptionType::Dh).await?;
        let collection = ss.get_default_collection().await?;
        if let Ok(true) = collection.is_locked().await {
            debug!("Unlocking secret collection");
            let _ = collection.unlock().await;
        }

        let search_items = ss.search_items(props.clone()).await?;

        let item = search_items.unlocked.first().context("No item in collection")?;

        let secret = item.get_secret().await?;

        debug!("Password acquired successfully");

        Ok(String::from_utf8_lossy(&secret).into_owned())
    }

    async fn store_password(&self, username: &str, password: &str) -> anyhow::Result<()> {
        let props = HashMap::from([("snx-rs.username", username)]);

        let ss = SecretService::connect(EncryptionType::Dh).await?;
        let collection = ss.get_default_collection().await?;

        if let Ok(true) = collection.is_locked().await {
            debug!("Unlocking secret collection");
            let _ = collection.unlock().await;
        }

        debug!("Attempting to store user password in the keychain");

        collection
            .create_item(
                &format!("snx-rs - {username}"),
                props,
                password.as_bytes(),
                true,
                "text/plain",
            )
            .await?;

        Ok(())
    }
}
