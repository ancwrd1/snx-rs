use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use tokio::sync::oneshot;
use tracing::debug;

use crate::{
    http::SnxHttpClient, ipsec::IpsecConfigurator, model::SnxSession, params::TunnelParams, tunnel::SnxTunnel,
};

pub(crate) struct SnxIpsecTunnel(IpsecConfigurator);

impl SnxIpsecTunnel {
    pub(crate) async fn create(params: Arc<TunnelParams>, session: SnxSession) -> anyhow::Result<Self> {
        let client = SnxHttpClient::new(params.clone());
        let client_settings = client.get_client_settings(&session.session_id).await?;
        debug!("Client settings: {:?}", client_settings);

        let ipsec_params = client.get_ipsec_tunnel_params(&session.session_id).await?;
        let mut configurator = IpsecConfigurator::new(params, ipsec_params, client_settings);
        configurator.configure().await?;

        Ok(Self(configurator))
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

        connected.store(true, Ordering::SeqCst);

        let result = tokio::select! {
            _ = stop_receiver => {
                debug!("Terminating IPSec tunnel due to stop command");
                Ok(())
            }

            err = self.0.run_keepalive() => {
                debug!("Terminating IPSec tunnel due to keepalive failure");
                err
            }
        };

        result
    }
}

impl Drop for SnxIpsecTunnel {
    fn drop(&mut self) {
        debug!("Cleaning up ipsec tunnel");
        std::thread::scope(|s| {
            s.spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                rt.block_on(self.0.cleanup());
            });
        });
    }
}
