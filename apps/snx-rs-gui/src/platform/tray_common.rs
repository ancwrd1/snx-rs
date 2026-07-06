// Tray implementation shared by the macOS and Windows backends, both built on the `tray-icon` crate.
// The only platform differences are in update_tray: macOS marks the icon as a template and runs as an
// accessory, Windows sets a tooltip.

use std::{
    cell::RefCell,
    sync::{Arc, LazyLock, Mutex},
};

use anyhow::anyhow;
#[cfg(target_os = "macos")]
use objc2::MainThreadMarker;
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
use snxcore::{
    model::{
        ConnectionStatus,
        params::{ColorTheme, DEFAULT_PROFILE_UUID, TunnelParams},
    },
    profiles::ConnectionProfilesStore,
};
use tokio::sync::mpsc::{Receiver, Sender};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu},
};
use uuid::Uuid;

use crate::{
    assets,
    platform::{TrayCommand, TrayEvent},
    theme::{SystemColorTheme, ThemeMonitor},
};

const ICON_SIZE: u32 = 256;

#[derive(Default, Clone)]
struct MenuIds {
    connect: Vec<(MenuId, Uuid)>,
    disconnect: Option<MenuId>,
    status: Option<MenuId>,
    settings: Option<MenuId>,
    about: Option<MenuId>,
    exit: Option<MenuId>,
}

static MENU_IDS: LazyLock<Mutex<MenuIds>> = LazyLock::new(|| Mutex::new(MenuIds::default()));

thread_local! {
    static TRAY_ICON: RefCell<Option<TrayIcon>> = const { RefCell::new(None) };
}

pub struct AppTray {
    command_sender: Sender<TrayCommand>,
    command_receiver: Option<Receiver<TrayCommand>>,
    event_sender: Sender<TrayEvent>,
    status: Arc<anyhow::Result<ConnectionStatus>>,
    no_tray: bool,
    theme_monitor: ThemeMonitor,
}

impl AppTray {
    pub async fn new(event_sender: Sender<TrayEvent>, no_tray: bool) -> anyhow::Result<Self> {
        let (tx, rx) = tokio::sync::mpsc::channel(16);
        let status = Arc::new(Err(anyhow!(crate::tr!("error-no-service-connection"))));

        Ok(Self {
            command_sender: tx.clone(),
            command_receiver: Some(rx),
            event_sender,
            status,
            no_tray,
            theme_monitor: ThemeMonitor::new(tx),
        })
    }

    pub fn sender(&self) -> Sender<TrayCommand> {
        self.command_sender.clone()
    }

    fn icon_theme(&self) -> &'static assets::IconTheme {
        let tunnel_params = TunnelParams::load(TunnelParams::default_config_path()).unwrap_or_default();

        let system_theme = match tunnel_params.icon_theme {
            ColorTheme::AutoDetect => self.theme_monitor.current_theme(),
            ColorTheme::Dark => SystemColorTheme::Light,
            ColorTheme::Light => SystemColorTheme::Dark,
        };

        if system_theme.is_dark() {
            &assets::DARK_THEME
        } else {
            &assets::LIGHT_THEME
        }
    }

    fn icon_rgba(&self) -> Vec<u8> {
        let theme = self.icon_theme();

        let argb = match &*self.status {
            Ok(ConnectionStatus::Connected(_)) => theme.connected.clone(),
            Ok(ConnectionStatus::Disconnected) => theme.disconnected.clone(),
            Ok(ConnectionStatus::Mfa(_) | ConnectionStatus::Connecting) => theme.acquiring.clone(),
            _ => theme.error.clone(),
        };

        // assets stores ARGB (rotate_right of tiny_skia's RGBA); convert back to RGBA.
        let mut rgba = argb;
        for pixel in rgba.chunks_exact_mut(4) {
            pixel.rotate_left(1);
        }
        rgba
    }

    fn status_label(&self) -> String {
        match &*self.status {
            Ok(s) => s.to_string(),
            Err(e) => e.to_string(),
        }
    }

    fn update_tray(&self) {
        let icon_rgba = self.icon_rgba();
        let tooltip = self.status_label();
        let disconnected = (*self.status)
            .as_ref()
            .is_ok_and(|s| matches!(s, ConnectionStatus::Disconnected));
        let profiles: Vec<(String, Uuid)> = ConnectionProfilesStore::instance()
            .all()
            .into_iter()
            .map(|p| (p.profile_name.clone(), p.profile_id))
            .collect();

        let _ = slint::invoke_from_event_loop(move || {
            let (menu, ids) = build_menu(&tooltip, disconnected, &profiles);
            if let Ok(mut guard) = MENU_IDS.lock() {
                *guard = ids;
            }

            let icon = if icon_rgba.is_empty() {
                None
            } else {
                Icon::from_rgba(icon_rgba, ICON_SIZE, ICON_SIZE).ok()
            };

            TRAY_ICON.with(|cell| {
                let mut slot = cell.borrow_mut();
                match slot.as_mut() {
                    Some(tray) => {
                        tray.set_menu(Some(Box::new(menu)));
                        let _ = tray.set_icon(icon);
                        // A template image is tinted by the menu bar for the current appearance.
                        #[cfg(target_os = "macos")]
                        tray.set_icon_as_template(true);
                        #[cfg(windows)]
                        let _ = tray.set_tooltip(Some(&tooltip));
                    }
                    None => {
                        // A menu-bar-only app must run as an accessory so the status item appears
                        // reliably and no Dock icon shows. The packaged bundle sets this through
                        // LSUIElement; a bare binary does not, so set it here.
                        #[cfg(target_os = "macos")]
                        if let Some(mtm) = MainThreadMarker::new() {
                            NSApplication::sharedApplication(mtm)
                                .setActivationPolicy(NSApplicationActivationPolicy::Accessory);
                        }

                        let mut builder = TrayIconBuilder::new().with_menu(Box::new(menu));
                        #[cfg(target_os = "macos")]
                        {
                            builder = builder.with_icon_as_template(true);
                        }
                        #[cfg(windows)]
                        {
                            builder = builder.with_tooltip(&tooltip);
                        }
                        if let Some(icon) = icon {
                            builder = builder.with_icon(icon);
                        }
                        match builder.build() {
                            Ok(tray) => *slot = Some(tray),
                            Err(e) => tracing::warn!("failed to build tray icon: {}", e),
                        }
                    }
                }
            });
        });
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let mut rx = self.command_receiver.take().unwrap();

        if !self.no_tray {
            self.update_tray();
            spawn_menu_event_forwarder(self.event_sender.clone());
        }

        while let Some(command) = rx.recv().await {
            match command {
                TrayCommand::Update(status) => {
                    if let Some(status) = status {
                        self.status = status;
                    }
                    if !self.no_tray {
                        self.update_tray();
                    }
                }
                TrayCommand::Exit => {
                    let _ = slint::invoke_from_event_loop(|| {
                        TRAY_ICON.with(|cell| {
                            cell.borrow_mut().take();
                        });
                    });
                    break;
                }
            }
        }

        Ok(())
    }
}

