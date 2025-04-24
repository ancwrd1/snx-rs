use std::{collections::VecDeque, str::FromStr, sync::Arc, time::Duration};

use anyhow::anyhow;
use futures::{SinkExt, StreamExt};
use tokio::net::UnixStream;
use tokio_util::codec::{Decoder, LengthDelimitedCodec};
use tracing::warn;

use crate::{
    browser::{spawn_otp_listener, BrowserController},
    model::{
        params::TunnelParams, ConnectionStatus, MfaChallenge, MfaType, PromptInfo, TunnelServiceRequest,
        TunnelServiceResponse,
    },
    platform::{self},
    prompt::SecurePrompt,
    server::DEFAULT_LISTEN_PATH,
    server_info,
};

const RECV_TIMEOUT: Duration = Duration::from_secs(2);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(120);
const SERVICE_CONNECT_TIMEOUT: Duration = Duration::from_secs(1);

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

pub struct ServiceController<B, P> {
    prompt: P,
    mfa_prompts: Option<VecDeque<PromptInfo>>,
    password_from_keychain: String,
    username: String,
    mfa_index: usize,
    browser_controller: B,
    stream: Option<UnixStream>,
    otp_cancel_sender: Option<tokio::sync::oneshot::Sender<()>>,
}

impl<B, P> ServiceController<B, P>
where
    B: BrowserController + Send + Sync,
    P: SecurePrompt + Send + Sync,
{
    pub fn new(prompt: P, browser_controller: B) -> Self {
        Self {
            prompt,
            mfa_prompts: None,
            password_from_keychain: String::new(),
            username: String::new(),
            mfa_index: 0,
            browser_controller,
            stream: None,
            otp_cancel_sender: None,
        }
    }

    async fn get_stream(&mut self) -> anyhow::Result<&mut UnixStream> {
        match self.stream.take() {
            Some(stream) => {
                self.stream = Some(stream);
                Ok(self.stream.as_mut().unwrap())
            }
            None => Ok(self.stream.insert(
                tokio::time::timeout(SERVICE_CONNECT_TIMEOUT, UnixStream::connect(DEFAULT_LISTEN_PATH)).await??,
            )),
        }
    }

    pub async fn command(
        &mut self,
        command: ServiceCommand,
        params: Arc<TunnelParams>,
    ) -> anyhow::Result<ConnectionStatus> {
        match command {
            ServiceCommand::Status => self.do_status(params, false).await,
            ServiceCommand::Connect => self.do_connect(params).await,
            ServiceCommand::Disconnect => self.do_disconnect(params).await,
            ServiceCommand::Reconnect => {
                let _ = self.do_disconnect(params.clone()).await;
                self.do_connect(params).await
            }
            ServiceCommand::Info => self.do_info(params).await,
        }
    }

    #[async_recursion::async_recursion]
    pub async fn do_status(&mut self, params: Arc<TunnelParams>, with_mfa: bool) -> anyhow::Result<ConnectionStatus> {
        let response = self.send_receive(TunnelServiceRequest::GetStatus, RECV_TIMEOUT).await?;
        match response {
            TunnelServiceResponse::ConnectionStatus(status) => {
                if let (true, ConnectionStatus::Mfa(mfa)) = (with_mfa, &status) {
                    self.process_mfa_request(mfa, params).await
                } else {
                    Ok(status)
                }
            }
            TunnelServiceResponse::Error(e) => Err(anyhow!(e)),
            TunnelServiceResponse::Ok => Err(anyhow!("Unexpected response")),
        }
    }

    async fn process_mfa_request(
        &mut self,
        mfa: &MfaChallenge,
        params: Arc<TunnelParams>,
    ) -> anyhow::Result<ConnectionStatus> {
        match self.get_mfa_input(mfa, params.clone()).await {
            Ok(input) => {
                let result = self.do_challenge_code(input.clone(), params.clone()).await;
                if result.is_ok()
                    && mfa.mfa_type == MfaType::PasswordInput
                    && self.mfa_index == params.password_factor
                    && !params.no_keychain
                    && !input.is_empty()
                {
                    let _ = platform::store_password(&self.username, &input).await;
                }
                result
            }
            Err(e) => {
                let _ = self.send_receive(TunnelServiceRequest::Disconnect, RECV_TIMEOUT).await;
                Err(e)
            }
        }
    }

    async fn get_mfa_input(&mut self, mfa: &MfaChallenge, params: Arc<TunnelParams>) -> anyhow::Result<String> {
        match mfa.mfa_type {
            MfaType::PasswordInput => {
                self.mfa_index += 1;

                let prompt = self
                    .mfa_prompts
                    .as_mut()
                    .and_then(|p| p.pop_front())
                    .unwrap_or_else(|| PromptInfo::new("", &mfa.prompt));

                if !params.password.is_empty() && self.mfa_index == params.password_factor {
                    Ok(params.password.clone())
                } else if !self.password_from_keychain.is_empty() && self.mfa_index == params.password_factor {
                    Ok(self.password_from_keychain.clone())
                } else {
                    let input = self.prompt.get_secure_input(prompt).await?;
                    Ok(input)
                }
            }
            MfaType::IdentityProvider => {
                let (tx, rx) = tokio::sync::oneshot::channel();
                self.otp_cancel_sender = Some(tx);
                let receiver = spawn_otp_listener(rx);

                self.browser_controller.open(&mfa.prompt)?;

                match receiver.await {
                    Ok(Ok(otp)) => {
                        self.browser_controller.close();
                        Ok(otp)
                    }
                    _ => {
                        warn!("Unable to acquire OTP from the browser");
                        Err(anyhow!("Unable to acquire OTP from the browser!"))
                    }
                }
            }
            MfaType::UserNameInput => {
                let prompt = PromptInfo::new("Username is required for authentication", &mfa.prompt);
                let input = self.prompt.get_plain_input(prompt).await?;
                self.username = input.clone();

                if !self.username.is_empty() && !params.no_keychain && params.password.is_empty() {
                    if let Ok(password) = platform::acquire_password(&self.username).await {
                        self.password_from_keychain = password;
                    }
                }

                Ok(input)
            }
        }
    }

    async fn do_connect(&mut self, params: Arc<TunnelParams>) -> anyhow::Result<ConnectionStatus> {
        if params.server_name.is_empty() {
            anyhow::bail!("Missing required parameter: server-name");
        }

        if params.login_type.is_empty() {
            anyhow::bail!("Missing required parameter: login-type");
        }

        if !params.user_name.is_empty() && !params.no_keychain && params.password.is_empty() {
            if let Ok(password) = platform::acquire_password(&params.user_name).await {
                self.password_from_keychain = password;
            }
        }

        self.fill_mfa_prompts(params.clone()).await;

        self.username = params.user_name.clone();

        let response = self
            .send_receive(TunnelServiceRequest::Connect((*params).clone()), CONNECT_TIMEOUT)
            .await;

        loop {
            match response {
                Ok(TunnelServiceResponse::Ok) => match self.do_status(params.clone(), true).await {
                    Ok(ConnectionStatus::Connecting) => continue,
                    other => break other,
                },
                Ok(TunnelServiceResponse::Error(error)) => break Err(anyhow!(error)),
                Ok(TunnelServiceResponse::ConnectionStatus(status)) => break Ok(status),
                Err(e) => break Err(e),
            }
        }
    }

    async fn do_challenge_code(&mut self, code: String, params: Arc<TunnelParams>) -> anyhow::Result<ConnectionStatus> {
        let response = self
            .send_receive(
                TunnelServiceRequest::ChallengeCode(code, (*params).clone()),
                CONNECT_TIMEOUT,
            )
            .await;
        match response {
            Ok(TunnelServiceResponse::Ok) => self.do_status(params, true).await,
            Ok(TunnelServiceResponse::Error(e)) => {
                self.send_receive(TunnelServiceRequest::Disconnect, RECV_TIMEOUT)
                    .await?;
                Err(anyhow!(e))
            }
            Ok(_) => Err(anyhow!("Invalid response!")),
            Err(e) => Err(e),
        }
    }

    async fn do_disconnect(&mut self, params: Arc<TunnelParams>) -> anyhow::Result<ConnectionStatus> {
        if let Some(cancel_sender) = self.otp_cancel_sender.take() {
            let _ = cancel_sender.send(());
        }
        self.send_receive(TunnelServiceRequest::Disconnect, RECV_TIMEOUT)
            .await?;
        self.do_status(params, false).await
    }

    async fn send_receive(
        &mut self,
        request: TunnelServiceRequest,
        timeout: Duration,
    ) -> anyhow::Result<TunnelServiceResponse> {
        let mut stream = self
            .get_stream()
            .await
            .map_err(|_| anyhow!("Unable to connect to the service!"))?;

        let mut codec = LengthDelimitedCodec::new().framed(&mut stream);

        let data = serde_json::to_vec(&request)?;

        let Ok(_) = codec.send(data.into()).await else {
            self.stream = None;
            anyhow::bail!("Cannot send request to the service!")
        };

        if let Ok(Some(Ok(bytes))) = tokio::time::timeout(timeout, codec.next()).await {
            Ok(serde_json::from_slice(&bytes)?)
        } else {
            self.stream = None;
            anyhow::bail!("Cannot read reply from the service!")
        }
    }

    async fn fill_mfa_prompts(&mut self, params: Arc<TunnelParams>) {
        self.mfa_index = 0;
        self.mfa_prompts
            .replace(server_info::get_login_prompts(&params).await.unwrap_or_default());
    }

    async fn do_info(&self, params: Arc<TunnelParams>) -> anyhow::Result<ConnectionStatus> {
        crate::util::print_login_options(&params).await?;

        Ok(ConnectionStatus::default())
    }
}
