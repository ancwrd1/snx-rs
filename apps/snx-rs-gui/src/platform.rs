use std::{str::FromStr, sync::Arc};

use anyhow::anyhow;
use snxcore::model::{ConnectionStatus, params::DEFAULT_PROFILE_UUID};
use uuid::Uuid;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
use linux as platform_impl;
#[cfg(feature = "mobile-access")]
pub use platform_impl::webkit_main;
pub use platform_impl::{
    init_gui_backend, send_notification, theme::spawn_theme_monitor, tray::AppTray, user_tag, wait_restart_signal,
};
use snxcore::{browser::BrowserController, model::params::TunnelParams};
#[cfg(windows)]
use windows as platform_impl;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrayEvent {
    Connect(Uuid),
    Disconnect,
    Settings,
    Status,
    Exit,
    About,
}

impl TrayEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            TrayEvent::Connect(_) => "connect",
            TrayEvent::Disconnect => "disconnect",
            TrayEvent::Settings => "settings",
            TrayEvent::Status => "status",
            TrayEvent::Exit => "exit",
            TrayEvent::About => "about",
        }
    }
}

impl FromStr for TrayEvent {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "connect" => Ok(TrayEvent::Connect(DEFAULT_PROFILE_UUID)),
            "disconnect" => Ok(TrayEvent::Disconnect),
            "settings" => Ok(TrayEvent::Settings),
            "status" => Ok(TrayEvent::Status),
            "exit" => Ok(TrayEvent::Exit),
            "about" => Ok(TrayEvent::About),
            _ => Err(anyhow!(crate::tr!("error-unknown-event", event = s))),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TrayCommand {
    Update(Option<Arc<anyhow::Result<ConnectionStatus>>>),
    Exit,
}

#[cfg(feature = "mobile-access")]
pub fn new_browser_controller(params: Arc<TunnelParams>) -> impl BrowserController {
    crate::webkit::WebKitBrowser::new(params)
}

#[cfg(not(feature = "mobile-access"))]
pub fn new_browser_controller(_params: Arc<TunnelParams>) -> impl BrowserController {
    snxcore::browser::SystemBrowser::new(crate::ui::prompt::SlintPrompt)
}
