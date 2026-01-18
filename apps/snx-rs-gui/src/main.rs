use std::{cell::RefCell, collections::HashMap, sync::Arc, time::Duration};

use clap::{CommandFactory, Parser};
use gtk4::{
    Application, ApplicationWindow, License, Window,
    glib::{self, clone},
    prelude::{ApplicationExt, ApplicationExtManual, Cast, GtkWindowExt, IsA, WidgetExt},
};
use i18n::tr;
use snxcore::{
    browser::BrowserController,
    controller::{ServiceCommand, ServiceController},
    model::{ConnectionStatus, params::TunnelParams},
    platform::{Platform, PlatformAccess, SingleInstance},
    prompt::SecurePrompt,
};
use tokio::sync::mpsc;
use tracing::{level_filters::LevelFilter, warn};

use crate::{
    params::CmdlineParams,
    profiles::ConnectionProfilesStore,
    prompt::GtkPrompt,
    status::show_status_dialog,
    theme::init_theme_monitoring,
    tray::{TrayCommand, TrayEvent},
};

mod assets;
mod dbus;
mod ipc;
mod params;
mod profiles;
mod prompt;
mod settings;
mod status;
mod theme;
mod tray;
#[cfg(feature = "mobile-access")]
mod webkit;

pub const POLL_INTERVAL: Duration = Duration::from_secs(1);

thread_local! {
    static WINDOWS: RefCell<HashMap<String, Window>> = RefCell::new(HashMap::new());
}

pub fn main_window() -> ApplicationWindow {
    get_window("main").unwrap().downcast::<ApplicationWindow>().unwrap()
}

pub fn get_window(name: &str) -> Option<Window> {
    WINDOWS.with(|cell| cell.borrow().get(name).cloned())
}