fn build_menu(status_label: &str, disconnected: bool, profiles: &[(String, Uuid)]) -> (Menu, MenuIds) {
    let menu = Menu::new();
    let mut ids = MenuIds::default();

    let label = MenuItem::new(status_label, false, None);
    let _ = menu.append(&label);
    let _ = menu.append(&PredefinedMenuItem::separator());

    if disconnected {
        if profiles.len() < 2 {
            let item = MenuItem::new(crate::tr!("tray-menu-connect"), true, None);
            ids.connect.push((item.id().clone(), DEFAULT_PROFILE_UUID));
            let _ = menu.append(&item);
        } else {
            let submenu = Submenu::new(crate::tr!("tray-menu-connect"), true);
            for (name, uuid) in profiles {
                let item = MenuItem::new(name, true, None);
                ids.connect.push((item.id().clone(), *uuid));
                let _ = submenu.append(&item);
            }
            let _ = menu.append(&submenu);
        }
    } else {
        let item = MenuItem::new(crate::tr!("tray-menu-disconnect"), true, None);
        ids.disconnect = Some(item.id().clone());
        let _ = menu.append(&item);
    }

    let status_item = MenuItem::new(crate::tr!("tray-menu-status"), true, None);
    ids.status = Some(status_item.id().clone());
    let _ = menu.append(&status_item);

    let settings_item = MenuItem::new(crate::tr!("tray-menu-settings"), true, None);
    ids.settings = Some(settings_item.id().clone());
    let _ = menu.append(&settings_item);

    let about_item = MenuItem::new(crate::tr!("tray-menu-about"), true, None);
    ids.about = Some(about_item.id().clone());
    let _ = menu.append(&about_item);

    let exit_item = MenuItem::new(crate::tr!("tray-menu-exit"), true, None);
    ids.exit = Some(exit_item.id().clone());
    let _ = menu.append(&exit_item);

    (menu, ids)
}

fn spawn_menu_event_forwarder(event_sender: Sender<TrayEvent>) {
    std::thread::spawn(move || {
        let receiver = MenuEvent::receiver();
        while let Ok(event) = receiver.recv() {
            let mapped = {
                let Ok(ids) = MENU_IDS.lock() else { continue };
                if let Some((_, uuid)) = ids.connect.iter().find(|(id, _)| id == &event.id) {
                    Some(TrayEvent::Connect(*uuid))
                } else if ids.disconnect.as_ref() == Some(&event.id) {
                    Some(TrayEvent::Disconnect)
                } else if ids.status.as_ref() == Some(&event.id) {
                    Some(TrayEvent::Status)
                } else if ids.settings.as_ref() == Some(&event.id) {
                    Some(TrayEvent::Settings)
                } else if ids.about.as_ref() == Some(&event.id) {
                    Some(TrayEvent::About)
                } else if ids.exit.as_ref() == Some(&event.id) {
                    Some(TrayEvent::Exit)
                } else {
                    None
                }
            };

            if let Some(ev) = mapped
                && event_sender.blocking_send(ev).is_err()
            {
                break;
            }
        }
    });
}
