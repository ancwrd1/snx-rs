use std::{
    mem::zeroed,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use anyhow::anyhow;
use async_trait::async_trait;
use windows::Win32::NetworkManagement::{
    IpHelper::{GetIfEntry2, MIB_IF_ROW2},
    Ndis::NET_LUID_LH,
};

use crate::{model::LiveStats, platform::StatsPoller};

const NO_SAMPLE: u64 = u64::MAX;
const MIN_SAMPLE_NS: u64 = 500_000_000;

pub struct WindowsStatsPoller {
    luid: NET_LUID_LH,
    baseline_in: u64,
    baseline_out: u64,
    anchor: Instant,
    prev_ns: AtomicU64,
    prev_rx: AtomicU64,
    prev_tx: AtomicU64,
    last_bps_rx: AtomicU64,
    last_bps_tx: AtomicU64,
}

impl WindowsStatsPoller {
    pub fn new(device_name: &str) -> anyhow::Result<Self> {
        let luid = super::alias_to_luid(device_name)?;
        let row = read_row(luid)?;

        Ok(Self {
            luid,
            baseline_in: row.InOctets,
            baseline_out: row.OutOctets,
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
impl StatsPoller for WindowsStatsPoller {
    async fn poll(&self) -> anyhow::Result<LiveStats> {
        let row = read_row(self.luid)?;

        let bytes_rx = row.InOctets.saturating_sub(self.baseline_in);
        let bytes_tx = row.OutOctets.saturating_sub(self.baseline_out);
        let packets_rx = row.InUcastPkts.saturating_add(row.InNUcastPkts);
        let packets_tx = row.OutUcastPkts.saturating_add(row.OutNUcastPkts);

        let ordering = Ordering::Relaxed;
        let now_ns = self.anchor.elapsed().as_nanos() as u64;
        let prev_ns = self.prev_ns.load(ordering);
        let elapsed_ns = now_ns.saturating_sub(prev_ns);

        let (bps_rx, bps_tx) = if prev_ns == NO_SAMPLE {
            self.prev_rx.store(bytes_rx, ordering);
            self.prev_tx.store(bytes_tx, ordering);
            self.prev_ns.store(now_ns, ordering);
            (0, 0)
        } else if elapsed_ns < MIN_SAMPLE_NS {
            (self.last_bps_rx.load(ordering), self.last_bps_tx.load(ordering))
        } else {
            let prev_rx = self.prev_rx.load(ordering);
            let prev_tx = self.prev_tx.load(ordering);
            let dt = elapsed_ns as f64 / 1e9;
            let rates = (
                (bytes_rx.saturating_sub(prev_rx) as f64 / dt) as u64,
                (bytes_tx.saturating_sub(prev_tx) as f64 / dt) as u64,
            );
            self.last_bps_rx.store(rates.0, ordering);
            self.last_bps_tx.store(rates.1, ordering);
            self.prev_rx.store(bytes_rx, ordering);
            self.prev_tx.store(bytes_tx, ordering);
            self.prev_ns.store(now_ns, ordering);
            rates
        };

        Ok(LiveStats {
            last_rtt_ms: None,
            bytes_rx,
            bytes_tx,
            packets_rx,
            packets_tx,
            errors_rx: row.InErrors,
            errors_tx: row.OutErrors,
            bps_rx,
            bps_tx,
        })
    }
}

fn read_row(luid: NET_LUID_LH) -> anyhow::Result<MIB_IF_ROW2> {
    let mut row: MIB_IF_ROW2 = unsafe { zeroed() };
    row.InterfaceLuid = luid;

    unsafe { GetIfEntry2(&mut row) }
        .ok()
        .map_err(|e| anyhow!("GetIfEntry2 failed: {:?}", e))?;

    Ok(row)
}
