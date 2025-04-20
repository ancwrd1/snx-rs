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
    connection_status: ConnectionStatus,
    session: Option<Arc<VpnSession>>,
    connector: Option<Box<dyn TunnelConnector + Send>>,
}

impl ConnectionState {
    fn reset(&mut self) {
        self.session = None;
        self.connector = None;
        self.connection_status = ConnectionStatus::disconnected();
    }
}

struct CancelState {
    sender: Option<mpsc::Sender<()>>,
}

pub struct CommandServer {
    listen_path: PathBuf,
    connection_state: Arc<Mutex<ConnectionState>>,
}

impl Default for CommandServer {
    fn default() -> Self {
        Self {
            listen_path: DEFAULT_LISTEN_PATH.into(),
            connection_state: Arc::new(Mutex::new(ConnectionState::default())),
        }
    }
}

impl CommandServer {
    pub fn with_listen_path<P: AsRef<Path>>(listen_path: P) -> Self {
        Self {
            listen_path: listen_path.as_ref().to_owned(),
            connection_state: Arc::new(Mutex::new(ConnectionState::default())),
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
                        let result = if let Some(ref mut connector) = self.connection_state.lock().await.connector {
                            connector.handle_tunnel_event(event.clone()).await
                        } else {
                            Ok(())
                        };

                        if result.is_err() {
                            cancel_state.lock().await.sender = None;
                            self.connection_state.lock().await.reset();
                        }

                        match event {
                            TunnelEvent::Connected => {
                                self.connection_state.lock().await.connection_status = ConnectionStatus::connected();
                            }
                            TunnelEvent::Disconnected => {
                                cancel_state.lock().await.sender = None;
                                self.connection_state.lock().await.reset();
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
    state: Arc<Mutex<ConnectionState>>,
    cancel_state: Arc<Mutex<CancelState>>,
    event_sender: mpsc::Sender<TunnelEvent>,
    cancel_sender: mpsc::Sender<()>,
    cancel_receiver: mpsc::Receiver<()>,
}

impl ServerHandler {
    async fn new(
        state: Arc<Mutex<ConnectionState>>,
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
                Ok(()) => TunnelServiceResponse::Ok,
                Err(e) => {
                    self.state.lock().await.reset();
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
                    self.state.lock().await.reset();
                    TunnelServiceResponse::Error(e.to_string())
                }
            },
        }
    }

    async fn is_connected(&self) -> bool {
        self.state.lock().await.connection_status.connected_since.is_some()
    }

    async fn connect_for_session(&mut self, session: Arc<VpnSession>) -> anyhow::Result<()> {
        let mut state = self.state.lock().await;
        let Some(ref mut connector) = state.connector else {
            anyhow::bail!("No tunnel connector!");
        };

        if let SessionState::PendingChallenge(ref challenge) = session.state {
            debug!("Pending multi-factor, awaiting for it");
            state.session = Some(session.clone());
            state.connection_status = ConnectionStatus::mfa(challenge.clone());
            return Ok(());
        }

        let (command_sender, command_receiver) = mpsc::channel(16);

        let tunnel = connector.create_tunnel(session, command_sender).await?;

        let sender = self.event_sender.clone();
        tokio::spawn(async move {
            if let Err(e) = tunnel.run(command_receiver, sender).await {
                warn!("Tunnel error: {}", e);
            }
        });

        state.connection_status = ConnectionStatus::connected();

        Ok(())
    }

    async fn connect(&mut self, params: Arc<TunnelParams>) -> anyhow::Result<()> {
        if self.is_connected().await {
            Ok(())
        } else {
            self.state.lock().await.reset();

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

            self.cancel_state.lock().await.sender = Some(self.cancel_sender.clone());

            let session = tokio::select! {
                _ = self.cancel_receiver.recv() => {
                    self.cancel_state.lock().await.sender = None;
                    anyhow::bail!("Connection cancelled!");
                }
                res = fut => res?
            };

            self.state.lock().await.connector = Some(connector);
            self.connect_for_session(session).await
        }
    }

    async fn challenge_code(&mut self, code: &str) -> anyhow::Result<()> {
        let session = self
            .state
            .lock()
            .await
            .session
            .clone()
            .ok_or_else(|| anyhow!("No session"))?;

        let new_session = if let Some(ref mut connector) = self.state.lock().await.connector {
            tokio::select! {
                _ = self.cancel_receiver.recv() => {
                    self.cancel_state.lock().await.sender = None;
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
            debug!("Disconnecting pending session");
            let _ = sender.send(()).await;
        }

        if let Some(ref mut connector) = self.state.lock().await.connector {
            connector.delete_session().await;
            let _ = connector.terminate_tunnel(true).await;
        }
        self.state.lock().await.reset();
        Ok(())
    }

    async fn get_status(&self) -> ConnectionStatus {
        self.state.lock().await.connection_status.clone()
    }
}
