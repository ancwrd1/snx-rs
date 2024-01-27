use std::sync::{Arc, Mutex};

use anyhow::anyhow;
use tokio::sync::oneshot;
use tracing::{debug, warn};

use crate::{
    ccc::CccHttpClient,
    model::{
        params::{TunnelParams, TunnelType},
        proto::AuthResponse,
        *,
    },
    tunnel::{ipsec::IpsecTunnel, ssl::SslTunnel},
};

mod ipsec;
mod ssl;

#[async_trait::async_trait]
pub trait CheckpointTunnel {
    async fn run(
        mut self: Box<Self>,
        stop_receiver: oneshot::Receiver<()>,
        connected: Arc<Mutex<ConnectionStatus>>,
    ) -> anyhow::Result<()>;
}

pub struct TunnelConnector(Arc<TunnelParams>);

impl TunnelConnector {
    pub fn new(params: Arc<TunnelParams>) -> Self {
        Self(params)
    }

    pub async fn authenticate(&self) -> anyhow::Result<CccSession> {
        debug!("Authenticating to endpoint: {}", self.0.server_name);
        let client = CccHttpClient::new(self.0.clone(), None);

        let data = client.authenticate().await?;

        self.process_auth_response(data).await
    }

    pub async fn challenge_code(&self, session: Arc<CccSession>, user_input: &str) -> anyhow::Result<CccSession> {
        debug!("Authenticating with challenge code to endpoint: {}", self.0.server_name);
        let client = CccHttpClient::new(self.0.clone(), Some(session));

        let data = client.challenge_code(user_input).await?;

        self.process_auth_response(data).await
    }

    async fn process_auth_response(&self, data: AuthResponse) -> anyhow::Result<CccSession> {
        let session_id = data.session_id.unwrap_or_default();

        match data.authn_status.as_str() {
            "continue" => {
                return Ok(CccSession {
                    session_id,
                    state: SessionState::Pending {
                        prompt: data.prompt.map(|p| p.0),
                    },
                })
            }
            "done" => {}
            other => {
                warn!("Authn status: {}", other);
                return Err(anyhow!("Authentication failed!"));
            }
        }

        let active_key = match (data.is_authenticated, data.active_key) {
            (Some(true), Some(ref key)) => key.clone(),
            _ => {
                let msg = match (data.error_message, data.error_id, data.error_code) {
                    (Some(message), Some(id), Some(code)) => format!("[{} {}] {}", code, id.0, message.0),
                    _ => "Authentication failed!".to_owned(),
                };
                warn!("{}", msg);
                return Err(anyhow!(msg));
            }
        };

        debug!("Authentication OK, session id: {session_id}");

        Ok(CccSession {
            session_id,
            state: SessionState::Authenticated {
                active_key: active_key.0,
            },
        })
    }

    pub async fn create_tunnel(&self, session: Arc<CccSession>) -> anyhow::Result<Box<dyn CheckpointTunnel + Send>> {
        match self.0.tunnel_type {
            TunnelType::Ssl => Ok(Box::new(SslTunnel::create(self.0.clone(), session).await?)),
            TunnelType::Ipsec => Ok(Box::new(IpsecTunnel::create(self.0.clone(), session).await?)),
        }
    }
}
