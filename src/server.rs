use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::anyhow;
use tokio::sync::oneshot;
use tracing::{debug, trace, warn};

use crate::{
    model::{
        params::TunnelParams, CccSession, ConnectionStatus, SessionState, TunnelServiceRequest, TunnelServiceResponse,
    },
    tunnel::{self, TunnelConnector},
};

pub const LISTEN_PORT: u16 = 7779;

const MAX_PACKET_SIZE: usize = 1_000_000;

pub struct CommandServer {
    port: u16,
    stopper: Option<oneshot::Sender<()>>,
    connected: Arc<Mutex<ConnectionStatus>>,
    session: Option<Arc<CccSession>>,
    connector: Option<Box<dyn TunnelConnector + Send>>,
}

impl CommandServer {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            stopper: None,
            connected: Arc::new(Mutex::new(ConnectionStatus::default())),
            session: None,
            connector: None,
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        debug!("Starting command server on port {}", self.port);

        let socket = Arc::new(tokio::net::UdpSocket::bind(("127.0.0.1", self.port)).await?);

        let mut buf = vec![0u8; MAX_PACKET_SIZE];
        loop {
            let (size, addr) = socket.recv_from(&mut buf).await?;
            let resp = self.handle(&buf[0..size]).await;
            trace!("Response: {:?}", resp);
            let json = serde_json::to_vec(&resp)?;
            let _ = socket.send_to(&json, addr).await;
        }
    }

    async fn handle(&mut self, packet: &[u8]) -> TunnelServiceResponse {
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
                match self.connect(Arc::new(params)).await {
                    Ok(_) => TunnelServiceResponse::Ok,
                    Err(e) => {
                        self.session = None;
                        self.connector = None;
                        *self.connected.lock().unwrap() = ConnectionStatus::default();
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
                TunnelServiceResponse::ConnectionStatus(self.get_status())
            }
            TunnelServiceRequest::ChallengeCode(code, _) => {
                debug!("Handling challenge code command");
                match self.challenge_code(&code).await {
                    Ok(_) => TunnelServiceResponse::Ok,
                    Err(e) => {
                        self.session = None;
                        self.connector = None;
                        *self.connected.lock().unwrap() = ConnectionStatus::default();
                        TunnelServiceResponse::Error(e.to_string())
                    }
                }
            }
        }
    }

    fn is_connected(&self) -> bool {
        self.connected.lock().unwrap().connected_since.is_some()
    }

    async fn connect_for_session(&mut self, session: Arc<CccSession>) -> anyhow::Result<()> {
        let connector = if let Some(ref mut connector) = self.connector {
            connector
        } else {
            return Err(anyhow!("No tunnel connector!"));
        };

        if let SessionState::PendingChallenge(ref challenge) = session.state {
            debug!("Pending multi-factor, awaiting for it");
            self.session = Some(session.clone());
            *self.connected.lock().unwrap() = ConnectionStatus {
                mfa: Some(challenge.clone()),
                ..Default::default()
            };
            return Ok(());
        }

        let tunnel = connector.create_tunnel(session).await?;

        let (tx, rx) = oneshot::channel();
        let (status_sender, status_receiver) = oneshot::channel();
        self.stopper = Some(tx);

        let connected = self.connected.clone();

        tokio::spawn(async move {
            if let Err(e) = tunnel.run(rx, connected.clone(), status_sender).await {
                warn!("Tunnel error: {}", e);
            }
            *connected.lock().unwrap() = ConnectionStatus::default();
        });
        status_receiver.await?;
        Ok(())
    }

    async fn connect(&mut self, params: Arc<TunnelParams>) -> anyhow::Result<()> {
        if !self.is_connected() {
            self.session = None;
            self.connector = None;

            let mut connector = tunnel::new_tunnel_connector(params.clone()).await?;
            let session = connector.authenticate().await?;
            self.connector = Some(connector);
            self.connect_for_session(session).await
        } else {
            Err(anyhow!("Tunnel is already connected!"))
        }
    }

    async fn challenge_code(&mut self, code: &str) -> anyhow::Result<()> {
        if let Some(ref mut connector) = self.connector {
            match self.session.as_ref() {
                Some(session) => {
                    let new_session = connector.challenge_code(session.clone(), code).await?;
                    self.connect_for_session(new_session).await
                }
                None => Err(anyhow!("No session")),
            }
        } else {
            Err(anyhow!("No connector to send the challenge code to!"))
        }
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        self.session = None;
        self.connector = None;

        if let Some(stopper) = self.stopper.take() {
            let _ = stopper.send(());
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

    fn get_status(&self) -> ConnectionStatus {
        self.connected.lock().unwrap().clone()
    }
}
