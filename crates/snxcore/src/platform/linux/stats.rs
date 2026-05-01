use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use anyhow::anyhow;
use async_trait::async_trait;
use futures::StreamExt;
use rtnetlink::{Handle, packet_route::link::LinkAttribute};

use crate::{model::LiveStats, platform::StatsPoller};

const NO_SAMPLE: u64 = u64::MAX;

// Rates are recomputed only when at least this much time has passed since the last
// rate sample. Decouples the rate window from caller cadence so that two concurrent
// clients polling milliseconds apart cannot produce a tiny dt and a wildly inflated rate.
const MIN_SAMPLE_NS: u64 = 500_000_000;

// The prev_*/last_bps_* atomics are not a consistent set. Concurrent poll() calls could
// observe a mix of fields from different samples and produce a junk rate for one tick.
// Safe today because each connection has exactly one poller driven by frontend
// get_status() calls; revisit (e.g. seqlock) if poll() ever fans out.
pub struct LinuxStatsPoller {
    handle: Handle,
    device_name: String,
    device_index: u32,
    anchor: Instant,
    prev_ns: AtomicU64,
    prev_rx: AtomicU64,
    prev_tx: AtomicU64,
    last_bps_rx: AtomicU64,
    last_bps_tx: AtomicU64,
}

impl LinuxStatsPoller {
    pub async fn new(device_name: &str) -> anyhow::Result<Self> {
        let handle = super::new_netlink_connection()?;
        let device_index = super::resolve_device_index(&handle, device_name).await?;
        Ok(Self {
            handle,
            device_name: device_name.to_owned(),
            device_index,
            anchor: Instant::now(),
            prev_ns: AtomicU64::new(NO_SAMPLE),
            prev_rx: AtomicU64::new(0),
            prev_tx: AtomicU64::new(0),
            last_bps_rx: AtomicU64::new(0),
            last_bps_tx: AtomicU64::new(0),
        })
    }
}

#[async_trait]
impl StatsPoller for LinuxStatsPoller {
    async fn poll(&self) -> anyhow::Result<LiveStats> {
        let mut links = self.handle.link().get().match_index(self.device_index).execute();

        let msg = match links.next().await {
            Some(Ok(msg)) => msg,
            Some(Err(e)) => return Err(e.into()),
            None => return Err(anyhow!(i18n::tr!("error-device-not-found", device = &self.device_name))),
        };

        let stats = msg
            .attributes
            .into_iter()
            .find_map(|nla| match nla {
                LinkAttribute::Stats64(s) => Some(s),
                _ => None,
            })
            .ok_or_else(|| anyhow!(i18n::tr!("error-device-not-found", device = &self.device_name)))?;

        let ordering = Ordering::Relaxed;

        let now_ns = self.anchor.elapsed().as_nanos() as u64;
        let prev_ns = self.prev_ns.load(ordering);
        let elapsed_ns = now_ns.saturating_sub(prev_ns);

        let (bps_rx, bps_tx) = if prev_ns == NO_SAMPLE {
            self.prev_rx.store(stats.rx_bytes, ordering);
            self.prev_tx.store(stats.tx_bytes, ordering);
            self.prev_ns.store(now_ns, ordering);
            (0, 0)
        } else if elapsed_ns < MIN_SAMPLE_NS {
            (
                self.last_bps_rx.load(ordering),
                self.last_bps_tx.load(ordering),
            )
        } else {
            let prev_rx = self.prev_rx.load(ordering);
            let prev_tx = self.prev_tx.load(ordering);
            let dt = elapsed_ns as f64 / 1e9;
            let rates = (
                (stats.rx_bytes.saturating_sub(prev_rx) as f64 / dt) as u64,
                (stats.tx_bytes.saturating_sub(prev_tx) as f64 / dt) as u64,
            );
            self.last_bps_rx.store(rates.0, ordering);
            self.last_bps_tx.store(rates.1, ordering);
            self.prev_rx.store(stats.rx_bytes, ordering);
            self.prev_tx.store(stats.tx_bytes, ordering);
            self.prev_ns.store(now_ns, ordering);
            rates
        };

        Ok(LiveStats {
            last_rtt_ms: None,
            bytes_rx: stats.rx_bytes,
            bytes_tx: stats.tx_bytes,
            packets_rx: stats.rx_packets,
            packets_tx: stats.tx_packets,
            errors_rx: stats.rx_errors,
            errors_tx: stats.tx_errors,
            bps_rx,
            bps_tx,
        })
    }
}
