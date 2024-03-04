use std::{
    net::{IpAddr, ToSocketAddrs},
    sync::Arc,
};

use anyhow::anyhow;
use tokio::{net::UdpSocket, sync::mpsc};
use tracing::debug;

use crate::{
    ccc::CccHttpClient,
    model::{params::TunnelParams, CccSession},
    platform::{self, IpsecConfigurator, UdpEncap, UdpSocketExt},
    tunnel::{
        ipsec::{keepalive::KeepaliveRunner, natt::start_natt_listener},
        CheckpointTunnel, TunnelCommand, TunnelEvent,
    },
    util,
};

pub mod keepalive;
pub mod natt;

pub(crate) struct IpsecTunnel {
    configurator: Box<dyn IpsecConfigurator + Send + Sync>,
    keepalive_runner: KeepaliveRunner,
    natt_socket: Arc<UdpSocket>,
}

impl IpsecTunnel {
    pub(crate) async fn create(params: Arc<TunnelParams>, session: Arc<CccSession>) -> anyhow::Result<Self> {
        let ipsec_session = session
            .ipsec_session
            .as_ref()
            .ok_or_else(|| anyhow!("No IPSEC session!"))?;

        let client = CccHttpClient::new(params.clone(), Some(session.clone()));
        let client_settings = client.get_client_settings().await?;

        let gateway_address = format!("{}:4500", params.server_name)
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| anyhow!("No gateway address!"))?
            .ip();

        let ipv4address = match gateway_address {
            IpAddr::V4(v4) => v4,
            _ => client_settings.gw_internal_ip,
        };

        debug!(
            "Resolved gateway address: {}, acquired internal address: {}",
            ipv4address, client_settings.gw_internal_ip
        );

        let keepalive_runner = KeepaliveRunner::new(ipsec_session.address, ipv4address);

        let natt_socket = UdpSocket::bind("0.0.0.0:0").await?;
        natt_socket.set_encap(UdpEncap::EspInUdp)?;

        let mut configurator = platform::new_ipsec_configurator(
            params,
            ipsec_session.clone(),
            unsafe { libc::getpid() } as u32,
            natt_socket.local_addr()?.port(),
            ipv4address,
            util::ranges_to_subnets(&client_settings.updated_policies.range.settings).collect(),
        )
        .await?;

        configurator.configure().await?;

        Ok(Self {
            configurator: Box::new(configurator),
            keepalive_runner,
            natt_socket: Arc::new(natt_socket),
        })
    }
}

#[async_trait::async_trait]
impl CheckpointTunnel for IpsecTunnel {
    async fn run(
        mut self: Box<Self>,
        mut command_receiver: mpsc::Receiver<TunnelCommand>,
        event_sender: mpsc::Sender<TunnelEvent>,
    ) -> anyhow::Result<()> {
        debug!("Running IPSec tunnel");

        let natt_stopper = start_natt_listener(self.natt_socket.clone(), event_sender.clone()).await?;

        let _ = event_sender.send(TunnelEvent::Connected).await;

        let fut = async {
            while let Some(cmd) = command_receiver.recv().await {
                match cmd {
                    TunnelCommand::Terminate => break,
                    TunnelCommand::ReKey(session) => {
                        debug!("Rekey command received, configuring xfrm");
                        let _ = self.configurator.re_key(&session).await;
                    }
                }
            }
        };
        let result = tokio::select! {
            _ = fut => {
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

impl Drop for IpsecTunnel {
    fn drop(&mut self) {
        debug!("Cleaning up ipsec tunnel");
        std::thread::scope(|s| {
            s.spawn(|| crate::util::block_on(self.configurator.cleanup()));
        });
    }
}
