use std::{
    net::Ipv4Addr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::anyhow;
use tracing::{debug, trace, warn};

use crate::{
    model::params::TunnelParams,
    platform::{self, UdpSocketExt},
};

const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(20);
const KEEPALIVE_RETRY_INTERVAL: Duration = Duration::from_secs(5);
const KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(5);
const KEEPALIVE_MAX_RETRIES: u32 = 5;

// picked from wireshark logs
fn make_keepalive_packet() -> [u8; 84] {
    let mut data = [0u8; 84];

    // 0x00000011 looks like a packet type, KEEPALIVE in this case
    data[0..4].copy_from_slice(&0x00000011u32.to_be_bytes());

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
    src: Ipv4Addr,
    dst: Ipv4Addr,
}

impl KeepaliveRunner {
    pub fn new(src: Ipv4Addr, dst: Ipv4Addr) -> Self {
        Self { src, dst }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let src = self.src.to_string();

        let udp = tokio::net::UdpSocket::bind((src, TunnelParams::IPSEC_KEEPALIVE_PORT)).await?;
        udp.connect((self.dst, TunnelParams::IPSEC_KEEPALIVE_PORT)).await?;

        // disable UDP checksum validation for incoming packets.
        // Checkpoint gateway doesn't set it correctly.
        udp.set_no_check(true)?;

        let mut num_failures = 0;

        loop {
            if platform::is_online() {
                trace!("Sending keepalive to {}", self.dst);

                let data = make_keepalive_packet();
                let result = udp.send_receive(&data, KEEPALIVE_TIMEOUT).await;

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
            } else {
                num_failures = 0;
                platform::poll_online();
            }

            let interval = if num_failures == 0 {
                KEEPALIVE_INTERVAL
            } else {
                KEEPALIVE_RETRY_INTERVAL
            };

            tokio::time::sleep(interval).await;
        }

        debug!("Keepalive failed!");

        Err(anyhow!("Keepalive failed!"))
    }
}
