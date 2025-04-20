use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::anyhow;
use futures::{pin_mut, SinkExt, StreamExt};
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, trace, warn};

use crate::{
    model::{
        params::TunnelParams, ConnectionStatus, SessionState, TunnelServiceRequest, TunnelServiceResponse, VpnSession,
    },
    tunnel::{self, TunnelConnector, TunnelEvent},
};

pub const DEFAULT_LISTEN_PATH: &str = "/var/run/snx-rs.sock";

const MAX_PACKET_SIZE: usize = 1_000_000;

#[derive(Default)]
pub struct ServerState {
    connection_status: ConnectionStatus,
    session: Option<Arc<VpnSession>>,
    connector: Option<Box<dyn TunnelConnector + Send>>,
}

impl ServerState {
    fn reset(&mut self) {
        self.session = None;
        self.connector = None;
        self.connection_status = ConnectionStatus::disconnected();
    }
}

pub struct CommandServer {
    listen_path: PathBuf,
    state: Arc<Mutex<ServerState>>,
}

impl Default for CommandServer {
    fn default() -> Self {
        Self {
            listen_path: DEFAULT_LISTEN_PATH.into(),
            state: Arc::new(Mutex::new(ServerState::default())),
        }
    }
}

impl CommandServer {
    pub fn with_listen_path<P: AsRef<Path>>(listen_path: P) -> Self {
        Self {
            listen_path: listen_path.as_ref().to_owned(),
            state: Arc::new(Mutex::new(ServerState::default())),
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        debug!("Starting command server on {}", self.listen_path.display());

        let socket = tokio::net::UnixListener::bind(&self.listen_path)?;

        let (event_sender, mut event_receiver) = mpsc::channel::<TunnelEvent>(16);

        loop {
            let accept = socket.accept();
            pin_mut!(accept);

            let event_fut = event_receiver.recv();
            pin_mut!(event_fut);

            tokio::select! {
                event = event_fut => {
                    if let Some(event) = event {
                        let result = if let Some(ref mut connector) = self.state.lock().await.connector {
                            connector.handle_tunnel_event(event.clone()).await
                        } else {
                            Ok(())
                        };

                        if result.is_err() {
                            self.state.lock().await.reset();
                        }

                        match event {
                            TunnelEvent::Connected => {
                                self.state.lock().await.connection_status = ConnectionStatus::connected();
                            }
                            TunnelEvent::Disconnected => {
                                self.state.lock().await.reset();
                            }
                            _ => {}
                        }
                    }
                }
                result = accept => {
                    let (stream, _) = result?;
                    let sender = event_sender.clone();
                    let state = self.state.clone();
                    tokio::spawn(async move {
                        let mut handler = ServerHandler::new(state, sender.clone()).await;
                        handler.handle(stream).await.inspect_err(|e| warn!("Error handling connection: {}", e))
                    });
                }
            }
        }
    }
}

struct ServerHandler {
    state: Arc<Mutex<ServerState>>,
    event_sender: mpsc::Sender<TunnelEvent>,
}

impl ServerHandler {
    async fn new(state: Arc<Mutex<ServerState>>, event_sender: mpsc::Sender<TunnelEvent>) -> Self {
        Self { state, event_sender }
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
            TunnelServiceRequest::Connect(params) => {
                trace!("Handling connect command");
                match self.connect(Arc::new(params)).await {
                    Ok(()) => TunnelServiceResponse::Ok,
                    Err(e) => {
                        self.state.lock().await.reset();
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
            TunnelServiceRequest::GetStatus => TunnelServiceResponse::ConnectionStatus(self.get_status().await),
            TunnelServiceRequest::ChallengeCode(code, _) => {
                debug!("Handling challenge code command");
                match self.challenge_code(&code).await {
                    Ok(()) => TunnelServiceResponse::Ok,
                    Err(e) => {
                        warn!("Challenge code error: {:#}", e);
                        self.state.lock().await.reset();
                        TunnelServiceResponse::Error(e.to_string())
                    }
                }
            }
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
            let session = if params.ike_persist {
                debug!("Attempting to load IKE session");
                match connector.restore_session().await {
                    Ok(session) => session,
                    Err(_) => {
                        connector = tunnel::new_tunnel_connector(params.clone()).await?;
                        connector.authenticate().await?
                    }
                }
            } else {
                connector.authenticate().await?
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
            connector.challenge_code(session, code).await?
        } else {
            anyhow::bail!("No connector to send the challenge code to!")
        };

        self.connect_for_session(new_session).await
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
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
