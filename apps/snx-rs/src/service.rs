use std::{
    ffi::OsString,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use snxcore::{
    platform::{NetworkInterface, Platform, PlatformAccess, SingleInstance},
    server::CommandServer,
    tunnel::connector::CheckPointConnectorFactory,
};
use tokio::sync::Notify;
use tracing::{error, metadata::LevelFilter, warn};
use windows_service::{
    define_windows_service,
    service::{
        ServiceAccess, ServiceControl, ServiceControlAccept, ServiceDependency, ServiceErrorControl, ServiceExitCode,
        ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
    service_manager::{ServiceManager, ServiceManagerAccess},
};

pub const SERVICE_NAME: &str = "snx-rs";
pub const SERVICE_DISPLAY_NAME: &str = "SNX-RS VPN Client";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

define_windows_service!(ffi_service_main, service_main);

pub fn run() -> anyhow::Result<()> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
    Ok(())
}

fn service_main(_arguments: Vec<OsString>) {
    // Set up file-based logging before anything else — under SCM there is no
    // stdout/stderr, so any tracing output would otherwise be silently dropped.
    let _ = init_file_logging();

    if let Err(e) = run_service() {
        error!("Service exited with error: {e:?}");
    }
}

fn log_path() -> PathBuf {
    Platform::get().data_dir().join("snx-rs.log")
}

fn init_file_logging() -> anyhow::Result<()> {
    let path = log_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = fs::OpenOptions::new().create(true).append(true).open(&path)?;

    let level = std::env::var("SNX_RS_LOG")
        .ok()
        .and_then(|s| s.parse::<LevelFilter>().ok())
        .unwrap_or(LevelFilter::INFO);

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(level)
        .with_ansi(false)
        .with_writer(Mutex::new(file))
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

fn run_service() -> anyhow::Result<()> {
    let shutdown = Arc::new(Notify::new());

    let status_handle = {
        let shutdown = shutdown.clone();
        service_control_handler::register(SERVICE_NAME, move |control| match control {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                shutdown.notify_waiters();
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        })?
    };

    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;

    let result = runtime.block_on(service_loop(shutdown));

    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(if result.is_ok() { 0 } else { 1 }),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    result
}

async fn service_loop(shutdown: Arc<Notify>) -> anyhow::Result<()> {
    Platform::get().init();

    let instance = Platform::get().new_single_instance("snx-rs-service.lock")?;
    if !instance.is_single() {
        anyhow::bail!("Another snx-rs instance is already running");
    }

    if let Err(e) = Platform::get()
        .new_network_interface()
        .start_network_state_monitoring()
        .await
    {
        warn!("Unable to start network monitoring: {e}");
    }

    let factory = CheckPointConnectorFactory::default();
    let server = CommandServer::new(factory);
    let server_fut = server.run();

    tokio::select! {
        result = server_fut => result,
        _ = shutdown.notified() => Ok(()),
    }
}

pub fn install() -> anyhow::Result<()> {
    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE,
    )?;

    let exe = std::env::current_exe()?;

    let service_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(SERVICE_DISPLAY_NAME),
        service_type: SERVICE_TYPE,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: exe,
        launch_arguments: vec![OsString::from("-m"), OsString::from("service")],
        dependencies: vec![ServiceDependency::from_system_identifier("tcpip")],
        account_name: None, // LocalSystem
        account_password: None,
    };

    let service = manager.create_service(&service_info, ServiceAccess::CHANGE_CONFIG)?;

    let _ = service.set_description("SNX-RS VPN client");

    Ok(())
}

pub fn uninstall() -> anyhow::Result<()> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
    let service = manager.open_service(SERVICE_NAME, ServiceAccess::DELETE)?;

    service.delete()?;

    Ok(())
}
