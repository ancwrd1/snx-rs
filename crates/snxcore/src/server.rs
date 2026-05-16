use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::{Context, anyhow};
use futures::{SinkExt, StreamExt};
use i18n::tr;
use interprocess::local_socket::{GenericNamespaced, ToNsName, traits::tokio::Listener};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, warn};

use crate::{
    model::{
        ConnectionStatus, LiveStats, SessionState, TunnelServiceRequest, TunnelServiceResponse, TunnelSession,
        params::TunnelParams,
    },
    platform::{NetworkInterface, Platform, PlatformAccess, StatsPoller},
    tunnel::{TunnelCommand, TunnelConnector, TunnelConnectorFactory, TunnelEvent, VpnTunnel},
};

pub const DEFAULT_NAME: &str = "snx-rs.sock";
const MAX_PACKET_SIZE: usize = 1_000_000;

enum ConnectorRequest {
    RestoreSession {
        reply: oneshot::Sender<anyhow::Result<Arc<TunnelSession>>>,
    },
    Authenticate {
        reply: oneshot::Sender<anyhow::Result<Arc<TunnelSession>>>,
        cancel: oneshot::Receiver<()>,
    },
    ChallengeCode {
        session: Arc<TunnelSession>,
        code: String,
        reply: oneshot::Sender<anyhow::Result<Arc<TunnelSession>>>,
        cancel: oneshot::Receiver<()>,
    },
    CreateTunnel {
        session: Arc<TunnelSession>,
        command_sender: mpsc::Sender<TunnelCommand>,
        reply: oneshot::Sender<anyhow::Result<Box<dyn VpnTunnel + Send>>>,
    },
    HandleEvent {
        event: TunnelEvent,
        reply: oneshot::Sender<anyhow::Result<()>>,
    },
    DeleteSession {
        reply: oneshot::Sender<anyhow::Result<()>>,
    },
    TerminateTunnel {
        signout: bool,
        reply: oneshot::Sender<anyhow::Result<()>>,
    },
}

#[derive(Clone)]
struct ConnectorHandle {
    tx: mpsc::Sender<ConnectorRequest>,
}

impl ConnectorHandle {
    async fn send_recv<T>(
        &self,
        make_req: impl FnOnce(oneshot::Sender<anyhow::Result<T>>) -> ConnectorRequest,
    ) -> anyhow::Result<T> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(make_req(reply))
            .await
            .map_err(|_| anyhow!("Connector actor stopped"))?;
        rx.await?
    }

    async fn restore_session(&self) -> anyhow::Result<Arc<TunnelSession>> {
        self.send_recv(|reply| ConnectorRequest::RestoreSession { reply }).await
    }

    async fn authenticate(&self, cancel: oneshot::Receiver<()>) -> anyhow::Result<Arc<TunnelSession>> {
        self.send_recv(|reply| ConnectorRequest::Authenticate { reply, cancel })
            .await
    }

    async fn challenge_code(
        &self,
        session: Arc<TunnelSession>,
        code: String,
        cancel: oneshot::Receiver<()>,
    ) -> anyhow::Result<Arc<TunnelSession>> {
        self.send_recv(|reply| ConnectorRequest::ChallengeCode {
            session,
            code,
            reply,
            cancel,
        })
        .await
    }

    async fn create_tunnel(
        &self,
        session: Arc<TunnelSession>,
        command_sender: mpsc::Sender<TunnelCommand>,
    ) -> anyhow::Result<Box<dyn VpnTunnel + Send>> {
        self.send_recv(|reply| ConnectorRequest::CreateTunnel {
            session,
            command_sender,
            reply,
        })
        .await
    }

    async fn handle_event(&self, event: TunnelEvent) -> anyhow::Result<()> {
        self.send_recv(|reply| ConnectorRequest::HandleEvent { event, reply })
            .await
    }

    async fn delete_session(&self) -> anyhow::Result<()> {
        self.send_recv(|reply| ConnectorRequest::DeleteSession { reply }).await
    }

    async fn terminate_tunnel(&self, signout: bool) -> anyhow::Result<()> {
        self.send_recv(|reply| ConnectorRequest::TerminateTunnel { signout, reply })
            .await
    }
}

