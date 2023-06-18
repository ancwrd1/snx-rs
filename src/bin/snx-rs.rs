use std::{
    future::Future,
    pin::Pin,
    sync::{atomic::AtomicBool, Arc},
};

use anyhow::anyhow;
use base64::Engine;
use clap::Parser;
use futures::pin_mut;
use tokio::signal::unix;
use tokio::sync::oneshot;
use tracing::{debug, metadata::LevelFilter};

use snx_rs::{
    params::{CmdlineParams, OperationMode, TunnelParams},
    server::CommandServer,
    tunnel::SnxTunnelConnector,
};

const LISTEN_PORT: u16 = 7779;

fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cmdline_params = CmdlineParams::parse();

    let mode = cmdline_params.mode;

    let mut params = if let Some(ref config_file) = cmdline_params.config_file {
        TunnelParams::load(config_file)?
    } else {
        TunnelParams::default()
    };
    params.merge(cmdline_params);

    // decode password
    params.password =
        String::from_utf8_lossy(&base64::engine::general_purpose::STANDARD.decode(&params.password)?).into_owned();

    let params = Arc::new(params);

    if !is_root() {
        return Err(anyhow!("Please run me as a root user!"));
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
            if params.user_name.is_empty() || params.server_name.is_empty() || params.password.is_empty() {
                return Err(anyhow!(
                    "Missing required parameters: server name, user name and password!"
                ));
            }

            let connector = SnxTunnelConnector::new(params.clone());
            let session = Arc::new(connector.authenticate(None).await?);

            let connected = Arc::new(AtomicBool::new(false));

            let tunnel = connector.create_tunnel(session).await?;
            Box::pin(tunnel.run(rx, connected))
        }
        OperationMode::Command => {
            debug!("Running in command mode");
            let server = CommandServer::new(LISTEN_PORT);
            Box::pin(server.run())
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
