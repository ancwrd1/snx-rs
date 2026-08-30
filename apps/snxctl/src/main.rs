use clap::{CommandFactory, Parser};
use futures::pin_mut;
use i18n::tr;
use snxcore::model::ConnectionStatus;
use snxcore::{
    browser::SystemBrowser,
    controller::{ServiceCommand, ServiceController},
    model::params::TunnelParams,
    profiles::ConnectionProfilesStore,
    prompt::TtyPrompt,
    tunnel::{TunnelConnectorFactory, connector::CheckPointConnectorFactory},
};
use std::time::{Duration, Instant};
use std::{future::Future, io, path::PathBuf, sync::Arc};
use tracing::level_filters::LevelFilter;

const REKEY_STATE_TIMEOUT: Duration = Duration::from_secs(5);

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
    #[clap(
        long = "profile",
        short = 'p',
        global = true,
        help = "Connection profile name or UUID (ignored if --config-file is given)"
    )]
    profile: Option<String>,
    #[clap(subcommand)]
    command: SnxCommand,
}

#[derive(Parser)]
enum IpsecCommand {
    #[clap(name = "ike-state", about = "Print IPsec IKE state")]
    IkeState,
    #[clap(name = "rekey", about = "Rekey IPsec CHILD SA")]
    Rekey,
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
    #[clap(name = "list", about = "List connection profiles")]
    List,
    #[clap(subcommand, name = "ipsec", about = "IPsec commands")]
    Ipsec(IpsecCommand),
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
            SnxCommand::Ipsec(IpsecCommand::IkeState) => ServiceCommand::Status,
            SnxCommand::Ipsec(IpsecCommand::Rekey) => ServiceCommand::Rekey,
            SnxCommand::Info | SnxCommand::List | SnxCommand::Completions { .. } => {
                unreachable!("Handled separately in main")
            }
        }
    }
}

async fn await_termination<F, R>(f: F) -> Option<anyhow::Result<R>>
where
    F: Future<Output = anyhow::Result<R>>,
{
    let ctrl_c = tokio::signal::ctrl_c();
    pin_mut!(ctrl_c);

    #[cfg(unix)]
    let term_signal = {
        use tokio::signal::unix;
        let mut sig = unix::signal(unix::SignalKind::terminate()).ok()?;
        async move { sig.recv().await }
    };
    #[cfg(not(unix))]
    let term_signal = std::future::pending::<Option<()>>();

    pin_mut!(term_signal);

    let select = futures::future::select(ctrl_c, term_signal);

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

    let tunnel_params = if let Some(path) = params.config_file.clone() {
        Arc::new(TunnelParams::load(path)?)
    } else if let Some(name_or_uuid) = params.profile.as_deref() {
        match ConnectionProfilesStore::instance().find_by_name_or_uuid(name_or_uuid) {
            Some(p) => p,
            None => anyhow::bail!(tr!("error-profile-not-found", profile = name_or_uuid)),
        }
    } else {
        Arc::new(TunnelParams::load(TunnelParams::default_config_path()).unwrap_or_default())
    };

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(
            tunnel_params
                .log_level
                .parse::<LevelFilter>()
                .unwrap_or(LevelFilter::OFF),
        )
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let connector = CheckPointConnectorFactory::default().new_gateway_connector(tunnel_params.clone());

    match params.command {
        SnxCommand::Info => {
            let info = connector.get_gateway_information().await?;
            info.print_login_options(&tunnel_params.server_name);
        }
        SnxCommand::List => {
            let profiles = ConnectionProfilesStore::instance().all();
            for profile in profiles {
                println!("{}: {}", profile.profile_id, profile.profile_name);
            }
        }
        other => {
            let ike_state = matches!(other, SnxCommand::Ipsec(IpsecCommand::IkeState));
            let rekey = matches!(other, SnxCommand::Ipsec(IpsecCommand::Rekey));
            let command = other.into();

            let info = connector.get_gateway_information().await?;
            let mut service_controller = ServiceController::new_with_prompts(
                TtyPrompt,
                SystemBrowser::default(),
                info.get_login_prompts(&tunnel_params.login_type),
            );

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

            let as_ike_state = |status: &ConnectionStatus| {
                if let ConnectionStatus::Connected(info) = status
                    && let Some(ref state) = info.ike_state
                {
                    Some(state.clone())
                } else {
                    None
                }
            };

            let mut current_state = as_ike_state(&status);

            if ike_state || rekey {
                let mut state_changed = !rekey;

                if rekey {
                    let now = Instant::now();
                    while now.elapsed() < REKEY_STATE_TIMEOUT {
                        if let Ok(status) = service_controller
                            .command(ServiceCommand::Status, tunnel_params.clone())
                            .await
                            && let new_state = as_ike_state(&status)
                            && new_state != current_state
                        {
                            current_state = new_state;
                            state_changed = true;
                            break;
                        }

                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }

                if !state_changed {
                    println!("{}", tr!("cli-rekey-state-pending"));
                }

                if let Some(state) = current_state {
                    println!("{}", state.print());
                } else {
                    println!("{}", tr!("cli-no-ike-state"));
                }
            } else {
                println!("{}", status.print());
            }
        }
    }

    Ok(())
}
