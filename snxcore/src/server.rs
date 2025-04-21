use std::{
    fs::Permissions,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::anyhow;
use futures::{FutureExt, SinkExt, StreamExt};
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, warn};

use crate::{
    model::{
        params::TunnelParams, ConnectionStatus, SessionState, TunnelServiceRequest, TunnelServiceResponse, VpnSession,
    },
    tunnel::{self, TunnelConnector, TunnelEvent},
};

pub const DEFAULT_LISTEN_PATH: &str = "/var/run/snx-rs.sock";

const MAX_PACKET_SIZE: usize = 1_000_000;

#[derive(Default)]
struct ConnectionState {
    connection_status: Mutex<ConnectionStatus>,
    session: Mutex<Option<Arc<VpnSession>>>,
    connector: Mutex<Option<Box<dyn TunnelConnector + Send>>>,
}

impl ConnectionState {
    async fn reset(&self) {
        *self.session.lock().await = None;
        *self.connector.lock().await = None;
        *self.connection_status.lock().await = ConnectionStatus::Disconnected
    }
}

struct CancelState {
    sender: Option<mpsc::Sender<()>>,
}

pub struct CommandServer {
    listen_path: PathBuf,
    connection_state: Arc<ConnectionState>,
}

impl Default for CommandServer {
    fn default() -> Self {
        Self::with_listen_path(DEFAULT_LISTEN_PATH)
    }
}

impl CommandServer {
    pub fn with_listen_path<P: AsRef<Path>>(listen_path: P) -> Self {
        Self {
            listen_path: listen_path.as_ref().to_owned(),
            connection_state: Arc::new(ConnectionState::default()),
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        debug!("Starting command server on {}", self.listen_path.display());
        let _ = std::fs::remove_file(&self.listen_path);

        let socket = tokio::net::UnixListener::bind(&self.listen_path)?;

        let _ = std::fs::set_permissions(&self.listen_path, Permissions::from_mode(0o777));

        let (event_sender, mut event_receiver) = mpsc::channel::<TunnelEvent>(16);

        let cancel_state = Arc::new(Mutex::new(CancelState { sender: None }));

        loop {
            let event_fut = event_receiver.recv();

            tokio::select! {
                event = event_fut => {
                    if let Some(event) = event {
                        let result = if let Some(connector) = self.connection_state.connector.lock().await.as_mut() {
                            connector.handle_tunnel_event(event.clone()).await
                        } else {
                            Ok(())
                        };

                        if result.is_err() {
                            cancel_state.lock().await.sender = None;
                            self.connection_state.reset().await;
                        }

                        match event {
                            TunnelEvent::Connected => {
                                *self.connection_state.connection_status.lock().await = ConnectionStatus::connected();
                            }
                            TunnelEvent::Disconnected => {
                                cancel_state.lock().await.sender = None;
                                self.connection_state.reset().await;
                            }
                            _ => {}
                        }
                    }
                }
                result = socket.accept() => {
                    let (stream, _) = result?;
                    let sender = event_sender.clone();
                    let state = self.connection_state.clone();

                    let cancel_state = cancel_state.clone();
                    tokio::spawn(async move {
                        let mut handler = ServerHandler::new(state, cancel_state, sender).await;
                        handler.handle(stream).await.inspect_err(|e| warn!("Error handling connection: {}", e))
                    });
                }
            }
        }
    }
}

struct ServerHandler {
    state: Arc<ConnectionState>,
    cancel_state: Arc<Mutex<CancelState>>,
    event_sender: mpsc::Sender<TunnelEvent>,
    cancel_sender: mpsc::Sender<()>,
    cancel_receiver: mpsc::Receiver<()>,
}

impl ServerHandler {
    async fn new(
        state: Arc<ConnectionState>,
        cancel_state: Arc<Mutex<CancelState>>,
        event_sender: mpsc::Sender<TunnelEvent>,
    ) -> Self {
        let (cancel_sender, cancel_receiver) = mpsc::channel(16);
        Self {
            state,
            cancel_state,
            event_sender,
            cancel_sender,
            cancel_receiver,
        }
    }

    async fn handle(&mut self, stream: tokio::net::UnixStream) -> anyhow::Result<()> {
        let mut codec = tokio_util::codec::LengthDelimitedCodec::builder()
            .max_frame_length(MAX_PACKET_SIZE)
            .new_framed(stream);

        while let Some(Ok(packet)) = codec.next().await {
            let reply = self.handle_packet(&packet).await;
            let reply = serde_json::to_vec(&reply)?;
            codec.send(reply.into()).await?;
        }

        Ok(())
    }

