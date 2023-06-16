use anyhow::anyhow;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::{debug, warn};

use crate::{
    http::SnxHttpClient,
    model::*,
    params::{TunnelParams, TunnelType},
    tunnel::{ipsec::SnxIpsecTunnel, ssl::SnxSslTunnel},
    util,
};

mod ipsec;
mod ssl;

#[async_trait::async_trait]
pub trait SnxTunnel {
    async fn run(
        mut self: Box<Self>,
        stop_receiver: oneshot::Receiver<()>,
        connected: Arc<AtomicBool>,
    ) -> anyhow::Result<()>;
}

pub struct SnxTunnelConnector(TunnelParams);

impl SnxTunnelConnector {
    pub fn new(params: &TunnelParams) -> Self {
        Self(params.clone())
    }

    pub async fn authenticate(&self, session_id: Option<&str>) -> anyhow::Result<SnxSession> {
        debug!("Connecting to http endpoint: {}", self.0.server_name);
        let client = SnxHttpClient::new(&self.0);

        let data = client.authenticate(session_id).await?;

        let active_key = match (data.is_authenticated, data.active_key) {
            (true, Some(ref key)) => key.clone(),
            _ => {
                warn!("Authentication failed!");
                return Err(anyhow!("Authentication failed!"));
            }
        };

        let session_id = data.session_id.unwrap_or_default();
        let cookie = util::decode_from_hex(active_key.as_bytes())?;
        let cookie = String::from_utf8_lossy(&cookie).into_owned();

        debug!("Authentication OK, session id: {session_id}");

        Ok(SnxSession { session_id, cookie })
    }

    pub async fn create_tunnel(&self, session: SnxSession) -> anyhow::Result<Box<dyn SnxTunnel>> {
        match self.0.tunnel_type {
            TunnelType::Ssl => Ok(Box::new(SnxSslTunnel::create(self.0.clone(), session).await?)),
            TunnelType::Ipsec => Ok(Box::new(SnxIpsecTunnel::create(self.0.clone(), session).await?)),
        }
    }
}
