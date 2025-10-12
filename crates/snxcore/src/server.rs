use std::sync::Arc;

use futures::{FutureExt, SinkExt, StreamExt};
use i18n::tr;
use interprocess::local_socket::{GenericNamespaced, ToNsName, traits::tokio::Listener};
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::{
    model::{
        ConnectionStatus, SessionState, TunnelServiceRequest, TunnelServiceResponse, VpnSession, params::TunnelParams,
    },
    state::{ConnectionState, ConnectionStateActor},
    tunnel::{self, TunnelEvent},
};

pub const DEFAULT_NAME: &str = "snx-rs.sock";

const MAX_PACKET_SIZE: usize = 1_000_000;

pub struct CommandServer {
    name: String,
    connection_state_actor: Arc<ConnectionStateActor>,
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
            connection_state_actor: Arc::new(ConnectionStateActor::start(ConnectionState::default())),
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
                        let result = self.connection_state_actor.handle_tunnel_event(event.clone()).await;

                        if result.is_err() {
                            self.connection_state_actor.reset().await?;
                        }

                        match event {
                            TunnelEvent::Connected(info) => {
                                self.connection_state_actor.set_status(ConnectionStatus::connected(info)).await?;
                            }
                            TunnelEvent::Disconnected => {
                                self.connection_state_actor.reset().await?;
                            }
                            TunnelEvent::Rekeyed(address) => {
                                let mut status = self.connection_state_actor.get_status().await?;
                                if let ConnectionStatus::Connected(ref mut info) = status {
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
                    let state_actor = self.connection_state_actor.clone();

                    tokio::spawn(async move {
                        let mut handler = ServerHandler::new(state_actor, sender).await;
                        handler.handle(stream).await
                    });
                }
            }
        }
    }
}

struct ServerHandler {
    state_actor: Arc<ConnectionStateActor>,
    event_sender: mpsc::Sender<TunnelEvent>,
    cancel_sender: mpsc::Sender<()>,
    cancel_receiver: mpsc::Receiver<()>,
}

impl ServerHandler {
    async fn new(state_actor: Arc<ConnectionStateActor>, event_sender: mpsc::Sender<TunnelEvent>) -> Self {
        let (cancel_sender, cancel_receiver) = mpsc::channel(16);
        Self {
            state_actor,
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
                    let _ = self.state_actor.reset().await;
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
                    let _ = self.state_actor.reset().await;
                    TunnelServiceResponse::Error(e.to_string())
                }
            },
        }
    }

    async fn is_connected(&self) -> bool {
        self.state_actor
            .get_status()
            .await
            .map(|status| status != ConnectionStatus::Disconnected)
            .unwrap_or_default()
    }

    async fn connect_for_session(&mut self, session: Arc<VpnSession>) -> anyhow::Result<TunnelServiceResponse> {
        self.state_actor.set_session(session.clone()).await?;
        if let SessionState::PendingChallenge(ref challenge) = session.state {
            debug!("Pending multi-factor, awaiting for it");
            self.state_actor
                .set_status(ConnectionStatus::mfa(challenge.clone()))
                .await?;
            return Ok(TunnelServiceResponse::Ok);
        }

        self.state_actor.set_status(ConnectionStatus::Connecting).await?;

        let (command_sender, command_receiver) = mpsc::channel(16);

        let tunnel = self.state_actor.create_tunnel(session, command_sender).await?;

        let sender = self.event_sender.clone();
        tokio::spawn(async move {
            if let Err(e) = tunnel.run(command_receiver, sender).await {
                warn!("Tunnel error: {}", e);
            }
        });

        Ok(TunnelServiceResponse::Ok)
    }

    async fn connect(&mut self, params: Arc<TunnelParams>) -> anyhow::Result<TunnelServiceResponse> {
        if self.is_connected().await {
            Ok(TunnelServiceResponse::Error(
                "Another connection is already in progress!".to_owned(),
            ))
        } else {
            self.state_actor.reset().await?;
            self.state_actor.set_status(ConnectionStatus::Connecting).await?;
            self.state_actor.set_cancel_state(self.cancel_sender.clone()).await?;

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

            self.state_actor.set_connector(connector).await?;

            Ok(self.connect_for_session(session).await?)
        }
    }

    async fn challenge_code(&mut self, code: &str) -> anyhow::Result<TunnelServiceResponse> {
        let new_session = tokio::select! {
            _ = self.cancel_receiver.recv() => anyhow::bail!(tr!("error-connection-cancelled")),
            res = self.state_actor.challenge_code(code) => res?
        };

        self.connect_for_session(new_session).await
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        self.state_actor.disconnect().await?;
        self.state_actor.cancel_connection().await?;

        Ok(())
    }

    async fn get_status(&self) -> ConnectionStatus {
        self.state_actor
            .get_status()
            .await
            .unwrap_or(ConnectionStatus::Disconnected)
    }
}
