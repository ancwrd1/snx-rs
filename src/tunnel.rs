use std::{
    sync::{atomic::AtomicU64, Arc},
    time::Duration,
};

use anyhow::{anyhow, Error};
use futures::{
    channel::mpsc::{self, Receiver, Sender},
    future, SinkExt, StreamExt, TryStreamExt,
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_native_tls::native_tls::TlsConnector;
use tracing::{debug, warn};

use crate::{
    codec::SnxCodec,
    http::SnxHttpClient,
    ipsec::IpsecConfigurator,
    model::*,
    params::{TunnelParams, TunnelType},
    tunnel::{ipsec::SnxIpsecTunnel, ssl::SnxSslTunnel},
    util,
};

mod ipsec;
mod ssl;

pub type SnxPacketSender = Sender<SnxPacket>;
pub type SnxPacketReceiver = Receiver<SnxPacket>;

const CHANNEL_SIZE: usize = 1024;

#[async_trait::async_trait]
pub trait SnxTunnel {
    async fn run(mut self: Box<Self>) -> anyhow::Result<()>;
}

fn make_channel<S>(stream: S) -> (SnxPacketSender, SnxPacketReceiver)
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let framed = tokio_util::codec::Framed::new(stream, SnxCodec);

    let (tx_in, rx_in) = mpsc::channel(CHANNEL_SIZE);
    let (tx_out, rx_out) = mpsc::channel(CHANNEL_SIZE);

    let channel = async move {
        let (mut sink, stream) = framed.split();

        let mut rx = rx_out.map(Ok::<_, anyhow::Error>);
        let to_wire = sink.send_all(&mut rx);

        let mut tx = tx_in.sink_map_err(anyhow::Error::from);
        let from_wire = stream.map_err(Into::into).forward(&mut tx);

        future::select(to_wire, from_wire).await;
    };

    tokio::spawn(channel);

    (tx_out, rx_in)
}

pub struct SnxTunnelConnector(TunnelParams);

impl SnxTunnelConnector {
    pub fn new(params: &TunnelParams) -> Self {
        Self(params.clone())
    }

    pub async fn authenticate(&self, session_id: Option<&str>) -> anyhow::Result<SnxSession> {
        debug!("Connecting to http endpoint: {}", self.0.server_name);
        let client = SnxHttpClient::new(&self.0);

        let data = client.authenticate(session_id).await?;

        let active_key = match (data.is_authenticated, data.active_key) {
            (true, Some(ref key)) => key.clone(),
            _ => {
                warn!("Authentication failed!");
                return Err(anyhow!("Authentication failed!"));
            }
        };

        let session_id = data.session_id.unwrap_or_default();
        let cookie = util::decode_from_hex(active_key.as_bytes())?;
        let cookie = String::from_utf8_lossy(&cookie).into_owned();

        debug!("Authentication OK, session id: {session_id}");

        Ok(SnxSession { session_id, cookie })
    }

    pub async fn create_tunnel(&self, session: SnxSession) -> anyhow::Result<Box<dyn SnxTunnel>> {
        match self.0.tunnel_type {
            TunnelType::Ssl => self.create_ssl_tunnel(session).await,
            TunnelType::Ipsec => self.create_ipsec_tunnel(session).await,
        }
    }

    async fn create_ssl_tunnel(&self, session: SnxSession) -> Result<Box<dyn SnxTunnel>, Error> {
        debug!("Creating SSL tunnel");

        let tcp = tokio::net::TcpStream::connect((self.0.server_name.as_str(), 443)).await?;

        let tls: tokio_native_tls::TlsConnector = TlsConnector::builder().build()?.into();
        let stream = tls.connect(self.0.server_name.as_str(), tcp).await?;

        let (sender, receiver) = make_channel(stream);

        debug!("Tunnel connected");

        Ok(Box::new(SnxSslTunnel {
            params: self.0.clone(),
            session,
            auth_timeout: Duration::default(),
            keepalive: Duration::default(),
            ip_address: "0.0.0.0".to_string(),
            sender,
            receiver: Some(receiver),
            keepalive_counter: Arc::new(AtomicU64::default()),
        }))
    }

    async fn create_ipsec_tunnel(&self, session: SnxSession) -> anyhow::Result<Box<dyn SnxTunnel>> {
        let client = SnxHttpClient::new(&self.0);
        let client_settings = client.get_client_settings(&session.session_id).await?;
        debug!("Client settings: {:?}", client_settings);

        let params = client.get_ipsec_tunnel_params(&session.session_id).await?;
        let mut configurator = IpsecConfigurator::new(self.0.clone(), params, client_settings);
        configurator.configure().await?;

        Ok(Box::new(SnxIpsecTunnel(configurator)))
    }
}
