use std::sync::Arc;

use anyhow::{Context, anyhow};
use futures::{FutureExt, SinkExt, StreamExt};
use i18n::tr;
use interprocess::local_socket::{GenericNamespaced, ToNsName, traits::tokio::Listener};
use tokio::sync::{Mutex, RwLock, mpsc};
use tracing::{debug, warn};

use crate::{
    model::{
        ConnectionStatus, SessionState, TunnelServiceRequest, TunnelServiceResponse, VpnSession, params::TunnelParams,
    },
    tunnel::{self, TunnelConnector, TunnelEvent},
};

pub const DEFAULT_NAME: &str = "snx-rs.sock";

const MAX_PACKET_SIZE: usize = 1_000_000;

#[derive(Default)]
struct CancelState {
    sender: Option<mpsc::Sender<()>>,
}

impl CancelState {
    async fn cancel(&mut self) {
        if let Some(sender) = self.sender.take() {
            debug!("Disconnecting current session");
            let _ = sender.send(()).await;
        }
    }
}

#[derive(Default)]
struct ConnectionState {
    connection_status: RwLock<ConnectionStatus>,
    session: Mutex<Option<Arc<VpnSession>>>,
    connector: Mutex<Option<Box<dyn TunnelConnector + Send>>>,
    cancel_state: Arc<Mutex<CancelState>>,
}

impl ConnectionState {
    async fn reset(&self) {
        *self.session.lock().await = None;
        *self.connector.lock().await = None;
        *self.connection_status.write().await = ConnectionStatus::Disconnected;
        self.cancel_state.lock().await.sender = None;
    }
}

pub struct CommandServer {
    name: String,
    connection_state: Arc<ConnectionState>,
}

impl Default for CommandServer {
    fn default() -> Self {
        Self::with_name(DEFAULT_NAME)
    }
}

