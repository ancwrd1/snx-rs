use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use tokio::sync::mpsc;

use crate::{
    model::{
        params::{TunnelParams, TunnelType},
        *,
    },
    tunnel::connector::{CccTunnelConnector, IpsecTunnelConnector},
};

mod connector;
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
    RemoteControlData(Bytes),
}

#[async_trait]
pub trait CheckpointTunnel {
    async fn run(
        mut self: Box<Self>,
        command_receiver: mpsc::Receiver<TunnelCommand>,
        event_sender: mpsc::Sender<TunnelEvent>,
    ) -> anyhow::Result<()>;
}

#[async_trait]
pub trait TunnelConnector {
    async fn authenticate(&mut self) -> anyhow::Result<Arc<CccSession>>;
    async fn challenge_code(&mut self, session: Arc<CccSession>, user_input: &str) -> anyhow::Result<Arc<CccSession>>;
    async fn create_tunnel(
        &mut self,
        session: Arc<CccSession>,
        command_sender: mpsc::Sender<TunnelCommand>,
    ) -> anyhow::Result<Box<dyn CheckpointTunnel + Send>>;
    async fn terminate_tunnel(&mut self) -> anyhow::Result<()>;
    async fn rekey_tunnel(&mut self) -> anyhow::Result<()>;
    async fn handle_tunnel_event(&mut self, event: TunnelEvent) -> anyhow::Result<()>;
}

pub async fn new_tunnel_connector(params: Arc<TunnelParams>) -> anyhow::Result<Box<dyn TunnelConnector + Send>> {
    match params.tunnel_type {
        TunnelType::Ssl => Ok(Box::new(CccTunnelConnector::new(params).await?)),
        TunnelType::Ipsec => Ok(Box::new(IpsecTunnelConnector::new(params).await?)),
    }
}