    async fn handle_packet(&mut self, packet: &[u8]) -> TunnelServiceResponse {
        let req = match serde_json::from_slice::<TunnelServiceRequest>(packet) {
            Ok(req) => req,
            Err(e) => {
                warn!("Command deserialization error: {:#}", e);
                return TunnelServiceResponse::Error(e.to_string());
            }
        };

        match req {
            TunnelServiceRequest::Connect(params) => match self.connect(Arc::new(params)).await {
                Ok(response) => response,
                Err(e) => {
                    self.state.reset().await;
                    TunnelServiceResponse::Error(e.to_string())
                }
            },
            TunnelServiceRequest::Disconnect => match self.disconnect().await {
                Ok(()) => TunnelServiceResponse::Ok,
                Err(e) => TunnelServiceResponse::Error(e.to_string()),
            },
            TunnelServiceRequest::GetStatus => TunnelServiceResponse::ConnectionStatus(self.get_status().await),
            TunnelServiceRequest::ChallengeCode(code, _) => match self.challenge_code(&code).await {
                Ok(()) => TunnelServiceResponse::Ok,
                Err(e) => {
                    warn!("Challenge code error: {:#}", e);
                    self.state.reset().await;
                    TunnelServiceResponse::Error(e.to_string())
                }
            },
        }
    }

    async fn is_connected(&self) -> bool {
        self.cancel_state.lock().await.sender.is_some()
            || *self.state.connection_status.lock().await != ConnectionStatus::Disconnected
    }

    async fn connect_for_session(&mut self, session: Arc<VpnSession>) -> anyhow::Result<()> {
        if let SessionState::PendingChallenge(ref challenge) = session.state {
            debug!("Pending multi-factor, awaiting for it");
            *self.state.session.lock().await = Some(session.clone());
            *self.state.connection_status.lock().await = ConnectionStatus::mfa(challenge.clone());
            return Ok(());
        }

        let (command_sender, command_receiver) = mpsc::channel(16);

        if let Some(connector) = self.state.connector.lock().await.as_mut() {
            let tunnel = connector.create_tunnel(session, command_sender).await?;

            let sender = self.event_sender.clone();
            tokio::spawn(async move {
                if let Err(e) = tunnel.run(command_receiver, sender).await {
                    warn!("Tunnel error: {}", e);
                }
            });

            *self.state.connection_status.lock().await = ConnectionStatus::connected();

            Ok(())
        } else {
            Err(anyhow!("No tunnel connector!"))
        }
    }

    async fn connect(&mut self, params: Arc<TunnelParams>) -> anyhow::Result<TunnelServiceResponse> {
        if self.is_connected().await {
            Ok(TunnelServiceResponse::Error(
                "Another connection is already in progress!".to_owned(),
            ))
        } else {
            self.state.reset().await;
            *self.state.connection_status.lock().await = ConnectionStatus::Connecting;
            self.cancel_state.lock().await.sender = Some(self.cancel_sender.clone());

            let mut connector = tunnel::new_tunnel_connector(params.clone()).await?;
            let fut = if params.ike_persist {
                debug!("Attempting to load IKE session");
                match connector.restore_session().await {
                    Ok(session) => futures::future::ready(Ok(session)).boxed(),
                    Err(_) => {
                        connector = tunnel::new_tunnel_connector(params.clone()).await?;
                        connector.authenticate()
                    }
                }
            } else {
                connector.authenticate()
            };

            let session = tokio::select! {
                _ = self.cancel_receiver.recv() => {
                    *self.state.connection_status.lock().await = ConnectionStatus::Disconnected;
                    self.cancel_state.lock().await.sender = None;
                    anyhow::bail!("Connection cancelled!");
                }
                res = fut => res?
            };

            *self.state.connector.lock().await = Some(connector);

            Ok(self
                .connect_for_session(session)
                .await
                .map(|_| TunnelServiceResponse::Ok)?)
        }
    }

    async fn challenge_code(&mut self, code: &str) -> anyhow::Result<()> {
        let session = self
            .state
            .session
            .lock()
            .await
            .clone()
            .ok_or_else(|| anyhow!("No session"))?;

        let new_session = if let Some(connector) = self.state.connector.lock().await.as_mut() {
            tokio::select! {
                _ = self.cancel_receiver.recv() => {
                    anyhow::bail!("Connection cancelled!");
                }
                res = connector.challenge_code(session, code) => res?
            }
        } else {
            anyhow::bail!("No connector to send the challenge code to!")
        };

        self.connect_for_session(new_session).await
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        if let Some(sender) = self.cancel_state.lock().await.sender.take() {
            debug!("Disconnecting current session");
            let _ = sender.send(()).await;
        }

        if let Some(connector) = self.state.connector.lock().await.as_mut() {
            connector.delete_session().await;
            let _ = connector.terminate_tunnel(true).await;
        }
        self.state.reset().await;

        Ok(())
    }

    async fn get_status(&self) -> ConnectionStatus {
        self.state.connection_status.lock().await.clone()
    }
}