impl CommandServer {
    pub fn with_name<S: AsRef<str>>(name: S) -> Self {
        Self {
            name: name.as_ref().to_owned(),
            connection_state: Arc::new(ConnectionState::default()),
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        debug!("Starting command server: {}", self.name);

        let listener = interprocess::local_socket::ListenerOptions::new()
            .name(self.name.to_ns_name::<GenericNamespaced>()?)
            .create_tokio()?;

        let (event_sender, mut event_receiver) = mpsc::channel::<TunnelEvent>(16);

        loop {
            tokio::select! {
                event = event_receiver.recv() => {
                    if let Some(event) = event {
                        let result = if let Some(connector) = self.connection_state.connector.lock().await.as_mut() {
                            connector.handle_tunnel_event(event.clone()).await
                        } else {
                            Ok(())
                        };

                        if result.is_err() {
                            self.connection_state.reset().await;
                        }

                        match event {
                            TunnelEvent::Connected(info) => {
                                *self.connection_state.connection_status.write().await = ConnectionStatus::connected(info);
                            }
                            TunnelEvent::Disconnected => {
                                self.connection_state.reset().await;
                            }
                            TunnelEvent::Rekeyed(address) => {
                                let mut guard = self.connection_state.connection_status.write().await;
                                if let ConnectionStatus::Connected(ref mut info) = *guard {
                                   info.ip_address = address;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                result = listener.accept() => {
                    let stream = result?;
                    let sender = event_sender.clone();
                    let state = self.connection_state.clone();

                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
                        rt.block_on(async move {
                            let mut handler = ServerHandler::new(state, sender).await;
                            let _ = handler.handle(stream).await;
                        });

                    });
                }
            }
        }
    }
}

struct ServerHandler {
    state: Arc<ConnectionState>,
    event_sender: mpsc::Sender<TunnelEvent>,
    cancel_sender: mpsc::Sender<()>,
    cancel_receiver: mpsc::Receiver<()>,
}

impl ServerHandler {
    async fn new(state: Arc<ConnectionState>, event_sender: mpsc::Sender<TunnelEvent>) -> Self {
        let (cancel_sender, cancel_receiver) = mpsc::channel(16);
        Self {
            state,
            event_sender,
            cancel_sender,
            cancel_receiver,
        }
    }

    async fn handle(&mut self, stream: interprocess::local_socket::tokio::Stream) -> anyhow::Result<()> {
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
                Ok(response) => response,
                Err(e) => {
                    warn!("Challenge code error: {:#}", e);
                    self.state.reset().await;
                    TunnelServiceResponse::Error(e.to_string())
                }
            },
        }
    }

    async fn is_connected(&self) -> bool {
        *self.state.connection_status.read().await != ConnectionStatus::Disconnected
    }

    async fn connect_for_session(&mut self, session: Arc<VpnSession>) -> anyhow::Result<TunnelServiceResponse> {
        *self.state.session.lock().await = Some(session.clone());
        if let SessionState::PendingChallenge(ref challenge) = session.state {
            debug!("Pending multi-factor, awaiting for it");
            *self.state.connection_status.write().await = ConnectionStatus::mfa(challenge.clone());
            return Ok(TunnelServiceResponse::Ok);
        }

        *self.state.connection_status.write().await = ConnectionStatus::Connecting;

        let (command_sender, command_receiver) = mpsc::channel(16);

        if let Some(connector) = self.state.connector.lock().await.as_mut() {
            let tunnel = connector.create_tunnel(session, command_sender).await?;

            let sender = self.event_sender.clone();
            tokio::spawn(async move {
                if let Err(e) = tunnel.run(command_receiver, sender).await {
                    warn!("Tunnel error: {}", e);
                }
            });

            Ok(TunnelServiceResponse::Ok)
        } else {
            Err(anyhow!(tr!("error-no-connector")))
        }
    }

    async fn connect(&mut self, params: Arc<TunnelParams>) -> anyhow::Result<TunnelServiceResponse> {
        if self.is_connected().await {
            Ok(TunnelServiceResponse::Error(
                "Another connection is already in progress!".to_owned(),
            ))
        } else {
            self.state.reset().await;
            *self.state.connection_status.write().await = ConnectionStatus::Connecting;
            self.state.cancel_state.lock().await.sender = Some(self.cancel_sender.clone());

            let mut connector = tunnel::new_tunnel_connector(params.clone()).await?;
            let fut = if params.ike_persist {
                debug!("Attempting to load IKE session");
                match connector.restore_session().await {
                    Ok(session) => futures::future::ready(Ok(session)).boxed(),
                    Err(_) => connector.authenticate(),
                }
            } else {
                connector.authenticate()
            };

            let session = tokio::select! {
                _ = self.cancel_receiver.recv() => anyhow::bail!(tr!("error-connection-cancelled")),
                res = fut => res?
            };

            *self.state.connector.lock().await = Some(connector);

            Ok(self.connect_for_session(session).await?)
        }
    }

    async fn challenge_code(&mut self, code: &str) -> anyhow::Result<TunnelServiceResponse> {
        let session = self.state.session.lock().await.clone().context("No session")?;

        let new_session = if let Some(connector) = self.state.connector.lock().await.as_mut() {
            tokio::select! {
                _ = self.cancel_receiver.recv() => anyhow::bail!(tr!("error-connection-cancelled")),
                res = connector.challenge_code(session, code) => res?
            }
        } else {
            anyhow::bail!(tr!("error-no-connector-for-challenge-code"))
        };

        self.connect_for_session(new_session).await
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        self.state.cancel_state.lock().await.cancel().await;

        if let Some(connector) = self.state.connector.lock().await.as_mut() {
            connector.delete_session().await;
            let _ = connector.terminate_tunnel(true).await;
        }
        self.state.reset().await;

        Ok(())
    }

    async fn get_status(&self) -> ConnectionStatus {
        self.state.connection_status.read().await.clone()
    }
}
