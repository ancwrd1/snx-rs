use std::{
    net::Ipv4Addr,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use anyhow::{Context, anyhow};
use bytes::Bytes;
use chrono::Local;
use futures::{
    SinkExt, StreamExt,
    channel::mpsc::{Receiver, Sender},
    pin_mut,
};
use ipnet::Ipv4Net;
use isakmp::esp::{EspCodec, EspEncapType};
use tokio::time::MissedTickBehavior;
use tracing::{debug, error};

use crate::{
    ccc::CccHttpClient,
    model::{
        ConnectionInfo, VpnSession,
        params::{TransportType, TunnelParams},
    },
    platform::{self, NetworkInterface, ResolverConfig, RoutingConfigurator, new_resolver_configurator},
    server_info,
    tunnel::{TunnelCommand, TunnelEvent, VpnTunnel, device::TunDevice, ipsec::keepalive::KeepaliveRunner},
    util,
};

const SEND_TIMEOUT: Duration = Duration::from_secs(120);

pub type PacketSender = Sender<Bytes>;
pub type PacketReceiver = Receiver<Bytes>;

pub(crate) struct TunIpsecTunnel {
    params: Arc<TunnelParams>,
    session: Arc<VpnSession>,
    sender: PacketSender,
    receiver: Option<PacketReceiver>,
    tun_device: Option<TunDevice>,
    ready: Arc<AtomicBool>,
    gateway_address: Ipv4Addr,
    encap_type: EspEncapType,
    esp_transport: TransportType,
    subnets: Vec<Ipv4Net>,
}

impl TunIpsecTunnel {
    pub(crate) async fn create(
        params: Arc<TunnelParams>,
        session: Arc<VpnSession>,
        sender: PacketSender,
        receiver: PacketReceiver,
        esp_transport: TransportType,
    ) -> anyhow::Result<Self> {
        let server_info = server_info::get(&params).await?;
        let client = CccHttpClient::new(params.clone(), Some(session.clone()));
        let client_settings = client.get_client_settings().await?;

        let subnets = util::ranges_to_subnets(&client_settings.updated_policies.range.settings).collect::<Vec<_>>();

        let (port, encap_type) = match esp_transport {
            TransportType::Tcpt => (server_info.connectivity_info.tcpt_port, EspEncapType::Udp),
            _ => (server_info.connectivity_info.tcpt_port, EspEncapType::None),
        };

        let gateway_address = util::resolve_ipv4_host(&format!("{}:{}", params.server_name, port))?;

        debug!(
            "Resolved gateway address: {}, acquired internal address: {}",
            gateway_address, client_settings.gw_internal_ip
        );

        let ready = Arc::new(AtomicBool::new(false));

        ready.store(true, Ordering::SeqCst);

        Ok(Self {
            params,
            session,
            sender,
            receiver: Some(receiver),
            tun_device: None,
            ready,
            gateway_address,
            encap_type,
            esp_transport,
            subnets,
        })
    }

    async fn send(&mut self, packet: Bytes) -> anyhow::Result<()> {
        tokio::time::timeout(SEND_TIMEOUT, self.sender.send(packet)).await??;

        Ok(())
    }

    async fn cleanup(&mut self) {
        if let Some(device) = self.tun_device.take() {
            if let Some(session) = self.session.ipsec_session.as_ref() {
                let configurator = platform::new_routing_configurator(device.name(), session.address);
                let _ = configurator.remove_default_route(self.gateway_address).await;
                let _ = configurator.remove_keepalive_route(self.gateway_address).await;
                if !self.params.no_dns {
                    let config = crate::tunnel::ipsec::make_resolver_config(session, &self.params);
                    let _ = self.setup_dns(&config, device.name(), true).await;
                }
                let _ = platform::new_network_interface().delete_device(device.name()).await;
            }
        }
    }

    pub async fn setup_routing(&self, dev_name: &str) -> anyhow::Result<()> {
        let session = self.session.ipsec_session.as_ref().context("No IPSec session!")?;

        let configurator = platform::new_routing_configurator(dev_name, session.address);

        let mut subnets = self.params.add_routes.clone();

        let mut default_route_set = false;

        if !self.params.no_routing {
            if self.params.default_route {
                configurator.setup_default_route(self.gateway_address).await?;
                default_route_set = true;
            } else {
                subnets.extend(&self.subnets);
            }
        }

        configurator
            .setup_keepalive_route(self.gateway_address, !default_route_set)
            .await?;

        subnets.retain(|s| !s.contains(&self.gateway_address));

        if !subnets.is_empty() {
            let _ = configurator.add_routes(&subnets, &self.params.ignore_routes).await;
        }

        Ok(())
    }

    pub async fn setup_dns(
        &self,
        resolver_config: &ResolverConfig,
        dev_name: &str,
        cleanup: bool,
    ) -> anyhow::Result<()> {
        let resolver = new_resolver_configurator(dev_name)?;

        if cleanup {
            resolver.cleanup(resolver_config).await?;
        } else {
            resolver.configure(resolver_config).await?;
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl VpnTunnel for TunIpsecTunnel {
    async fn run(
        mut self: Box<Self>,
        mut command_receiver: tokio::sync::mpsc::Receiver<TunnelCommand>,
        event_sender: tokio::sync::mpsc::Sender<TunnelEvent>,
    ) -> anyhow::Result<()> {
        debug!(
            "Running IPSec ({}) tunnel for session {}",
            self.esp_transport, self.session.ccc_session_id,
        );

        let name_hint = self
            .params
            .if_name
            .as_deref()
            .unwrap_or(TunnelParams::DEFAULT_SSL_IF_NAME);

        let Some(ref ipsec_session) = self.session.ipsec_session else {
            anyhow::bail!("No IPSEC session!");
        };

        let mut tun = TunDevice::new(name_hint, ipsec_session.address, Some(ipsec_session.netmask))?;
        let tun_name = tun.name().to_owned();

        self.setup_routing(&tun_name).await?;

        let session = self.session.ipsec_session.as_ref().context("No IPSec session!")?;

        let resolver_config = crate::tunnel::ipsec::make_resolver_config(session, &self.params);

        if !self.params.no_dns {
            self.setup_dns(&resolver_config, &tun_name, false).await?;
        }

        let _ = platform::new_network_interface().configure_device(&tun_name).await;

        let (mut tun_sender, mut tun_receiver) = tun.take_inner().context("No tun device")?.into_framed().split();

        self.tun_device = Some(tun);

        let mut snx_receiver = self.receiver.take().context("No receiver")?;

        let esp_codec_in = Arc::new(RwLock::new(EspCodec::new(
            self.gateway_address,
            session.address,
            self.encap_type,
        )));
        esp_codec_in
            .write()
            .unwrap()
            .set_params(ipsec_session.esp_in.spi, ipsec_session.esp_in.clone());

        let esp_codec_out = Arc::new(RwLock::new(EspCodec::new(
            session.address,
            self.gateway_address,
            self.encap_type,
        )));
        esp_codec_out
            .write()
            .unwrap()
            .set_params(ipsec_session.esp_out.spi, ipsec_session.esp_out.clone());

        let sender = event_sender.clone();

        tokio::task::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
            while sender.send(TunnelEvent::RekeyCheck).await.is_ok() {
                interval.tick().await;
            }
            Ok::<_, anyhow::Error>(())
        });

        let esp_codec = esp_codec_in.clone();

        tokio::spawn(async move {
            while let Some(item) = snx_receiver.next().await {
                let codec = esp_codec.clone();
                let result = tokio::task::spawn_blocking(move || codec.read().unwrap().decode(&item));

                match result.await {
                    Ok(Ok(packet)) => {
                        let _ = tun_sender.send(packet.into()).await;
                    }
                    Ok(Err(e)) => {
                        error!("Failed to decode packet: {}", e);
                    }
                    Err(e) => {
                        error!("Failed to spawn blocking task: {}", e);
                    }
                }
            }
            Ok::<_, anyhow::Error>(())
        });

        let session = self.session.ipsec_session.as_ref().context("No IPSec session!")?;

        let info = ConnectionInfo {
            since: Local::now(),
            server_name: self.params.server_name.clone(),
            tunnel_type: self.params.tunnel_type,
            transport_type: session.transport_type,
            ip_address: Ipv4Net::with_netmask(session.address, session.netmask)?,
            dns_servers: resolver_config.dns_servers,
            search_domains: resolver_config.search_domains,
            interface_name: tun_name,
            dns_configured: !self.params.no_dns,
            routing_configured: !self.params.no_routing,
            default_route: self.params.default_route,
        };
        let _ = event_sender.send(TunnelEvent::Connected(info)).await;
        let ready = self.ready.clone();

        let esp_codec_in = esp_codec_in.clone();
        let esp_codec_out = esp_codec_out.clone();

        let params = self.params.clone();
        let session = self.session.clone();

        let command_fut = async {
            while let Some(cmd) = command_receiver.recv().await {
                match cmd {
                    TunnelCommand::Terminate(signout) => {
                        if signout || !params.ike_persist {
                            debug!("Signing out");
                            let client = CccHttpClient::new(params.clone(), Some(session.clone()));
                            let _ = client.signout().await;
                        }
                        break;
                    }
                    TunnelCommand::ReKey(session) => {
                        debug!(
                            "Rekey command received, new lifetime: {}, reconfiguring ESP codec",
                            session.lifetime.as_secs()
                        );
                        ready.store(false, Ordering::SeqCst);

                        esp_codec_in
                            .write()
                            .unwrap()
                            .add_params(session.esp_in.spi, session.esp_in.clone());

                        esp_codec_out
                            .write()
                            .unwrap()
                            .set_params(session.esp_out.spi, session.esp_out.clone());

                        ready.store(true, Ordering::SeqCst);
                    }
                }
            }
        };
        pin_mut!(command_fut);

        let keepalive_runner = KeepaliveRunner::new(
            ipsec_session.address,
            self.gateway_address,
            if self.params.no_keepalive || !platform::get_features().await.ipsec_keepalive {
                Arc::new(AtomicBool::new(false))
            } else {
                ready.clone()
            },
        );

        let ka_run = keepalive_runner.run();
        pin_mut!(ka_run);

        let result = loop {
            tokio::select! {
                () = &mut command_fut => {
                    debug!("Terminating IPSec tunnel due to stop command");
                    break Ok(());
                }

                err = &mut ka_run => {
                    debug!("Terminating IPSec tunnel due to keepalive failure");
                    break err;
                }

                result = tun_receiver.next() => {
                    if let Some(Ok(item)) = result {
                        let codec = esp_codec_out.clone();
                        let result = tokio::task::spawn_blocking(move || codec.read().unwrap().encode(&item)).await;
                        match result {
                            Ok(Ok(packet)) => {
                                self.send(packet).await?;
                            },
                            Ok(Err(e)) => {
                                error!("Failed to encode packet: {}", e);
                            }
                            Err(e) => {
                                error!("Failed to spawn blocking task: {}", e);
                            }
                        }
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

impl Drop for TunIpsecTunnel {
    fn drop(&mut self) {
        debug!("Cleaning up IPSec ({}) tunnel", self.esp_transport);
        std::thread::scope(|s| {
            s.spawn(|| util::block_on(self.cleanup()));
        });
    }
}
