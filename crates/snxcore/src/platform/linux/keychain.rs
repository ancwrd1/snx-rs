use std::collections::HashMap;

use anyhow::Context;
use secrecy::{ExposeSecret, SecretString};
use secret_service::{EncryptionType, SecretService};
use tracing::debug;
use uuid::Uuid;

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
    async fn acquire_password(&self, profile_id: Uuid) -> anyhow::Result<String> {
        let attribute = format!("snx-rs.{}", profile_id);
        let props = HashMap::from([(attribute.as_str(), "password")]);

        debug!(
            "Attempting to acquire password from the keychain for profile {}",
            profile_id
        );

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

    async fn store_password(&self, profile_id: Uuid, password: &SecretString) -> anyhow::Result<()> {
        let attribute = format!("snx-rs.{}", profile_id);
        let props = HashMap::from([(attribute.as_str(), "password")]);

        let ss = SecretService::connect(EncryptionType::Dh).await?;
        let collection = ss.get_default_collection().await?;

        if let Ok(true) = collection.is_locked().await {
            debug!("Unlocking secret collection");
            let _ = collection.unlock().await;
        }

        debug!(
            "Attempting to store password in the keychain for profile {}",
            profile_id
        );

        collection
            .create_item(
                &format!("User password for snx-rs profile {}", profile_id),
                props,
                password.expose_secret().as_bytes(),
                true,
                "text/plain",
            )
            .await?;

        Ok(())
    }

    async fn delete_password(&self, profile_id: Uuid) -> anyhow::Result<()> {
        let attribute = format!("snx-rs.{}", profile_id);
        let props = HashMap::from([(attribute.as_str(), "password")]);

        let ss = SecretService::connect(EncryptionType::Dh).await?;
        let collection = ss.get_default_collection().await?;

        if let Ok(true) = collection.is_locked().await {
            debug!("Unlocking secret collection");
            let _ = collection.unlock().await;
        }

        debug!("Deleting user password from the keychain for profile {}", profile_id);

        if let Ok(items) = collection.search_items(props).await {
            for item in items {
                let _ = item.delete().await;
            }
        }

        Ok(())
    }
}
