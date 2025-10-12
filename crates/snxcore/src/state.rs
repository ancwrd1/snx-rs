use std::sync::Arc;

use anyhow::Context;
use i18n::tr;
use tokio::{sync::mpsc, sync::mpsc::Sender};

use crate::{
    model::{ConnectionStatus, VpnSession},
    tunnel::{TunnelCommand, TunnelConnector, TunnelEvent, VpnTunnel},
};

#[derive(Default)]
pub struct ConnectionState {
    connection_status: ConnectionStatus,
    session: Option<Arc<VpnSession>>,
    connector: Option<Box<dyn TunnelConnector + Send + Sync>>,
    cancel_sender: Option<Sender<()>>,
}

impl ConnectionState {
    fn reset(&mut self) {
        self.session = None;
        self.connector = None;
        self.connection_status = ConnectionStatus::Disconnected;
        self.cancel_sender = None;
    }
}

pub enum ConnectionStateRequest {
    GetStatus,
    SetStatus(ConnectionStatus),
    SetSession(Arc<VpnSession>),
    CancelConnection,
    SetConnector(Box<dyn TunnelConnector + Send + Sync>),
    ChallengeCode(String),
    Disconnect,
    CreateTunnel(Arc<VpnSession>, Sender<TunnelCommand>),
    Reset,
    HandleTunnelEvent(TunnelEvent),
    SetCancelState(Sender<()>),
}

pub enum ConnectionStateResponse {
    None,
    Status(ConnectionStatus),
    Session(Option<Arc<VpnSession>>),
    Error(anyhow::Error),
    Tunnel(Box<dyn VpnTunnel + Send>),
}

pub struct ConnectionStateActor {
    sender: Sender<(ConnectionStateRequest, Sender<ConnectionStateResponse>)>,
}

impl ConnectionStateActor {
    async fn handle_request(
        msg: ConnectionStateRequest,
        state: &mut ConnectionState,
    ) -> anyhow::Result<ConnectionStateResponse> {
        let reply = match msg {
            ConnectionStateRequest::GetStatus => ConnectionStateResponse::Status(state.connection_status.clone()),
            ConnectionStateRequest::SetStatus(status) => {
                state.connection_status = status;
                ConnectionStateResponse::None
            }
            ConnectionStateRequest::SetSession(session) => {
                state.session = Some(session);
                ConnectionStateResponse::None
            }
            ConnectionStateRequest::CancelConnection => {
                if let Some(sender) = state.cancel_sender.take() {
                    let _ = sender.send(()).await;
                }
                state.reset();
                ConnectionStateResponse::None
            }
            ConnectionStateRequest::SetConnector(connector) => {
                state.connector = Some(connector);
                ConnectionStateResponse::None
            }
            ConnectionStateRequest::ChallengeCode(code) => {
                let session = state.session.clone().context("No session")?;

                if let Some(connector) = state.connector.as_mut() {
                    ConnectionStateResponse::Session(Some(connector.challenge_code(session, &code).await?))
                } else {
                    ConnectionStateResponse::Error(anyhow::anyhow!(tr!("error-no-connector-for-challenge-code")))
                }
            }
            ConnectionStateRequest::Disconnect => {
                if let Some(sender) = state.cancel_sender.take() {
                    let _ = sender.send(()).await;
                }
                if let Some(connector) = state.connector.as_mut() {
                    connector.delete_session().await;
                    let _ = connector.terminate_tunnel(true).await;
                }
                *state = ConnectionState::default();
                ConnectionStateResponse::None
            }
            ConnectionStateRequest::CreateTunnel(session, command_sender) => {
                let connector = state
                    .connector
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!(tr!("error-no-connector")))?;
                ConnectionStateResponse::Tunnel(connector.create_tunnel(session, command_sender).await?)
            }
            ConnectionStateRequest::Reset => {
                state.reset();
                ConnectionStateResponse::None
            }
            ConnectionStateRequest::HandleTunnelEvent(event) => {
                if let Some(connector) = state.connector.as_mut() {
                    match connector.handle_tunnel_event(event).await {
                        Ok(_) => ConnectionStateResponse::None,
                        Err(err) => ConnectionStateResponse::Error(err),
                    }
                } else {
                    ConnectionStateResponse::None
                }
            }
            ConnectionStateRequest::SetCancelState(sender) => {
                state.cancel_sender = Some(sender);
                ConnectionStateResponse::None
            }
        };

