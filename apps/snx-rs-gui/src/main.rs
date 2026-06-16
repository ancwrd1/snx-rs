#![windows_subsystem = "windows"]

use std::{sync::Arc, time::Duration};

use clap::{CommandFactory, Parser};
use i18n::tr;
use snxcore::{
    controller::{ServiceCommand, ServiceController},
    model::{ConnectionStatus, params::TunnelParams},
    platform::{Platform, PlatformAccess, SingleInstance},
    profiles::ConnectionProfilesStore,
    prompt::SecurePrompt,
    tunnel::{TunnelConnectorFactory, connector::CheckPointConnectorFactory},
};
use tokio::sync::mpsc;
use tracing::{level_filters::LevelFilter, warn};

use crate::{
    params::CmdlineParams,
    platform::{TrayCommand, TrayEvent},
    ui::{
        about::AboutWindowController, open_window, prompt::SlintPrompt, settings::SettingsWindowController,
        status::StatusWindowController,
    },
};

mod assets;
mod ipc;
mod params;
mod platform;
mod theme;
mod ui;
#[cfg(feature = "mobile-access")]
mod webkit;

pub const POLL_INTERVAL: Duration = Duration::from_secs(1);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cmdline_params = params::CmdlineParams::parse();

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

    #[cfg(feature = "mobile-access")]
    if let Some(url) = cmdline_params.webkit.as_deref() {
        let code = tokio::task::block_in_place(|| platform::webkit_main(url, cmdline_params.webkit_ignore_cert));
        std::process::exit(code);
    }

    let instance = Platform::get().new_single_instance(format!("/tmp/snx-rs-gui-{}.lock", platform::user_tag()))?;
    if !instance.is_single() {
        if let Some(mut command) = cmdline_params.command {
            if matches!(command, TrayEvent::Connect(_)) && default_params.server_name.is_empty() {
                command = TrayEvent::Settings;
            }
            if let Err(e) = ipc::send_event(command).await {
                warn!("Failed to send event: {}", e);
            }
        } else {
            eprintln!("Another instance of snx-rs-gui is already running.");
        }
        return Ok(());
    }

    let (tray_event_sender, tray_event_receiver) = mpsc::channel(16);

    if let Err(e) = ipc::start_ipc_listener(tray_event_sender.clone()) {
        warn!("Failed to start IPC listener: {}", e);
    }

    platform::init_gui_backend()?;

    let connector_factory = CheckPointConnectorFactory::default();

    tokio::spawn(async move {
        if let Err(e) = platform::wait_restart_signal().await {
            warn!("Restart signal handler exited: {}", e);
        }
    });

    let mut my_tray = create_tray(tray_event_sender.clone(), cmdline_params.no_tray).await?;
    let tray_command_sender = my_tray.sender();

    tokio::spawn(async move { my_tray.run().await });

    {
        let cmd_sender = tray_command_sender.clone();
        let evt_sender = tray_event_sender.clone();
        tokio::spawn(async move { status_poll(cmd_sender, evt_sender).await });
    }

    let no_tray = cmdline_params.no_tray;
    let tray_command_sender_for_events = tray_command_sender.clone();
    let tray_event_sender_for_status = tray_event_sender.clone();

    tokio::spawn(async move {
        handle_tray_events(
            connector_factory,
            tray_event_receiver,
            tray_command_sender_for_events,
            tray_event_sender_for_status,
            no_tray,
        )
        .await;
    });

    if let Some(mut command) = cmdline_params.command {
        ui::spawn_from_event_loop(async move {
            let params = ConnectionProfilesStore::instance().get_default();
            if matches!(command, TrayEvent::Connect(_)) && params.server_name.is_empty() {
                command = TrayEvent::Settings;
            }
            tray_event_sender.send(command).await
        });
    } else if cmdline_params.no_tray {
        ui::spawn_from_event_loop(async move { tray_event_sender.send(TrayEvent::Status).await });
    }

    tokio::task::block_in_place(slint::run_event_loop_until_quit)?;

    Ok(())
}

#[cfg(unix)]
fn current_exe_path() -> anyhow::Result<std::path::PathBuf> {
    let exe = std::env::current_exe()?;
    let s = exe.to_string_lossy();
    if let Some(stripped) = s.strip_suffix(" (deleted)") {
        Ok(std::path::PathBuf::from(stripped))
    } else {
        Ok(exe)
    }
}

