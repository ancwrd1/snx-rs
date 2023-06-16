use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use tokio::sync::oneshot;
use tracing::{debug, warn};

use crate::{
    params::{TunnelParams, TunnelServiceRequest, TunnelServiceResponse},
    tunnel::SnxTunnelConnector,
};

const MAX_PACKET_SIZE: usize = 1_000_000;

pub struct CommandServer {
    port: u16,
    stopper: Option<oneshot::Sender<()>>,
    connected: Arc<AtomicBool>,
}

impl CommandServer {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            stopper: None,
            connected: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        debug!("Starting command server on port {}", self.port);

        let socket = Arc::new(tokio::net::UdpSocket::bind(("127.0.0.1", self.port)).await?);

        let mut buf = vec![0u8; MAX_PACKET_SIZE];
        loop {
            let (size, addr) = socket.recv_from(&mut buf).await?;
            let resp = self.handle(&buf[0..size]).await;
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
                match self.connect(params).await {
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
                match self.get_status().await {
                    Ok(b) => TunnelServiceResponse::ConnectionStatus(b),
                    Err(e) => TunnelServiceResponse::Error(e.to_string()),
                }
            }
        }
    }

    async fn connect(&mut self, params: TunnelParams) -> anyhow::Result<()> {
        if !self.connected.load(Ordering::SeqCst) {
            let connector = SnxTunnelConnector::new(&params);
            let session = connector.authenticate(None).await?;

            let tunnel = connector.create_tunnel(session).await?;

            let (tx, rx) = oneshot::channel();
            self.stopper = Some(tx);

            tokio::spawn(tunnel.run(rx, self.connected.clone()));
        } else {
            warn!("Tunnel is already connected");
        }
        Ok(())
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        if let Some(stopper) = self.stopper.take() {
            let _ = stopper.send(());
        } else {
            debug!("Tunnel is already disconnected");
        }
        Ok(())
    }

    async fn get_status(&self) -> anyhow::Result<bool> {
        Ok(self.connected.load(Ordering::SeqCst))
    }
}
