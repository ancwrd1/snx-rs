use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use anyhow::anyhow;
use tokio::sync::mpsc;
use tracing::{debug, trace, warn};

use crate::{
    model::params::TunnelParams,
    platform::{NetworkInterface, Platform, PlatformAccess, UdpSocketExt},
    tunnel::TunnelEvent,
};

const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(20);
const KEEPALIVE_RETRY_INTERVAL: Duration = Duration::from_secs(2);
const LOOP_INTERVAL: Duration = Duration::from_secs(1);
const KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(2);
const KEEPALIVE_MAX_RETRIES: u32 = 5;

fn make_keepalive_packet() -> [u8; 12] {
    static KEEPALIVE_COUNTER: AtomicUsize = AtomicUsize::new(1);

    let mut data = [0u8; 12];

    let counter = (KEEPALIVE_COUNTER.fetch_add(1, Ordering::SeqCst) & 0xffff_ffff) as u32;

    data[0..4].copy_from_slice(&counter.to_be_bytes());
    data[4..6].copy_from_slice(&0x0003u16.to_be_bytes());
    data[6..8].copy_from_slice(&0x0002u16.to_be_bytes());
    data[8..10].copy_from_slice(&0x000cu16.to_be_bytes());

    data
}

pub struct KeepaliveRunner {
    dst: Ipv4Addr,
    ready: Arc<AtomicBool>,
    event_sender: Option<mpsc::Sender<TunnelEvent>>,
}

impl KeepaliveRunner {
    pub fn new(dst: Ipv4Addr, ready: Arc<AtomicBool>) -> Self {
        Self {
            dst,
            ready,
            event_sender: None,
        }
    }

    pub fn set_event_sender(&mut self, event_sender: mpsc::Sender<TunnelEvent>) {
        self.event_sender = Some(event_sender);
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let udp = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;

        let target = SocketAddr::V4(SocketAddrV4::new(self.dst, TunnelParams::IPSEC_KEEPALIVE_PORT));

        // Disable UDP checksum validation for incoming packets.
        // Checkpoint gateway doesn't set it correctly.
        udp.set_no_check(true)?;

        let mut num_failures = 0;

        loop {
            let sleep_interval =
                if Platform::get().new_network_interface().is_online() && self.ready.load(Ordering::SeqCst) {
                    trace!("Sending keepalive to {}", self.dst);

                    let data = make_keepalive_packet();
                    let started = Instant::now();
                    let result = udp.send_receive(&data, KEEPALIVE_TIMEOUT, target).await;

                    if let Ok(reply) = result {
                        let rtt = started.elapsed();
                        trace!(
                            "Received keepalive response from {}, size: {}, rtt: {} ms",
                            self.dst,
                            reply.len(),
                            rtt.as_millis()
                        );
                        if let Some(tx) = &self.event_sender {
                            let _ = tx.try_send(TunnelEvent::Rtt(rtt));
                        }
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
                    if num_failures == 0 {
                        KEEPALIVE_INTERVAL
                    } else {
                        KEEPALIVE_RETRY_INTERVAL
                    }
                } else {
                    num_failures = 0;
                    LOOP_INTERVAL
                };

            tokio::time::sleep(sleep_interval).await;
        }

        debug!("Keepalive failed!");

        Err(anyhow!(i18n::tr!("error-keepalive-failed")))
    }
}
