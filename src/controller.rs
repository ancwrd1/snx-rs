use std::{str::FromStr, sync::Arc, time::Duration};

use anyhow::anyhow;
use base64::Engine;
use directories_next::ProjectDirs;
use tracing::level_filters::LevelFilter;

use crate::{
    http::SnxHttpClient,
    model::{params::TunnelParams, TunnelServiceRequest, TunnelServiceResponse},
    platform::UdpSocketExt,
    prompt,
};

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
    pub fn with_params(params: TunnelParams) -> Self {
        Self { params }
    }

    pub fn new() -> anyhow::Result<Self> {
        let dir = ProjectDirs::from("", "", "snx-rs").ok_or(anyhow!("No project directory!"))?;
        let config_file = dir.config_dir().join("snx-rs.conf");

        if !config_file.exists() {
            return Err(anyhow!("No config file: {}", config_file.display()));
        }
        let mut params = TunnelParams::load(config_file)?;

        if !params.password.is_empty() {
            params.password =
                String::from_utf8_lossy(&base64::engine::general_purpose::STANDARD.decode(&params.password)?)
                    .into_owned();
        }

        Ok(Self { params })
    }

    pub async fn command(&self, command: SnxCtlCommand) -> anyhow::Result<()> {
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(self.params.log_level.parse::<LevelFilter>().unwrap_or(LevelFilter::OFF))
            .finish();
        tracing::subscriber::set_global_default(subscriber)?;

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

    #[async_recursion::async_recursion]
    async fn do_status(&self) -> anyhow::Result<()> {
        let response = self.send_receive(TunnelServiceRequest::GetStatus, RECV_TIMEOUT).await;
        match response {
            Ok(TunnelServiceResponse::ConnectionStatus(status)) => {
                match status.connected_since {
                    Some(timestamp) => println!("Connected since {}", timestamp),
                    None => {
                        if status.mfa_pending {
                            let input = prompt::get_input_from_tty("Enter challenge code: ")?;
                            self.do_challenge_code(input).await?;
                        } else {
                            println!("Disconnected");
                        }
                    }
                }
                Ok(())
            }
            Ok(_) => Err(anyhow!("Invalid response!")),
            Err(e) => Err(e),
        }
    }

    async fn do_connect(&self) -> anyhow::Result<()> {
        let mut params = self.params.clone();

        let has_creds = params.client_cert.is_some() || (!params.user_name.is_empty() && !params.password.is_empty());

        if params.server_name.is_empty() || !has_creds {
            return Err(anyhow!(
                "Missing required parameters in the config file: server name and/or user credentials"
            ));
        }

        if params.password.is_empty() && params.client_cert.is_none() {
            match crate::platform::acquire_password(&params.user_name).await {
                Ok(password) => params.password = password,
                Err(e) => return Err(e),
            }
        }

        let response = self
            .send_receive(TunnelServiceRequest::Connect(params), CONNECT_TIMEOUT)
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

    async fn do_challenge_code(&self, code: String) -> anyhow::Result<()> {
        let response = self
            .send_receive(
                TunnelServiceRequest::ChallengeCode(code, self.params.clone()),
                CONNECT_TIMEOUT,
            )
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
        udp.connect(format!("127.0.0.1:{}", crate::server::LISTEN_PORT)).await?;

        let data = serde_json::to_vec(&request)?;

        let result = udp.send_receive(&data, timeout).await?;

        Ok(serde_json::from_slice(&result)?)
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
