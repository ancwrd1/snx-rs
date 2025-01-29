use std::{collections::VecDeque, future::Future, sync::Arc};

use anyhow::anyhow;
use clap::Parser;
use futures::pin_mut;
use tokio::{
    signal::unix,
    sync::{mpsc, oneshot},
};
use tracing::{debug, metadata::LevelFilter, warn};

use snxcore::{
    browser::run_otp_listener,
    ccc::CccHttpClient,
    model::{
        params::{OperationMode, TunnelParams},
        MfaType, SessionState,
    },
    platform,
    prompt::{SecurePrompt, TtyPrompt, OTP_TIMEOUT},
    server::CommandServer,
    server_info, tunnel,
};

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

    if cmdline_params.mode != OperationMode::Info && !is_root() {
        return Err(anyhow!("This program should be run as a root user!"));
    }

    platform::init();

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
        return Err(anyhow!("Missing required parameters: server name!"));
    }
    let client = CccHttpClient::new(Arc::new(params), None);
    let info = client.get_server_info().await?;
    snxcore::util::print_login_options(&info);

    Ok(())
}

async fn main_command() -> anyhow::Result<()> {
    if let Err(e) = platform::start_network_state_monitoring().await {
        warn!("Unable to start network monitoring: {}", e);
    }
    let server = CommandServer::new(snxcore::server::LISTEN_PORT);

    await_termination(server.run()).await
}

async fn main_standalone(params: TunnelParams) -> anyhow::Result<()> {
    // TODO: reuse code from CommandServer and ServiceController

    let (command_sender, command_receiver) = mpsc::channel(16);

    if params.server_name.is_empty() || params.login_type.is_empty() {
        return Err(anyhow!("Missing required parameters: server name and/or login type"));
    }

    let mut mfa_prompts = if params.server_prompt {
        server_info::get_mfa_prompts(&params).await.unwrap_or_default()
    } else {
        VecDeque::default()
    };

    let params = Arc::new(params);
    let mut connector = tunnel::new_tunnel_connector(params.clone()).await?;

    let mut session = if params.ike_persist {
        debug!("Attempting to load IKE session");
        match connector.restore_session().await {
            Ok(session) => session,
            Err(_) => connector.authenticate().await?,
        }
    } else {
        connector.authenticate().await?
    };

    while let SessionState::PendingChallenge(challenge) = session.state.clone() {
        match challenge.mfa_type {
            MfaType::PasswordInput => {
                let prompt = mfa_prompts.pop_front().unwrap_or_else(|| challenge.prompt.clone());
                match TtyPrompt.get_secure_input(&prompt) {
                    Ok(input) => {
                        session = connector.challenge_code(session, &input).await?;
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
            MfaType::SamlSso => {
                println!("For SAML authentication open the following URL in your browser:");
                println!("{}", challenge.prompt);
                let (tx, rx) = oneshot::channel();
                tokio::spawn(run_otp_listener(tx));
                let otp = tokio::time::timeout(OTP_TIMEOUT, rx).await??;
                session = connector.challenge_code(session, &otp).await?;
            }
            MfaType::UserNameInput => {
                let input = TtyPrompt.get_plain_input(&challenge.prompt)?;
                session = connector.challenge_code(session, &input).await?;
            }
        }
    }

    let tunnel = connector.create_tunnel(session, command_sender).await?;

    if let Err(e) = platform::start_network_state_monitoring().await {
        warn!("Unable to start network monitoring: {}", e);
    }

    println!(
        "Connected to {} via {}, press Ctrl-C to exit.",
        params.server_name, params.tunnel_type
    );

    let (event_sender, event_receiver) = mpsc::channel(16);
    let tunnel_fut = await_termination(tunnel.run(command_receiver, event_sender));

    pin_mut!(tunnel_fut);
    pin_mut!(event_receiver);

    loop {
        tokio::select! {
            event = event_receiver.recv() => {
                if let Some(event) = event {
                    let _ = connector.handle_tunnel_event(event).await;
                }
            }
            result = &mut tunnel_fut => {
                break result;
            }
        }
    }
}
