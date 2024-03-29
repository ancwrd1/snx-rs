use std::path::PathBuf;
use std::{
    sync::{mpsc, Arc},
    time::Duration,
};

use anyhow::anyhow;
use gtk::{glib, glib::ControlFlow};
use ksni::{menu::StandardItem, Icon, MenuItem, Tray, TrayService};

use snxcore::{
    controller::{ServiceCommand, ServiceController},
    model::{params::TunnelParams, ConnectionStatus},
    platform,
    prompt::SecurePrompt,
};

use crate::params::CmdlineParams;
use crate::{prompt, webkit};

const TITLE: &str = "SNX-RS VPN client";
const PING_DURATION: Duration = Duration::from_secs(1);

struct MyTray {
    command_sender: mpsc::SyncSender<Option<ServiceCommand>>,
    status: anyhow::Result<ConnectionStatus>,
    connecting: bool,
    config_file: PathBuf,
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
        glib::idle_add(|| {
            gtk::main_quit();
            ControlFlow::Break
        });
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
    fn settings(&mut self) {
        let mut params = TunnelParams::load(&self.config_file).unwrap_or_default();
        let _ = params.decode_password();
        super::settings::start_settings_dialog(Arc::new(params));
    }
}

impl Tray for MyTray {
    fn title(&self) -> String {
        TITLE.to_owned()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        let theme = crate::assets::current_icon_theme();

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
                label: "Settings...".to_string(),
                activate: Box::new(|this: &mut Self| this.settings()),
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

pub fn show_tray_icon(params: CmdlineParams) -> anyhow::Result<()> {
    let (tx, rx) = mpsc::sync_channel(1);
    let service = TrayService::new(MyTray {
        command_sender: tx.clone(),
        status: Err(anyhow!("No service connection")),
        connecting: false,
        config_file: params.config_file().clone(),
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
    let mut prev_theme = None;

    while let Ok(Some(command)) = rx.recv() {
        let theme = platform::system_color_theme().ok();
        if theme != prev_theme {
            prev_theme = theme;
            handle.update(|_| {});
        }

        if let Ok(mut controller) =
            ServiceController::new(prompt::GtkPrompt, webkit::WebkitBrowser, params.config_file())
        {
            if command == ServiceCommand::Connect {
                handle.update(|tray: &mut MyTray| tray.connecting = true);
            }

            let result =
                std::thread::scope(|s| s.spawn(|| snxcore::util::block_on(controller.command(command))).join());
            let status = match result {
                Ok(result) => result,
                Err(_) => Err(anyhow!("Internal error")),
            };

            let status_str = format!("{:?}", status);

            match status {
                Err(ref e) if command == ServiceCommand::Connect => {
                    let _ = prompt::GtkPrompt.show_notification("Connection failed", &e.to_string());
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
