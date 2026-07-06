#![allow(unsafe_code)]

pub mod theme;
pub mod tray;
#[cfg(feature = "mobile-access")]
pub mod webkit;

use std::os::unix::process::CommandExt;

use block2::RcBlock;
use objc2::runtime::Bool;
use objc2_foundation::{NSBundle, NSError, NSString};
use objc2_user_notifications::{
    UNAuthorizationOptions, UNMutableNotificationContent, UNNotificationRequest, UNUserNotificationCenter,
};
use tokio::signal::unix::{SignalKind, signal};
use tracing::warn;
use uuid::Uuid;
#[cfg(feature = "mobile-access")]
pub use webkit::webkit_main;

use crate::current_exe_path;

pub fn user_tag() -> String {
    nix::unistd::Uid::current().to_string()
}

pub fn init_gui_backend() -> anyhow::Result<()> {
    slint::BackendSelector::new().select()?;
    Ok(())
}

pub async fn wait_restart_signal() -> anyhow::Result<()> {
    let mut sig = signal(SignalKind::user_defined1())?;
    if sig.recv().await.is_some() {
        let exe = current_exe_path()?;
        let args: Vec<String> = std::env::args().skip(1).collect();
        let err = std::process::Command::new(exe).args(args).exec();
        anyhow::bail!("re-exec failed: {}", err);
    }
    Ok(())
}

pub async fn send_notification(summary: &str, message: &str) -> anyhow::Result<()> {
    let summary = summary.to_owned();
    let message = message.to_owned();

    // Post from the main thread. UNUserNotificationCenter needs a bundle identifier, so skip when
    // running unbundled (a dev binary), where querying it would abort.
    let _ = slint::invoke_from_event_loop(move || {
        if NSBundle::mainBundle().bundleIdentifier().is_none() {
            return;
        }

        let center = UNUserNotificationCenter::currentNotificationCenter();

        // Ask once for permission; delivery is best-effort if the user declines.
        let noop = RcBlock::new(|_granted: Bool, _error: *mut NSError| {});
        center.requestAuthorizationWithOptions_completionHandler(
            UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound,
            &noop,
        );

        let content = UNMutableNotificationContent::new();
        content.setTitle(&NSString::from_str(&summary));
        content.setBody(&NSString::from_str(&message));

        let id = NSString::from_str(&Uuid::new_v4().to_string());
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(&id, &content, None);
        center.addNotificationRequest_withCompletionHandler(&request, None);
    })
    .inspect_err(|e| warn!("Failed to post notification: {e}"));

    Ok(())
}
