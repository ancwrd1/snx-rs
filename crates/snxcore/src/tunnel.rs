use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use bytes::Bytes;
use ipnet::Ipv4Net;
use tokio::sync::mpsc;

use crate::model::{
    ConnectionInfo, IPsecSession, TunnelSession,
    params::TunnelParams,
    proto::{AuthResponse, CertificateResponse, ClientSettingsResponse, GatewayInformation},
    wrappers::SessionId,
};

pub mod connector;
pub mod device;
pub mod gateway;
pub mod ipsec;
pub mod ssl;

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
        self: Box<Self>,
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
pub trait GatewayConnector {
    async fn authenticate(&self, username: &str) -> anyhow::Result<AuthResponse>;
    async fn challenge_code(&self, session_id: &SessionId, user_input: &str) -> anyhow::Result<AuthResponse>;
    async fn get_client_settings(&self, session_id: &SessionId) -> anyhow::Result<ClientSettingsResponse>;
    async fn get_gateway_information(&self) -> anyhow::Result<GatewayInformation>;
    async fn enroll_certificate(&self, registration_key: &str, password: &str) -> anyhow::Result<CertificateResponse>;
    async fn renew_certificate(&self, pkcs12: &[u8], password: &str) -> anyhow::Result<CertificateResponse>;
    async fn signout(&self, session_id: &SessionId) -> anyhow::Result<()>;
}

pub trait TunnelConnectorFactory: Clone {
    fn new_tunnel_connector(
        &self,
        params: Arc<TunnelParams>,
    ) -> impl Future<Output = anyhow::Result<Box<dyn TunnelConnector + Send + Sync>>> + Send;
    fn new_gateway_connector(&self, params: Arc<TunnelParams>) -> Arc<dyn GatewayConnector + Send + Sync>;
}
