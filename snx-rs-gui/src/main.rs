use std::{cell::OnceCell, sync::Arc, time::Duration};

use clap::Parser;
use gtk4::prelude::WidgetExt;
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
    status::show_status_dialog,
    theme::init_theme_monitoring,
    tray::{TrayCommand, TrayEvent},
};
use snxcore::{
    browser::SystemBrowser,
    controller::{ServiceCommand, ServiceController},
    model::{params::TunnelParams, ConnectionInfo, ConnectionStatus},
    platform::SingleInstance,
    prompt::SecurePrompt,
};

mod assets;
mod dbus;
mod params;
mod prompt;
mod settings;
mod status;
mod theme;
mod tray;

const PING_DURATION: Duration = Duration::from_secs(2);

thread_local! {
    static MAIN_WINDOW: OnceCell<ApplicationWindow> = const { OnceCell::new() };
}

pub fn main_window() -> ApplicationWindow {
    MAIN_WINDOW.with(|cell| cell.get().cloned()).unwrap()
}

const APP_CSS: &str = include_str!("../assets/app.css");

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

    let tray_command_sender2 = tray_command_sender.clone();
    let cmdline_params2 = cmdline_params.clone();

    tokio::spawn(async move { status_poll(tray_command_sender2, cmdline_params2).await });

    let app = Application::builder().application_id("com.github.snx-rs").build();

    glib::spawn_future_local(clone!(
        #[weak]
        app,
        async move {
            let mut cancel_sender = None;

            while let Some(v) = tray_event_receiver.recv().await {
                let params = Arc::new(TunnelParams::load(cmdline_params.config_file()).unwrap_or_default());
                match v {
                    TrayEvent::Connect => {
                        let sender = tray_command_sender.clone();
                        let (tx, rx) = mpsc::channel(16);
                        cancel_sender = Some(tx);
                        tokio::spawn(async move { do_connect(sender, params, rx).await });
                    }
                    TrayEvent::Disconnect => {
                        let sender = tray_command_sender.clone();
                        let cancel_sender = cancel_sender.take();
                        tokio::spawn(async move { do_disconnect(sender, params, cancel_sender).await });
                    }
                    TrayEvent::Settings => settings::start_settings_dialog(tray_command_sender.clone(), params),
                    TrayEvent::Exit => {
                        let _ = tray_command_sender.send(TrayCommand::Exit).await;
                        app.quit();
                    }
                    TrayEvent::About => do_about(),

                    TrayEvent::Status => {
                        let status = ServiceController::new(GtkPrompt, SystemBrowser)
                            .command(ServiceCommand::Status, params.clone())
                            .await;
                        if let Ok(ConnectionStatus::Connected(info)) = status {
                            do_status(info);
                        } else {
                            let _ = GtkPrompt
                                .show_notification("Status error", "Unable to get a connection information")
                                .await;
                        }
                    }
                }
            }
        }
    ));

    app.connect_activate(move |app| {
        let app_window = ApplicationWindow::builder().application(app).visible(false).build();

        let provider = gtk4::CssProvider::new();
        provider.load_from_data(APP_CSS);

        gtk4::style_context_add_provider_for_display(
            &app_window.display(),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        MAIN_WINDOW.with(move |cell| {
            let _ = cell.set(app_window);
        });
    });

    app.run_with_args::<&str>(&[]);

    Ok(())
}

fn do_about() {
    glib::idle_add(|| {
        let dialog = gtk4::AboutDialog::builder()
            .modal(true)
            .transient_for(&main_window())
            .version(env!("CARGO_PKG_VERSION"))
            .logo_icon_name("network-vpn")
            .website("https://github.com/ancwrd1/snx-rs")
            .authors([env!("CARGO_PKG_AUTHORS")])
            .license_type(License::Agpl30)
            .program_name("SNX-RS VPN Client for Linux")
            .title("SNX-RS VPN Client for Linux")
            .build();

        dialog.present();
        ControlFlow::Break
    });
}

fn do_status(info: ConnectionInfo) {
    glib::idle_add(move || {
        let info = info.clone();
        glib::spawn_future_local(async move { show_status_dialog(info).await });
        ControlFlow::Break
    });
}

fn init_logging(params: &TunnelParams) {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(params.log_level.parse::<LevelFilter>().unwrap_or(LevelFilter::OFF))
        .finish();

    tracing::subscriber::set_global_default(subscriber).unwrap();
}

async fn status_poll(sender: mpsc::Sender<TrayCommand>, params: CmdlineParams) {
    let mut prev_status = Arc::new(Err(anyhow::anyhow!("No service connection!")));

    let mut controller = ServiceController::new(GtkPrompt, SystemBrowser);

    loop {
        let tunnel_params = Arc::new(TunnelParams::load(params.config_file()).unwrap_or_default());

        let status = controller.command(ServiceCommand::Status, tunnel_params.clone()).await;
        let status_str = format!("{status:?}");

        if status_str != format!("{:?}", *prev_status) {
            prev_status = Arc::new(status);
            let _ = sender.send(TrayCommand::Update(Some(prev_status.clone()))).await;
        }

        tokio::time::sleep(PING_DURATION).await;
    }
}

async fn do_disconnect(
    sender: mpsc::Sender<TrayCommand>,
    params: Arc<TunnelParams>,
    cancel_sender: Option<mpsc::Sender<()>>,
) {
    let mut controller = ServiceController::new(GtkPrompt, SystemBrowser);
    let status = controller.command(ServiceCommand::Disconnect, params).await;
    let _ = sender.send(TrayCommand::Update(Some(Arc::new(status)))).await;
    if let Some(cancel_sender) = cancel_sender {
        let _ = cancel_sender.send(()).await;
    }
}

async fn do_connect(
    sender: mpsc::Sender<TrayCommand>,
    params: Arc<TunnelParams>,
    mut cancel_receiver: mpsc::Receiver<()>,
) {
    let _ = sender.send(TrayCommand::Update(None)).await;

    let mut controller = ServiceController::new(GtkPrompt, SystemBrowser);

    let mut status = tokio::select! {
        _ = cancel_receiver.recv() => Err(anyhow::anyhow!("Connection cancelled!")),
        status = controller.command(ServiceCommand::Connect, params.clone()) => status
    };

    if let Err(ref e) = status {
        let _ = GtkPrompt.show_notification("Connection error", &e.to_string()).await;
        status = controller.command(ServiceCommand::Status, params).await;
    } else if let Ok(ConnectionStatus::Connected(_)) = status {
        let _ = GtkPrompt
            .show_notification("Connection succeeded", &format!("Connected to {}", params.server_name))
            .await;
    };

    let _ = sender.send(TrayCommand::Update(Some(Arc::new(status)))).await;
}
