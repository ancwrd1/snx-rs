use std::{sync::Arc, time::Duration};

use clap::Parser;
use gtk::{
    glib::{self, ControlFlow},
    prelude::{ApplicationExt, ApplicationExtManual, DialogExt, GtkWindowExt},
    Application, License,
};
use tracing::level_filters::LevelFilter;
use tray_icon::menu::MenuEvent;

use snxcore::{controller::ServiceCommand, model::params::TunnelParams, platform::SingleInstance};

use crate::theme::init_theme_monitoring;

mod assets;
mod dbus;
mod params;
mod prompt;
mod settings;
mod theme;
mod tray;
mod webkit;

const PING_DURATION: Duration = Duration::from_secs(1);

fn main() -> anyhow::Result<()> {
    let params = params::CmdlineParams::parse();

    let tunnel_params = Arc::new(TunnelParams::load(params.config_file()).unwrap_or_default());

    let instance = SingleInstance::new("/tmp/snx-rs-gui.s")?;
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

    let _ = init_theme_monitoring();

    let app = Application::builder().application_id("com.github.snx-rs").build();

    app.connect_activate(move |_| {
        let params = params.clone();

        let mut my_tray = tray::AppTray::new(params.clone()).unwrap();
        let sender = my_tray.sender();

        let tx_copy = sender.clone();
        std::thread::spawn(move || loop {
            let _ = tx_copy.send_blocking(Some(ServiceCommand::Status));
            std::thread::sleep(PING_DURATION);
        });

        std::thread::spawn(move || {
            while let Ok(v) = MenuEvent::receiver().recv() {
                match v.id.0.as_str() {
                    "connect" => {
                        let _ = sender.send_blocking(Some(ServiceCommand::Connect));
                    }
                    "disconnect" => {
                        let _ = sender.send_blocking(Some(ServiceCommand::Disconnect));
                    }
                    "settings" => {
                        let params = TunnelParams::load(params.config_file()).unwrap_or_default();
                        settings::start_settings_dialog(Arc::new(params));
                    }
                    "exit" => {
                        let _ = sender.send_blocking(None);
                        glib::idle_add(|| {
                            gtk::main_quit();
                            ControlFlow::Break
                        });
                    }
                    "about" => {
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
                    _ => {}
                }
            }
        });

        glib::spawn_future_local(async move { my_tray.run().await });

        gtk::main();
    });

    app.run_with_args::<&str>(&[]);

    Ok(())
}
