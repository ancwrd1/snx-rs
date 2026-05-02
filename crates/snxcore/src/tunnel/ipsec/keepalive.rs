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
                    let now = Instant::now();
                    let result = udp.send_receive(&data, KEEPALIVE_TIMEOUT, target).await;

                    if let Ok(reply) = result {
                        let rtt = now.elapsed();
                        trace!(
                            "Received keepalive response from {}, size: {}, rtt: {} ms",
                            self.dst,
                            reply.len(),
                            rtt.as_millis()
                        );
                        if let Some(tx) = &self.event_sender {
                            let _ = tx.send(TunnelEvent::Rtt(rtt)).await;
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

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicBool;

    use tokio::net::UdpSocket;

    use super::*;

    #[tokio::test]
    async fn successful_keepalive_emits_rtt_event() {
        let echo = UdpSocket::bind(("127.0.0.1", TunnelParams::IPSEC_KEEPALIVE_PORT))
            .await
            .expect("bind echo socket");
        let echo_task = tokio::spawn(async move {
            let mut buf = [0u8; 256];
            while let Ok((size, peer)) = echo.recv_from(&mut buf).await {
                let _ = echo.send_to(&buf[..size], peer).await;
            }
        });

        let ready = Arc::new(AtomicBool::new(true));
        let mut runner = KeepaliveRunner::new(Ipv4Addr::LOCALHOST, ready);
        let (tx, mut rx) = mpsc::channel(8);
        runner.set_event_sender(tx);

        let runner_task = tokio::spawn(async move { runner.run().await });

        let event = tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("rtt event within timeout")
            .expect("event channel open");
        assert!(matches!(event, TunnelEvent::Rtt(_)));

        runner_task.abort();
        echo_task.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn failed_keepalive_returns_error_after_max_retries() {
        // Nothing listens on 127.0.0.2:IPSEC_KEEPALIVE_PORT, so every send_receive call times out.
        let ready = Arc::new(AtomicBool::new(true));
        let runner = KeepaliveRunner::new(Ipv4Addr::new(127, 0, 0, 2), ready);

        let result = tokio::time::timeout(Duration::from_secs(120), runner.run())
            .await
            .expect("runner should exit after exhausting retries");
        assert!(result.is_err());
    }
}