pub fn set_window<W: Cast + IsA<Window>>(name: &str, window: Option<W>) {
    WINDOWS.with(|cell| {
        if let Some(window) = window {
            cell.borrow_mut().insert(name.to_string(), window.upcast::<Window>())
        } else {
            cell.borrow_mut().remove(name)
        }
    });
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cmdline_params = params::CmdlineParams::parse();

    // Handle completions immediately and exit
    if let Some(shell) = cmdline_params.completions {
        clap_complete::generate(
            shell,
            &mut CmdlineParams::command(),
            "snx-rs-gui",
            &mut std::io::stdout(),
        );
        return Ok(());
    }

    let default_params = TunnelParams::load(TunnelParams::default_config_path()).unwrap_or_default();

    init_logging(&default_params);

    if let Some(locale) = default_params.locale.as_ref().and_then(|v| v.parse().ok()) {
        i18n::set_locale(Some(locale));
    }

    let uid = unsafe { libc::getuid() };

    let platform = Platform::get();
    let instance = platform.new_single_instance(format!("/tmp/snx-rs-gui-{uid}.lock"))?;
    if !instance.is_single() {
        if let Some(mut command) = cmdline_params.command {
            if matches!(command, TrayEvent::Connect(_)) && default_params.server_name.is_empty() {
                command = TrayEvent::Settings;
            }
            if let Err(e) = ipc::send_event(command).await {
                warn!("Failed to send event: {}", e);
            }
        }
        return Ok(());
    }

    let _ = init_theme_monitoring().await;

    let (tray_event_sender, mut tray_event_receiver) = mpsc::channel(16);

    if let Err(e) = ipc::start_ipc_listener(tray_event_sender.clone()) {
        warn!("Failed to start IPC listener: {}", e);
    }

    let mut my_tray = tray::AppTray::new(tray_event_sender.clone()).await?;

    let tray_command_sender = my_tray.sender();

    tokio::spawn(async move { my_tray.run().await });

    let tray_command_sender2 = tray_command_sender.clone();
    let tray_event_sender2 = tray_event_sender.clone();

    tokio::spawn(async move { status_poll(tray_command_sender2, tray_event_sender2).await });

    let app = Application::builder().application_id("com.github.snx-rs").build();

    let tray_event_sender2 = tray_event_sender.clone();

    glib::spawn_future_local(clone!(
        #[weak]
        app,
        async move {
            let mut cancel_sender = None;

            while let Some(v) = tray_event_receiver.recv().await {
                match v {
                    TrayEvent::Connect(uuid) => {
                        let sender = tray_command_sender.clone();
                        let (tx, rx) = mpsc::channel(16);
                        cancel_sender = Some(tx);

                        if let Some(profile) = ConnectionProfilesStore::instance().get(uuid) {
                            ConnectionProfilesStore::instance().set_connected(uuid);
                            tokio::spawn(async move { do_connect(sender, profile, rx).await });
                        }
                    }
                    TrayEvent::Disconnect => {
                        let sender = tray_command_sender.clone();
                        let cancel_sender = cancel_sender.take();
                        let params = ConnectionProfilesStore::instance().get_connected();
                        tokio::spawn(async move { do_disconnect(sender, params, cancel_sender).await });
                    }
                    TrayEvent::Settings => settings::start_settings_dialog(
                        main_window(),
                        tray_command_sender.clone(),
                        ConnectionProfilesStore::instance().get_connected().profile_id,
                    ),
                    TrayEvent::Exit => {
                        let _ = tray_command_sender.send(TrayCommand::Exit).await;
                        app.quit();
                    }
                    TrayEvent::About => do_about(),

                    TrayEvent::Status => {
                        do_status(tray_event_sender2.clone());
                    }
                }
            }
        }
    ));

    app.connect_activate(move |app| {
        let app_window = ApplicationWindow::builder().application(app).visible(false).build();

        let provider = gtk4::CssProvider::new();
        provider.load_from_data(assets::APP_CSS);

        gtk4::style_context_add_provider_for_display(
            &app_window.display(),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        set_window("main", Some(app_window));
    });

    if let Some(mut command) = cmdline_params.command {
        tokio::spawn(async move {
            let params = ConnectionProfilesStore::instance().get_default();
            if matches!(command, TrayEvent::Connect(_)) && params.server_name.is_empty() {
                command = TrayEvent::Settings;
            }

            tray_event_sender.send(command).await
        });
    }

    app.run_with_args::<&str>(&[]);

    Ok(())
}

fn do_about() {
    glib::idle_add_once(|| {
        if let Some(dialog) = get_window("about") {
            dialog.present();
            return;
        }

        let dialog = gtk4::AboutDialog::builder()
            .transient_for(&main_window())
            .version(env!("CARGO_PKG_VERSION"))
            .logo_icon_name("network-vpn")
            .website("https://github.com/ancwrd1/snx-rs")
            .authors([env!("CARGO_PKG_AUTHORS")])
            .license_type(License::Agpl30)
            .program_name(tr!("app-title"))
            .title(tr!("app-title"))
            .build();

        set_window("about", Some(dialog.clone()));

        dialog.connect_close_request(|_| {
            set_window("about", None::<gtk4::Dialog>);
            glib::signal::Propagation::Proceed
        });
        dialog.present();
    });
}

fn do_status(sender: mpsc::Sender<TrayEvent>) {
    glib::idle_add_once(move || {
        glib::spawn_future_local(async move { show_status_dialog(sender).await });
    });
}

fn init_logging(params: &TunnelParams) {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(params.log_level.parse::<LevelFilter>().unwrap_or(LevelFilter::OFF))
        .finish();

    tracing::subscriber::set_global_default(subscriber).unwrap();
}

async fn status_poll(command_sender: mpsc::Sender<TrayCommand>, event_sender: mpsc::Sender<TrayEvent>) {
    let mut controller = ServiceController::new(
        GtkPrompt,
        new_browser_controller(ConnectionProfilesStore::instance().get_connected()),
    );

    let mut first_run = true;
    let mut old_status = Arc::new(Ok(ConnectionStatus::Disconnected));

    loop {
        let params = ConnectionProfilesStore::instance().get_connected();
        let status = controller.command(ServiceCommand::Status, params.clone()).await;

        if !status::same_status(&status, &old_status) {
            let is_disconnected = matches!(status, Ok(ConnectionStatus::Disconnected));

            old_status = Arc::new(status);

            let _ = command_sender.send(TrayCommand::Update(Some(old_status.clone()))).await;

            if first_run {
                first_run = false;
                if params.auto_connect && is_disconnected {
                    let _ = event_sender.send(TrayEvent::Connect(params.profile_id)).await;
                }
            }
        }

        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

async fn do_disconnect(
    sender: mpsc::Sender<TrayCommand>,
    params: Arc<TunnelParams>,
    cancel_sender: Option<mpsc::Sender<()>>,
) {
    let mut controller = ServiceController::new(GtkPrompt, new_browser_controller(params.clone()));
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

    let mut controller = ServiceController::new(GtkPrompt, new_browser_controller(params.clone()));

    let mut status = tokio::select! {
        _ = cancel_receiver.recv() => Err(anyhow::anyhow!(tr!("error-connection-cancelled"))),
        status = controller.command(ServiceCommand::Connect, params.clone()) => status
    };

    if let Err(ref e) = status {
        let message = tr!("app-connection-error");
        let _ = GtkPrompt.show_notification(&message, &e.to_string()).await;
        status = controller.command(ServiceCommand::Status, params).await;
    } else if let Ok(ConnectionStatus::Connected(_)) = status {
        let message = tr!("app-connection-success");
        let _ = GtkPrompt
            .show_notification(&message, &tr!("connection-connected-to", server = params.server_name))
            .await;
    };

    let _ = sender.send(TrayCommand::Update(Some(Arc::new(status)))).await;
}

#[cfg(feature = "mobile-access")]
fn new_browser_controller(params: Arc<TunnelParams>) -> impl BrowserController {
    webkit::WebKitBrowser::new(params)
}

#[cfg(not(feature = "mobile-access"))]
fn new_browser_controller(_params: Arc<TunnelParams>) -> impl BrowserController {
    snxcore::browser::SystemBrowser
}
