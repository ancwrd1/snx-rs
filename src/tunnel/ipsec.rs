use std::sync::{Arc, Mutex};

use chrono::Local;
use tokio::net::UdpSocket;
use tokio::sync::oneshot;
use tracing::{debug, warn};

use crate::{
    http::CccHttpClient,
    model::{params::TunnelParams, CheckpointSession, ConnectionStatus},
    platform::{IpsecConfigurator, UdpEncap, UdpSocketExt},
    tunnel::{ipsec::keepalive::KeepaliveRunner, CheckpointTunnel},
};

mod keepalive;

pub(crate) struct IpsecTunnel {
    configurator: Box<dyn IpsecConfigurator + Send>,
    keepalive_runner: KeepaliveRunner,
}

impl IpsecTunnel {
    pub(crate) async fn create(params: Arc<TunnelParams>, session: Arc<CheckpointSession>) -> anyhow::Result<Self> {
        let client = CccHttpClient::new(params.clone());
        let client_settings = client.get_client_settings(&session.session_id).await?;
        debug!("Client settings: {:?}", client_settings);

        let ipsec_params = client.get_ipsec_tunnel_params(&session.session_id).await?;

        let keepalive_runner =
            KeepaliveRunner::new(ipsec_params.om_addr.into(), client_settings.gw_internal_ip.parse()?);

        let pid = unsafe { libc::getpid() };

        let mut configurator =
            crate::platform::new_ipsec_configurator(params, ipsec_params, client_settings, pid as u32).await?;

        configurator.configure().await?;

        Ok(Self {
            configurator: Box::new(configurator),
            keepalive_runner,
        })
    }
}

#[async_trait::async_trait]
impl CheckpointTunnel for IpsecTunnel {
    async fn run(
        self: Box<Self>,
        stop_receiver: oneshot::Receiver<()>,
        connected: Arc<Mutex<ConnectionStatus>>,
    ) -> anyhow::Result<()> {
        debug!("Running IPSec tunnel");

        let sender = start_decap_listener(self.configurator.decap_socket()).await?;

        *connected.lock().unwrap() = ConnectionStatus {
            connected_since: Some(Local::now()),
            ..Default::default()
        };

        let result = tokio::select! {
            _ = stop_receiver => {
                debug!("Terminating IPSec tunnel due to stop command");
                Ok(())
            }

            err = self.keepalive_runner.run() => {
                debug!("Terminating IPSec tunnel due to keepalive failure");
                err
            }
        };

        let _ = sender.send(());

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

// start a dummy UDP listener with UDP_ENCAP option.
// this is necessary in order to perform automatic decapsulation of incoming ESP packets
pub async fn start_decap_listener(udp: Arc<UdpSocket>) -> anyhow::Result<oneshot::Sender<()>> {
    udp.set_encap(UdpEncap::EspInUdp)?;

    let (tx, mut rx) = oneshot::channel();

    debug!("Listening for NAT-T packets on port {}", udp.local_addr()?);

    tokio::spawn(async move {
        let mut buf = [0u8; 1024];

        loop {
            tokio::select! {
                result = udp.recv_from(&mut buf) => {
                    if let Ok((size, from)) = result {
                        warn!("Received unexpected NON-ESP data from {}, length: {}", from, size);
                    }
                }
                _ = &mut rx => {
                    break;
                }
            }
        }
        debug!("NAT-T listener stopped");
    });

    Ok(tx)
}
