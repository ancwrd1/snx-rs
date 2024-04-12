use std::sync::Arc;
use std::time::Duration;

use ::tray_icon::menu::MenuEvent;
use clap::Parser;
use gtk::{
    glib::{self, ControlFlow},
    prelude::{ApplicationExt, ApplicationExtManual},
    Application,
};
use tracing::level_filters::LevelFilter;

use snxcore::{
    controller::{ServiceCommand, ServiceController},
    model::params::TunnelParams,
    platform::SingleInstance,
};

pub mod assets;
pub mod params;
pub mod prompt;
pub mod settings;
pub mod tray_icon;
pub mod webkit;

const PING_DURATION: Duration = Duration::from_secs(1);

fn main() -> anyhow::Result<()> {
    let params = params::CmdlineParams::parse();

    let tunnel_params = Arc::new(TunnelParams::load(&params.config_file())?);

    let instance = SingleInstance::new("/tmp/snx-rs-gui.s")?;
    if !instance.is_single() {
        return Ok(());
    }

    let _ = snxcore::platform::init_theme_monitoring();

    let app = Application::builder().application_id("com.github.snx-rs").build();

    app.connect_activate(move |_| {
        let service_controller = ServiceController::new(
            prompt::GtkPrompt,
            webkit::WebkitBrowser(tunnel_params.clone()),
            tunnel_params.clone(),
        )
        .unwrap();

        let subscriber = tracing_subscriber::fmt()
            .with_max_level(
                service_controller
                    .params
                    .log_level
                    .parse::<LevelFilter>()
                    .unwrap_or(LevelFilter::OFF),
            )
            .finish();
        tracing::subscriber::set_global_default(subscriber).unwrap();

        let params = params.clone();

        let mut my_tray = tray_icon::create_tray_icon(params.clone()).unwrap();
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
