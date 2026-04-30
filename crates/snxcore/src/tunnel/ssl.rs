use std::{
    net::Ipv4Addr,
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
    time::Duration,
};

use anyhow::{Context, anyhow};
use chrono::Local;
use codec::{SlimPacketType, SlimProtocolCodec};
use futures::{
    SinkExt, StreamExt, TryStreamExt,
    channel::mpsc::{self, Receiver, Sender},
    pin_mut,
};
use i18n::tr;
use ipnet::Ipv4Net;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_native_tls::native_tls::{Certificate, TlsConnector};
use tracing::{debug, trace, warn};

use crate::{
    ccc::CccHttpClient,
    model::{
        ConnectionInfo, VpnSession,
        params::{TransportType, TunnelParams, TunnelType},
        proto::{ClientHelloData, HelloReply, HelloReplyData, LoginOption, OfficeMode, OptionalRequest},
    },
    platform::{
        DeviceConfig, NetworkInterface, Platform, PlatformAccess, ResolverConfig, RoutingConfig, RoutingConfigurator,
    },
    server_info,
    sexpr::SExpression,
    tunnel::{TunnelCommand, TunnelEvent, VpnTunnel, device::TunDevice, ssl::keepalive::KeepaliveRunner},
    util,
};

pub mod codec;
pub mod connector;
pub mod keepalive;

const REAUTH_LEEWAY: Duration = Duration::from_secs(60);
const SEND_TIMEOUT: Duration = Duration::from_secs(120);
const CHANNEL_SIZE: usize = 1024;

pub type PacketSender = Sender<SlimPacketType>;
pub type PacketReceiver = Receiver<SlimPacketType>;

fn make_channel<S>(stream: S) -> (PacketSender, PacketReceiver)
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let framed = tokio_util::codec::Framed::new(stream, SlimProtocolCodec);

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
    terminate_sender: Option<Sender<()>>,
}

