use std::sync::{Arc, Mutex};

use anyhow::anyhow;
use tokio::sync::oneshot;
use tracing::{debug, warn};

use crate::{
    http::SnxHttpClient,
    model::{
        params::{TunnelParams, TunnelType},
        snx::AuthResponseData,
        *,
    },
    tunnel::{ipsec::SnxIpsecTunnel, ssl::SnxSslTunnel},
};

mod ipsec;
mod ssl;

#[async_trait::async_trait]
pub trait SnxTunnel {
    async fn run(
        mut self: Box<Self>,
        stop_receiver: oneshot::Receiver<()>,
        connected: Arc<Mutex<ConnectionStatus>>,
    ) -> anyhow::Result<()>;
}

pub struct SnxTunnelConnector(Arc<TunnelParams>);

impl SnxTunnelConnector {
    pub fn new(params: Arc<TunnelParams>) -> Self {
        Self(params)
    }

    pub async fn authenticate(&self, session_id: Option<&str>) -> anyhow::Result<SnxSession> {
        debug!("Authenticating to endpoint: {}", self.0.server_name);
        let client = SnxHttpClient::new(self.0.clone());

        let data = client.authenticate(session_id).await?;

        self.process_auth_response(data).await
    }

    pub async fn challenge_code(&self, session_id: &str, user_input: &str) -> anyhow::Result<SnxSession> {
        debug!(
            "Authenticating with challenge code {} to endpoint: {}",
            user_input, self.0.server_name
        );
        let client = SnxHttpClient::new(self.0.clone());

        let data = client.challenge_code(session_id, user_input).await?;

        self.process_auth_response(data).await
    }

    async fn process_auth_response(&self, data: AuthResponseData) -> anyhow::Result<SnxSession> {
        let session_id = data.session_id.unwrap_or_default();

        match data.authn_status.as_str() {
            "continue" => {
                return Ok(SnxSession {
                    session_id,
                    cookie: None,
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
                warn!("Authentication failed!");
                return Err(anyhow!("Authentication failed!"));
            }
        };

        debug!("Authentication OK, session id: {session_id}");

        Ok(SnxSession {
            session_id,
            cookie: Some(cookie.0),
        })
    }

    pub async fn create_tunnel(&self, session: Arc<SnxSession>) -> anyhow::Result<Box<dyn SnxTunnel + Send>> {
        match self.0.tunnel_type {
            TunnelType::Ssl => Ok(Box::new(SnxSslTunnel::create(self.0.clone(), session).await?)),
            TunnelType::Ipsec => Ok(Box::new(SnxIpsecTunnel::create(self.0.clone(), session).await?)),
        }
    }
}
