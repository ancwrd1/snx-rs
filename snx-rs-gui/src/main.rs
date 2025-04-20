use std::{
    cell::OnceCell,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use clap::Parser;
use gtk4::{
    glib::{self, clone, ControlFlow},
    prelude::{ApplicationExt, ApplicationExtManual, GtkWindowExt},
    Application, ApplicationWindow, License,
};
use tokio::sync::mpsc;
use tracing::level_filters::LevelFilter;

use crate::{
    params::CmdlineParams,
    prompt::GtkPrompt,
    theme::init_theme_monitoring,
    tray::{TrayCommand, TrayEvent},
};
use snxcore::{
    browser::SystemBrowser,
    controller::{ServiceCommand, ServiceController},
    model::params::TunnelParams,
    platform::SingleInstance,
    prompt::SecurePrompt,
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
    pub static MAIN_WINDOW: OnceCell<ApplicationWindow> = const { OnceCell::new() };
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cmdline_params = params::CmdlineParams::parse();

    let tunnel_params = Arc::new(TunnelParams::load(cmdline_params.config_file()).unwrap_or_default());

    let uid = unsafe { libc::getuid() };

    let instance = SingleInstance::new(format!("/tmp/snx-rs-gui-{}.lock", uid))?;
    if !instance.is_single() {
        return Ok(());
    }

    init_logging(&tunnel_params);

    let _ = init_theme_monitoring().await;

    let (tray_event_sender, mut tray_event_receiver) = mpsc::channel(16);

    let mut my_tray = tray::AppTray::new(&cmdline_params, tray_event_sender).await?;

    let tray_command_sender = my_tray.sender();

    tokio::spawn(async move { my_tray.run().await });

    let connecting = Arc::new(AtomicBool::new(false));

    let connecting2 = connecting.clone();
    let tray_command_sender2 = tray_command_sender.clone();
    let cmdline_params2 = cmdline_params.clone();

    tokio::spawn(async move { status_poll(connecting2, tray_command_sender2, cmdline_params2).await });

    let app = Application::builder().application_id("com.github.snx-rs").build();

    glib::spawn_future_local(clone!(
        #[weak]
        app,
        async move {
            while let Some(v) = tray_event_receiver.recv().await {
                let params = Arc::new(TunnelParams::load(cmdline_params.config_file()).unwrap_or_default());
                match v {
                    TrayEvent::Connect => {
                        let connecting = connecting.clone();
                        let sender = tray_command_sender.clone();
                        tokio::spawn(async move { do_connect(connecting, sender, params).await });
                    }
                    TrayEvent::Disconnect => {
                        let sender = tray_command_sender.clone();
                        tokio::spawn(async move { do_disconnect(sender, params).await });
                    }
                    TrayEvent::Settings => {
                        MAIN_WINDOW.with(|cell| {
                            settings::start_settings_dialog(cell.get(), tray_command_sender.clone(), params);
                        });
                    }
                    TrayEvent::Exit => {
                        let _ = tray_command_sender.send(TrayCommand::Exit).await;
                        app.quit();
                    }
                    TrayEvent::About => {
                        do_about();
                    }
                }
            }
        }
    ));

    app.connect_activate(move |app| {
        let app_window = ApplicationWindow::builder().application(app).visible(false).build();

        MAIN_WINDOW.with(move |cell| {
            let _ = cell.set(app_window);
        });
    });

    app.run_with_args::<&str>(&[]);

    Ok(())
}

fn do_about() {
    glib::idle_add(|| {
        MAIN_WINDOW.with(|cell| {
            let dialog = gtk4::AboutDialog::builder()
                .modal(true)
                .transient_for(cell.get().unwrap())
                .version(env!("CARGO_PKG_VERSION"))
                .logo_icon_name("network-vpn")
                .website("https://github.com/ancwrd1/snx-rs")
                .authors([env!("CARGO_PKG_AUTHORS")])
                .license_type(License::Agpl30)
                .program_name("SNX-RS VPN Client for Linux")
                .title("SNX-RS VPN Client for Linux")
                .build();

            dialog.present();
        });
        ControlFlow::Break
    });
}

fn init_logging(params: &TunnelParams) {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(params.log_level.parse::<LevelFilter>().unwrap_or(LevelFilter::OFF))
        .finish();

    tracing::subscriber::set_global_default(subscriber).unwrap();
}

async fn status_poll(connecting: Arc<AtomicBool>, sender: mpsc::Sender<TrayCommand>, params: CmdlineParams) {
    let mut prev_status = Arc::new(Err(anyhow::anyhow!("No service connection!")));

    let mut controller = ServiceController::new(GtkPrompt, SystemBrowser);

    loop {
        if !connecting.load(Ordering::SeqCst) {
            let tunnel_params =
                Arc::new(TunnelParams::load(params.config_file.clone().unwrap_or_default()).unwrap_or_default());
            let status = controller.command(ServiceCommand::Status, tunnel_params.clone()).await;
            let status_str = format!("{status:?}");

            if status_str != format!("{:?}", *prev_status) {
                prev_status = Arc::new(status);
                let _ = sender
                    .send(TrayCommand::Update {
                        connecting: None,
                        status: Some(prev_status.clone()),
                    })
                    .await;
            }
        }

        tokio::time::sleep(PING_DURATION).await;
    }
}

async fn do_disconnect(sender: mpsc::Sender<TrayCommand>, params: Arc<TunnelParams>) {
    let mut controller = ServiceController::new(GtkPrompt, SystemBrowser);
    let status = controller.command(ServiceCommand::Disconnect, params).await;
    let _ = sender
        .send(TrayCommand::Update {
            connecting: None,
            status: Some(Arc::new(status)),
        })
        .await;
}

async fn do_connect(connecting: Arc<AtomicBool>, sender: mpsc::Sender<TrayCommand>, params: Arc<TunnelParams>) {
    connecting.store(true, Ordering::SeqCst);

    let _ = sender
        .send(TrayCommand::Update {
            connecting: Some(true),
            status: None,
        })
        .await;

    let mut controller = ServiceController::new(GtkPrompt, SystemBrowser);
    let mut status = controller.command(ServiceCommand::Connect, params.clone()).await;

    connecting.store(false, Ordering::SeqCst);

    if let Err(ref e) = status {
        let _ = GtkPrompt.show_notification("Connection error", &e.to_string());
        status = controller.command(ServiceCommand::Status, params).await;
    }

    let _ = sender
        .send(TrayCommand::Update {
            connecting: Some(false),
            status: Some(Arc::new(status)),
        })
        .await;
}