        Ok(reply)
    }

    async fn ask(&self, msg: ConnectionStateRequest) -> anyhow::Result<ConnectionStateResponse> {
        let (sender, mut receiver) = mpsc::channel(1);
        self.sender.send((msg, sender)).await?;
        match receiver.recv().await.ok_or_else(|| anyhow::anyhow!("channel closed"))? {
            ConnectionStateResponse::Error(e) => Err(e),
            other => Ok(other),
        }
    }

    pub fn start(mut state: ConnectionState) -> Self {
        let (sender, mut receiver) = mpsc::channel::<(ConnectionStateRequest, Sender<ConnectionStateResponse>)>(1);

        tokio::spawn(async move {
            while let Some((msg, reply_sender)) = receiver.recv().await {
                let reply = Self::handle_request(msg, &mut state)
                    .await
                    .unwrap_or_else(ConnectionStateResponse::Error);
                let _ = reply_sender.send(reply).await;
            }
        });
        Self { sender }
    }

    pub async fn get_status(&self) -> anyhow::Result<ConnectionStatus> {
        match self.ask(ConnectionStateRequest::GetStatus).await? {
            ConnectionStateResponse::Status(status) => Ok(status),
            _ => anyhow::bail!("unexpected response"),
        }
    }

    pub async fn set_status(&self, status: ConnectionStatus) -> anyhow::Result<()> {
        self.ask(ConnectionStateRequest::SetStatus(status)).await?;
        Ok(())
    }

    pub async fn set_session(&self, session: Arc<VpnSession>) -> anyhow::Result<()> {
        self.ask(ConnectionStateRequest::SetSession(session)).await?;
        Ok(())
    }

    pub async fn cancel_connection(&self) -> anyhow::Result<()> {
        self.ask(ConnectionStateRequest::CancelConnection).await?;
        Ok(())
    }

    pub async fn set_connector(&self, connector: Box<dyn TunnelConnector + Send + Sync>) -> anyhow::Result<()> {
        self.ask(ConnectionStateRequest::SetConnector(connector)).await?;
        Ok(())
    }

    pub async fn challenge_code<S: AsRef<str>>(&self, code: S) -> anyhow::Result<Arc<VpnSession>> {
        match self
            .ask(ConnectionStateRequest::ChallengeCode(code.as_ref().to_owned()))
            .await?
        {
            ConnectionStateResponse::Session(Some(session)) => Ok(session),
            _ => anyhow::bail!("unexpected response"),
        }
    }

    pub async fn disconnect(&self) -> anyhow::Result<()> {
        self.ask(ConnectionStateRequest::Disconnect).await?;
        Ok(())
    }

    pub async fn create_tunnel(
        &self,
        session: Arc<VpnSession>,
        command_sender: mpsc::Sender<TunnelCommand>,
    ) -> anyhow::Result<Box<dyn VpnTunnel + Send>> {
        match self
            .ask(ConnectionStateRequest::CreateTunnel(session, command_sender))
            .await?
        {
            ConnectionStateResponse::Tunnel(tunnel) => Ok(tunnel),
            _ => anyhow::bail!("unexpected response"),
        }
    }

    pub async fn reset(&self) -> anyhow::Result<()> {
        self.ask(ConnectionStateRequest::Reset).await?;
        Ok(())
    }

    pub async fn handle_tunnel_event(&self, event: TunnelEvent) -> anyhow::Result<()> {
        match self.ask(ConnectionStateRequest::HandleTunnelEvent(event)).await? {
            ConnectionStateResponse::None => Ok(()),
            _ => anyhow::bail!("unexpected response"),
        }
    }

    pub async fn set_cancel_state(&self, sender: mpsc::Sender<()>) -> anyhow::Result<()> {
        self.ask(ConnectionStateRequest::SetCancelState(sender)).await?;
        Ok(())
    }
}
