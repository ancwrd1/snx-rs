use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::anyhow;
use futures::{
    channel::mpsc::{self, Receiver, Sender},
    pin_mut, SinkExt, StreamExt, TryStreamExt,
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_native_tls::native_tls::{Certificate, TlsConnector};
use tracing::{debug, trace, warn};
use tun::TunPacket;

use codec::{SslPacketCodec, SslPacketType};

use crate::tunnel::TunnelEvent;
use crate::{
    model::{params::TunnelParams, proto::*, *},
    platform,
    sexpr2::SExpression,
    tunnel::{CheckpointTunnel, TunnelCommand},
};

pub mod codec;
#[cfg(unix)]
pub mod device;

const REAUTH_LEEWAY: Duration = Duration::from_secs(60);
const MAX_KEEP_ALIVE_ATTEMPTS: u64 = 3;
const SEND_TIMEOUT: Duration = Duration::from_secs(120);
const CHANNEL_SIZE: usize = 1024;

type PacketSender = Sender<SslPacketType>;
type PacketReceiver = Receiver<SslPacketType>;

fn make_channel<S>(stream: S) -> (PacketSender, PacketReceiver)
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let framed = tokio_util::codec::Framed::new(stream, SslPacketCodec);

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

pub(crate) struct SslTunnel {
    params: Arc<TunnelParams>,
    session: Arc<CccSession>,
    auth_timeout: Duration,
    keepalive: Duration,
    ip_address: String,
    sender: PacketSender,
    receiver: Option<PacketReceiver>,
    keepalive_counter: Arc<AtomicU64>,
}

impl SslTunnel {
    pub(crate) async fn create(params: Arc<TunnelParams>, session: Arc<CccSession>) -> anyhow::Result<Self> {
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

        if params.ignore_server_cert {
            warn!("Disabling all certificate checks!!!");
            builder.danger_accept_invalid_certs(true);
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

    fn new_hello_request(&self, keep_address: bool) -> ClientHelloData {
        ClientHelloData {
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
            cookie: self.session.active_key().to_owned(),
        }
    }

    async fn client_hello(&mut self) -> anyhow::Result<HelloReplyData> {
        let req = self.new_hello_request(false);
        self.send(req).await?;

        let receiver = self.receiver.as_mut().unwrap();

        let reply = receiver.next().await.ok_or_else(|| anyhow!("Channel closed!"))?;

        let reply = match reply {
            SslPacketType::Control(expr) => {
                let result: HelloReply = expr.try_into()?;
                self.ip_address = result.data.office_mode.ipaddr.clone();
                self.auth_timeout = Duration::from_secs(result.data.timeouts.authentication) - REAUTH_LEEWAY;
                self.keepalive = Duration::from_secs(result.data.timeouts.keepalive);
                result
            }
            _ => return Err(anyhow!("Unexpected reply")),
        };

        Ok(reply.data)
    }

    async fn keepalive(&mut self) -> anyhow::Result<()> {
        if self.keepalive_counter.load(Ordering::SeqCst) >= MAX_KEEP_ALIVE_ATTEMPTS {
            let msg = "No response for keepalive packets, tunnel appears stuck";
            warn!(msg);
            return Err(anyhow!("{}", msg));
        }

        let req = KeepaliveRequestData { id: "0".to_string() };

        self.keepalive_counter.fetch_add(1, Ordering::SeqCst);

        self.send(req).await?;

        Ok(())
    }

    async fn send<P>(&mut self, packet: P) -> anyhow::Result<()>
    where
        P: Into<SslPacketType>,
    {
        tokio::time::timeout(SEND_TIMEOUT, self.sender.send(packet.into())).await??;

        Ok(())
    }
}

#[async_trait::async_trait]
impl CheckpointTunnel for SslTunnel {
    async fn run(
        mut self: Box<Self>,
        mut command_receiver: tokio::sync::mpsc::Receiver<TunnelCommand>,
        event_sender: tokio::sync::mpsc::Sender<TunnelEvent>,
    ) -> anyhow::Result<()> {
        debug!("Running SSL tunnel for session {}", self.session.session_id);

        let reply = self.client_hello().await?;
        trace!("Hello reply: {:?}", reply);

        let tun_name = self
            .params
            .if_name
            .clone()
            .unwrap_or(TunnelParams::DEFAULT_IF_NAME.to_owned());

        let tun = device::TunDevice::new(&tun_name, &reply)?;
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
                    SslPacketType::Control(expr) => {
                        debug!("Control packet received");
                        match expr {
                            SExpression::Object(Some(name), _) if name == "keepalive" => {
                                keepalive_counter.fetch_sub(1, Ordering::SeqCst);
                            }
                            _ => {}
                        }
                    }
                    SslPacketType::Data(data) => {
                        trace!("snx => {}: {}", data.len(), dev_name2);
                        keepalive_counter.store(0, Ordering::SeqCst);
                        let tun_packet = TunPacket::new(data);
                        tun_sender.send(tun_packet).await?;
                    }
                }
            }
            Ok::<_, anyhow::Error>(())
        });

        let _ = event_sender.send(TunnelEvent::Connected).await;

        let stop_fut = command_receiver.recv();
        pin_mut!(stop_fut);

        let result = loop {
            tokio::select! {
                _ = &mut stop_fut => {
                    break Ok(());
                }
                _ = tokio::time::sleep(self.keepalive) => {
                    if platform::is_online() {
                        self.keepalive().await?;
                    }
                }

                result = tun_receiver.next() => {
                    if let Some(Ok(item)) = result {
                        let data = item.into_bytes().to_vec();
                        trace!("{} => snx: {}", dev_name, data.len());
                        self.send(data).await?;
                    } else {
                        break Err(anyhow!("Receive failed"));
                    }
                }
            }
        };

        let _ = event_sender.send(TunnelEvent::Disconnected).await;

        result
    }
}
