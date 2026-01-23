use std::{future::Future, sync::Arc};

use clap::{CommandFactory, Parser};
use futures::pin_mut;
use i18n::tr;
use secrecy::ExposeSecret;
use snxcore::{
    ccc::CccHttpClient,
    model::{
        MfaType, PromptInfo, SessionState,
        params::{OperationMode, TunnelParams, TunnelType},
    },
    otp::OtpListener,
    platform::{NetworkInterface, Platform, PlatformAccess, SingleInstance},
    prompt::{SecurePrompt, TtyPrompt},
    server::CommandServer,
    server_info, tunnel,
    tunnel::TunnelEvent,
};
use tokio::{signal::unix, sync::mpsc};
use tracing::{debug, metadata::LevelFilter, warn};

use crate::cmdline::CmdlineParams;

mod cmdline;

fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

async fn await_termination<F, R>(f: F) -> anyhow::Result<()>
where
    F: Future<Output = anyhow::Result<R>>,
{
    let ctrl_c = tokio::signal::ctrl_c();
    pin_mut!(ctrl_c);

    let mut sig = unix::signal(unix::SignalKind::terminate())?;
    let term = sig.recv();
    pin_mut!(term);

    let select = futures::future::select(ctrl_c, term);

    tokio::select! {
        result = f => {
            result?;
            Ok(())
        }

        _ = select => {
            debug!("Application terminated due to a signal");
            Ok(())
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cmdline_params = CmdlineParams::parse();

    // Handle completions immediately and exit
    if let Some(shell) = cmdline_params.completions {
        clap_complete::generate(shell, &mut CmdlineParams::command(), "snx-rs", &mut std::io::stdout());
        return Ok(());
    }

    if cmdline_params.mode != OperationMode::Info && !is_root() {
        anyhow::bail!(tr!("error-no-root-privileges"));
    }

    Platform::get().init();

    let mode = cmdline_params.mode;

    let mut params = if let Some(ref config_file) = cmdline_params.config_file {
        TunnelParams::load(config_file)?
    } else {
        TunnelParams::default()
    };
    cmdline_params.merge_into_tunnel_params(&mut params);

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(params.log_level.parse::<LevelFilter>().unwrap_or(LevelFilter::OFF))
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    debug!(">>> Starting snx-rs client version {}", env!("CARGO_PKG_VERSION"));

    match mode {
        OperationMode::Standalone => {
            debug!("Running in standalone mode");
            main_standalone(params).await
        }
        OperationMode::Command => {
            debug!("Running in command mode");
            main_command().await
        }
        OperationMode::Info => main_info(params).await,
    }
}

async fn main_info(params: TunnelParams) -> anyhow::Result<()> {
    if params.server_name.is_empty() {
        anyhow::bail!(tr!("error-missing-server-name"));
    }
    snxcore::util::print_login_options(&params).await?;

    Ok(())
}

async fn main_command() -> anyhow::Result<()> {
    let platform = Platform::get();
    let instance = platform.new_single_instance("/var/run/snx-rs.lock")?;
    if !instance.is_single() {
        eprintln!("{}", tr!("cli-another-instance-running"));
        return Ok(());
    }

    if let Err(e) = Platform::get()
        .new_network_interface()
        .start_network_state_monitoring()
        .await
    {
        warn!("Unable to start network monitoring: {}", e);
    }
    let server = CommandServer::default();

    await_termination(server.run()).await
}

async fn main_standalone(params: TunnelParams) -> anyhow::Result<()> {
    // TODO: reuse code from CommandServer and ServiceController

    let (command_sender, command_receiver) = mpsc::channel(16);

    if params.server_name.is_empty() || params.login_type.is_empty() {
        anyhow::bail!(tr!("error-missing-required-parameters"));
    }

    let mut mfa_prompts = server_info::get_login_prompts(&params).await.unwrap_or_default();

    let params = Arc::new(params);
    let mut connector = tunnel::new_tunnel_connector(params.clone()).await?;

    let mut session = if params.ike_persist {
        debug!("Attempting to load IKE session");
        match connector.restore_session().await {
            Ok(session) => session,
            Err(_) => {
                connector = tunnel::new_tunnel_connector(params.clone()).await?;
                connector.authenticate().await?
            }
        }
    } else {
        connector.authenticate().await?
    };

    let mut mfa_index = 0;

    while let SessionState::PendingChallenge(challenge) = session.state.clone() {
        match challenge.mfa_type {
            MfaType::PasswordInput => {
                mfa_index += 1;

                let prompt = mfa_prompts
                    .pop_front()
                    .unwrap_or_else(|| PromptInfo::new("", &challenge.prompt));

                let input = if !params.password.expose_secret().is_empty() && mfa_index == params.password_factor {
                    Ok(params.password.expose_secret().to_owned())
                } else if let Some(ref mfa_code) = params.mfa_code
                    && mfa_index != params.password_factor
                {
                    Ok(mfa_code.clone())
                } else {
                    TtyPrompt.get_secure_input(prompt).await
                };

                match input {
                    Ok(input) => {
                        session = connector.challenge_code(session, &input).await?;
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
            MfaType::IdentityProvider => {
                println!("{}", tr!("cli-identity-provider-auth"));
                println!("{}", challenge.prompt);
                let otp = OtpListener::new().await?.acquire_otp().await?;
                session = connector.challenge_code(session, &otp).await?;
            }
            MfaType::MobileAccess => {
                println!("{}", tr!("cli-mobile-access-auth"));

                let prompt = PromptInfo::new(challenge.prompt, tr!("label-password"));
                let input = TtyPrompt.get_secure_input(prompt).await?;
                session = connector.challenge_code(session, &input).await?;
            }
            MfaType::UserNameInput => {
                let prompt = PromptInfo::new(tr!("label-username-required"), &challenge.prompt);
                let input = TtyPrompt.get_plain_input(prompt).await?;
                session = connector.challenge_code(session, &input).await?;
            }
        }
    }

    let tunnel = connector.create_tunnel(session.clone(), command_sender).await?;

    if let Err(e) = Platform::get()
        .new_network_interface()
        .start_network_state_monitoring()
        .await
    {
        warn!("Unable to start network monitoring: {}", e);
    }

    let (event_sender, mut event_receiver) = mpsc::channel(16);
    let tunnel_fut = await_termination(tunnel.run(command_receiver, event_sender));

    pin_mut!(tunnel_fut);

    loop {
        tokio::select! {
            event = event_receiver.recv() => {
                if let Some(event) = event {
                    let _ = connector.handle_tunnel_event(event.clone()).await;

                    if let TunnelEvent::Connected(info) = event {
                        println!("{}", info.print());
                        println!("{}", tr!("cli-tunnel-connected"));
                    }
                }
            }
            result = &mut tunnel_fut => {
                if params.tunnel_type == TunnelType::Ssl || !params.ike_persist {
                    debug!("Signing out");
                    let client = CccHttpClient::new(params.clone(), Some(session));
                    let _ = client.signout().await;
                }
                println!("\n{}", tr!("cli-tunnel-disconnected"));
                break result;
            }
        }
    }
}
