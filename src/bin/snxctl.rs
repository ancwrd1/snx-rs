use std::{path::Path, str::FromStr, time::Duration};

use anyhow::anyhow;
use base64::Engine;
use directories_next::ProjectDirs;

use snx_rs::{
    model::params::TunnelParams,
    server::{TunnelServiceRequest, TunnelServiceResponse},
};

const RECV_TIMEOUT: Duration = Duration::from_secs(2);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq)]
enum SnxCtlCommand {
    Status,
    Connect,
    Disconnect,
    Reconnect,
}

impl FromStr for SnxCtlCommand {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "status" => Ok(Self::Status),
            "connect" => Ok(Self::Connect),
            "disconnect" => Ok(Self::Disconnect),
            "reconnect" => Ok(Self::Reconnect),
            other => Err(anyhow!("Invalid command: {}", other)),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();

    if args.len() != 2 {
        return Err(anyhow!("usage: {} {{status|connect|disconnect|reconnect}}", args[0]));
    }

    let command: SnxCtlCommand = args.get(1).map(AsRef::as_ref).unwrap_or("status").parse()?;

    let dir = ProjectDirs::from("", "", "snx-rs").ok_or(anyhow!("No project directory!"))?;
    let config_file = dir.config_dir().join("snx-rs.conf");

    match command {
        SnxCtlCommand::Status => do_status().await,
        SnxCtlCommand::Connect => do_connect(&config_file).await,
        SnxCtlCommand::Disconnect => do_disconnect().await,
        SnxCtlCommand::Reconnect => {
            let _ = do_disconnect().await;
            do_connect(&config_file).await
        }
    }
}

async fn do_status() -> anyhow::Result<()> {
    let response = send_receive(TunnelServiceRequest::GetStatus, RECV_TIMEOUT).await;
    match response {
        Ok(TunnelServiceResponse::ConnectionStatus(status)) => {
            println!("{}", if status { "connected" } else { "disconnected" });
            Ok(())
        }
        Ok(_) => Err(anyhow!("Invalid response!")),
        Err(e) => Err(e),
    }
}

async fn do_connect(config_file: &Path) -> anyhow::Result<()> {
    if !config_file.exists() {
        return Err(anyhow!("No config file: {}", config_file.display()));
    }
    let mut params = TunnelParams::load(config_file)?;
    params.password =
        String::from_utf8_lossy(&base64::engine::general_purpose::STANDARD.decode(&params.password)?).into_owned();

    let response = send_receive(TunnelServiceRequest::Connect(params), CONNECT_TIMEOUT).await;
    match response {
        Ok(TunnelServiceResponse::Ok) => {
            println!("ok");
            Ok(())
        }
        Ok(TunnelServiceResponse::Error(error)) => {
            println!("Error: {}", error);
            Ok(())
        }
        Ok(_) => Err(anyhow!("Invalid response!")),
        Err(e) => Err(e),
    }
}

async fn do_disconnect() -> anyhow::Result<()> {
    send_receive(TunnelServiceRequest::Disconnect, RECV_TIMEOUT).await?;
    Ok(())
}

async fn send_receive(request: TunnelServiceRequest, timeout: Duration) -> anyhow::Result<TunnelServiceResponse> {
    let udp = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
    let data = serde_json::to_vec(&request)?;
    let send_fut = udp.send_to(&data, format!("127.0.0.1:{}", snx_rs::server::LISTEN_PORT));

    let mut buf = [0u8; 65536];
    let recv_fut = tokio::time::timeout(timeout, udp.recv_from(&mut buf));

    let result = futures::future::join(send_fut, recv_fut).await;

    if let (Ok(_), Ok(Ok((size, _)))) = result {
        let response = serde_json::from_slice(&buf[0..size])?;
        Ok(response)
    } else {
        Err(anyhow!("Cannot send request to the service!"))
    }
}
