use std::time::Duration;

use anyhow::Context;
use tracing::debug;
use uuid::Uuid;

use crate::platform::{Platform, PlatformAccess};

/// How long an ESP child SA is used before it is replaced. IKEv1 asks the
/// gateway for this in quick mode; IKEv2 does not negotiate one at all
/// (RFC 7296 leaves it to each side's policy), so both ends up here.
pub(crate) const DEFAULT_ESP_LIFETIME: Duration = Duration::from_secs(3600);

/// Rekeying starts this far before the lifetime runs out, so the replacement
/// is in place before the old SA stops being accepted.
pub(crate) const ESP_LIFETIME_LEEWAY: Duration = Duration::from_secs(60);

pub mod auth;
pub mod ikev1;
pub mod ikev2;
pub mod imp;
pub mod keepalive;
pub mod natt;
pub mod scv;

const SESSIONS_NAME: &str = "ike-sessions.db";

/// The saved IKE session for one profile and server, so a reconnect can resume
/// instead of authenticating again.
///
/// The two IKE versions keep their own tables: the blobs are each version's
/// own format, and a profile that switches between them must not be handed the
/// other's bytes.
pub(crate) struct SessionStore {
    table: &'static str,
    profile_id: Uuid,
    server_name: String,
}

impl SessionStore {
    pub(crate) fn new(table: &'static str, profile_id: Uuid, server_name: String) -> Self {
        Self {
            table,
            profile_id,
            server_name,
        }
    }

    fn connection(&self) -> anyhow::Result<rusqlite::Connection> {
        let _ = std::fs::create_dir_all(Platform::get().data_dir());

        let conn = rusqlite::Connection::open(Platform::get().data_dir().join(SESSIONS_NAME))?;

        conn.execute(
            &format!(
                "CREATE TABLE IF NOT EXISTS {}(
                    id integer not null primary key,
                    profile_uuid text not null,
                    server_name text not null,
                    data blob not null,
                    timestamp text not null)",
                self.table
            ),
            rusqlite::params![],
        )?;

        Ok(conn)
    }

    pub(crate) fn save(&self, data: &[u8]) -> anyhow::Result<()> {
        let mut conn = self.connection()?;
        let trans = conn.transaction()?;

        trans.execute(
            &format!(
                "DELETE FROM {} WHERE profile_uuid = ?1 AND server_name = ?2",
                self.table
            ),
            rusqlite::params![self.profile_id, &self.server_name],
        )?;
        trans.execute(
            &format!(
                "INSERT INTO {} (profile_uuid, server_name, data, timestamp) VALUES (?1, ?2, ?3, current_timestamp)",
                self.table
            ),
            rusqlite::params![self.profile_id, &self.server_name, data],
        )?;
        trans.commit()?;

        debug!("Saved IKE session: {}: {}", self.server_name, self.profile_id);

        Ok(())
    }

    pub(crate) fn load(&self) -> anyhow::Result<Vec<u8>> {
        self.connection()?
            .query_row_and_then(
                &format!(
                    "SELECT data FROM {} WHERE profile_uuid = ?1 AND server_name = ?2",
                    self.table
                ),
                rusqlite::params![self.profile_id, &self.server_name],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .context("No saved IKE session")
    }

    pub(crate) fn delete(&self) -> anyhow::Result<()> {
        self.connection()?.execute(
            &format!(
                "DELETE FROM {} WHERE profile_uuid = ?1 AND server_name = ?2",
                self.table
            ),
            rusqlite::params![self.profile_id, &self.server_name],
        )?;

        debug!("Deleted IKE session: {}: {}", self.server_name, self.profile_id);

        Ok(())
    }
}
