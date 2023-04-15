use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use anyhow::anyhow;
use futures::{
    channel::mpsc::{self, Receiver, Sender},
    future, SinkExt, StreamExt, TryStreamExt,
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_native_tls::native_tls::TlsConnector;
use tracing::{debug, trace, warn};
use tun::TunPacket;

use crate::{auth::SnxHttpAuthenticator, codec::SnxCodec, device::TunDevice, model::*, params::TunnelParams, util};

pub type SnxPacketSender = Sender<SnxPacket>;
pub type SnxPacketReceiver = Receiver<SnxPacket>;

const CHANNEL_SIZE: usize = 1024;
const REAUTH_LEEWAY: Duration = Duration::from_secs(60);

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

pub struct SnxClient(TunnelParams);

impl SnxClient {
    pub fn new(params: &TunnelParams) -> Self {
        Self(params.clone())
    }

    pub async fn authenticate(&self, session_id: Option<&str>) -> anyhow::Result<(String, String)> {
        debug!("Connecting to http endpoint: {}", self.0.server_name);
        let client = SnxHttpAuthenticator::new(&self.0);

        let server_response = client.authenticate(session_id).await?;

        let active_key = match (
            server_response.data.is_authenticated.as_str(),
            server_response.data.active_key,
        ) {
            ("true", Some(ref key)) => key.clone(),
            _ => {
                warn!("Authentication failed!");
                return Err(anyhow!("Authentication failed!"));
            }
        };

        let session_id = server_response.data.session_id.unwrap_or_default();
        let cookie = util::decode_from_hex(active_key.as_bytes())?;
        let cookie = String::from_utf8_lossy(&cookie).into_owned();

        debug!("Authentication OK, session id: {session_id}");

        Ok((session_id, cookie))
    }

    pub async fn create_tunnel<S, C>(&self, session_id: S, cookie: C) -> anyhow::Result<SnxTunnel>
    where
        S: AsRef<str>,
        C: AsRef<str>,
    {
        debug!("Creating TLS tunnel");

        let tcp = tokio::net::TcpStream::connect((self.0.server_name.as_str(), 443)).await?;

        let tls: tokio_native_tls::TlsConnector = TlsConnector::builder().build()?.into();
        let stream = tls.connect(self.0.server_name.as_str(), tcp).await?;

        let (sender, receiver) = make_channel(stream);

        debug!("Tunnel connected");

        Ok(SnxTunnel {
            params: self.0.clone(),
            cookie: cookie.as_ref().to_owned(),
            session_id: session_id.as_ref().to_owned(),
            auth_timeout: Duration::default(),
            keepalive: Duration::default(),
            ip_address: "0.0.0.0".to_string(),
            sender,
            receiver: Some(receiver),
            keepalive_counter: Arc::new(AtomicU64::default()),
        })
    }
}

pub struct SnxTunnel {
    params: TunnelParams,
    cookie: String,
    session_id: String,
    auth_timeout: Duration,
    keepalive: Duration,
    ip_address: String,
    sender: SnxPacketSender,
    receiver: Option<SnxPacketReceiver>,
    keepalive_counter: Arc<AtomicU64>,
}

impl SnxTunnel {
    fn new_hello_request(&self, keep_address: bool) -> ClientHello {
        ClientHello {
            client_version: "1".to_string(),
            protocol_version: "1".to_string(),
            protocol_minor_version: "1".to_string(),
            office_mode: OfficeMode {
                ipaddr: self.ip_address.clone(),
                keep_address: Some(keep_address.to_string()),
                dns_servers: None,
                dns_suffix: None,
            },
            optional: Some(OptionalRequest {
                client_type: "4".to_string(),
            }),
            cookie: self.cookie.clone(),
        }
    }

    pub async fn client_hello(&mut self) -> anyhow::Result<HelloReply> {
        let req = self.new_hello_request(false);
        self.send(req).await?;

        let receiver = self.receiver.as_mut().unwrap();

        let reply = receiver.next().await.ok_or_else(|| anyhow!("Channel closed!"))?;

        let reply = match reply {
            SnxPacket::Control(name, value) if name == HelloReply::NAME => {
                let result = serde_json::from_value::<HelloReply>(value)?;
                self.ip_address = result.office_mode.ipaddr.clone();
                self.auth_timeout = result
                    .timeouts
                    .authentication
                    .parse::<u64>()
                    .ok()
                    .map(Duration::from_secs)
                    .ok_or_else(|| anyhow!("Invalid auth timeout!"))?
                    - REAUTH_LEEWAY;
                self.keepalive = result
                    .timeouts
                    .keepalive
                    .parse::<u64>()
                    .ok()
                    .map(Duration::from_secs)
                    .ok_or_else(|| anyhow!("Invalid keepalive timeout!"))?;
                result
            }
            _ => return Err(anyhow!("Unexpected reply")),
        };

        Ok(reply)
    }

    async fn keepalive(&mut self) -> anyhow::Result<()> {
        if self.keepalive_counter.load(Ordering::SeqCst) >= 3 {
            let msg = "No response for keepalive packets, tunnel appears stuck";
            warn!(msg);
            return Err(anyhow!("{}", msg));
        }

        let req = KeepaliveRequest { id: "0".to_string() };

        self.keepalive_counter.fetch_add(1, Ordering::SeqCst);

        self.send(req).await?;

        Ok(())
    }

    async fn reauth(&mut self) -> anyhow::Result<()> {
        let client = SnxClient::new(&self.params);

        let (session_id, cookie) = client.authenticate(Some(&self.session_id)).await?;

        self.session_id = session_id;
        self.cookie = cookie;

        let req = self.new_hello_request(true);
        self.send(req).await?;

        Ok(())
    }

    async fn send<P>(&mut self, packet: P) -> anyhow::Result<()>
    where
        P: Into<SnxPacket>,
    {
        self.sender.send(packet.into()).await?;
        Ok(())
    }

    pub async fn run(mut self, tun: TunDevice) -> anyhow::Result<()> {
        debug!("Running tunnel for session {}", self.session_id);

        let dev_name = tun.name().to_owned();

        let (mut tun_sender, mut tun_receiver) = tun.into_inner().into_framed().split();

        let mut snx_receiver = self.receiver.take().unwrap();

        let dev_name2 = dev_name.clone();
        let keepalive_counter = self.keepalive_counter.clone();

        tokio::spawn(async move {
            while let Some(item) = snx_receiver.next().await {
                match item {
                    SnxPacket::Control(name, _) => {
                        debug!("Control packet received: {name}");
                        if name == KeepaliveRequest::NAME {
                            keepalive_counter.fetch_sub(1, Ordering::SeqCst);
                        }
                    }
                    SnxPacket::Data(data) => {
                        trace!("snx => {}: {}", data.len(), dev_name2);
                        keepalive_counter.store(0, Ordering::SeqCst);
                        let tun_packet = TunPacket::new(data);
                        tun_sender.send(tun_packet).await?;
                    }
                }
            }
            Ok::<_, anyhow::Error>(())
        });

        let mut now = Instant::now();

        loop {
            tokio::select! {
                _ = tokio::time::sleep(self.keepalive) => {
                    self.keepalive().await?;
                }

                result = tun_receiver.next() => {
                    if let Some(Ok(item)) = result {
                        let data = item.into_bytes().to_vec();
                        trace!("{} => snx: {}", dev_name, data.len());
                        self.send(data).await?;
                    } else {
                        break;
                    }
                }
            }

            if self.params.reauth && (Instant::now() - now) >= self.auth_timeout {
                self.reauth().await?;
                now = Instant::now();
            }
        }

        Ok(())
    }
}
