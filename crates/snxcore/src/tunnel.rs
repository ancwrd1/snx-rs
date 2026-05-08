use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use bytes::Bytes;
use ipnet::Ipv4Net;
use tokio::sync::mpsc;

use crate::{
    gateway::{GatewayConnector, ccc::CccGatewayConnector},
    model::{
        params::{TunnelParams, TunnelType},
        proto::LoginOption,
        *,
    },
    tunnel::{ipsec::connector::IPsecTunnelConnector, ssl::connector::SslTunnelConnector},
};

pub mod device;
mod ipsec;
mod ssl;

#[derive(Debug, Clone, PartialEq)]
pub enum TunnelCommand {
    Terminate(bool),
    ReKey(IPsecSession),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TunnelEvent {
    Connected(Box<ConnectionInfo>),
    Disconnected,
    RekeyCheck,
    RemoteControlData(Bytes),
    Rekeyed(Ipv4Net),
    Rtt(Duration),
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
    async fn authenticate(&mut self) -> anyhow::Result<Arc<TunnelSession>>;
    async fn delete_session(&mut self) -> anyhow::Result<()>;
    async fn restore_session(&mut self) -> anyhow::Result<Arc<TunnelSession>>;
    async fn challenge_code(
        &mut self,
        session: Arc<TunnelSession>,
        user_input: &str,
    ) -> anyhow::Result<Arc<TunnelSession>>;
    async fn create_tunnel(
        &mut self,
        session: Arc<TunnelSession>,
        command_sender: mpsc::Sender<TunnelCommand>,
    ) -> anyhow::Result<Box<dyn VpnTunnel + Send>>;
    async fn terminate_tunnel(&mut self, signout: bool) -> anyhow::Result<()>;
    async fn handle_tunnel_event(&mut self, event: TunnelEvent) -> anyhow::Result<()>;
}

#[async_trait]
pub trait TunnelConnectorFactory: Clone {
    async fn new_tunnel_connector(
        &self,
        params: Arc<TunnelParams>,
    ) -> anyhow::Result<Box<dyn TunnelConnector + Send + Sync>>;
    fn new_gateway_connector(&self, params: Arc<TunnelParams>) -> Arc<dyn GatewayConnector + Send + Sync>;
}

#[derive(Clone, Default)]
pub struct CheckPointTunnelConnectorFactory {}

#[async_trait]
impl TunnelConnectorFactory for CheckPointTunnelConnectorFactory {
    async fn new_tunnel_connector(
        &self,
        params: Arc<TunnelParams>,
    ) -> anyhow::Result<Box<dyn TunnelConnector + Send + Sync>> {
        match params.tunnel_type {
            TunnelType::IPsec if params.login_type != LoginOption::MOBILE_ACCESS_ID => Ok(Box::new(
                IPsecTunnelConnector::new(params.clone(), self.new_gateway_connector(params)).await?,
            )),
            _ => Ok(Box::new(
                SslTunnelConnector::new(params.clone(), self.new_gateway_connector(params)).await?,
            )),
        }
    }

    fn new_gateway_connector(&self, params: Arc<TunnelParams>) -> Arc<dyn GatewayConnector + Send + Sync> {
        Arc::new(CccGatewayConnector::new(params.clone()))
    }
}
