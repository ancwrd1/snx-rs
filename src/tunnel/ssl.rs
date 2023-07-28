use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use anyhow::anyhow;
use futures::{
    channel::mpsc::{self, Receiver, Sender},
    SinkExt, StreamExt, TryStreamExt,
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::oneshot,
};
use tokio_native_tls::native_tls::{Certificate, TlsConnector};
use tracing::{debug, trace, warn};
use tun::TunPacket;

use crate::{
    codec::SnxCodec,
    model::{params::TunnelParams, snx::*, *},
    tun::TunDevice,
    tunnel::{SnxTunnel, SnxTunnelConnector},
};

const REAUTH_LEEWAY: Duration = Duration::from_secs(60);
const MAX_KEEP_ALIVE_ATTEMPTS: u64 = 3;
const SEND_TIMEOUT: Duration = Duration::from_secs(120);
const CHANNEL_SIZE: usize = 1024;

type SnxPacketSender = Sender<SnxPacket>;
type SnxPacketReceiver = Receiver<SnxPacket>;

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

        futures::future::select(to_wire, from_wire).await;
    };

    tokio::spawn(channel);

    (tx_out, rx_in)
}

pub(crate) struct SnxSslTunnel {
    params: Arc<TunnelParams>,
    session: Arc<SnxSession>,
    auth_timeout: Duration,
    keepalive: Duration,
    ip_address: String,
    sender: SnxPacketSender,
    receiver: Option<SnxPacketReceiver>,
    keepalive_counter: Arc<AtomicU64>,
}

impl SnxSslTunnel {
    pub(crate) async fn create(params: Arc<TunnelParams>, session: Arc<SnxSession>) -> anyhow::Result<Self> {
        let tcp = tokio::net::TcpStream::connect((params.server_name.as_str(), 443)).await?;

        let mut builder = TlsConnector::builder();

        if let Some(ref ca_cert) = params.ca_cert {
            let data = tokio::fs::read(ca_cert).await?;
            let cert = Certificate::from_pem(&data).or_else(|_| Certificate::from_der(&data))?;
            builder.add_root_certificate(cert);
        }

        if params.no_cert_check {
            builder.danger_accept_invalid_hostnames(true);
        }

        let tls: tokio_native_tls::TlsConnector = builder.build()?.into();
        let stream = tls.connect(params.server_name.as_str(), tcp).await?;

        let (sender, receiver) = make_channel(stream);

        debug!("Tunnel connected");

        Ok(Self {
            params,
            session,
            auth_timeout: Duration::default(),
            keepalive: Duration::default(),
            ip_address: "0.0.0.0".to_string(),
            sender,
            receiver: Some(receiver),
            keepalive_counter: Arc::new(AtomicU64::default()),
        })
    }

    fn new_hello_request(&self, keep_address: bool) -> ClientHello {
        ClientHello {
            client_version: 1,
            protocol_version: 1,
            protocol_minor_version: 1,
            office_mode: OfficeMode {
                ipaddr: self.ip_address.clone(),
                keep_address: Some(keep_address),
                dns_servers: None,
                dns_suffix: None,
            },
            optional: Some(OptionalRequest {
                client_type: "4".to_string(),
            }),
            cookie: self.session.cookie.clone(),
        }
    }

    async fn client_hello(&mut self) -> anyhow::Result<HelloReply> {
        let req = self.new_hello_request(false);
        self.send(req).await?;

        let receiver = self.receiver.as_mut().unwrap();

        let reply = receiver.next().await.ok_or_else(|| anyhow!("Channel closed!"))?;

        let reply = match reply {
            SnxPacket::Control(name, value) if name == HelloReply::NAME => {
                let result = serde_json::from_value::<HelloReply>(value)?;
                self.ip_address = result.office_mode.ipaddr.clone();
                self.auth_timeout = Duration::from_secs(result.timeouts.authentication) - REAUTH_LEEWAY;
                self.keepalive = Duration::from_secs(result.timeouts.keepalive);
                result
            }
            _ => return Err(anyhow!("Unexpected reply")),
        };

        Ok(reply)
    }

    async fn keepalive(&mut self) -> anyhow::Result<()> {
        if self.keepalive_counter.load(Ordering::SeqCst) >= MAX_KEEP_ALIVE_ATTEMPTS {
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
        let connector = SnxTunnelConnector::new(self.params.clone());

        let session = Arc::new(connector.authenticate(Some(&self.session.session_id)).await?);

        self.session = session;

        let req = self.new_hello_request(true);
        self.send(req).await?;

        Ok(())
    }

    async fn send<P>(&mut self, packet: P) -> anyhow::Result<()>
    where
        P: Into<SnxPacket>,
    {
        tokio::time::timeout(SEND_TIMEOUT, self.sender.send(packet.into())).await??;

        Ok(())
    }
}

#[async_trait::async_trait]
impl SnxTunnel for SnxSslTunnel {
    async fn run(
        mut self: Box<Self>,
        mut stop_receiver: oneshot::Receiver<()>,
        connected: Arc<AtomicBool>,
    ) -> anyhow::Result<()> {
        debug!("Running SSL tunnel for session {}", self.session.session_id);

        let reply = self.client_hello().await?;

        let tun = TunDevice::new(&reply)?;
        tun.setup_dns_and_routing(&self.params).await?;

        let dev_name = tun.name().to_owned();

        let _ = crate::util::run_command("nmcli", ["device", "set", &dev_name, "managed", "no"]).await;

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

        connected.store(true, Ordering::SeqCst);

        loop {
            tokio::select! {
                _ = &mut stop_receiver => {
                    break;
                }
                _ = tokio::time::sleep(self.keepalive) => {
                    if crate::net::is_online() {
                        self.keepalive().await?;
                    }
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
