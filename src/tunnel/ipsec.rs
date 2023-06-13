use futures::pin_mut;
use tokio::signal::unix;
use tracing::debug;

use crate::{
    http::SnxHttpClient, ipsec::IpsecConfigurator, model::SnxSession, params::TunnelParams, tunnel::SnxTunnel,
};

pub(crate) struct SnxIpsecTunnel(IpsecConfigurator);

impl SnxIpsecTunnel {
    pub(crate) async fn create(params: TunnelParams, session: SnxSession) -> anyhow::Result<Self> {
        let client = SnxHttpClient::new(&params);
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
    async fn run(self: Box<Self>) -> anyhow::Result<()> {
        debug!("Running IPSec tunnel");

        let ctrl_c = tokio::signal::ctrl_c();
        pin_mut!(ctrl_c);

        let mut sig = unix::signal(unix::SignalKind::terminate())?;
        let term = sig.recv();
        pin_mut!(term);

        let select = futures::future::select(ctrl_c, term);

        tokio::select! {
            _ = select => {
                debug!("Terminating IPSec tunnel normally");
                self.0.cleanup().await;
                Ok(())
            }

            err = self.0.run_keepalive() => {
                debug!("Terminating IPSec tunnel due to keepalive failure");
                self.0.cleanup().await;
                err
            }
        }
    }
}
