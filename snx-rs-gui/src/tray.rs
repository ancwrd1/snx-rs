use std::{path::PathBuf, sync::Arc};

use anyhow::anyhow;
use ksni::{Handle, Icon, MenuItem, TrayMethods, menu::StandardItem};
use tokio::sync::mpsc::{Receiver, Sender};

use snxcore::model::{
    ConnectionStatus,
    params::{IconTheme, TunnelParams},
};

use crate::{assets, params::CmdlineParams, theme::SystemColorTheme, theme::system_color_theme};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrayEvent {
    Connect,
    Disconnect,
    Settings,
    Status,
    Exit,
    About,
}

#[derive(Debug, Clone)]
pub enum TrayCommand {
    Update(Option<Arc<anyhow::Result<ConnectionStatus>>>),
    Exit,
}

enum PixmapOrName {
    Pixmap(Icon),
    Name(&'static str),
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

    fn icon_name(&self) -> &'static str {
        match &*self.status {
            Ok(ConnectionStatus::Connected(_)) => "network-vpn-symbolic",
            Ok(ConnectionStatus::Disconnected) => "network-vpn-disconnected-symbolic",
            Ok(ConnectionStatus::Mfa(_) | ConnectionStatus::Connecting) => "network-vpn-acquiring-symbolic",
            _ => "network-vpn-disabled-symbolic",
        }
    }

    async fn update(&self) {
        let status_label = self.status_label();

        // Custom pixmaps are supported under GNOME or KDE.
        // See https://github.com/AyatanaIndicators/libayatana-appindicator-glib/issues/47
        let icon = if self.pixmap_icons_supported() {
            PixmapOrName::Pixmap(self.icon())
        } else {
            PixmapOrName::Name(self.icon_name())
        };

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

        let status_enabled = matches!(
            self.status.as_ref().as_ref(),
            Ok(ConnectionStatus::Connected(_)) | Err(_)
        );

        self.tray_icon
            .update(move |tray| {
                tray.status_label = status_label;
                tray.icon = icon;
                tray.connect_enabled = connect_enabled;
                tray.disconnect_enabled = disconnect_enabled;
                tray.status_enabled = status_enabled;
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

    fn pixmap_icons_supported(&self) -> bool {
        std::env::var("XDG_CURRENT_DESKTOP")
            .map(|s| s.to_lowercase())
            .is_ok_and(|s| s.contains("gnome") || s.contains("kde"))
    }
}

struct KsniTray {
    status_label: String,
    connect_enabled: bool,
    disconnect_enabled: bool,
    status_enabled: bool,
    icon: PixmapOrName,
    event_sender: Sender<TrayEvent>,
}

impl KsniTray {
    fn new(event_sender: Sender<TrayEvent>) -> Self {
        Self {
            status_label: String::new(),
            connect_enabled: false,
            disconnect_enabled: false,
            status_enabled: false,
            icon: PixmapOrName::Name(""),
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
        if let PixmapOrName::Pixmap(icon) = &self.icon {
            vec![icon.clone()]
        } else {
            vec![]
        }
    }

    fn icon_name(&self) -> String {
        if let PixmapOrName::Name(name) = &self.icon {
            name.to_string()
        } else {
            String::new()
        }
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
                label: "Connection status...".to_string(),
                enabled: self.status_enabled,
                activate: Box::new(|tray: &mut KsniTray| tray.send_tray_event(TrayEvent::Status)),
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
