use std::sync::{Arc, Mutex};

use anyhow::anyhow;
use tokio::sync::oneshot;
use tracing::{debug, warn};

use crate::{
    http::CccHttpClient,
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

    pub async fn authenticate(&self, session_id: Option<&str>) -> anyhow::Result<TunnelSession> {
        debug!("Authenticating to endpoint: {}", self.0.server_name);
        let client = CccHttpClient::new(self.0.clone());

        let data = client.authenticate(session_id).await?;

        self.process_auth_response(data).await
    }

    pub async fn challenge_code(&self, session_id: &str, user_input: &str) -> anyhow::Result<TunnelSession> {
        debug!("Authenticating with challenge code to endpoint: {}", self.0.server_name);
        let client = CccHttpClient::new(self.0.clone());

        let data = client.challenge_code(session_id, user_input).await?;

        self.process_auth_response(data).await
    }

    async fn process_auth_response(&self, data: AuthResponse) -> anyhow::Result<TunnelSession> {
        let session_id = data.session_id.unwrap_or_default();

        match data.authn_status.as_str() {
            "continue" => {
                return Ok(TunnelSession {
                    session_id,
                    state: SessionState::Pending(data.prompt.map(|p| p.0)),
                })
            }
            "done" => {}
            other => {
                warn!("Authn status: {}", other);
                return Err(anyhow!("Authentication failed!"));
            }
        }

        let cookie = match (data.is_authenticated, data.active_key) {
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

        Ok(TunnelSession {
            session_id,
            state: SessionState::Authenticated(cookie.0),
        })
    }

    pub async fn create_tunnel(&self, session: Arc<TunnelSession>) -> anyhow::Result<Box<dyn CheckpointTunnel + Send>> {
        match self.0.tunnel_type {
            TunnelType::Ssl => Ok(Box::new(SslTunnel::create(self.0.clone(), session).await?)),
            TunnelType::Ipsec => Ok(Box::new(IpsecTunnel::create(self.0.clone(), session).await?)),
        }
    }
}
