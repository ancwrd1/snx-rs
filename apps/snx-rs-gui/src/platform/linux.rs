use std::os::unix::process::CommandExt;

use slint::winit_030::winit::platform::{wayland::WindowAttributesExtWayland, x11::WindowAttributesExtX11};
use tokio::signal::unix::{SignalKind, signal};

pub mod dbus;
pub mod theme;
pub mod tray;
#[cfg(feature = "mobile-access")]
pub mod webkit;

pub use dbus::send_notification;
#[cfg(feature = "mobile-access")]
pub use webkit::webkit_main;

use crate::current_exe_path;

const APP_ID: &str = env!("CARGO_PKG_NAME");

pub fn user_tag() -> String {
    nix::unistd::Uid::current().to_string()
}

pub fn init_gui_backend() -> anyhow::Result<()> {
    slint::BackendSelector::new()
        .with_winit_window_attributes_hook(|attr| {
            let attr = WindowAttributesExtWayland::with_name(attr, APP_ID, "");
            WindowAttributesExtX11::with_name(attr, APP_ID, APP_ID)
        })
        .select()?;

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
