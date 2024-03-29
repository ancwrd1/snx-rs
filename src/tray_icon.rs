#![cfg(feature = "tray-icon")]

use std::{io::Cursor, sync::mpsc, time::Duration};

use anyhow::anyhow;
use directories_next::ProjectDirs;
use ksni::{menu::StandardItem, Icon, MenuItem, Tray, TrayService};
use once_cell::sync::Lazy;

use crate::{
    browser::BrowserController,
    controller::{ServiceCommand, ServiceController},
    model::ConnectionStatus,
    platform::{self, SingleInstance, SystemColorTheme},
    prompt::SecurePrompt,
    util,
};

struct IconTheme {
    acquiring: Vec<u8>,
    error: Vec<u8>,
    disconnected: Vec<u8>,
    connected: Vec<u8>,
}

const DARK_THEME: Lazy<IconTheme> = Lazy::new(|| IconTheme {
    acquiring: png_to_argb(include_bytes!("../assets/icons/dark/network-vpn-acquiring.png")).unwrap_or_default(),
    error: png_to_argb(include_bytes!("../assets/icons/dark/network-vpn-error.png")).unwrap_or_default(),
    disconnected: png_to_argb(include_bytes!("../assets/icons/dark/network-vpn-disconnected.png")).unwrap_or_default(),
    connected: png_to_argb(include_bytes!("../assets/icons/dark/network-vpn-connected.png")).unwrap_or_default(),
});

const LIGHT_THEME: Lazy<IconTheme> = Lazy::new(|| IconTheme {
    acquiring: png_to_argb(include_bytes!("../assets/icons/light/network-vpn-acquiring.png")).unwrap_or_default(),
    error: png_to_argb(include_bytes!("../assets/icons/light/network-vpn-error.png")).unwrap_or_default(),
    disconnected: png_to_argb(include_bytes!("../assets/icons/light/network-vpn-disconnected.png")).unwrap_or_default(),
    connected: png_to_argb(include_bytes!("../assets/icons/light/network-vpn-connected.png")).unwrap_or_default(),
});

const TITLE: &str = "SNX-RS VPN client";
const PING_DURATION: Duration = Duration::from_secs(1);

fn png_to_argb(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    let decoder = png::Decoder::new(Cursor::new(data));
    let mut reader = decoder.read_info()?;
    let mut buf = vec![0; reader.output_buffer_size()];

    let info = reader.next_frame(&mut buf)?;
    let mut bytes = buf[..info.buffer_size()].to_vec();

    bytes.chunks_mut(4).for_each(|c| c.rotate_right(1));

    Ok(bytes)
}

struct MyTray {
    command_sender: mpsc::SyncSender<Option<ServiceCommand>>,
    status: anyhow::Result<ConnectionStatus>,
    connecting: bool,
}

impl MyTray {
    fn connect(&mut self) {
        let _ = self.command_sender.send(Some(ServiceCommand::Connect));
    }

    fn disconnect(&mut self) {
        let _ = self.command_sender.send(Some(ServiceCommand::Disconnect));
    }

    fn quit(&mut self) {
        let _ = self.command_sender.send(None);
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
    fn edit_config(&mut self) {
        if let Ok(dir) = ProjectDirs::from("", "", "snx-rs").ok_or(anyhow!("No project directory!")) {
            let config_file = dir.config_dir().join("snx-rs.conf");
            let _ = opener::open(config_file);
        }
    }
}

impl Tray for MyTray {
    fn title(&self) -> String {
        TITLE.to_owned()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        let theme = if platform::system_color_theme().unwrap_or_default() == SystemColorTheme::Dark {
            DARK_THEME
        } else {
            LIGHT_THEME
        };

        let data: &[u8] = if self.connecting {
            &theme.acquiring
        } else {
            match self.status {
                Ok(ref status) => {
                    if status.connected_since.is_some() {
                        &theme.connected
                    } else {
                        &theme.disconnected
                    }
                }
                Err(_) => &theme.error,
            }
        };

        vec![Icon {
            width: 256,
            height: 256,
            data: data.to_vec(),
        }]
    }
    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            MenuItem::Standard(StandardItem {
                label: self.status_label(),
                enabled: false,
                ..Default::default()
            }),
            MenuItem::Separator,
            MenuItem::Standard(StandardItem {
                label: "Connect".to_string(),
                enabled: self
                    .status
                    .as_ref()
                    .is_ok_and(|status| status.connected_since.is_none() && status.mfa.is_none())
                    && !self.connecting,
                activate: Box::new(|this: &mut Self| this.connect()),
                ..Default::default()
            }),
            MenuItem::Standard(StandardItem {
                label: "Disconnect".to_string(),
                enabled: self
                    .status
                    .as_ref()
                    .is_ok_and(|status| status.connected_since.is_some()),
                activate: Box::new(|this: &mut Self| this.disconnect()),
                ..Default::default()
            }),
            MenuItem::Standard(StandardItem {
                label: "Edit configuration".to_string(),
                icon_name: "edit-text".to_owned(),
                activate: Box::new(|this: &mut Self| this.edit_config()),
                ..Default::default()
            }),
            MenuItem::Separator,
            MenuItem::Standard(StandardItem {
                label: "Exit".to_string(),
                icon_name: "application-exit".to_owned(),
                activate: Box::new(|this: &mut Self| this.quit()),
                ..Default::default()
            }),
        ]
    }
}

pub fn show_tray_icon(browser_controller: &BrowserController) -> anyhow::Result<()> {
    let instance = SingleInstance::new("/tmp/snxctl.s")?;
    if !instance.is_single() {
        return Ok(());
    }

    let (tx, rx) = mpsc::sync_channel(1);
    let service = TrayService::new(MyTray {
        command_sender: tx.clone(),
        status: Err(anyhow!("No service connection")),
        connecting: false,
    });
    let handle = service.handle();
    service.spawn();

    let tx_copy = tx.clone();
    std::thread::spawn(move || loop {
        let _ = tx_copy.send(Some(ServiceCommand::Status));
        std::thread::sleep(PING_DURATION);
    });

    let mut prev_command = ServiceCommand::Info;
    let mut prev_status = String::new();

    while let Ok(Some(command)) = rx.recv() {
        if let Ok(mut controller) = ServiceController::new(SecurePrompt::gui(), &browser_controller) {
            if command == ServiceCommand::Connect {
                handle.update(|tray: &mut MyTray| tray.connecting = true);
            }

            let result = std::thread::scope(|s| s.spawn(|| util::block_on(controller.command(command))).join());
            let status = match result {
                Ok(result) => result,
                Err(_) => Err(anyhow!("Internal error")),
            };

            let status_str = format!("{:?}", status);

            match status {
                Err(ref e) if command == ServiceCommand::Connect => {
                    let _ = SecurePrompt::gui().show_notification("Connection failed", &e.to_string());
                }
                _ => {}
            }

            if command != prev_command || status_str != prev_status {
                handle.update(|tray: &mut MyTray| {
                    tray.connecting = false;
                    tray.status = status;
                });
            }
            prev_command = command;
            prev_status = status_str;
        }
    }

    Ok(())
}
