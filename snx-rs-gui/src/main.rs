use std::{sync::Arc, time::Duration};

use clap::Parser;
use gtk::{
    glib::{self, ControlFlow},
    prelude::{ApplicationExt, ApplicationExtManual, DialogExt, GtkWindowExt},
    Application, License,
};
use tracing::level_filters::LevelFilter;

use snxcore::{controller::ServiceCommand, model::params::TunnelParams, platform::SingleInstance};

use crate::theme::init_theme_monitoring;
use crate::tray::{TrayCommand, TrayEvent};

mod assets;
mod dbus;
mod params;
mod prompt;
mod settings;
mod theme;
mod tray;
const PING_DURATION: Duration = Duration::from_secs(2);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let params = params::CmdlineParams::parse();

    let tunnel_params = Arc::new(TunnelParams::load(params.config_file()).unwrap_or_default());

    let uid = unsafe { libc::getuid() };
    let instance = SingleInstance::new(format!("/tmp/snx-rs-gui-{}.lock", uid))?;
    if !instance.is_single() {
        return Ok(());
    }

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(
            tunnel_params
                .log_level
                .parse::<LevelFilter>()
                .unwrap_or(LevelFilter::OFF),
        )
        .finish();

    tracing::subscriber::set_global_default(subscriber).unwrap();

    let _ = init_theme_monitoring().await;

    let (event_sender, event_receiver) = async_channel::bounded(16);

    let mut my_tray = tray::AppTray::new(&params, event_sender).await?;

    let sender = my_tray.sender();

    let tx_copy = sender.clone();

    tokio::spawn(async move {
        loop {
            let _ = tx_copy.send(TrayCommand::Service(ServiceCommand::Status)).await;
            tokio::time::sleep(PING_DURATION).await;
        }
    });

    tokio::spawn(async move { my_tray.run().await });

    let params = params.clone();
    let sender2 = sender.clone();

    tokio::spawn(async move {
        while let Ok(v) = event_receiver.recv().await {
            match v {
                TrayEvent::Connect => {
                    let _ = sender2.send(TrayCommand::Service(ServiceCommand::Connect)).await;
                }
                TrayEvent::Disconnect => {
                    let _ = sender2.send(TrayCommand::Service(ServiceCommand::Disconnect)).await;
                }
                TrayEvent::Settings => {
                    let params = TunnelParams::load(params.config_file()).unwrap_or_default();
                    settings::start_settings_dialog(sender2.clone(), Arc::new(params));
                }
                TrayEvent::Exit => {
                    let _ = sender2.send(TrayCommand::Exit).await;
                    glib::idle_add(|| {
                        gtk::main_quit();
                        ControlFlow::Break
                    });
                }
                TrayEvent::About => {
                    glib::idle_add(|| {
                        let dialog = gtk::AboutDialog::builder()
                            .version(env!("CARGO_PKG_VERSION"))
                            .logo_icon_name("network-vpn")
                            .website("https://github.com/ancwrd1/snx-rs")
                            .authors(["Dmitry Pankratov"])
                            .license_type(License::Agpl30)
                            .program_name("SNX-RS VPN Client for Linux")
                            .title("SNX-RS VPN Client for Linux")
                            .build();

                        dialog.run();
                        dialog.close();

                        ControlFlow::Break
                    });
                }
            }
        }
    });

    let app = Application::builder().application_id("com.github.snx-rs").build();

    app.connect_activate(move |_| {
        if tunnel_params.ike_persist {
            let _ = sender.send_blocking(TrayCommand::Service(ServiceCommand::Connect));
        }

        gtk::main();
    });

    app.run_with_args::<&str>(&[]);

    Ok(())
}
