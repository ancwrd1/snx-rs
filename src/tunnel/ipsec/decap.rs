use std::sync::Arc;

use tokio::net::UdpSocket;
use tokio::sync::oneshot;
use tracing::{debug, warn};

use crate::platform::{UdpEncap, UdpSocketExt};

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