fn spawn_connector_actor(mut connector: Box<dyn TunnelConnector + Send>) -> ConnectorHandle {
    let (tx, mut rx) = mpsc::channel::<ConnectorRequest>(16);
    tokio::spawn(async move {
        while let Some(req) = rx.recv().await {
            match req {
                ConnectorRequest::RestoreSession { reply } => {
                    let _ = reply.send(connector.restore_session().await);
                }
                ConnectorRequest::Authenticate { reply, cancel } => {
                    let res = tokio::select! {
                        _ = cancel => Err(anyhow!(tr!("error-connection-cancelled"))),
                        res = connector.authenticate() => res,
                    };
                    let _ = reply.send(res);
                }
                ConnectorRequest::ChallengeCode {
                    session,
                    code,
                    reply,
                    cancel,
                } => {
                    let res = tokio::select! {
                        _ = cancel => Err(anyhow!(tr!("error-connection-cancelled"))),
                        res = connector.challenge_code(session, &code) => res,
                    };
                    let _ = reply.send(res);
                }
                ConnectorRequest::CreateTunnel {
                    session,
                    command_sender,
                    reply,
                } => {
                    let _ = reply.send(connector.create_tunnel(session, command_sender).await);
                }
                ConnectorRequest::HandleEvent { event, reply } => {
                    let _ = reply.send(connector.handle_tunnel_event(event).await);
                }
                ConnectorRequest::DeleteSession { reply } => {
                    let _ = reply.send(connector.delete_session().await);
                }
                ConnectorRequest::TerminateTunnel { signout, reply } => {
                    let _ = reply.send(connector.terminate_tunnel(signout).await);
                }
            }
        }
    });
    ConnectorHandle { tx }
}

#[derive(Default)]
struct ConnectionStateInner {
    status: ConnectionStatus,
    session: Option<Arc<TunnelSession>>,
    connector: Option<ConnectorHandle>,
    cancel: Option<oneshot::Sender<()>>,
    stats_poller: Option<Arc<dyn StatsPoller + Send + Sync>>,
}

#[derive(Default)]
struct ConnectionState {
    inner: Mutex<ConnectionStateInner>,
}

impl ConnectionState {
    fn lock(&self) -> MutexGuard<'_, ConnectionStateInner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn reset(&self) {
        let mut g = self.lock();
        g.status = ConnectionStatus::Disconnected;
        g.session = None;
        g.connector = None;
        g.cancel = None;
        g.stats_poller = None;
    }

    fn connector(&self) -> Option<ConnectorHandle> {
        self.lock().connector.clone()
    }

    fn fire_cancel(&self) {
        if let Some(tx) = self.lock().cancel.take() {
            let _ = tx.send(());
        }
    }
}

pub struct CommandServer<F> {
    name: String,
    connection_state: Arc<ConnectionState>,
    connector_factory: F,
}

impl<F: TunnelConnectorFactory + Send + Sync + 'static> CommandServer<F> {
    pub fn new(connector_factory: F) -> Self {
        Self::with_name(DEFAULT_NAME, connector_factory)
    }

    pub fn with_name<S: AsRef<str>>(name: S, connector_factory: F) -> Self {
        Self {
            name: name.as_ref().to_owned(),
            connection_state: Arc::new(ConnectionState::default()),
            connector_factory,
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        debug!("Starting command server: {}", self.name);

        let options =
            interprocess::local_socket::ListenerOptions::new().name(self.name.to_ns_name::<GenericNamespaced>()?);

        #[cfg(target_os = "windows")]
        let options = {
            use interprocess::os::windows::{
                local_socket::ListenerOptionsExt, security_descriptor::SecurityDescriptor,
            };
            let sddl = widestring::U16CString::from_str("D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGWGX;;;IU)")
                .map_err(|e| anyhow!("invalid SDDL string: {e}"))?;
            let sd = SecurityDescriptor::deserialize(&sddl)
                .map_err(|e| anyhow!("failed to build pipe security descriptor: {e}"))?;
            options.security_descriptor(sd)
        };

        let listener = options.create_tokio()?;

        let (event_sender, mut event_receiver) = mpsc::channel::<TunnelEvent>(16);

        let state = self.connection_state.clone();

        tokio::spawn(async move {
            while let Some(event) = event_receiver.recv().await {
                Self::handle_tunnel_event(event, state.clone()).await;
            }
        });

        while let Ok(stream) = listener.accept().await {
            let sender = event_sender.clone();
            let state = self.connection_state.clone();

            let factory = self.connector_factory.clone();

            tokio::spawn(async move {
                let mut handler = ServerHandler::new(state, sender, factory);
                handler.handle(stream).await
            });
        }

        Ok(())
    }

    async fn handle_tunnel_event(event: TunnelEvent, state: Arc<ConnectionState>) {
        if let Some(handle) = state.connector()
            && let Err(e) = handle.handle_event(event.clone()).await
        {
            warn!("Tunnel error: {}", e);
            state.reset();
            return;
        }

        match event {
            TunnelEvent::Connected(info) => {
                let poller = Platform::get()
                    .new_network_interface()
                    .new_stats_poller(&info.interface_name)
                    .await;
                let mut g = state.lock();
                if let Ok(p) = poller {
                    g.stats_poller = Some(Arc::new(p));
                }
                g.status = ConnectionStatus::Connected(info);
            }
            TunnelEvent::Disconnected => state.reset(),
            TunnelEvent::Rekeyed(address) => {
                let mut g = state.lock();
                if let ConnectionStatus::Connected(ref mut info) = g.status {
                    info.ip_address = address;
                }
            }
            TunnelEvent::Rtt(rtt) => {
                let mut g = state.lock();
                if let ConnectionStatus::Connected(ref mut info) = g.status {
                    info.live.last_rtt_ms = Some(rtt.as_millis() as u64);
                }
            }
            _ => {}
        }
    }
}

