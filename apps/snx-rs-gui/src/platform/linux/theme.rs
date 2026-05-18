use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use futures::StreamExt;
use tokio::sync::mpsc::Sender;
use tracing::debug;
use zbus::Connection;

use super::dbus::DesktopSettingsProxy;
use crate::platform::TrayCommand;

pub fn spawn_theme_monitor(theme: Arc<AtomicU32>, tray_sender: Sender<TrayCommand>) {
    tokio::spawn(async move {
        if let Err(e) = init_theme_monitoring(theme, tray_sender).await {
            tracing::warn!("Theme monitor exited: {e}");
        }
    });
}

async fn init_theme_monitoring(theme: Arc<AtomicU32>, tray_sender: Sender<TrayCommand>) -> anyhow::Result<()> {
    let connection = Connection::session().await?;
    let proxy = DesktopSettingsProxy::new(&connection).await?;
    let scheme = proxy.read_one("org.freedesktop.appearance", "color-scheme").await?;
    let scheme = u32::try_from(scheme)?;
    theme.store(scheme, Ordering::SeqCst);

    debug!("System color scheme: {}", scheme);

    tokio::spawn(async move {
        let mut stream = proxy.receive_setting_changed().await?;
        while let Some(signal) = stream.next().await {
            let args = signal.args()?;
            if args.namespace == "org.freedesktop.appearance" && args.key == "color-scheme" {
                let scheme = u32::try_from(args.value)?;
                debug!("New system color scheme: {}", scheme);
                theme.store(scheme, Ordering::SeqCst);
                let _ = tray_sender.send(TrayCommand::Update(None)).await;
            }
        }
        Ok::<_, anyhow::Error>(())
    });

    Ok(())
}
