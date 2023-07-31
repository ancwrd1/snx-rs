use crate::http::SnxHttpClient;
use crate::model::params::TunnelParams;
use crate::model::{TunnelServiceRequest, TunnelServiceResponse};
use anyhow::anyhow;
use base64::Engine;
use directories_next::ProjectDirs;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

const RECV_TIMEOUT: Duration = Duration::from_secs(2);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SnxCtlCommand {
    Status,
    Connect,
    Disconnect,
    Reconnect,
    Info,
}

impl FromStr for SnxCtlCommand {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "status" => Ok(Self::Status),
            "connect" => Ok(Self::Connect),
            "disconnect" => Ok(Self::Disconnect),
            "reconnect" => Ok(Self::Reconnect),
            "info" => Ok(Self::Info),
            other => Err(anyhow!("Invalid command: {}", other)),
        }
    }
}

pub struct SnxController {
    params: TunnelParams,
}

impl SnxController {
    pub fn new() -> anyhow::Result<Self> {
        let dir = ProjectDirs::from("", "", "snx-rs").ok_or(anyhow!("No project directory!"))?;
        let config_file = dir.config_dir().join("snx-rs.conf");

        if !config_file.exists() {
            return Err(anyhow!("No config file: {}", config_file.display()));
        }
        let mut params = TunnelParams::load(config_file)?;
        params.password =
            String::from_utf8_lossy(&base64::engine::general_purpose::STANDARD.decode(&params.password)?).into_owned();

        Ok(Self { params })
    }

    pub async fn command(&self, command: SnxCtlCommand) -> anyhow::Result<()> {
        match command {
            SnxCtlCommand::Status => self.do_status().await,
            SnxCtlCommand::Connect => self.do_connect().await,
            SnxCtlCommand::Disconnect => self.do_disconnect().await,
            SnxCtlCommand::Reconnect => {
                let _ = self.do_disconnect().await;
                self.do_connect().await
            }
            SnxCtlCommand::Info => self.do_info().await,
        }
    }

    async fn do_status(&self) -> anyhow::Result<()> {
        let response = self.send_receive(TunnelServiceRequest::GetStatus, RECV_TIMEOUT).await;
        match response {
            Ok(TunnelServiceResponse::ConnectionStatus(status)) => {
                match status.connected_since {
                    Some(timestamp) => println!("Connected since {}", timestamp),
                    None => println!("Disconnected"),
                }
                Ok(())
            }
            Ok(_) => Err(anyhow!("Invalid response!")),
            Err(e) => Err(e),
        }
    }

    async fn do_connect(&self) -> anyhow::Result<()> {
        let response = self
            .send_receive(TunnelServiceRequest::Connect(self.params.clone()), CONNECT_TIMEOUT)
            .await;
        match response {
            Ok(TunnelServiceResponse::Ok) => self.do_status().await,
            Ok(TunnelServiceResponse::Error(error)) => {
                println!("Error: {}", error);
                Ok(())
            }
            Ok(_) => Err(anyhow!("Invalid response!")),
            Err(e) => Err(e),
        }
    }

    async fn do_disconnect(&self) -> anyhow::Result<()> {
        self.send_receive(TunnelServiceRequest::Disconnect, RECV_TIMEOUT)
            .await?;
        Ok(())
    }

    async fn send_receive(
        &self,
        request: TunnelServiceRequest,
        timeout: Duration,
    ) -> anyhow::Result<TunnelServiceResponse> {
        let udp = tokio::net::UdpSocket::bind("127.0.0.1:0").await?;
        let data = serde_json::to_vec(&request)?;
        let send_fut = udp.send_to(&data, format!("127.0.0.1:{}", crate::server::LISTEN_PORT));

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

    async fn do_info(&self) -> anyhow::Result<()> {
        let client = SnxHttpClient::new(Arc::new(self.params.clone()));
        let info = client.get_server_info().await?;
        let response_data = info
            .get("ResponseData")
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();

        println!("{}", serde_json::to_string_pretty(&response_data)?);

        Ok(())
    }
}
