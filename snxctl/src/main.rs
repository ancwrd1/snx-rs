use std::{future::Future, path::PathBuf, sync::Arc};

use clap::Parser;
use futures::pin_mut;
use tokio::signal::unix;
use tracing::{debug, level_filters::LevelFilter};

use snxcore::{
    browser::SystemBrowser,
    controller::{ServiceCommand, ServiceController},
    model::params::TunnelParams,
    prompt::TtyPrompt,
};

#[derive(Parser)]
#[clap(about = "VPN client for Check Point security gateway", name = "snxctl", version = env!("CARGO_PKG_VERSION"))]
pub struct CmdlineParams {
    #[clap(
        long = "config-file",
        short = 'c',
        global = true,
        help = "Configuration file to use [default: $HOME/.config/snx-rs/snx-rs.conf]"
    )]
    config_file: Option<PathBuf>,
    #[clap(subcommand)]
    command: SnxCommand,
}

#[derive(Parser)]
enum SnxCommand {
    #[clap(name = "connect", about = "Connect a tunnel")]
    Connect,
    #[clap(name = "disconnect", about = "Disconnect a tunnel")]
    Disconnect,
    #[clap(name = "reconnect", about = "Reconnect a tunnel")]
    Reconnect,
    #[clap(name = "status", about = "Show connection status")]
    Status,
    #[clap(name = "info", about = "Show server information")]
    Info,
}

impl From<SnxCommand> for ServiceCommand {
    fn from(value: SnxCommand) -> Self {
        match value {
            SnxCommand::Connect => ServiceCommand::Connect,
            SnxCommand::Disconnect => ServiceCommand::Disconnect,
            SnxCommand::Reconnect => ServiceCommand::Reconnect,
            SnxCommand::Status => ServiceCommand::Status,
            SnxCommand::Info => ServiceCommand::Info,
        }
    }
}

async fn await_termination<F, R>(f: F) -> Option<anyhow::Result<R>>
where
    F: Future<Output = anyhow::Result<R>>,
{
    let ctrl_c = tokio::signal::ctrl_c();
    pin_mut!(ctrl_c);

    let mut sig = unix::signal(unix::SignalKind::terminate()).ok()?;
    let term = sig.recv();
    pin_mut!(term);

    let select = futures::future::select(ctrl_c, term);

    tokio::select! {
        result = f => {
            Some(result)
        }

        _ = select => {
            debug!("Application terminated due to a signal");
            None
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let params = CmdlineParams::parse();

    let config_file = params
        .config_file
        .clone()
        .unwrap_or_else(TunnelParams::default_config_path);

    let tunnel_params = Arc::new(TunnelParams::load(config_file).unwrap_or_default());

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(
            tunnel_params
                .log_level
                .parse::<LevelFilter>()
                .unwrap_or(LevelFilter::OFF),
        )
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let command = params.command.into();

    let mut service_controller = ServiceController::new(TtyPrompt, SystemBrowser);

    let status = match await_termination(service_controller.command(command, tunnel_params.clone())).await {
        Some(status) => status?,
        None => {
            let _ = service_controller
                .command(ServiceCommand::Disconnect, tunnel_params.clone())
                .await;
            println!("\nApplication terminated due to a signal");
            std::process::exit(1);
        }
    };

    if command != ServiceCommand::Info {
        println!("{}", status);
    }

    Ok(())
}
