use std::{cell::OnceCell, sync::Arc, time::Duration};

use clap::Parser;
use gtk4::{
    glib::{self, clone, ControlFlow},
    prelude::{ApplicationExt, ApplicationExtManual, WidgetExt},
    Application, ApplicationWindow, License,
};
use tracing::level_filters::LevelFilter;

use snxcore::{controller::ServiceCommand, model::params::TunnelParams, platform::SingleInstance};

use crate::{
    theme::init_theme_monitoring,
    tray::{TrayCommand, TrayEvent},
};

mod assets;
mod dbus;
mod params;
mod prompt;
mod settings;
mod theme;
mod tray;

const PING_DURATION: Duration = Duration::from_secs(2);

thread_local! {
    static MAIN_WINDOW: OnceCell<ApplicationWindow> = OnceCell::new();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let params = params::CmdlineParams::parse();

    let tunnel_params = Arc::new(TunnelParams::load(params.config_file()).unwrap_or_default());

    let uid = unsafe { libc::getuid() };
    let instance = SingleInstance::new(format!("/tmp/snx-rs-gui-{}.lock", uid))?;
    if !instance.is_single() {
        return Ok(());
    }

    gtk4::init()?;

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

    let app = Application::builder().application_id("com.github.snx-rs").build();

    glib::spawn_future_local(clone!(
        #[weak]
        app,
        async move {
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
                        MAIN_WINDOW.with(|cell| {
                            settings::start_settings_dialog(cell.get(), sender2.clone(), Arc::new(params));
                        });
                    }
                    TrayEvent::Exit => {
                        let _ = sender2.send(TrayCommand::Exit).await;
                        app.quit();
                    }
                    TrayEvent::About => {
                        glib::idle_add(|| {
                            let dialog = gtk4::AboutDialog::builder()
                                .version(env!("CARGO_PKG_VERSION"))
                                .logo_icon_name("network-vpn")
                                .website("https://github.com/ancwrd1/snx-rs")
                                .authors(["Dmitry Pankratov"])
                                .license_type(License::Agpl30)
                                .program_name("SNX-RS VPN Client for Linux")
                                .title("SNX-RS VPN Client for Linux")
                                .build();

                            dialog.show();

                            ControlFlow::Break
                        });
                    }
                }
            }
        }
    ));

    app.connect_activate(move |app| {
        let app_window = ApplicationWindow::builder().application(app).visible(false).build();

        MAIN_WINDOW.with(move |cell| {
            cell.set(app_window).unwrap();
        });
    });

    app.run_with_args::<&str>(&[]);

    Ok(())
}
