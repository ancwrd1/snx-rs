use std::{collections::VecDeque, str::FromStr, sync::Arc, time::Duration};

use anyhow::anyhow;
use directories_next::ProjectDirs;
use tokio::sync::oneshot;
use tracing::warn;

use crate::{
    browser::BrowserController,
    ccc::CccHttpClient,
    model::{
        params::TunnelParams, ConnectionStatus, MfaChallenge, MfaType, TunnelServiceRequest, TunnelServiceResponse,
    },
    platform::{self, UdpSocketExt},
    prompt::{run_otp_listener, SecurePrompt, OTP_TIMEOUT},
    server_info,
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

pub struct ServiceController<'a> {
    pub params: TunnelParams,
    prompt: SecurePrompt,
    mfa_prompts: Option<VecDeque<String>>,
    password: String,
    browser_controller: &'a BrowserController,
}

impl<'a> ServiceController<'a> {
    pub fn new(prompt: SecurePrompt, browser_controller: &'a BrowserController) -> anyhow::Result<Self> {
        let dir = ProjectDirs::from("", "", "snx-rs").ok_or(anyhow!("No project directory!"))?;
        let config_file = dir.config_dir().join("snx-rs.conf");

        if !config_file.exists() {
            return Err(anyhow!("No config file: {}", config_file.display()));
        }
        let mut params = TunnelParams::load(config_file)?;

        params.decode_password()?;

        Ok(Self {
            params,
            prompt,
            mfa_prompts: None,
            password: String::new(),
            browser_controller,
        })
    }

    pub async fn command(&mut self, command: ServiceCommand) -> anyhow::Result<ConnectionStatus> {
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
    pub async fn do_status(&mut self) -> anyhow::Result<ConnectionStatus> {
        let response = self.send_receive(TunnelServiceRequest::GetStatus, RECV_TIMEOUT).await?;
        match response {
            TunnelServiceResponse::ConnectionStatus(status) => {
                if let (None, Some(mfa)) = (status.connected_since, &status.mfa) {
                    match self.get_mfa_input(mfa).await {
                        Ok(input) => {
                            let result = self.do_challenge_code(input.clone()).await;
                            if result.is_ok()
                                && mfa.mfa_type == MfaType::UserInput
                                && !self.password.is_empty()
                                && !self.params.no_keychain
                            {
                                let _ = platform::store_password(&self.params.user_name, &input).await;
                                self.password.clear();
                            }
                            result
                        }
                        Err(e) => {
                            let _ = self.send_receive(TunnelServiceRequest::Disconnect, RECV_TIMEOUT).await;
                            Err(e)
                        }
                    }
                } else {
                    Ok(status)
                }
            }
            TunnelServiceResponse::Error(e) => Err(anyhow!(e)),
            TunnelServiceResponse::Ok => Err(anyhow!("Unexpected response")),
        }
    }

    async fn get_mfa_input(&mut self, mfa: &MfaChallenge) -> anyhow::Result<String> {
        match mfa.mfa_type {
            MfaType::UserInput => {
                let prompt = self
                    .mfa_prompts
                    .as_mut()
                    .and_then(|p| p.pop_front())
                    .unwrap_or_else(|| mfa.prompt.clone());
                let input = self.prompt.get_secure_input(&prompt)?;
                if self.password.is_empty() {
                    self.password = input.clone();
                }
                Ok(input)
            }
            MfaType::SamlSso => {
                let (tx, rx) = oneshot::channel();
                tokio::spawn(run_otp_listener(tx));

                self.browser_controller.open(&mfa.prompt)?;

                match tokio::time::timeout(OTP_TIMEOUT, rx).await {
                    Ok(Ok(otp)) => {
                        let _ = self.browser_controller.close();
                        Ok(otp)
                    }
                    _ => {
                        warn!("Unable to acquire OTP from the browser");
                        Err(anyhow!("Unable to acquire OTP from the browser!"))
                    }
                }
            }
        }
    }

    async fn do_connect(&mut self) -> anyhow::Result<ConnectionStatus> {
        self.fill_mfa_prompts().await;

        let params = self.params.clone();

        if params.server_name.is_empty() || params.login_type.is_empty() {
            return Err(anyhow!(
                "Missing required parameters in the config file: server name and/or login type"
            ));
        }

        if !params.user_name.is_empty() && !params.no_keychain && params.password.is_empty() {
            if let Ok(password) = platform::acquire_password(&self.params.user_name).await {
                self.password = password;
            }
        }

        let response = self
            .send_receive(TunnelServiceRequest::Connect(self.params.clone()), CONNECT_TIMEOUT)
            .await;
        match response {
            Ok(TunnelServiceResponse::Ok) => self.do_status().await,
            Ok(TunnelServiceResponse::Error(error)) => Err(anyhow!(error)),
            Ok(_) => Err(anyhow!("Invalid response!")),
            Err(e) => Err(e),
        }
    }

    async fn do_challenge_code(&mut self, code: String) -> anyhow::Result<ConnectionStatus> {
        let response = self
            .send_receive(
                TunnelServiceRequest::ChallengeCode(code, self.params.clone()),
                CONNECT_TIMEOUT,
            )
            .await;
        match response {
            Ok(TunnelServiceResponse::Ok) => self.do_status().await,
            Ok(TunnelServiceResponse::Error(e)) => {
                self.send_receive(TunnelServiceRequest::Disconnect, RECV_TIMEOUT)
                    .await?;
                Err(anyhow!(e))
            }
            Ok(_) => Err(anyhow!("Invalid response!")),
            Err(e) => Err(e),
        }
    }

    async fn do_disconnect(&mut self) -> anyhow::Result<ConnectionStatus> {
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

    async fn fill_mfa_prompts(&mut self) {
        self.mfa_prompts
            .replace(server_info::get_mfa_prompts(&self.params).await.unwrap_or_default());
    }

    async fn do_info(&self) -> anyhow::Result<ConnectionStatus> {
        let client = CccHttpClient::new(Arc::new(self.params.clone()), None);
        let info = client.get_server_info().await?;

        crate::util::print_login_options(&info);

        Ok(ConnectionStatus::default())
    }
}
