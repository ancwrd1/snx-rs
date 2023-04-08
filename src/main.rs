use anyhow::anyhow;
use base64::Engine;
use clap::Parser;

use crate::device::TunDevice;
use crate::tunnel::SnxClient;

pub mod auth;
pub mod codec;
pub mod device;
pub mod model;
pub mod sexpr;
pub mod tunnel;
pub mod util;

#[derive(Parser)]
#[clap(about = "VPN client for Checkpoint security gateway", name = "snx-rs")]
struct SnxParams {
    #[clap(long = "server", short = 's', help = "Server name to connect to")]
    server: String,

    #[clap(long = "username", short = 'u', help = "User name")]
    username: String,

    #[clap(
        long = "password",
        short = 'p',
        help = "Password (base64 or @filename)"
    )]
    password: String,

    #[clap(
        long = "tun",
        short = 't',
        default_value = "snxrs-tun",
        help = "Name for tun interface"
    )]
    tun_name: String,
}

fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pretty_env_logger::init_timed();

    let params = SnxParams::parse();

    if !is_root() {
        return Err(anyhow!("Please run me as a root user!"));
    }

    let password = if params.password.starts_with('@') {
        std::fs::read_to_string(&params.password[1..])?
            .trim()
            .to_owned()
    } else {
        String::from_utf8_lossy(
            &base64::engine::general_purpose::STANDARD.decode(&params.password)?,
        )
        .into_owned()
    };

    let mut builder = SnxClient::builder();
    builder
        .server_name(params.server)
        .auth(params.username, password);

    let client = builder.build();

    let mut tunnel = client.connect().await?;
    let reply = tunnel.client_hello().await?;

    println!("Tunnel established");

    let device = TunDevice::new(params.tun_name, &reply)?;
    device.setup_dns_and_routing().await?;

    tunnel.run(device).await?;

    Ok(())
}
