use futures::pin_mut;
use tokio::signal::unix;
use tracing::debug;

use crate::{ipsec::IpsecConfigurator, tunnel::SnxTunnel};

pub(crate) struct SnxIpsecTunnel(pub(crate) IpsecConfigurator);

#[async_trait::async_trait]
impl SnxTunnel for SnxIpsecTunnel {
    async fn run(self: Box<Self>) -> anyhow::Result<()> {
        debug!("Running IPSec tunnel");

        let ctrl_c = tokio::signal::ctrl_c();
        pin_mut!(ctrl_c);

        let mut term = unix::signal(unix::SignalKind::terminate())?;
        let fut = term.recv();
        pin_mut!(fut);

        let _ = futures::future::select(ctrl_c, fut).await;

        debug!("Terminating IPSec tunnel");
        self.0.cleanup().await;

        Ok(())
    }
}
