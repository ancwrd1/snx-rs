use std::sync::Arc;

use anyhow::anyhow;
use futures::pin_mut;
use tokio::sync::mpsc;
use tracing::{debug, trace, warn};

use crate::{
    model::{
        params::TunnelParams, ConnectionStatus, SessionState, TunnelServiceRequest, TunnelServiceResponse, VpnSession,
    },
    tunnel::{self, TunnelConnector, TunnelEvent},
};

pub const LISTEN_PORT: u16 = 7779;

const MAX_PACKET_SIZE: usize = 1_000_000;

pub struct CommandServer {
    port: u16,
    connection_status: ConnectionStatus,
    session: Option<Arc<VpnSession>>,
    connector: Option<Box<dyn TunnelConnector + Send>>,
}

impl CommandServer {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            connection_status: ConnectionStatus::default(),
            session: None,
            connector: None,
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        debug!("Starting command server on port {}", self.port);

        let socket = Arc::new(tokio::net::UdpSocket::bind(("127.0.0.1", self.port)).await?);
        let (event_sender, mut event_receiver) = mpsc::channel::<TunnelEvent>(16);

        loop {
            let recv = async {
                let mut buf = vec![0u8; MAX_PACKET_SIZE];
                let (size, addr) = socket.recv_from(&mut buf).await?;
                Ok::<_, anyhow::Error>((buf[0..size].to_vec(), addr))
            };
            pin_mut!(recv);

            let event_fut = event_receiver.recv();
            pin_mut!(event_fut);

            tokio::select! {
                event = event_fut => {
                    if let Some(event) = event {
                        if let Some(ref mut connector) = self.connector {
                            if connector.handle_tunnel_event(event.clone()).await.is_err() {
                                self.reset();
                            }
                        }
                        match event {
                            TunnelEvent::Connected => {
                                self.connection_status = ConnectionStatus::connected();
                            }
                            TunnelEvent::Disconnected => {
                                self.reset();
                            }
                            _ => {}
                        }
                    }
                }
                result = recv => {
                    let (data, addr) = result?;
                    let resp = self.handle(&data, event_sender.clone()).await;
                    trace!("Response: {:?}", resp);
                    let json = serde_json::to_vec(&resp)?;
                    let _ = socket.send_to(&json, addr).await;
                }
            }
        }
    }

    async fn handle(&mut self, packet: &[u8], event_sender: mpsc::Sender<TunnelEvent>) -> TunnelServiceResponse {
        trace!("Command received");
        let req = match serde_json::from_slice::<TunnelServiceRequest>(packet) {
            Ok(req) => req,
            Err(e) => {
                warn!("{}", e);
                return TunnelServiceResponse::Error(e.to_string());
            }
        };

        match req {
            TunnelServiceRequest::Connect(params) => {
                trace!("Handling connect command");
                match self.connect(Arc::new(params), event_sender).await {
                    Ok(()) => TunnelServiceResponse::Ok,
                    Err(e) => {
                        self.reset();
                        TunnelServiceResponse::Error(e.to_string())
                    }
                }
            }
            TunnelServiceRequest::Disconnect => {
                debug!("Handling disconnect command");

                match self.disconnect().await {
                    Ok(()) => TunnelServiceResponse::Ok,
                    Err(e) => TunnelServiceResponse::Error(e.to_string()),
                }
            }
            TunnelServiceRequest::GetStatus => {
                trace!("Handling get status command");
                TunnelServiceResponse::ConnectionStatus(self.get_status().clone())
            }
            TunnelServiceRequest::ChallengeCode(code, _) => {
                debug!("Handling challenge code command");
                match self.challenge_code(&code, event_sender).await {
                    Ok(()) => TunnelServiceResponse::Ok,
                    Err(e) => {
                        warn!("{}", e);
                        self.reset();
                        TunnelServiceResponse::Error(e.to_string())
                    }
                }
            }
        }
    }

    fn is_connected(&self) -> bool {
        self.connection_status.connected_since.is_some()
    }

    async fn connect_for_session(
        &mut self,
        session: Arc<VpnSession>,
        event_sender: mpsc::Sender<TunnelEvent>,
    ) -> anyhow::Result<()> {
        let Some(ref mut connector) = self.connector else {
            return Err(anyhow!("No tunnel connector!"));
        };

        if let SessionState::PendingChallenge(ref challenge) = session.state {
            debug!("Pending multi-factor, awaiting for it");
            self.session = Some(session.clone());
            self.connection_status = ConnectionStatus::mfa(challenge.clone());
            return Ok(());
        }

        let (command_sender, command_receiver) = mpsc::channel(16);

        let tunnel = connector.create_tunnel(session, command_sender).await?;

        tokio::spawn(async move {
            if let Err(e) = tunnel.run(command_receiver, event_sender).await {
                warn!("Tunnel error: {}", e);
            }
        });

        self.connection_status = ConnectionStatus::connected();

        Ok(())
    }

    async fn connect(
        &mut self,
        params: Arc<TunnelParams>,
        event_sender: mpsc::Sender<TunnelEvent>,
    ) -> anyhow::Result<()> {
        if self.is_connected() {
            Ok(())
        } else {
            self.reset();

            let mut connector = tunnel::new_tunnel_connector(params.clone()).await?;
            let session = if params.ike_persist {
                debug!("Attempting to load IKE session");
                match connector.restore_session().await {
                    Ok(session) => session,
                    Err(_) => connector.authenticate().await?,
                }
            } else {
                connector.authenticate().await?
            };
            self.connector = Some(connector);
            self.connect_for_session(session, event_sender).await
        }
    }

    async fn challenge_code(&mut self, code: &str, event_sender: mpsc::Sender<TunnelEvent>) -> anyhow::Result<()> {
        if let Some(ref mut connector) = self.connector {
            match self.session.as_ref() {
                Some(session) => {
                    let new_session = connector.challenge_code(session.clone(), code).await?;
                    self.connect_for_session(new_session, event_sender).await
                }
                None => Err(anyhow!("No session")),
            }
        } else {
            Err(anyhow!("No connector to send the challenge code to!"))
        }
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        if let Some(ref mut connector) = self.connector {
            connector.delete_session().await;
            let _ = connector.terminate_tunnel().await;
        }
        self.reset();
        Ok(())
    }

    fn reset(&mut self) {
        self.session = None;
        self.connector = None;
        self.connection_status = ConnectionStatus::disconnected();
    }

    fn get_status(&self) -> &ConnectionStatus {
        &self.connection_status
    }
}
