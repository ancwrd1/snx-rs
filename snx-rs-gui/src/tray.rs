use std::{path::PathBuf, sync::Arc};

use anyhow::anyhow;
use ksni::{menu::StandardItem, Handle, Icon, MenuItem, TrayMethods};
use tokio::sync::mpsc::{Receiver, Sender};

use snxcore::{
    browser::BrowserController,
    controller::{ServiceCommand, ServiceController},
    model::params::IconTheme,
    model::{params::TunnelParams, ConnectionStatus},
    prompt::SecurePrompt,
};

use crate::{assets, params::CmdlineParams, prompt, theme::system_color_theme, theme::SystemColorTheme};

fn browser(_params: Arc<TunnelParams>) -> impl BrowserController {
    snxcore::browser::SystemBrowser
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrayEvent {
    Connect,
    Disconnect,
    Settings,
    Exit,
    About,
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
    tray_icon: Handle<KsniTray>,
}

impl AppTray {
    pub async fn new(params: &CmdlineParams, event_sender: Sender<TrayEvent>) -> anyhow::Result<Self> {
        let (tx, rx) = tokio::sync::mpsc::channel(16);

        let tray_icon = KsniTray::new(event_sender);
        let handle = tray_icon.spawn().await?;

        let app_tray = AppTray {
            command_sender: tx,
            command_receiver: Some(rx),
            status: Err(anyhow!("No service connection")),
            connecting: false,
            config_file: params.config_file().clone(),
            tray_icon: handle,
        };

        app_tray.update().await;

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

    fn icon(&self) -> Icon {
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

        Icon {
            width: 256,
            height: 256,
            data,
        }
    }

    async fn update(&self) {
        let status_label = self.status_label();
        let icon = self.icon();
        let connect_enabled = self
            .status
            .as_ref()
            .is_ok_and(|status| status.connected_since.is_none() && status.mfa.is_none())
            && !self.connecting;

        let disconnect_enabled = self
            .status
            .as_ref()
            .is_ok_and(|status| status.connected_since.is_some());

        self.tray_icon
            .update(move |tray| {
                tray.status_label = status_label;
                tray.icon = icon;
                tray.connect_enabled = connect_enabled;
                tray.disconnect_enabled = disconnect_enabled;
            })
            .await;
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let mut prev_command = ServiceCommand::Info;
        let mut prev_status = String::new();
        let mut prev_theme = None;

        let mut rx = self.command_receiver.take().unwrap();

        while let Some(command) = rx.recv().await {
            let command = match command {
                TrayCommand::Service(command) => command,
                TrayCommand::Update => {
                    self.update().await;
                    continue;
                }
                TrayCommand::Exit => {
                    break;
                }
            };

            let theme = system_color_theme().ok();
            if theme != prev_theme {
                prev_theme = theme;
                self.update().await;
            }

            let tunnel_params = Arc::new(TunnelParams::load(&self.config_file).unwrap_or_default());

            let mut controller =
                ServiceController::new(prompt::GtkPrompt, browser(tunnel_params.clone()), tunnel_params);

            if command == ServiceCommand::Connect {
                self.connecting = true;
                self.update().await;
            }

            let result = tokio::spawn(async move { controller.command(command).await }).await;

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
                self.update().await;
            }
            prev_command = command;
            prev_status = status_str;
        }

        Ok(())
    }
}

struct KsniTray {
    status_label: String,
    connect_enabled: bool,
    disconnect_enabled: bool,
    icon: Icon,
    event_sender: Sender<TrayEvent>,
}

impl KsniTray {
    fn new(event_sender: Sender<TrayEvent>) -> Self {
        Self {
            status_label: String::new(),
            connect_enabled: false,
            disconnect_enabled: false,
            icon: Icon {
                width: 0,
                height: 0,
                data: Vec::new(),
            },
            event_sender,
        }
    }

    fn send_tray_event(&self, event: TrayEvent) {
        let sender = self.event_sender.clone();
        tokio::spawn(async move { sender.send(event).await });
    }
}

impl ksni::Tray for KsniTray {
    const MENU_ON_ACTIVATE: bool = true;

    fn id(&self) -> String {
        "SNX-RS".to_string()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        vec![self.icon.clone()]
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            MenuItem::Standard(StandardItem {
                label: self.status_label.clone(),
                enabled: false,
                ..Default::default()
            }),
            MenuItem::Separator,
            MenuItem::Standard(StandardItem {
                label: "Connect".to_string(),
                enabled: self.connect_enabled,
                activate: Box::new(|tray: &mut KsniTray| tray.send_tray_event(TrayEvent::Connect)),
                ..Default::default()
            }),
            MenuItem::Standard(StandardItem {
                label: "Disconnect".to_string(),
                enabled: self.disconnect_enabled,
                activate: Box::new(|tray: &mut KsniTray| tray.send_tray_event(TrayEvent::Disconnect)),
                ..Default::default()
            }),
            MenuItem::Standard(StandardItem {
                label: "Settings...".to_string(),
                activate: Box::new(|tray: &mut KsniTray| tray.send_tray_event(TrayEvent::Settings)),
                ..Default::default()
            }),
            MenuItem::Standard(StandardItem {
                label: "About...".to_string(),
                activate: Box::new(|tray: &mut KsniTray| tray.send_tray_event(TrayEvent::About)),
                ..Default::default()
            }),
            MenuItem::Standard(StandardItem {
                label: "Exit".to_string(),
                activate: Box::new(|tray: &mut KsniTray| tray.send_tray_event(TrayEvent::Exit)),
                ..Default::default()
            }),
        ]
    }
}
