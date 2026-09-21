#![allow(unsafe_code)]

pub mod theme;
pub mod tray;
#[cfg(feature = "mobile-access")]
pub mod webkit;

use std::env;

use snxcore::prompt::NotificationCategory;
use tauri_winrt_notification::Toast;
#[cfg(feature = "mobile-access")]
pub use webkit::webkit_main;

pub async fn send_notification(summary: &str, message: &str, _category: NotificationCategory) -> anyhow::Result<()> {
    Ok(Toast::new("com.github.snx-rs")
        .title(summary)
        .text1(message)
        .duration(tauri_winrt_notification::Duration::Short)
        .show()?)
}

pub fn user_tag() -> String {
    let raw = env::var("USERNAME").unwrap_or_else(|_| "user".into());
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn init_gui_backend() -> anyhow::Result<()> {
    slint::BackendSelector::new().select()?;
    Ok(())
}

pub async fn wait_restart_signal() -> anyhow::Result<()> {
    std::future::pending::<()>().await;
    Ok(())
}
