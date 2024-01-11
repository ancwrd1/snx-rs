use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::anyhow;
use tokio::sync::oneshot;
use tracing::{debug, warn};

use crate::model::{CheckpointSession, SessionState};
use crate::{
    controller::{ServiceController, ServiceCommand},
    model::{params::TunnelParams, ConnectionStatus, TunnelServiceRequest, TunnelServiceResponse},
    tunnel::TunnelConnector,
};

pub const LISTEN_PORT: u16 = 7779;

const MAX_PACKET_SIZE: usize = 1_000_000;

pub struct CommandServer {
    port: u16,
    stopper: Option<oneshot::Sender<()>>,
    connected: Arc<Mutex<ConnectionStatus>>,
    session_id: Option<String>,
}

impl CommandServer {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            stopper: None,
            connected: Arc::new(Mutex::new(ConnectionStatus::default())),
            session_id: None,
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        debug!("Starting command server on port {}", self.port);

        let socket = Arc::new(tokio::net::UdpSocket::bind(("127.0.0.1", self.port)).await?);

        let mut buf = vec![0u8; MAX_PACKET_SIZE];
        loop {
            let (size, addr) = socket.recv_from(&mut buf).await?;
            let resp = self.handle(&buf[0..size]).await;
            debug!("Response: {:?}", resp);
            let json = serde_json::to_vec(&resp)?;
            let _ = socket.send_to(&json, addr).await;
        }
    }

    async fn handle(&mut self, packet: &[u8]) -> TunnelServiceResponse {
        debug!("Command received");
        let req = match serde_json::from_slice::<TunnelServiceRequest>(packet) {
            Ok(req) => req,
            Err(e) => {
                warn!("{}", e);
                return TunnelServiceResponse::Error(e.to_string());
            }
        };

        match req {
            TunnelServiceRequest::Connect(params) => {
                debug!("Handling connect command");
                match self.connect(Arc::new(params)).await {
                    Ok(_) => TunnelServiceResponse::Ok,
                    Err(e) => TunnelServiceResponse::Error(e.to_string()),
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
                debug!("Handling get status command");
                TunnelServiceResponse::ConnectionStatus(self.get_status())
            }
            TunnelServiceRequest::ChallengeCode(code, params) => {
                debug!("Handling challenge code command");
                match self.challenge_code(&code, Arc::new(params)).await {
                    Ok(_) => TunnelServiceResponse::Ok,
                    Err(e) => TunnelServiceResponse::Error(e.to_string()),
                }
            }
        }
    }

    fn is_connected(&self) -> bool {
        self.connected.lock().unwrap().connected_since.is_some()
    }

    async fn connect_for_session(
        &mut self,
        params: Arc<TunnelParams>,
        session: Arc<CheckpointSession>,
    ) -> anyhow::Result<()> {
        if let SessionState::Pending(ref prompt) = session.state {
            debug!("Pending multi-factor, awaiting for it");
            self.session_id = Some(session.session_id.clone());
            *self.connected.lock().unwrap() = ConnectionStatus {
                mfa_pending: true,
                mfa_prompt: prompt.clone(),
                ..Default::default()
            };
            return Ok(());
        }

        let connector = TunnelConnector::new(params.clone());
        let tunnel = connector.create_tunnel(session).await?;

        let (tx, rx) = oneshot::channel();
        self.stopper = Some(tx);

        let connected = self.connected.clone();

        tokio::spawn(async move {
            if let Err(e) = tunnel.run(rx, connected.clone()).await {
                warn!("Tunnel error: {}", e);
                *connected.lock().unwrap() = ConnectionStatus::default();
                if params.reauthenticate {
                    let controller = ServiceController::with_params((*params).clone());
                    if let Err(e) = controller.command(ServiceCommand::Connect).await {
                        warn!("{}", e);
                    }
                }
            } else {
                *connected.lock().unwrap() = ConnectionStatus::default();
            }
        });
        Ok(())
    }

    async fn connect(&mut self, params: Arc<TunnelParams>) -> anyhow::Result<()> {
        if !self.is_connected() {
            self.session_id = None;

            let connector = TunnelConnector::new(params.clone());
            let session = Arc::new(connector.authenticate(None).await?);
            self.connect_for_session(params, session).await
        } else {
            Err(anyhow!("Tunnel is already connected!"))
        }
    }

    async fn challenge_code(&mut self, code: &str, params: Arc<TunnelParams>) -> anyhow::Result<()> {
        let connector = TunnelConnector::new(params.clone());
        let session = Arc::new(
            connector
                .challenge_code(self.session_id.as_deref().unwrap_or_default(), &code)
                .await?,
        );
        self.connect_for_session(params, session).await
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        self.session_id = None;

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
