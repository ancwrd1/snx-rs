use anyhow::anyhow;
use clap::Parser;
use tracing::debug;

use snx_rs::{
    device::TunDevice,
    params::{CmdlineParams, TunnelParams},
    tunnel::SnxClient,
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

    let client = SnxClient::new(&params);

    let (session_id, cookie) = client.authenticate(None).await?;

    let mut tunnel = client.create_tunnel(session_id, cookie).await?;

    let reply = tunnel.client_hello().await?;

    let device = TunDevice::new(&reply)?;

    device.setup_dns_and_routing(&params).await?;

    tunnel.run(device).await?;

    debug!("<<< Stopping snx-rs client");

    Ok(())
}
