use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

use anyhow::anyhow;
use tracing::{debug, trace, warn};

use crate::{
    model::params::TunnelParams,
    platform::{NetworkInterface, Platform, PlatformAccess, UdpSocketExt},
};

const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(20);
const KEEPALIVE_RETRY_INTERVAL: Duration = Duration::from_secs(2);
const KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(2);
const KEEPALIVE_MAX_RETRIES: u32 = 5;

fn make_keepalive_packet() -> [u8; 12] {
    static KEEPALIVE_COUNTER: AtomicUsize = AtomicUsize::new(1);

    let mut data = [0u8; 12];

    let counter = (KEEPALIVE_COUNTER.fetch_add(1, Ordering::SeqCst) & 0xffff_fffff) as u32;

    data[0..4].copy_from_slice(&counter.to_be_bytes());
    data[4..6].copy_from_slice(&0x0003u16.to_be_bytes());
    data[6..8].copy_from_slice(&0x0002u16.to_be_bytes());
    data[8..10].copy_from_slice(&0x000cu16.to_be_bytes());

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
            if Platform::get().new_network_interface().is_online() {
                if self.ready.load(Ordering::SeqCst) {
                    trace!("Sending keepalive to {}", self.dst);

                    let data = make_keepalive_packet();
                    let result = udp.send_receive(&data, KEEPALIVE_TIMEOUT, target).await;

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
                Platform::get().new_network_interface().poll_online();
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
