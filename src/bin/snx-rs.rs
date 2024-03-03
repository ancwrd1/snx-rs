use std::time::Duration;
use std::{
    future::Future,
    sync::{Arc, Mutex},
};

use anyhow::anyhow;
use clap::Parser;
use futures::future::Either;
use futures::pin_mut;
use tokio::sync::mpsc;
use tokio::{signal::unix, sync::oneshot};
use tracing::{debug, metadata::LevelFilter, warn};

use snx_rs::{
    ccc::CccHttpClient,
    model::{
        params::{CmdlineParams, OperationMode, TunnelParams},
        ConnectionStatus, MfaType, SessionState,
    },
    platform,
    prompt::{run_otp_listener, SecurePrompt, OTP_TIMEOUT},
    server::CommandServer,
    server_info, tunnel,
};

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
        return Err(anyhow!("Please run me as a root user!"));
    }

    let mode = cmdline_params.mode;

    let mut params = if let Some(ref config_file) = cmdline_params.config_file {
        TunnelParams::load(config_file)?
    } else {
        TunnelParams::default()
    };
    params.merge(cmdline_params);

    params.decode_password()?;

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(params.log_level.parse::<LevelFilter>().unwrap_or(LevelFilter::OFF))
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    debug!(">>> Starting snx-rs client version {}", env!("CARGO_PKG_VERSION"));

    let (tx, rx) = mpsc::channel(16);

    match mode {
        OperationMode::Standalone => {
            debug!("Running in standalone mode");

            if params.server_name.is_empty() || params.login_type.is_empty() {
                return Err(anyhow!("Missing required parameters: server name and/or login type"));
            }

            let mut mfa_prompts = server_info::get_mfa_prompts(&params).await.unwrap_or_default();

            let mut connector = tunnel::new_tunnel_connector(Arc::new(params)).await?;
            let mut session = connector.authenticate().await?;

            while let SessionState::PendingChallenge(challenge) = session.state.clone() {
                match challenge.mfa_type {
                    MfaType::UserInput => {
                        let prompt = mfa_prompts.pop_front().unwrap_or_else(|| challenge.prompt.clone());
                        match SecurePrompt::tty().get_secure_input(&prompt) {
                            Ok(input) => {
                                session = connector.challenge_code(session, &input).await?;
                            }
                            Err(e) => {
                                return Err(e);
                            }
                        }
                    }
                    MfaType::SamlSso => {
                        println!("For SAML authentication please open the following URL in your browser:");
                        println!("{}", challenge.prompt);
                        let otp = tokio::time::timeout(OTP_TIMEOUT, run_otp_listener()).await??;
                        session = connector.challenge_code(session, &otp).await?;
                    }
                }
            }

            let status = Arc::new(Mutex::new(ConnectionStatus::default()));
            let tunnel = connector.create_tunnel(session).await?;

            if let Err(e) = platform::start_network_state_monitoring().await {
                warn!("Unable to start network monitoring: {}", e);
            }

            let (status_sender, _) = oneshot::channel();
            let tunnel_fut = await_termination(tunnel.run(rx, status, status_sender));
            let mut interval = tokio::time::interval(Duration::from_secs(60));

            pin_mut!(tunnel_fut);

            let result = loop {
                let tick = interval.tick();
                pin_mut!(tick);
                match futures::future::select(tick, &mut tunnel_fut).await {
                    Either::Left(_) => match connector.rekey_tunnel(tx.clone()).await {
                        Ok(_) => {}
                        Err(e) => {
                            warn!("Rekey error: {:?}", e);
                            break Err(e);
                        }
                    },
                    Either::Right(result) => break result.0,
                }
            };

            let _ = connector.terminate_tunnel().await;
            result
        }
        OperationMode::Command => {
            debug!("Running in command mode");

            if let Err(e) = platform::start_network_state_monitoring().await {
                warn!("Unable to start network monitoring: {}", e);
            }
            let server = CommandServer::new(snx_rs::server::LISTEN_PORT);

            await_termination(server.run()).await
        }
        OperationMode::Info => {
            if params.server_name.is_empty() {
                return Err(anyhow!("Missing required parameters: server name!"));
            }
            let client = CccHttpClient::new(Arc::new(params), None);
            let info = client.get_server_info().await?;
            snx_rs::util::print_login_options(&info);

            Ok(())
        }
    }
}
