use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::anyhow;
use tracing::{debug, trace, warn};

use crate::{
    model::params::TunnelParams,
    platform::{self, NetworkInterface, UdpSocketExt},
};

const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(20);
const KEEPALIVE_RETRY_INTERVAL: Duration = Duration::from_secs(5);
const KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(5);
const KEEPALIVE_MAX_RETRIES: u32 = 5;

// picked from wireshark logs
fn make_keepalive_packet() -> [u8; 84] {
    let mut data = [0u8; 84];

    // 0x00000011 looks like a packet type, KEEPALIVE in this case
    data[0..4].copy_from_slice(&0x0000_0011u32.to_be_bytes());

    // 0x0001 is probably a direction: request or response. We get 0x0002 as a response back.
    data[4..6].copy_from_slice(&0x0001u16.to_be_bytes());

    // this looks like a content type, probably means TIMESTAMP
    data[6..8].copy_from_slice(&0x0002u16.to_be_bytes());

    // timestamp
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
    data[8..16].copy_from_slice(&timestamp.to_be_bytes());
    data
}

pub struct KeepaliveRunner {
    dst: Ipv4Addr,
    ready: Arc<AtomicBool>,
}

impl KeepaliveRunner {
    pub fn new(dst: Ipv4Addr, ready: Arc<AtomicBool>) -> Self {
        Self { dst, ready }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let udp = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;

        let target = SocketAddr::V4(SocketAddrV4::new(self.dst, TunnelParams::IPSEC_KEEPALIVE_PORT));

        // Disable UDP checksum validation for incoming packets.
        // Checkpoint gateway doesn't set it correctly.
        udp.set_no_check(true)?;

        let mut num_failures = 0;

        loop {
            if platform::new_network_interface().is_online() {
                if self.ready.load(Ordering::SeqCst) {
                    trace!("Sending keepalive to {}", self.dst);

                    let data = make_keepalive_packet();
                    let result = udp.send_receive_to(&data, KEEPALIVE_TIMEOUT, target).await;

                    if let Ok(reply) = result {
                        trace!("Received keepalive response from {}, size: {}", self.dst, reply.len());
                        num_failures = 0;
                    } else {
                        num_failures += 1;
                        if num_failures >= KEEPALIVE_MAX_RETRIES {
                            warn!("Maximum number of keepalive retries reached, exiting");
                            break;
                        }
                        warn!(
                            "Keepalive failed, retrying in {} secs",
                            KEEPALIVE_RETRY_INTERVAL.as_secs()
                        );
                    }
                }
            } else {
                num_failures = 0;
                platform::new_network_interface().poll_online();
            }

            let interval = if num_failures == 0 {
                KEEPALIVE_INTERVAL
            } else {
                KEEPALIVE_RETRY_INTERVAL
            };

            tokio::time::sleep(interval).await;
        }

        debug!("Keepalive failed!");

        Err(anyhow!(i18n::tr!("error-keepalive-failed")))
    }
}
