use std::{future::Future, io, path::PathBuf, sync::Arc};

use clap::{CommandFactory, Parser};
use futures::pin_mut;
use snxcore::{
    browser::SystemBrowser,
    controller::{ServiceCommand, ServiceController},
    model::params::TunnelParams,
    prompt::TtyPrompt,
};
use tokio::signal::unix;
use tracing::level_filters::LevelFilter;

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
    #[clap(name = "completions", about = "Generate shell completions")]
    Completions {
        #[clap(
            default_value = "bash",
            help = "The shell to generate completions for (bash, elvish, fish, zsh))"
        )]
        shell: clap_complete::Shell,
    },
}

impl From<SnxCommand> for ServiceCommand {
    fn from(value: SnxCommand) -> Self {
        match value {
            SnxCommand::Connect => ServiceCommand::Connect,
            SnxCommand::Disconnect => ServiceCommand::Disconnect,
            SnxCommand::Reconnect => ServiceCommand::Reconnect,
            SnxCommand::Status => ServiceCommand::Status,
            SnxCommand::Info => ServiceCommand::Info,
            SnxCommand::Completions { .. } => unreachable!("Handled separately in main"),
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
        result = f => Some(result),
        _ = select => None,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let params = CmdlineParams::parse();

    // Handle completions immediately and exit
    if let SnxCommand::Completions { shell } = &params.command {
        clap_complete::generate(*shell, &mut CmdlineParams::command(), "snxctl", &mut io::stdout());
        return Ok(());
    }

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
            println!("\n{}", i18n::translate("cli-app-terminated"));
            std::process::exit(1);
        }
    };

    if command != ServiceCommand::Info {
        println!("{}", status.print());
    }

    Ok(())
}