impl SslTunnel {
    pub(crate) async fn create(params: Arc<TunnelParams>, session: Arc<VpnSession>) -> anyhow::Result<Self> {
        let info = server_info::get(&params).await?;

        let address = util::server_name_with_port(&params.server_name, info.connectivity_info.tcpt_port);

        let tcp = tokio::net::TcpStream::connect(address.as_ref()).await?;

        let mut builder = TlsConnector::builder();

        for ca_cert in &params.ca_cert {
            let data = tokio::fs::read(ca_cert).await?;
            let cert = Certificate::from_pem(&data).or_else(|_| Certificate::from_der(&data))?;
            builder.add_root_certificate(cert);
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
            terminate_sender: None,
        })
    }

    fn new_hello_request(&self, keep_address: bool) -> ClientHelloData {
        let data = ClientHelloData {
            client_version: 2,
            protocol_version: 2,
            protocol_minor_version: None,
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
        };

        if self.params.login_type != LoginOption::MOBILE_ACCESS_ID {
            data
        } else {
            // Use SLIM v1 for mobile access
            ClientHelloData {
                client_version: 1,
                protocol_version: 1,
                protocol_minor_version: Some(1),
                ..data
            }
        }
    }

    async fn client_hello(&mut self) -> anyhow::Result<HelloReplyData> {
        let req = self.new_hello_request(false);
        trace!("Hello request: {:?}", req);
        self.send(req).await?;

        let receiver = self.receiver.as_mut().unwrap();

        let reply = receiver.next().await.context("Channel closed!")?;

        let reply = match reply {
            SlimPacketType::Control(expr) => {
                trace!("Hello reply: {:?}", expr);
                if matches!(&expr, SExpression::Object(Some(name), _) if name == "disconnect") {
                    anyhow::bail!(tr!("error-tunnel-disconnected", message = expr));
                }
                let hello_reply = expr.try_into::<HelloReply>()?;
                self.ip_address.clone_from(&hello_reply.data.office_mode.ipaddr);
                self.auth_timeout = Duration::from_secs(hello_reply.data.timeouts.authentication) - REAUTH_LEEWAY;
                self.keepalive = Duration::from_secs(hello_reply.data.timeouts.keepalive);
                hello_reply
            }
            _ => anyhow::bail!(tr!("error-unexpected-reply")),
        };

        Ok(reply.data)
    }

    async fn send<P>(&mut self, packet: P) -> anyhow::Result<()>
    where
        P: Into<SlimPacketType>,
    {
        tokio::time::timeout(SEND_TIMEOUT, self.sender.send(packet.into())).await??;

        Ok(())
    }

    async fn cleanup(&mut self) {
        let Some(device) = self.tun_device.take() else {
            return;
        };

        let platform = Platform::get();

        if let Ok(info) = server_info::get(&self.params).await
            && let Ok(dest_ip) = util::server_name_to_ipv4(&self.params.server_name, info.connectivity_info.tcpt_port)
            && let Ok(configurator) = platform.new_routing_configurator(device.name(), TunnelType::SSL).await
        {
            let _ = configurator
                .configure(&RoutingConfig::Cleanup {
                    destination: dest_ip,
                    enable_ipv6: self.params.disable_ipv6,
                })
                .await;
        }

        if !self.params.no_dns {
            let config = self.make_resolver_config().await;
            let _ = self.setup_dns(&config, device.name(), true).await;
        }

        if let Some(mut sender) = self.terminate_sender.take() {
            let _ = sender.send(()).await;
        }

        let _ = Platform::get()
            .new_network_interface()
            .delete_device(device.name())
            .await;

        debug!("Signing out");

        let client = CccHttpClient::new(self.params.clone(), Some(self.session.clone()));
        let _ = client.signout().await;
    }

    pub async fn setup_routing(&self, device_config: &DeviceConfig) -> anyhow::Result<()> {
        let platform = Platform::get();
        let configurator = platform
            .new_routing_configurator(&device_config.name, TunnelType::SSL)
            .await?;

        let dest_ip = util::server_name_to_ipv4(
            &self.params.server_name,
            server_info::get(&self.params).await?.connectivity_info.tcpt_port,
        )?;

        let config = if self.params.no_routing {
            RoutingConfig::Split {
                destination: dest_ip,
                routes: self.params.add_routes.clone(),
            }
        } else if self.params.default_route {
            RoutingConfig::Full {
                destination: dest_ip,
                disable_ipv6: self.params.disable_ipv6,
            }
        } else {
            let ranges = if let Some(ref range) = self.hello_reply.range {
                range.clone()
            } else {
                let client = CccHttpClient::new(self.params.clone(), Some(self.session.clone()));
                let client_settings = client.get_client_settings().await?;
                client_settings.updated_policies.range.settings
            };
            let subnets = util::ranges_to_subnets(&ranges).collect::<Vec<_>>();

            let mut routes = Vec::with_capacity(subnets.len() + self.params.add_routes.len());
            routes.extend(&self.params.add_routes);
            routes.extend(subnets);
            routes.retain(|r| !self.params.ignore_routes.contains(r));

            if device_config.address.prefix_len() < 32 {
                routes.push(device_config.address.trunc());
            }

            RoutingConfig::Split {
                destination: dest_ip,
                routes,
            }
        };

        configurator.configure(&config).await?;

        Ok(())
    }

    async fn make_resolver_config(&self) -> ResolverConfig {
        let features = Platform::get().get_features().await;
        let builder = ResolverConfig::builder(self.params.clone(), features);

        builder
            .search_domains(
                self.hello_reply
                    .office_mode
                    .dns_suffix
                    .as_ref()
                    .map(|s| s.0.as_slice())
                    .unwrap_or_default(),
            )
            .dns_servers(
                self.hello_reply
                    .office_mode
                    .dns_servers
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .cloned(),
            )
            .build()
    }

    pub async fn setup_dns(&self, config: &ResolverConfig, dev_name: &str, cleanup: bool) -> anyhow::Result<()> {
        let resolver = Platform::get().new_resolver_configurator(dev_name)?;

        if cleanup {
            resolver.cleanup(config).await?;
        } else {
            resolver.configure(config).await?;
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

        let name_hint = self
            .params
            .if_name
            .as_deref()
            .unwrap_or(TunnelParams::DEFAULT_SSL_IF_NAME);

        let mut tun = TunDevice::new(name_hint)?;
        let tun_name = tun.name().to_owned();

        let device_config = DeviceConfig {
            name: tun_name.clone(),
            mtu: self.params.mtu,
            address: netmask
                .and_then(|netmask| Ipv4Net::with_netmask(ip_address, netmask).ok())
                .unwrap_or_else(|| Ipv4Net::from(ip_address)),
            allow_forwarding: self.params.allow_forwarding,
        };

        Platform::get()
            .new_network_interface()
            .configure_device(&device_config)
            .await?;

        self.setup_routing(&device_config).await?;

        let resolver_config = self.make_resolver_config().await;

        if !self.params.no_dns {
            self.setup_dns(&resolver_config, &tun_name, false).await?;
        }

        let (mut tun_sender, mut tun_receiver) = tun.take_inner().context("No tun device")?.into_framed().split();

        self.tun_device = Some(tun);

        let mut snx_receiver = self.receiver.take().unwrap();

        let keepalive_counter = self.keepalive_counter.clone();

        let (terminate_sender, mut terminate_receiver) = mpsc::channel(1);
        self.terminate_sender = Some(terminate_sender);

        let fut = async move {
            while let Some(item) = snx_receiver.next().await {
                match item {
                    SlimPacketType::Control(SExpression::Object(Some(name), _)) if name == "keepalive" => {
                        keepalive_counter.store(0, Ordering::SeqCst);
                    }
                    SlimPacketType::Control(sexpr) => {
                        debug!("Control packet received: {}", sexpr);
                    }
                    SlimPacketType::Data(data) => {
                        tun_sender.send(data).await?;
                        keepalive_counter.store(0, Ordering::SeqCst);
                    }
                }
            }
            Ok::<_, anyhow::Error>(())
        };

        tokio::spawn(async move {
            tokio::select! {
                _ = terminate_receiver.next() => Ok::<_, anyhow::Error>(()),
                res = fut => res,
            }
        });

        let info = ConnectionInfo {
            since: Some(Local::now()),
            server_name: self.params.server_name.clone(),
            username: self.session.username.clone().unwrap_or_default(),
            login_type: self.params.login_type.clone(),
            tunnel_type: TunnelType::SSL,
            transport_type: TransportType::Tcpt,
            ip_address: Ipv4Net::with_netmask(ip_address, netmask.unwrap_or(Ipv4Addr::new(255, 255, 255, 255)))?,
            dns_servers: resolver_config.dns_servers,
            search_domains: resolver_config.search_domains,
            interface_name: tun_name,
            dns_configured: !self.params.no_dns,
            routing_configured: !self.params.no_routing,
            default_route: self.params.default_route,
            profile_id: self.params.profile_id,
            profile_name: self.params.profile_name.clone(),
        };

        let _ = event_sender.send(TunnelEvent::Connected(info)).await;

        let command_fut = command_receiver.recv();
        pin_mut!(command_fut);

        let keepalive_runner =
            KeepaliveRunner::new(self.keepalive, self.sender.clone(), self.keepalive_counter.clone());
        let ka_run = keepalive_runner.run();
        pin_mut!(ka_run);

        let result = loop {
            tokio::select! {
                event = &mut command_fut => match event {
                    Some(TunnelCommand::Terminate(_)) | None => {
                        break Ok(());
                    }
                    _ => {}
                },
                () = &mut ka_run => {
                    warn!("Keepalive failed, exiting");
                    break Err(anyhow!(tr!("error-keepalive-failed")));
                }

                result = tun_receiver.next() => {
                    if let Some(Ok(item)) = result {
                        self.send(item).await?;
                    } else {
                        break Err(anyhow!(tr!("error-receive-failed")));
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
