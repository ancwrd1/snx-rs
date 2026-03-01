use std::{str::FromStr, sync::Arc};

use anyhow::anyhow;
use ksni::{
    Handle, Icon, MenuItem, TrayMethods,
    menu::{StandardItem, SubMenu},
};
use snxcore::model::{
    ConnectionStatus,
    params::{DEFAULT_PROFILE_UUID, IconTheme, TunnelParams},
};
use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;

use crate::{
    assets,
    profiles::ConnectionProfilesStore,
    theme::{SystemColorTheme, system_color_theme},
};

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

enum PixmapOrName {
    Pixmap(Icon),
    Name(&'static str),
}

pub struct AppTray {
    command_sender: Sender<TrayCommand>,
    command_receiver: Option<Receiver<TrayCommand>>,
    status: Arc<anyhow::Result<ConnectionStatus>>,
    tray_icon: Option<Handle<KsniTray>>,
}

impl AppTray {
    pub async fn new(event_sender: Sender<TrayEvent>, no_tray: bool) -> anyhow::Result<Self> {
        let (tx, rx) = tokio::sync::mpsc::channel(16);

        let handle = if !no_tray {
            let tray_icon = KsniTray::new(event_sender);
            Some(tray_icon.spawn().await?)
        } else {
            None
        };

        let app_tray = AppTray {
            command_sender: tx,
            command_receiver: Some(rx),
            status: Arc::new(Err(anyhow!(crate::tr!("error-no-service-connection")))),
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
        let tunnel_params = TunnelParams::load(TunnelParams::default_config_path()).unwrap_or_default();

        let system_theme = match tunnel_params.icon_theme {
            IconTheme::AutoDetect => system_color_theme().ok().unwrap_or_default(),
            IconTheme::Dark => SystemColorTheme::Light,
            IconTheme::Light => SystemColorTheme::Dark,
        };

        if system_theme.is_dark() {
            &assets::DARK_THEME
        } else {
            &assets::LIGHT_THEME
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

        if let Some(ref tray_icon) = self.tray_icon {
            tray_icon
                .update(|tray| {
                    tray.status_label = status_label;
                    tray.icon = icon;
                    tray.connect_enabled = connect_enabled;
                    tray.disconnect_enabled = disconnect_enabled;
                })
                .await;
        }
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
    icon: PixmapOrName,
    event_sender: Sender<TrayEvent>,
}

impl KsniTray {
    fn new(event_sender: Sender<TrayEvent>) -> Self {
        Self {
            status_label: String::new(),
            connect_enabled: false,
            disconnect_enabled: false,
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

    fn icon_name(&self) -> String {
        if let PixmapOrName::Name(name) = &self.icon {
            name.to_string()
        } else {
            String::new()
        }
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        if let PixmapOrName::Pixmap(icon) = &self.icon {
            vec![icon.clone()]
        } else {
            vec![]
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let profiles = ConnectionProfilesStore::instance().all();
        let connect_item = if profiles.len() < 2 {
            MenuItem::Standard(StandardItem {
                label: crate::tr!("tray-menu-connect").to_string(),
                enabled: self.connect_enabled,
                activate: Box::new(|tray: &mut KsniTray| {
                    tray.send_tray_event(TrayEvent::Connect(DEFAULT_PROFILE_UUID))
                }),
                ..Default::default()
            })
        } else {
            MenuItem::SubMenu(SubMenu {
                label: crate::tr!("tray-menu-connect").to_string(),
                enabled: self.connect_enabled,
                submenu: profiles
                    .into_iter()
                    .map(|profile| {
                        MenuItem::Standard(StandardItem {
                            label: profile.profile_name.clone(),
                            enabled: self.connect_enabled,
                            activate: Box::new(move |tray: &mut KsniTray| {
                                tray.send_tray_event(TrayEvent::Connect(profile.profile_id))
                            }),
                            ..Default::default()
                        })
                    })
                    .collect(),
                ..Default::default()
            })
        };

        vec![
            MenuItem::Standard(StandardItem {
                label: self.status_label.clone(),
                enabled: false,
                ..Default::default()
            }),
            MenuItem::Separator,
            connect_item,
            MenuItem::Standard(StandardItem {
                label: crate::tr!("tray-menu-disconnect").to_string(),
                enabled: self.disconnect_enabled,
                activate: Box::new(|tray: &mut KsniTray| tray.send_tray_event(TrayEvent::Disconnect)),
                ..Default::default()
            }),
            MenuItem::Standard(StandardItem {
                label: crate::tr!("tray-menu-status").to_string(),
                activate: Box::new(|tray: &mut KsniTray| tray.send_tray_event(TrayEvent::Status)),
                ..Default::default()
            }),
            MenuItem::Standard(StandardItem {
                label: crate::tr!("tray-menu-settings").to_string(),
                activate: Box::new(|tray: &mut KsniTray| tray.send_tray_event(TrayEvent::Settings)),
                ..Default::default()
            }),
            MenuItem::Standard(StandardItem {
                label: crate::tr!("tray-menu-about").to_string(),
                activate: Box::new(|tray: &mut KsniTray| tray.send_tray_event(TrayEvent::About)),
                ..Default::default()
            }),
            MenuItem::Standard(StandardItem {
                label: crate::tr!("tray-menu-exit").to_string(),
                activate: Box::new(|tray: &mut KsniTray| tray.send_tray_event(TrayEvent::Exit)),
                ..Default::default()
            }),
        ]
    }
}
