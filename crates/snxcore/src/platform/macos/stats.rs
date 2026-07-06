use std::{
    ffi::CString,
    mem,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use anyhow::anyhow;
use async_trait::async_trait;
use libc::{c_int, c_uint};

use crate::{model::LiveStats, platform::StatsPoller};

const NO_SAMPLE: u64 = u64::MAX;
const MIN_SAMPLE_NS: u64 = 500_000_000;

// utun interfaces are ephemeral (created per tunnel, destroyed on fd drop, see net.rs), so the
// raw AF_LINK counters already start at zero for this session and need no baseline subtraction.
pub struct MacosStatsPoller {
    device_name: String,
    anchor: Instant,
    prev_ns: AtomicU64,
    prev_rx: AtomicU64,
    prev_tx: AtomicU64,
    last_bps_rx: AtomicU64,
    last_bps_tx: AtomicU64,
}

impl MacosStatsPoller {
    pub fn new(device_name: &str) -> anyhow::Result<Self> {
        read_if_data(device_name)?;
        Ok(Self {
            device_name: device_name.to_owned(),
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
impl StatsPoller for MacosStatsPoller {
    async fn poll(&self) -> anyhow::Result<LiveStats> {
        let counters = read_if_data(&self.device_name)?;

        let ordering = Ordering::Relaxed;
        let now_ns = self.anchor.elapsed().as_nanos() as u64;
        let prev_ns = self.prev_ns.load(ordering);
        let elapsed_ns = now_ns.saturating_sub(prev_ns);

        let (bps_rx, bps_tx) = if prev_ns == NO_SAMPLE {
            self.prev_rx.store(counters.bytes_rx, ordering);
            self.prev_tx.store(counters.bytes_tx, ordering);
            self.prev_ns.store(now_ns, ordering);
            (0, 0)
        } else if elapsed_ns < MIN_SAMPLE_NS {
            (self.last_bps_rx.load(ordering), self.last_bps_tx.load(ordering))
        } else {
            let prev_rx = self.prev_rx.load(ordering);
            let prev_tx = self.prev_tx.load(ordering);
            let dt = elapsed_ns as f64 / 1e9;
            let rates = (
                (counters.bytes_rx.saturating_sub(prev_rx) as f64 / dt) as u64,
                (counters.bytes_tx.saturating_sub(prev_tx) as f64 / dt) as u64,
            );
            self.last_bps_rx.store(rates.0, ordering);
            self.last_bps_tx.store(rates.1, ordering);
            self.prev_rx.store(counters.bytes_rx, ordering);
            self.prev_tx.store(counters.bytes_tx, ordering);
            self.prev_ns.store(now_ns, ordering);
            rates
        };

        Ok(LiveStats {
            last_rtt_ms: None,
            bytes_rx: counters.bytes_rx,
            bytes_tx: counters.bytes_tx,
            packets_rx: counters.packets_rx,
            packets_tx: counters.packets_tx,
            errors_rx: counters.errors_rx,
            errors_tx: counters.errors_tx,
            bps_rx,
            bps_tx,
        })
    }
}

struct IfCounters {
    bytes_rx: u64,
    bytes_tx: u64,
    packets_rx: u64,
    packets_tx: u64,
    errors_rx: u64,
    errors_tx: u64,
}

// getifaddrs(3)/if_data only exposes 32-bit ifi_ibytes/ifi_obytes, which wrap at 4 GiB on
// long-lived tunnels. NET_RT_IFLIST2 is the route-socket sysctl netstat -I and Activity
// Monitor read for the 64-bit if_data64 counters, so pull straight from that instead.
fn read_if_data(device_name: &str) -> anyhow::Result<IfCounters> {
    let cname =
        CString::new(device_name).map_err(|_| anyhow!(i18n::tr!("error-device-not-found", device = device_name)))?;
    let index = unsafe { libc::if_nametoindex(cname.as_ptr()) };
    if index == 0 {
        return Err(std::io::Error::last_os_error().into());
    }

    let mut mib: [c_int; 6] = [libc::CTL_NET, libc::PF_ROUTE, 0, libc::AF_INET, libc::NET_RT_IFLIST2, 0];

    let mut len: usize = 0;
    if unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as c_uint,
            std::ptr::null_mut(),
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    } != 0
    {
        return Err(std::io::Error::last_os_error().into());
    }

    let mut buf = vec![0u8; len];
    if unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as c_uint,
            buf.as_mut_ptr().cast(),
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    } != 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    buf.truncate(len);

    // Every route-socket message shares this 4-byte prefix (u16 msglen, u8 version, u8 type),
    // so peek it before deciding whether to interpret the record as a full if_msghdr2.
    const RTM_HDR_LEN: usize = 4;

    let mut offset = 0usize;
    while offset + RTM_HDR_LEN <= buf.len() {
        let base = unsafe { buf.as_ptr().add(offset) };
        let msglen = unsafe { base.cast::<u16>().read_unaligned() } as usize;
        if msglen < RTM_HDR_LEN || offset + msglen > buf.len() {
            break;
        }
        let msg_type = unsafe { *base.add(3) } as c_int;
        if msg_type == libc::RTM_IFINFO2 && msglen >= mem::size_of::<libc::if_msghdr2>() {
            let hdr = unsafe { base.cast::<libc::if_msghdr2>().read_unaligned() };
            if hdr.ifm_index as c_uint == index {
                let data = hdr.ifm_data;
                return Ok(IfCounters {
                    bytes_rx: data.ifi_ibytes,
                    bytes_tx: data.ifi_obytes,
                    packets_rx: data.ifi_ipackets,
                    packets_tx: data.ifi_opackets,
                    errors_rx: data.ifi_ierrors,
                    errors_tx: data.ifi_oerrors,
                });
            }
        }
        offset += msglen;
    }

    Err(anyhow!(i18n::tr!("error-device-not-found", device = device_name)))
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn poll_lo0_returns_live_stats() {
        use crate::platform::StatsPoller;

        let poller = super::MacosStatsPoller::new("lo0").expect("lo0 should exist");
        let stats = poller.poll().await.expect("poll should succeed without root");
        println!(
            "lo0: bytes_rx={} bytes_tx={} packets_rx={} packets_tx={}",
            stats.bytes_rx, stats.bytes_tx, stats.packets_rx, stats.packets_tx
        );
        assert!(stats.bytes_rx > 0, "expected lo0 to have carried some traffic");
        assert!(stats.bytes_tx > 0, "expected lo0 to have carried some traffic");
    }
}
