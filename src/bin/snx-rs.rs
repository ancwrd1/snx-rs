use std::{future::Future, pin::Pin, sync::Arc, sync::Mutex};

use anyhow::anyhow;
use base64::Engine;
use clap::Parser;
use futures::pin_mut;
use tokio::{signal::unix, sync::oneshot};
use tracing::{debug, metadata::LevelFilter, warn};

use snx_rs::{
    http::CccHttpClient,
    model::{
        params::{CmdlineParams, OperationMode, TunnelParams},
        ConnectionStatus, SessionState,
    },
    prompt::SecurePrompt,
    server::CommandServer,
    tunnel::TunnelConnector,
};

fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
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

    if !params.password.is_empty() {
        // decode password
        params.password =
            String::from_utf8_lossy(&base64::engine::general_purpose::STANDARD.decode(&params.password)?).into_owned();
    }

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(params.log_level.parse::<LevelFilter>().unwrap_or(LevelFilter::OFF))
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    debug!(">>> Starting snx-rs client version {}", env!("CARGO_PKG_VERSION"));

    let (_tx, rx) = oneshot::channel();

    let fut: Pin<Box<dyn Future<Output = anyhow::Result<()>>>> = match mode {
        OperationMode::Standalone => {
            debug!("Running in standalone mode");

            let has_creds = params.client_cert.is_some() || !params.user_name.is_empty();

            if params.server_name.is_empty() || !has_creds {
                return Err(anyhow!("Missing required parameters: server name and/or user name"));
            }

            let connector = TunnelConnector::new(Arc::new(params));
            let mut session = connector.authenticate(None).await?;

            while let SessionState::Pending(ref prompt) = session.state {
                let prompt = prompt.as_deref().unwrap_or("Multi-factor code: ");
                match SecurePrompt::tty().get_secure_input(prompt) {
                    Ok(input) => {
                        session = connector.challenge_code(&session.session_id, &input).await?;
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }

            let status = Arc::new(Mutex::new(ConnectionStatus::default()));
            let tunnel = connector.create_tunnel(Arc::new(session)).await?;

            if let Err(e) = snx_rs::platform::start_network_state_monitoring().await {
                warn!("Unable to start network monitoring: {}", e);
            }

            Box::pin(tunnel.run(rx, status))
        }
        OperationMode::Command => {
            debug!("Running in command mode");

            if let Err(e) = snx_rs::platform::start_network_state_monitoring().await {
                warn!("Unable to start network monitoring: {}", e);
            }
            let server = CommandServer::new(snx_rs::server::LISTEN_PORT);

            Box::pin(server.run())
        }
        OperationMode::Info => {
            if params.server_name.is_empty() {
                return Err(anyhow!("Missing required parameters: server name!"));
            }
            let client = CccHttpClient::new(Arc::new(params));
            let info = client.get_server_info().await?;
            println!("{}", serde_json::to_string_pretty(&info)?);
            Box::pin(futures::future::ok(()))
        }
    };

    let ctrl_c = tokio::signal::ctrl_c();
    pin_mut!(ctrl_c);

    let mut sig = unix::signal(unix::SignalKind::terminate())?;
    let term = sig.recv();
    pin_mut!(term);

    let select = futures::future::select(ctrl_c, term);

    let result = tokio::select! {
        result = fut => {
            result
        }

        _ = select => {
            debug!("Application terminated due to a signal");
            Ok(())
        }
    };

    debug!("<<< Stopping snx-rs client");

    result
}
