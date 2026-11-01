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

// The tray and Slint share muda's global menu event handler, which can only be set once, and the tray's
// events claim it first. Slint's native context menus would then never receive their activations,
// so it is made to draw them itself, as it does on Linux.
pub fn init_process_env() {
    // SAFETY: called from `main` before the async runtime or any other thread is started.
    unsafe { std::env::set_var("SLINT_NO_MUDA", "1") };
}

pub fn init_gui_backend() -> anyhow::Result<()> {
    slint::BackendSelector::new().select()?;
    Ok(())
}

pub async fn wait_restart_signal() -> anyhow::Result<()> {
    std::future::pending::<()>().await;
    Ok(())
}
