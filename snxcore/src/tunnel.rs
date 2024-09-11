use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use tokio::sync::mpsc;

use crate::{
    model::{
        params::{TunnelParams, TunnelType},
        *,
    },
    tunnel::{ipsec::connector::IpsecTunnelConnector, ssl::connector::CccTunnelConnector},
};

mod ipsec;
mod ssl;

#[derive(Debug, Clone, PartialEq)]
pub enum TunnelCommand {
    Terminate,
    ReKey(IpsecSession),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TunnelEvent {
    Connected,
    Disconnected,
    RekeyCheck,
    RemoteControlData(Bytes),
}

#[async_trait]
pub trait VpnTunnel {
    async fn run(
        mut self: Box<Self>,
        command_receiver: mpsc::Receiver<TunnelCommand>,
        event_sender: mpsc::Sender<TunnelEvent>,
    ) -> anyhow::Result<()>;
}

#[async_trait]
pub trait TunnelConnector {
    async fn authenticate(&mut self) -> anyhow::Result<Arc<VpnSession>>;
    async fn delete_session(&mut self);
    async fn restore_session(&mut self) -> anyhow::Result<Arc<VpnSession>>;
    async fn challenge_code(&mut self, session: Arc<VpnSession>, user_input: &str) -> anyhow::Result<Arc<VpnSession>>;
    async fn create_tunnel(
        &mut self,
        session: Arc<VpnSession>,
        command_sender: mpsc::Sender<TunnelCommand>,
    ) -> anyhow::Result<Box<dyn VpnTunnel + Send>>;
    async fn terminate_tunnel(&mut self) -> anyhow::Result<()>;
    async fn handle_tunnel_event(&mut self, event: TunnelEvent) -> anyhow::Result<()>;
}

pub async fn new_tunnel_connector(params: Arc<TunnelParams>) -> anyhow::Result<Box<dyn TunnelConnector + Send>> {
    match params.tunnel_type {
        TunnelType::Ssl => Ok(Box::new(CccTunnelConnector::new(params).await?)),
        TunnelType::Ipsec => Ok(Box::new(IpsecTunnelConnector::new(params).await?)),
    }
}
