use anyhow::anyhow;
use base64::Engine;
use clap::Parser;

use snx_rs::{device::TunDevice, params, tunnel::SnxClient};

fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut params = params::SnxParams::parse();
    params.load()?;

    let server = params
        .server_name
        .ok_or_else(|| anyhow!("No server specified!"))?;

    let username = params
        .user_name
        .ok_or_else(|| anyhow!("No username specified!"))?;

    let password = if let Some(password) = params.password {
        String::from_utf8_lossy(&base64::engine::general_purpose::STANDARD.decode(password)?)
            .into_owned()
    } else {
        return Err(anyhow!("No password specified!"));
    };

    if !is_root() {
        return Err(anyhow!("Please run me as a root user!"));
    }

    if let Some(level) = params.log_level {
        let subscriber = tracing_subscriber::fmt().with_max_level(level).finish();

        tracing::subscriber::set_global_default(subscriber)?;
    }

    let client = SnxClient::builder()
        .server_name(server)
        .auth(username, password)
        .reauth(params.reauth.unwrap_or_default())
        .build();

    let mut tunnel = client.connect().await?;
    let reply = tunnel.client_hello().await?;

    let device = TunDevice::new(&reply)?;
    device.setup_dns_and_routing(params.search_domains).await?;

    tunnel.run(device).await?;

    Ok(())
}
