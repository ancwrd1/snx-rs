use std::collections::HashMap;

use zbus::{zvariant, Connection};

#[zbus::proxy(
    interface = "org.freedesktop.portal.Settings",
    default_service = "org.freedesktop.portal.Desktop",
    default_path = "/org/freedesktop/portal/desktop"
)]
trait DesktopSettings {
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

pub async fn send_notification(summary: &str, message: &str) -> anyhow::Result<()> {
    let connection = Connection::session().await?;
    let proxy = NotificationsProxy::new(&connection).await?;
    proxy
        .notify(
            "SNX-RS VPN client",
            0,
            "emblem-error",
            summary,
            message,
            &[],
            HashMap::default(),
            10000,
        )
        .await?;
    Ok(())
}
