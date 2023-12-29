use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use tokio::sync::oneshot;
use tracing::debug;

use crate::{
    http::SnxHttpClient,
    model::{params::TunnelParams, SnxSession},
    platform::IpsecConfigurator,
    tunnel::{ipsec::keepalive::KeepaliveRunner, SnxTunnel},
};

mod decap;
mod keepalive;

pub(crate) struct SnxIpsecTunnel {
    configurator: Box<dyn IpsecConfigurator + Send>,
    keepalive_runner: KeepaliveRunner,
}

impl SnxIpsecTunnel {
    pub(crate) async fn create(params: Arc<TunnelParams>, session: Arc<SnxSession>) -> anyhow::Result<Self> {
        let client = SnxHttpClient::new(params.clone());
        let client_settings = client.get_client_settings(&session.session_id).await?;
        debug!("Client settings: {:?}", client_settings);

        let ipsec_params = client.get_ipsec_tunnel_params(&session.session_id).await?;

        let keepalive_runner =
            KeepaliveRunner::new(ipsec_params.om_addr.into(), client_settings.gw_internal_ip.parse()?);

        let mut configurator = crate::platform::new_ipsec_configurator(params, ipsec_params, client_settings);
        configurator.configure().await?;

        Ok(Self {
            configurator: Box::new(configurator),
            keepalive_runner,
        })
    }
}

#[async_trait::async_trait]
impl SnxTunnel for SnxIpsecTunnel {
    async fn run(
        self: Box<Self>,
        stop_receiver: oneshot::Receiver<()>,
        connected: Arc<AtomicBool>,
    ) -> anyhow::Result<()> {
        debug!("Running IPSec tunnel");

        let sender = decap::start_decap_listener().await?;

        connected.store(true, Ordering::SeqCst);

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

impl Drop for SnxIpsecTunnel {
    fn drop(&mut self) {
        debug!("Cleaning up ipsec tunnel");
        std::thread::scope(|s| {
            s.spawn(|| crate::util::block_on(self.configurator.cleanup()));
        });
    }
}