async fn create_tray(sender: mpsc::Sender<TrayEvent>, no_tray: bool) -> anyhow::Result<platform::AppTray> {
    let mut retry_count = 5;
    loop {
        match platform::AppTray::new(sender.clone(), no_tray).await {
            Ok(tray) => return Ok(tray),
            Err(e) => {
                if retry_count == 0 {
                    anyhow::bail!("Failed to create tray: {}", e);
                }
                warn!("Failed to create tray: {}, retrying in 2 seconds", e);
                retry_count -= 1;
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
}

async fn handle_tray_events<F>(
    factory: F,
    mut rx: mpsc::Receiver<TrayEvent>,
    tray_command_sender: mpsc::Sender<TrayCommand>,
    tray_event_sender: mpsc::Sender<TrayEvent>,
    no_tray: bool,
) where
    F: TunnelConnectorFactory + Send + 'static,
{
    let mut cancel_sender = None;

    while let Some(v) = rx.recv().await {
        match v {
            TrayEvent::Connect(uuid) => {
                let sender = tray_command_sender.clone();
                let (tx, rx) = mpsc::channel(16);
                cancel_sender = Some(tx);

                if let Some(profile) = ConnectionProfilesStore::instance().get(uuid) {
                    ConnectionProfilesStore::instance().set_connected(uuid);
                    let factory = factory.clone();
                    tokio::spawn(async move { on_connect(factory, sender, profile, rx).await });
                }
            }
            TrayEvent::Disconnect => {
                let sender = tray_command_sender.clone();
                let cancel_sender = cancel_sender.take();
                let params = ConnectionProfilesStore::instance().get_connected();
                tokio::spawn(async move { on_disconnect(sender, params, cancel_sender).await });
            }
            TrayEvent::Settings => on_settings(tray_command_sender.clone()),
            TrayEvent::Exit => {
                let _ = tray_command_sender.send(TrayCommand::Exit).await;
                let _ = slint::quit_event_loop();
            }
            TrayEvent::About => on_about(),

            TrayEvent::Status => {
                on_status(tray_event_sender.clone(), no_tray);
            }
        }
    }
}

fn init_logging(params: &TunnelParams) {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(params.log_level.parse::<LevelFilter>().unwrap_or(LevelFilter::OFF))
        .finish();

    tracing::subscriber::set_global_default(subscriber).unwrap();
}

async fn status_poll(command_sender: mpsc::Sender<TrayCommand>, event_sender: mpsc::Sender<TrayEvent>) {
    let mut controller = ServiceController::new(
        SlintPrompt,
        platform::new_browser_controller(ConnectionProfilesStore::instance().get_connected()),
    );

    let mut first_run = true;
    let mut old_status = Arc::new(Err(anyhow::anyhow!(tr!("app-connection-error"))));

    loop {
        let params = ConnectionProfilesStore::instance().get_connected();
        let status = controller.command(ServiceCommand::Status, params.clone()).await;

        if !ui::status::same_status(&status, &old_status, false) {
            if let Ok(ConnectionStatus::Connected(ref info)) = status {
                ConnectionProfilesStore::instance().set_connected(info.profile_id);
            }

            let is_disconnected = matches!(status, Ok(ConnectionStatus::Disconnected));

            old_status = Arc::new(status);

            let _ = command_sender.send(TrayCommand::Update(Some(old_status.clone()))).await;

            if first_run {
                first_run = false;
                if params.auto_connect && is_disconnected {
                    let sender = event_sender.clone();
                    ui::spawn_from_event_loop(async move { sender.send(TrayEvent::Connect(params.profile_id)).await });
                }
            }
        }

        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

fn on_about() {
    open_window(AboutWindowController::NAME, || Ok(AboutWindowController::new()?))
}

fn on_status(sender: mpsc::Sender<TrayEvent>, exit_on_close: bool) {
    open_window(StatusWindowController::NAME, move || {
        Ok(StatusWindowController::new(exit_on_close, sender)?)
    })
}

fn on_settings(sender: mpsc::Sender<TrayCommand>) {
    open_window(SettingsWindowController::NAME, move || {
        Ok(SettingsWindowController::new(sender)?)
    })
}

async fn on_disconnect(
    sender: mpsc::Sender<TrayCommand>,
    params: Arc<TunnelParams>,
    cancel_sender: Option<mpsc::Sender<()>>,
) {
    let mut controller = ServiceController::new(SlintPrompt, platform::new_browser_controller(params.clone()));
    let status = controller.command(ServiceCommand::Disconnect, params).await;
    let _ = sender.send(TrayCommand::Update(Some(Arc::new(status)))).await;
    if let Some(cancel_sender) = cancel_sender {
        let _ = cancel_sender.send(()).await;
    }
}

async fn on_connect<F>(
    factory: F,
    sender: mpsc::Sender<TrayCommand>,
    params: Arc<TunnelParams>,
    mut cancel_receiver: mpsc::Receiver<()>,
) where
    F: TunnelConnectorFactory,
{
    let _ = sender
        .send(TrayCommand::Update(Some(Arc::new(Ok(ConnectionStatus::Connecting)))))
        .await;

    let connector = factory.new_gateway_connector(params.clone());

    let prompts = match connector.get_gateway_information().await {
        Ok(info) => info.get_login_prompts(&params.login_type),
        Err(e) => {
            let _ = SlintPrompt
                .show_notification(&tr!("app-connection-error"), &e.to_string())
                .await;

            let _ = sender
                .send(TrayCommand::Update(Some(Arc::new(Ok(ConnectionStatus::Disconnected)))))
                .await;
            return;
        }
    };

    let mut controller =
        ServiceController::new_with_prompts(SlintPrompt, platform::new_browser_controller(params.clone()), prompts);

    let mut status = tokio::select! {
        _ = cancel_receiver.recv() => Err(anyhow::anyhow!(tr!("error-connection-cancelled"))),
        status = controller.command(ServiceCommand::Connect, params.clone()) => status
    };

    if let Err(ref e) = status {
        let message = tr!("app-connection-error");
        let _ = SlintPrompt.show_notification(&message, &e.to_string()).await;
        status = controller.command(ServiceCommand::Status, params).await;
    } else if let Ok(ConnectionStatus::Connected(_)) = status {
        let message = tr!("app-connection-success");
        let _ = SlintPrompt
            .show_notification(&message, &tr!("connection-connected-to", server = params.server_name))
            .await;
    };

    let _ = sender.send(TrayCommand::Update(Some(Arc::new(status)))).await;
}
