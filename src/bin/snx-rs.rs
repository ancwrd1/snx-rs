use anyhow::anyhow;
use base64::Engine;
use clap::Parser;
use tracing::debug;

use snx_rs::{
    params::{CmdlineParams, TunnelParams},
    tunnel::SnxTunnelConnector,
};

fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cmdline_params = CmdlineParams::parse();

    let mut params = if let Some(ref config_file) = cmdline_params.config_file {
        TunnelParams::load(config_file)?
    } else {
        TunnelParams::default()
    };
    params.merge(cmdline_params);

    // decode password
    params.password =
        String::from_utf8_lossy(&base64::engine::general_purpose::STANDARD.decode(&params.password)?).into_owned();

    if params.user_name.is_empty() || params.server_name.is_empty() || params.password.is_empty() {
        return Err(anyhow!(
            "Missing required parameters: server name, user name and password!"
        ));
    }

    if !is_root() {
        return Err(anyhow!("Please run me as a root user!"));
    }

    let subscriber = tracing_subscriber::fmt().with_max_level(params.log_level).finish();
    tracing::subscriber::set_global_default(subscriber)?;

    debug!(">>> Starting snx-rs client version {}", env!("CARGO_PKG_VERSION"));

    let connector = SnxTunnelConnector::new(&params);
    let session = connector.authenticate(None).await?;

    let tunnel = connector.create_tunnel(session).await?;
    tunnel.run().await?;

    debug!("<<< Stopping snx-rs client");

    Ok(())
}
