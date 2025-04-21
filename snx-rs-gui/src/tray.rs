use std::{path::PathBuf, sync::Arc};

use anyhow::anyhow;
use ksni::{menu::StandardItem, Handle, Icon, MenuItem, TrayMethods};
use tokio::sync::mpsc::{Receiver, Sender};

use snxcore::model::{
    params::{IconTheme, TunnelParams},
    ConnectionStatus,
};

use crate::{assets, params::CmdlineParams, theme::system_color_theme, theme::SystemColorTheme};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrayEvent {
    Connect,
    Disconnect,
    Settings,
    Exit,
    About,
}

#[derive(Debug, Clone)]
pub enum TrayCommand {
    Update(Option<Arc<anyhow::Result<ConnectionStatus>>>),
    Exit,
}

pub struct AppTray {
    command_sender: Sender<TrayCommand>,
    command_receiver: Option<Receiver<TrayCommand>>,
    status: Arc<anyhow::Result<ConnectionStatus>>,
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
            status: Arc::new(Err(anyhow!("No service connection"))),
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
        match &*self.status {
            Ok(status) => status.to_string(),
            Err(e) => e.to_string(),
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

        let data = match &*self.status {
            Ok(ConnectionStatus::Connected(_)) => theme.connected.clone(),
            Ok(ConnectionStatus::Disconnected) => theme.disconnected.clone(),
            Ok(ConnectionStatus::Mfa(_) | ConnectionStatus::Connecting) => theme.acquiring.clone(),
            _ => theme.error.clone(),
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
            .as_ref()
            .is_ok_and(|status| *status == ConnectionStatus::Disconnected);

        let disconnect_enabled = self
            .status
            .as_ref()
            .as_ref()
            .is_ok_and(|status| *status != ConnectionStatus::Disconnected);

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
        let mut rx = self.command_receiver.take().unwrap();

        while let Some(command) = rx.recv().await {
            match command {
                TrayCommand::Update(status) => {
                    if let Some(status) = status {
                        self.status = status;
                    }
                    self.update().await;
                }
                TrayCommand::Exit => {
                    break;
                }
            }
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
