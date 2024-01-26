use std::{str::FromStr, sync::Arc, time::Duration};

use anyhow::anyhow;
use base64::Engine;
use directories_next::ProjectDirs;

use crate::{
    ccc::CccHttpClient,
    model::{params::TunnelParams, ConnectionStatus, TunnelServiceRequest, TunnelServiceResponse},
    platform::UdpSocketExt,
    prompt::SecurePrompt,
};

const RECV_TIMEOUT: Duration = Duration::from_secs(2);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ServiceCommand {
    Status,
    Connect,
    Disconnect,
    Reconnect,
    Info,
}

impl FromStr for ServiceCommand {
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

pub struct ServiceController {
    pub params: TunnelParams,
    prompt: SecurePrompt,
}

impl ServiceController {
    pub fn with_params(params: TunnelParams) -> Self {
        Self {
            params,
            prompt: SecurePrompt::tty(),
        }
    }

    pub fn new(prompt: SecurePrompt) -> anyhow::Result<Self> {
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

        Ok(Self { params, prompt })
    }

    pub async fn command(&self, command: ServiceCommand) -> anyhow::Result<ConnectionStatus> {
        match command {
            ServiceCommand::Status => self.do_status().await,
            ServiceCommand::Connect => {
                self.do_status().await?;
                self.do_connect().await
            }
            ServiceCommand::Disconnect => {
                self.do_status().await?;
                self.do_disconnect().await
            }
            ServiceCommand::Reconnect => {
                let _ = self.do_disconnect().await;
                self.do_connect().await
            }
            ServiceCommand::Info => self.do_info().await,
        }
    }

    #[async_recursion::async_recursion]
    pub async fn do_status(&self) -> anyhow::Result<ConnectionStatus> {
        let response = self.send_receive(TunnelServiceRequest::GetStatus, RECV_TIMEOUT).await;
        match response {
            Ok(TunnelServiceResponse::ConnectionStatus(status)) => {
                if status.connected_since.is_none() && status.mfa_pending {
                    let prompt = status.mfa_prompt.as_deref().unwrap_or("Multi-factor code: ");
                    let input = self.prompt.get_secure_input(prompt)?;
                    self.do_challenge_code(input).await?;
                }
                Ok(status)
            }
            Ok(_) => Err(anyhow!("Invalid response!")),
            Err(e) => Err(e),
        }
    }

    async fn do_connect(&self) -> anyhow::Result<ConnectionStatus> {
        let mut params = self.params.clone();

        let has_creds = params.client_cert.is_some() || !params.user_name.is_empty();

        if params.server_name.is_empty() || !has_creds {
            return Err(anyhow!(
                "Missing required parameters in the config file: server name and/or user credentials"
            ));
        }

        if params.password.is_empty() && params.client_cert.is_none() {
            match crate::platform::acquire_password(&params.user_name, self.prompt.clone()).await {
                Ok(password) => params.password = password,
                Err(e) => return Err(e),
            }
        }

        let response = self
            .send_receive(TunnelServiceRequest::Connect(params), CONNECT_TIMEOUT)
            .await;
        match response {
            Ok(TunnelServiceResponse::Ok) => self.do_status().await,
            Ok(TunnelServiceResponse::Error(error)) => Err(anyhow!(error)),
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
            Ok(TunnelServiceResponse::Ok) => {
                self.do_status().await?;
                Ok(())
            }
            Ok(TunnelServiceResponse::Error(e)) => Err(anyhow!(e)),
            Ok(_) => Err(anyhow!("Invalid response!")),
            Err(e) => Err(e),
        }
    }

    async fn do_disconnect(&self) -> anyhow::Result<ConnectionStatus> {
        self.send_receive(TunnelServiceRequest::Disconnect, RECV_TIMEOUT)
            .await?;
        self.do_status().await
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

    async fn do_info(&self) -> anyhow::Result<ConnectionStatus> {
        let client = CccHttpClient::new(Arc::new(self.params.clone()));
        let info = client.get_server_info().await?;
        let response_data = info
            .get("ResponseData")
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();

        println!("{}", serde_json::to_string_pretty(&response_data)?);

        Ok(ConnectionStatus::default())
    }
}
