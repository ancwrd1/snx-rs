use std::{
    net::Ipv4Addr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use anyhow::Context;
use chrono::Local;
use i18n::tr;
use ipnet::Ipv4Net;
use tokio::{net::UdpSocket, sync::mpsc, time::MissedTickBehavior};
use tracing::debug;

use crate::{
    ccc::CccHttpClient,
    model::{ConnectionInfo, IpsecSession, VpnSession, params::TunnelParams},
    platform::{
        IpsecConfigurator, Platform, PlatformAccess, ResolverConfig, RoutingConfigurator, UdpEncap, UdpSocketExt,
    },
    server_info,
    tunnel::{
        TunnelCommand, TunnelEvent, VpnTunnel,
        ipsec::{keepalive::KeepaliveRunner, natt::start_natt_listener},
    },
    util,
};

pub(crate) struct NativeIpsecTunnel {
    configurator: Box<dyn IpsecConfigurator + Send + Sync>,
    keepalive_runner: KeepaliveRunner,
    natt_socket: Arc<UdpSocket>,
    ready: Arc<AtomicBool>,
    params: Arc<TunnelParams>,
    session: Arc<VpnSession>,
    device_name: String,
    gateway_address: Ipv4Addr,
    subnets: Vec<Ipv4Net>,
}

impl NativeIpsecTunnel {
    pub(crate) async fn create(params: Arc<TunnelParams>, session: Arc<VpnSession>) -> anyhow::Result<Self> {
        let server_info = server_info::get(&params).await?;

        let ipsec_session = session.ipsec_session.as_ref().context(tr!("error-no-ipsec-session"))?;

        let client = CccHttpClient::new(params.clone(), Some(session.clone()));
        let client_settings = client.get_client_settings().await?;

        let subnets = util::ranges_to_subnets(&client_settings.updated_policies.range.settings).collect::<Vec<_>>();

        let gateway_address = util::server_name_to_ipv4(&params.server_name, server_info.connectivity_info.natt_port)?;

        debug!(
            "Resolved gateway address: {}, acquired internal address: {}",
            gateway_address, client_settings.gw_internal_ip
        );

        let ready = Arc::new(AtomicBool::new(false));
        let keepalive_runner = KeepaliveRunner::new(
            server_info.connectivity_info.server_ip,
            if params.no_keepalive || !Platform::get().get_features().await.ipsec_keepalive {
                Arc::new(AtomicBool::new(false))
            } else {
                ready.clone()
            },
        );

        let natt_socket = UdpSocket::bind("0.0.0.0:0").await?;
        natt_socket.set_encap(UdpEncap::EspInUdp)?;

        let device_name = params
            .if_name
            .as_deref()
            .unwrap_or(TunnelParams::DEFAULT_IPSEC_IF_NAME)
            .to_owned();

        let mut configurator = Platform::get().new_ipsec_configurator(
            &device_name,
            ipsec_session.clone(),
            natt_socket.local_addr()?.port(),
            gateway_address,
            server_info.connectivity_info.natt_port,
            params.mtu,
        )?;

        configurator.configure().await?;
        ready.store(true, Ordering::SeqCst);

        Ok(Self {
            configurator: Box::new(configurator),
            keepalive_runner,
            natt_socket: Arc::new(natt_socket),
            ready,
            params,
            session,
            device_name,
            gateway_address,
            subnets,
        })
    }

    async fn setup_dns(&self, resolver_config: &ResolverConfig, cleanup: bool) -> anyhow::Result<()> {
        debug!("Configuring resolver: {:?}", resolver_config);

        let resolver = Platform::get().new_resolver_configurator(&self.device_name)?;

        if cleanup {
            resolver.cleanup(resolver_config).await?;
        } else {
            resolver.configure(resolver_config).await?;
        }

        Ok(())
    }

    async fn setup_routing(&self, session: &IpsecSession) -> anyhow::Result<()> {
        let platform = Platform::get();
        let configurator = platform.new_routing_configurator(&self.device_name, session.address);

        let mut subnets = self.params.add_routes.clone();

        let mut default_route_set = false;

        if !self.params.no_routing {
            if self.params.default_route {
                configurator
                    .setup_default_route(self.gateway_address, self.params.disable_ipv6)
                    .await?;
                default_route_set = true;
            } else {
                subnets.extend(&self.subnets);
                let network = Ipv4Net::with_netmask(session.address, session.netmask)?;
                if network.prefix_len() < 32 {
                    subnets.push(network.trunc());
                }
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

    async fn cleanup(&mut self) {
        if !self.params.no_dns
            && let Some(session) = self.session.ipsec_session.as_ref()
        {
            let config = crate::tunnel::ipsec::make_resolver_config(session, &self.params);
            let _ = self.setup_dns(&config, true).await;
        }
        self.configurator.cleanup().await;

        if let Some(session) = self.session.ipsec_session.as_ref() {
            let platform = Platform::get();
            let configurator = platform.new_routing_configurator(&self.device_name, session.address);
            let _ = configurator.remove_keepalive_route(self.gateway_address).await;
            let _ = configurator
                .remove_default_route(self.gateway_address, self.params.disable_ipv6)
                .await;
        }
    }
}

#[async_trait::async_trait]
impl VpnTunnel for NativeIpsecTunnel {
    async fn run(
        mut self: Box<Self>,
        mut command_receiver: mpsc::Receiver<TunnelCommand>,
        event_sender: mpsc::Sender<TunnelEvent>,
    ) -> anyhow::Result<()> {
        debug!("Running IPSec tunnel");

        let natt_stopper = start_natt_listener(self.natt_socket.clone(), event_sender.clone()).await?;

        let session = self
            .session
            .ipsec_session
            .as_ref()
            .context(tr!("error-no-ipsec-session"))?;

        self.setup_routing(session).await?;

        let resolver_config = crate::tunnel::ipsec::make_resolver_config(session, &self.params);

        if !self.params.no_dns {
            self.setup_dns(&resolver_config, false).await?;
        }

        let ip_address = Ipv4Net::with_netmask(session.address, session.netmask)?;

        let info = ConnectionInfo {
            since: Some(Local::now()),
            server_name: self.params.server_name.clone(),
            username: self.session.username.clone().unwrap_or_default(),
            login_type: self.params.login_type.clone(),
            tunnel_type: self.params.tunnel_type,
            transport_type: session.transport_type,
            ip_address,
            dns_servers: resolver_config.dns_servers,
            search_domains: resolver_config.search_domains,
            interface_name: self.device_name.clone(),
            dns_configured: !self.params.no_dns,
            routing_configured: !self.params.no_routing,
            default_route: self.params.default_route,
        };
        let _ = event_sender.send(TunnelEvent::Connected(info)).await;

        let sender = event_sender.clone();

        tokio::task::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
            while sender.send(TunnelEvent::RekeyCheck).await.is_ok() {
                interval.tick().await;
            }
            Ok::<_, anyhow::Error>(())
        });

        let fut = async {
            while let Some(cmd) = command_receiver.recv().await {
                match cmd {
                    TunnelCommand::Terminate(signout) => {
                        if signout || !self.params.ike_persist {
                            debug!("Signing out");
                            let client = CccHttpClient::new(self.params.clone(), Some(self.session.clone()));
                            let _ = client.signout().await;
                        }
                        break;
                    }
                    TunnelCommand::ReKey(session) => {
                        debug!(
                            "Rekey command received, new lifetime: {}, configuring xfrm",
                            session.lifetime.as_secs()
                        );
                        self.ready.store(false, Ordering::SeqCst);
                        let _ = self.configurator.rekey(&session).await;
                        self.ready.store(true, Ordering::SeqCst);
                        let address = Ipv4Net::with_netmask(session.address, session.netmask).unwrap_or(ip_address);
                        let _ = event_sender.send(TunnelEvent::Rekeyed(address)).await;
                    }
                }
            }
        };
        let result = tokio::select! {
            () = fut => {
                debug!("Terminating IPSec tunnel due to stop command");
                Ok(())
            }

            err = self.keepalive_runner.run() => {
                debug!("Terminating IPSec tunnel due to keepalive failure");
                err
            }
        };

        let _ = natt_stopper.send(());
        let _ = event_sender.send(TunnelEvent::Disconnected).await;

        result
    }
}

impl Drop for NativeIpsecTunnel {
    fn drop(&mut self) {
        debug!("Cleaning up IPSec tunnel");
        std::thread::scope(|s| {
            s.spawn(|| util::block_on(self.cleanup()));
        });
    }
}
