use std::{path::PathBuf, sync::Arc};

use anyhow::anyhow;
use async_channel::{Receiver, Sender};
use tray_icon::{
    menu::{ContextMenu, Menu, MenuItem, PredefinedMenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

use crate::{assets, params::CmdlineParams, prompt, theme::system_color_theme, theme::SystemColorTheme};

use snxcore::{
    browser::BrowserController,
    controller::{ServiceCommand, ServiceController},
    model::params::IconTheme,
    model::{params::TunnelParams, ConnectionStatus},
    prompt::SecurePrompt,
};

const TITLE: &str = "SNX-RS VPN client";

fn browser(_params: Arc<TunnelParams>) -> impl BrowserController {
    snxcore::browser::SystemBrowser
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrayCommand {
    Service(ServiceCommand),
    Update,
    Exit,
}

pub struct AppTray {
    command_sender: Sender<TrayCommand>,
    command_receiver: Option<Receiver<TrayCommand>>,
    status: anyhow::Result<ConnectionStatus>,
    connecting: bool,
    config_file: PathBuf,
    tray_icon: TrayIcon,
}

impl AppTray {
    pub fn new(params: &CmdlineParams) -> anyhow::Result<Self> {
        let (tx, rx) = async_channel::bounded(256);

        let tray_icon = TrayIconBuilder::new()
            .with_tooltip(TITLE)
            .with_menu_on_left_click(true)
            .build()?;

        let app_tray = AppTray {
            command_sender: tx,
            command_receiver: Some(rx),
            status: Err(anyhow!("No service connection")),
            connecting: false,
            config_file: params.config_file().clone(),
            tray_icon,
        };

        app_tray.update()?;

        Ok(app_tray)
    }

    pub fn sender(&self) -> Sender<TrayCommand> {
        self.command_sender.clone()
    }

    fn status_label(&self) -> String {
        if self.connecting {
            "...".to_owned()
        } else {
            match self.status {
                Ok(ref status) => {
                    if let Some(since) = status.connected_since {
                        if status.mfa.is_some() {
                            "Pending MFA prompt".to_owned()
                        } else {
                            format!("Connected since: {}", since.to_rfc2822())
                        }
                    } else {
                        "Tunnel disconnected".to_owned()
                    }
                }
                Err(ref e) => e.to_string(),
            }
        }
    }

    fn menu(&self) -> anyhow::Result<Box<dyn ContextMenu>> {
        let menu = Menu::new();
        menu.append(&MenuItem::new(self.status_label(), false, None))?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&MenuItem::with_id(
            "connect",
            "Connect",
            self.status
                .as_ref()
                .is_ok_and(|status| status.connected_since.is_none() && status.mfa.is_none())
                && !self.connecting,
            None,
        ))?;
        menu.append(&MenuItem::with_id(
            "disconnect",
            "Disconnect",
            self.status
                .as_ref()
                .is_ok_and(|status| status.connected_since.is_some()),
            None,
        ))?;

        menu.append(&MenuItem::with_id("settings", "Settings...", true, None))?;
        menu.append(&MenuItem::with_id("about", "About...", true, None))?;
        menu.append(&MenuItem::with_id("exit", "Exit", true, None))?;

        Ok(Box::new(menu))
    }

    fn icon_theme(&self) -> &'static assets::IconTheme {
        let tunnel_params = TunnelParams::load(&self.config_file).unwrap_or_default();

        let system_theme = match tunnel_params.icon_theme {
            IconTheme::Auto => system_color_theme().ok().unwrap_or_default(),
            IconTheme::Dark => SystemColorTheme::Light,
            IconTheme::Light => SystemColorTheme::Dark,
        };

        if system_theme.is_dark() {
            &assets::DARK_THEME_ARGB
        } else {
            &assets::LIGHT_THEME_ARGB
        }
    }

    fn icon(&self) -> anyhow::Result<Icon> {
        let theme = self.icon_theme();

        let data = if self.connecting {
            theme.acquiring.clone()
        } else {
            match self.status {
                Ok(ref status) => {
                    if status.connected_since.is_some() {
                        theme.connected.clone()
                    } else {
                        theme.disconnected.clone()
                    }
                }
                Err(_) => theme.error.clone(),
            }
        };

        Ok(Icon::from_rgba(data, 256, 256)?)
    }

    fn update(&self) -> anyhow::Result<()> {
        self.tray_icon.set_icon(Some(self.icon()?))?;
        self.tray_icon.set_menu(Some(self.menu()?));
        Ok(())
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let mut prev_command = ServiceCommand::Info;
        let mut prev_status = String::new();
        let mut prev_theme = None;

        let rx = self.command_receiver.take().unwrap();

        let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;

        while let Ok(command) = rx.recv().await {
            let command = match command {
                TrayCommand::Service(command) => command,
                TrayCommand::Update => {
                    self.update()?;
                    continue;
                }
                TrayCommand::Exit => {
                    break;
                }
            };

            let theme = system_color_theme().ok();
            if theme != prev_theme {
                prev_theme = theme;
                self.update()?;
            }

            let tunnel_params = Arc::new(TunnelParams::load(&self.config_file).unwrap_or_default());

            let mut controller =
                ServiceController::new(prompt::GtkPrompt, browser(tunnel_params.clone()), tunnel_params);

            if command == ServiceCommand::Connect {
                self.connecting = true;
                self.update()?;
            }

            let result = rt.spawn(async move { controller.command(command).await }).await;

            let status = match result {
                Ok(result) => result,
                Err(_) => Err(anyhow!("Internal error")),
            };

            let status_str = format!("{status:?}");

            match status {
                Err(ref e) if command == ServiceCommand::Connect => {
                    let _ = prompt::GtkPrompt.show_notification("Connection failed", &e.to_string());
                }
                _ => {}
            }

            if command != prev_command || status_str != prev_status {
                self.connecting = false;
                self.status = status;
                self.update()?;
            }
            prev_command = command;
            prev_status = status_str;
        }

        Ok(())
    }
}
