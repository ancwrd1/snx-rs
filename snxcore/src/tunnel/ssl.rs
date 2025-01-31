use std::{
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::{anyhow, Context};
use futures::{
    channel::mpsc::{self, Receiver, Sender},
    pin_mut, SinkExt, StreamExt, TryStreamExt,
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_native_tls::native_tls::{Certificate, TlsConnector};
use tracing::{debug, trace, warn};

use codec::{SslPacketCodec, SslPacketType};

use crate::ccc::CccHttpClient;
use crate::platform::{new_resolver_configurator, ResolverConfig};
use crate::tunnel::device;
use crate::tunnel::device::TunDevice;
use crate::{
    model::{params::TunnelParams, proto::*, *},
    platform,
    sexpr::SExpression,
    tunnel::{ssl::keepalive::KeepaliveRunner, TunnelCommand, TunnelEvent, VpnTunnel},
    util,
};

pub mod codec;
pub mod connector;
pub mod keepalive;

const REAUTH_LEEWAY: Duration = Duration::from_secs(60);
const SEND_TIMEOUT: Duration = Duration::from_secs(120);
const CHANNEL_SIZE: usize = 1024;

pub type PacketSender = Sender<SslPacketType>;
pub type PacketReceiver = Receiver<SslPacketType>;

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
    session: Arc<VpnSession>,
    auth_timeout: Duration,
    keepalive: Duration,
    ip_address: String,
    sender: PacketSender,
    receiver: Option<PacketReceiver>,
    keepalive_counter: Arc<AtomicI64>,
    tun_device: Option<TunDevice>,
    hello_reply: HelloReplyData,
}

impl SslTunnel {
    pub(crate) async fn create(params: Arc<TunnelParams>, session: Arc<VpnSession>) -> anyhow::Result<Self> {
        let tcp = tokio::net::TcpStream::connect((params.server_name.as_str(), 443)).await?;

        let mut builder = TlsConnector::builder();

        for ca_cert in &params.ca_cert {
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
            keepalive_counter: Arc::new(AtomicI64::default()),
            tun_device: None,
            hello_reply: HelloReplyData::default(),
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
        trace!("Hello request: {:?}", req);
        self.send(req).await?;

        let receiver = self.receiver.as_mut().unwrap();

        let reply = receiver.next().await.context("Channel closed!")?;

        let reply = match reply {
            SslPacketType::Control(expr) => {
                trace!("Hello reply: {:?}", expr);
                if matches!(&expr, SExpression::Object(Some(name), _) if name == "disconnect") {
                    anyhow::bail!("Tunnel disconnected, last message: {}", expr);
                }
                let hello_reply = expr.try_into::<HelloReply>()?;
                self.ip_address.clone_from(&hello_reply.data.office_mode.ipaddr);
                self.auth_timeout = Duration::from_secs(hello_reply.data.timeouts.authentication) - REAUTH_LEEWAY;
                self.keepalive = Duration::from_secs(hello_reply.data.timeouts.keepalive);
                hello_reply
            }
            _ => anyhow::bail!("Unexpected reply"),
        };

        Ok(reply.data)
    }

    async fn send<P>(&mut self, packet: P) -> anyhow::Result<()>
    where
        P: Into<SslPacketType>,
    {
        tokio::time::timeout(SEND_TIMEOUT, self.sender.send(packet.into())).await??;

        Ok(())
    }

    async fn cleanup(&mut self) {
        if let Some(device) = self.tun_device.take() {
            if let Ok(dest_ip) = util::resolve_ipv4_host(&format!("{}:443", self.params.server_name)) {
                let _ = platform::remove_default_route(dest_ip).await;
            }
            if !self.params.no_dns {
                let _ = self.setup_dns(device.name(), true).await;
            }
            platform::delete_device(device.name()).await;
            debug!("Signing out");
            let client = CccHttpClient::new(self.params.clone(), Some(self.session.clone()));
            let _ = client.signout().await;
        }
    }

    pub async fn setup_routing(&self, dev_name: &str) -> anyhow::Result<()> {
        let ipaddr = self.hello_reply.office_mode.ipaddr.parse()?;

        let dest_ip = util::resolve_ipv4_host(&format!("{}:443", self.params.server_name))?;

        let mut subnets = self.params.add_routes.clone();

        if !self.params.no_routing {
            if self.params.default_route {
                platform::setup_default_route(dev_name, dest_ip).await?;
            } else {
                subnets.extend(util::ranges_to_subnets(&self.hello_reply.range));
            }
        }

        subnets.retain(|s| !s.contains(&dest_ip));

        if !subnets.is_empty() {
            let _ = platform::add_routes(&subnets, dev_name, ipaddr, &self.params.ignore_routes).await;
        }

        Ok(())
    }

    pub async fn setup_dns(&self, dev_name: &str, cleanup: bool) -> anyhow::Result<()> {
        let search_domains = if let Some(ref suffixes) = self.hello_reply.office_mode.dns_suffix {
            suffixes
                .0
                .iter()
                .chain(self.params.search_domains.iter())
                .filter(|s| {
                    !s.is_empty()
                        && !self
                            .params
                            .ignore_search_domains
                            .iter()
                            .any(|d| d.to_lowercase() == s.to_lowercase())
                })
                .cloned()
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        let dns_servers = self
            .hello_reply
            .office_mode
            .dns_servers
            .clone()
            .unwrap_or_default()
            .iter()
            .chain(self.params.dns_servers.iter())
            .filter(|s| !self.params.ignore_dns_servers.iter().any(|d| *d == **s))
            .cloned()
            .collect::<Vec<_>>();

        let config = ResolverConfig {
            search_domains,
            dns_servers,
        };

        let resolver = new_resolver_configurator(dev_name)?;

        if cleanup {
            resolver.cleanup(&config).await?;
        } else {
            resolver.configure(&config).await?;
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl VpnTunnel for SslTunnel {
    async fn run(
        mut self: Box<Self>,
        mut command_receiver: tokio::sync::mpsc::Receiver<TunnelCommand>,
        event_sender: tokio::sync::mpsc::Sender<TunnelEvent>,
    ) -> anyhow::Result<()> {
        debug!("Running SSL tunnel for session {}", self.session.ccc_session_id);

        let reply = self.client_hello().await?;
        trace!("Hello reply: {:?}", reply);

        self.hello_reply = reply;

        let ip_address = self.hello_reply.office_mode.ipaddr.parse()?;
        let netmask = self.hello_reply.optional.as_ref().and_then(|o| o.subnet.parse().ok());

        let tun_name = self
            .params
            .if_name
            .as_deref()
            .unwrap_or(TunnelParams::DEFAULT_SSL_IF_NAME);

        let mut tun = device::TunDevice::new(tun_name, ip_address, netmask)?;

        self.setup_routing(tun_name).await?;

        if !self.params.no_dns {
            self.setup_dns(tun_name, false).await?;
        }

        let _ = platform::configure_device(tun_name).await;

        let (mut tun_sender, mut tun_receiver) = tun.take_inner().context("No tun device")?.into_framed().split();

        self.tun_device = Some(tun);

        let mut snx_receiver = self.receiver.take().unwrap();

        let keepalive_counter = self.keepalive_counter.clone();

        tokio::spawn(async move {
            while let Some(item) = snx_receiver.next().await {
                match item {
                    SslPacketType::Control(expr) => {
                        debug!("Control packet received");
                        match expr {
                            SExpression::Object(Some(name), _) if name == "keepalive" => {
                                let _ = keepalive_counter
                                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| (v > 0).then_some(v - 1));
                            }
                            _ => {}
                        }
                    }
                    SslPacketType::Data(data) => {
                        tun_sender.send(data).await?;
                        keepalive_counter.store(0, Ordering::SeqCst);
                    }
                }
            }
            Ok::<_, anyhow::Error>(())
        });

        let _ = event_sender.send(TunnelEvent::Connected).await;

        let command_fut = command_receiver.recv();
        pin_mut!(command_fut);

        let keepalive_runner =
            KeepaliveRunner::new(self.keepalive, self.sender.clone(), self.keepalive_counter.clone());
        let ka_run = keepalive_runner.run();
        pin_mut!(ka_run);

        let result = loop {
            tokio::select! {
                event = &mut command_fut => {
                    match event {
                        Some(TunnelCommand::Terminate) | None => {
                            break Ok(());
                        }
                        _ => {}
                    }
                }
                () = &mut ka_run => {
                    warn!("Keepalive failed, exiting");
                    break Err(anyhow!("Keepalive failed"));
                }

                result = tun_receiver.next() => {
                    if let Some(Ok(item)) = result {
                        self.send(item).await?;
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

impl Drop for SslTunnel {
    fn drop(&mut self) {
        debug!("Cleaning up SSL tunnel");
        std::thread::scope(|s| {
            s.spawn(|| util::block_on(self.cleanup()));
        });
    }
}
