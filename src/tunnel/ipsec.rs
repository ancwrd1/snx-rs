use anyhow::anyhow;
use std::sync::{Arc, Mutex};

use chrono::Local;
use tokio::sync::mpsc;
use tokio::{net::UdpSocket, sync::oneshot};
use tracing::{debug, warn};

use crate::tunnel::TunnelCommand;
use crate::{
    ccc::CccHttpClient,
    model::{params::TunnelParams, CccSession, ConnectionStatus},
    platform::{self, IpsecConfigurator, UdpEncap, UdpSocketExt},
    tunnel::{
        ipsec::{isakmp::Isakmp, keepalive::KeepaliveRunner},
        CheckpointTunnel,
    },
};

mod isakmp;
mod keepalive;

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
        debug!("Client settings: {:?}", client_settings);

        let isakmp = Isakmp::new(client_settings.gw_internal_ip, 4500);
        isakmp.probe().await?;

        let keepalive_runner = KeepaliveRunner::new(ipsec_session.address, client_settings.gw_internal_ip);

        let natt_socket = UdpSocket::bind("0.0.0.0:0").await?;
        natt_socket.set_encap(UdpEncap::EspInUdp)?;

        let mut configurator = platform::new_ipsec_configurator(
            params,
            ipsec_session.clone(),
            client_settings,
            unsafe { libc::getpid() } as u32,
            natt_socket.local_addr()?.port(),
        )
        .await?;

        configurator.configure().await?;

        Ok(Self {
            configurator: Box::new(configurator),
            keepalive_runner,
            natt_socket: Arc::new(natt_socket),
        })
    }

    // start a dummy UDP listener with UDP_ENCAP option.
    // this is necessary in order to perform automatic decapsulation of incoming ESP packets
    pub async fn start_natt_listener(&self) -> anyhow::Result<oneshot::Sender<()>> {
        let (tx, mut rx) = oneshot::channel();

        debug!("Listening for NAT-T packets on port {}", self.natt_socket.local_addr()?);

        let udp = self.natt_socket.clone();

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
}

#[async_trait::async_trait]
impl CheckpointTunnel for IpsecTunnel {
    async fn run(
        mut self: Box<Self>,
        mut command_receiver: mpsc::Receiver<TunnelCommand>,
        connected: Arc<Mutex<ConnectionStatus>>,
        status_sender: oneshot::Sender<()>,
    ) -> anyhow::Result<()> {
        debug!("Running IPSec tunnel");

        let sender = self.start_natt_listener().await?;

        *connected.lock().unwrap() = ConnectionStatus {
            connected_since: Some(Local::now()),
            ..Default::default()
        };
        if status_sender.send(()).is_ok() {
            debug!("IPSec tunnel connection status set")
        }

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
