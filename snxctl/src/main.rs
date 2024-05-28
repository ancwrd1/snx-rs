use std::{path::PathBuf, sync::Arc};

use clap::Parser;
use tracing::level_filters::LevelFilter;

use snxcore::{
    browser::BrowserController,
    controller::{ServiceCommand, ServiceController},
    model::params::TunnelParams,
    prompt::TtyPrompt,
};

struct SystemBrowser;

impl BrowserController for SystemBrowser {
    fn open(&self, url: &str) -> anyhow::Result<()> {
        Ok(opener::open(url)?)
    }

    fn close(&self) {}
}

#[derive(Parser)]
#[clap(about = "VPN client for Checkpoint security gateway", name = "snxctl")]
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

    let mut service_controller = ServiceController::new(TtyPrompt, SystemBrowser, tunnel_params)?;

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(
            service_controller
                .params
                .log_level
                .parse::<LevelFilter>()
                .unwrap_or(LevelFilter::OFF),
        )
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let command = params.command.into();

    match service_controller.command(command).await {
        Ok(status) if command != ServiceCommand::Info => {
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
        Err(e) => println!("Error: {}", e),
        _ => {}
    }

    Ok(())
}