struct ServerHandler<F> {
    state: Arc<ConnectionState>,
    event_sender: mpsc::Sender<TunnelEvent>,
    connector_factory: F,
}

impl<F: TunnelConnectorFactory + Send + Sync + 'static> ServerHandler<F> {
    fn new(state: Arc<ConnectionState>, event_sender: mpsc::Sender<TunnelEvent>, factory: F) -> Self {
        Self {
            state,
            event_sender,
            connector_factory: factory,
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
                    self.state.reset();
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
                    self.state.reset();
                    TunnelServiceResponse::Error(e.to_string())
                }
            },
        }
    }

    fn is_connected(&self) -> bool {
        self.state.lock().status != ConnectionStatus::Disconnected
    }

    async fn connect_for_session(
        &mut self,
        session: Arc<TunnelSession>,
        handle: ConnectorHandle,
    ) -> anyhow::Result<TunnelServiceResponse> {
        {
            let mut g = self.state.lock();
            g.session = Some(session.clone());
            if let SessionState::PendingChallenge(ref challenge) = session.state {
                debug!("Pending multi-factor, awaiting for it");
                g.status = ConnectionStatus::mfa(challenge.clone());
                return Ok(TunnelServiceResponse::Ok);
            }
            g.status = ConnectionStatus::Connecting;
        }

        let (command_sender, command_receiver) = mpsc::channel(16);
        let mut tunnel = handle.create_tunnel(session, command_sender).await?;

        let sender = self.event_sender.clone();
        tokio::spawn(async move {
            if let Err(e) = tunnel.run(command_receiver, sender).await {
                warn!("Tunnel error: {}", e);
            }
        });

        Ok(TunnelServiceResponse::Ok)
    }

    async fn connect(&mut self, params: Arc<TunnelParams>) -> anyhow::Result<TunnelServiceResponse> {
        if self.is_connected() {
            return Ok(TunnelServiceResponse::Error(
                "Another connection is already in progress!".to_owned(),
            ));
        }

        self.state.reset();

        let connector = self.connector_factory.new_tunnel_connector(params.clone()).await?;
        let handle = spawn_connector_actor(connector);

        let (cancel_tx, cancel_rx) = oneshot::channel();
        {
            let mut g = self.state.lock();
            g.status = ConnectionStatus::Connecting;
            g.cancel = Some(cancel_tx);
            g.connector = Some(handle.clone());
        }

        let session = if params.ike_persist {
            debug!("Attempting to load IKE session");
            match handle.restore_session().await {
                Ok(s) => s,
                Err(_) => handle.authenticate(cancel_rx).await?,
            }
        } else {
            handle.authenticate(cancel_rx).await?
        };

        self.connect_for_session(session, handle).await
    }

    async fn challenge_code(&mut self, code: &str) -> anyhow::Result<TunnelServiceResponse> {
        let (handle, session) = {
            let g = self.state.lock();
            (
                g.connector.clone().context("No connector")?,
                g.session.clone().context("No session")?,
            )
        };

        let (cancel_tx, cancel_rx) = oneshot::channel();
        self.state.lock().cancel = Some(cancel_tx);

        let new_session = handle.challenge_code(session, code.to_owned(), cancel_rx).await?;

        self.connect_for_session(new_session, handle).await
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        self.state.fire_cancel();

        if let Some(handle) = self.state.connector() {
            debug!("Disconnecting current session");
            let _ = handle.delete_session().await;
            let _ = handle.terminate_tunnel(true).await;
        }
        self.state.reset();

        Ok(())
    }

    async fn get_status(&self) -> ConnectionStatus {
        let (mut status, poller) = {
            let g = self.state.lock();
            (g.status.clone(), g.stats_poller.clone())
        };
        if let ConnectionStatus::Connected(ref mut info) = status
            && let Some(p) = poller
            && let Ok(live) = p.poll().await
        {
            info.live = LiveStats {
                last_rtt_ms: info.live.last_rtt_ms,
                ..live
            }
        }
        status
    }
}
