use std::{sync::Arc, time::Duration};

use anyhow::anyhow;
use futures::pin_mut;
use tokio::sync::mpsc;
use tracing::{debug, trace, warn};

use crate::{
    model::{
        params::TunnelParams, CccSession, ConnectionStatus, SessionState, TunnelServiceRequest, TunnelServiceResponse,
    },
    tunnel::{self, TunnelCommand, TunnelConnector, TunnelEvent},
};

pub const LISTEN_PORT: u16 = 7779;

const MAX_PACKET_SIZE: usize = 1_000_000;

pub struct CommandServer {
    port: u16,
    command_sender: Option<mpsc::Sender<TunnelCommand>>,
    connection_status: ConnectionStatus,
    session: Option<Arc<CccSession>>,
    connector: Option<Box<dyn TunnelConnector + Send>>,
}

impl CommandServer {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            command_sender: None,
            connection_status: ConnectionStatus::default(),
            session: None,
            connector: None,
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        debug!("Starting command server on port {}", self.port);

        let socket = Arc::new(tokio::net::UdpSocket::bind(("127.0.0.1", self.port)).await?);
        let (event_sender, mut event_receiver) = mpsc::channel::<TunnelEvent>(16);

        let mut interval = tokio::time::interval(Duration::from_secs(60));

        loop {
            let tick = interval.tick();
            pin_mut!(tick);

            let recv = async {
                let mut buf = vec![0u8; MAX_PACKET_SIZE];
                let (size, addr) = socket.recv_from(&mut buf).await?;
                Ok::<_, anyhow::Error>((buf[0..size].to_vec(), addr))
            };
            pin_mut!(recv);

            let event_fut = event_receiver.recv();
            pin_mut!(event_fut);

            tokio::select! {
                _ = tick => {
                    let _ = self.rekey_tunnel().await;
                }
                event = event_fut => {
                    if let Some(event) = event {
                        if let Some(ref mut connector) = self.connector {
                            let _ = connector.handle_tunnel_event(event.clone()).await;
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

    async fn rekey_tunnel(&mut self) -> anyhow::Result<()> {
        if self.is_connected() {
            if let Some(ref mut connector) = self.connector {
                if let Some(sender) = self.command_sender.clone() {
                    return connector.rekey_tunnel(sender).await;
                }
            }
        }
        Ok(())
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
                    Ok(_) => TunnelServiceResponse::Ok,
                    Err(e) => {
                        self.reset();
                        TunnelServiceResponse::Error(e.to_string())
                    }
                }
            }
            TunnelServiceRequest::Disconnect => {
                debug!("Handling disconnect command");

                match self.disconnect().await {
                    Ok(_) => TunnelServiceResponse::Ok,
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
                    Ok(_) => TunnelServiceResponse::Ok,
                    Err(e) => {
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
        session: Arc<CccSession>,
        event_sender: mpsc::Sender<TunnelEvent>,
    ) -> anyhow::Result<()> {
        let connector = if let Some(ref mut connector) = self.connector {
            connector
        } else {
            return Err(anyhow!("No tunnel connector!"));
        };

        if let SessionState::PendingChallenge(ref challenge) = session.state {
            debug!("Pending multi-factor, awaiting for it");
            self.session = Some(session.clone());
            self.connection_status = ConnectionStatus::mfa(challenge.clone());
            return Ok(());
        }

        let tunnel = connector.create_tunnel(session).await?;

        let (command_sender, command_receiver) = mpsc::channel(16);
        self.command_sender = Some(command_sender);

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
        if !self.is_connected() {
            self.reset();

            let mut connector = tunnel::new_tunnel_connector(params.clone()).await?;
            let session = connector.authenticate().await?;
            self.connector = Some(connector);
            self.connect_for_session(session, event_sender).await
        } else {
            Err(anyhow!("Tunnel is already connected!"))
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
        self.reset();

        if let Some(sender) = self.command_sender.take() {
            let _ = sender.send(TunnelCommand::Terminate).await;
            let mut num_waits = 0;
            while self.is_connected() && num_waits < 20 {
                tokio::time::sleep(Duration::from_millis(100)).await;
                num_waits += 1;
            }
            Ok(())
        } else {
            Err(anyhow!("Tunnel is already disconnected!"))
        }
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
