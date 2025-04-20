use std::{path::PathBuf, sync::Arc};

use clap::Parser;
use tracing::level_filters::LevelFilter;

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let params = CmdlineParams::parse();

    let config_file = params
        .config_file
        .clone()
        .unwrap_or_else(TunnelParams::default_config_path);

    let tunnel_params = Arc::new(TunnelParams::load(config_file).unwrap_or_default());

    let mut service_controller = ServiceController::new(TtyPrompt, SystemBrowser);

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

    let status = service_controller.command(command, tunnel_params.clone()).await?;

    if command != ServiceCommand::Info {
        if let Some(since) = status.connected_since {
            println!(
                "{} since: {}",
                if status.mfa.is_some() {
                    "MFA pending"
                } else {
                    "Connected"
                },
                since
            );
        } else {
            println!("Disconnected");
        }
    }

    Ok(())
}
