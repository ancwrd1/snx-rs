#![allow(clippy::too_many_arguments)]

use std::collections::HashMap;

use snxcore::prompt::NotificationCategory;
use zbus::{Connection, zvariant};

#[zbus::proxy(
    interface = "org.freedesktop.portal.Settings",
    default_service = "org.freedesktop.portal.Desktop",
    default_path = "/org/freedesktop/portal/desktop"
)]
pub trait DesktopSettings {
    #[zbus(signal)]
    fn setting_changed(&self, namespace: &str, key: &str, value: zvariant::Value<'_>) -> zbus::Result<()>;

    fn read_one(&self, namespace: &str, key: &str) -> zbus::Result<zvariant::OwnedValue>;
}

#[zbus::proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
pub trait Notifications {
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: HashMap<String, zvariant::OwnedValue>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;
}

pub async fn send_notification(summary: &str, message: &str, category: NotificationCategory) -> anyhow::Result<()> {
    let connection = Connection::session().await?;
    let proxy = NotificationsProxy::new(&connection).await?;

    let icon = match category {
        NotificationCategory::Info => "network-vpn",
        NotificationCategory::Warning => "emblem-warning",
        NotificationCategory::Error => "emblem-error",
    };

    proxy
        .notify(
            &i18n::tr!("app-title"),
            0,
            icon,
            summary,
            message,
            &[],
            HashMap::default(),
            10000,
        )
        .await?;
    Ok(())
}
