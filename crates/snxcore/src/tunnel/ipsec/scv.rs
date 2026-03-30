use crate::platform::{NetworkInterface, Platform, PlatformAccess};
use std::{
    net::Ipv4Addr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

const SCV_INTERVAL: Duration = Duration::from_secs(20);
const LOOP_INTERVAL: Duration = Duration::from_secs(1);
const SCV_PORT: u16 = 18233;

const SCV_COMPLIANCE_DATA: &[u8] = &[
    0x44, 0x56, 0x10, 0x0, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x7, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x0, 0x0, 0x45,
    0x4b, 0x77, 0x60, 0x61, 0x59, 0x01, 0x09, 0x33, 0x47, 0x77, 0xd0, 0x45, 0x77, 0x03, 0x05, 0x00, 0x00, 0x00, 0x51,
    0x94, 0xe8, 0x98, 0x03, 0xb5, 0xff, 00,
];

pub struct ScvRunner {
    dst: Vec<Ipv4Addr>,
    ready: Arc<AtomicBool>,
}

impl ScvRunner {
    pub fn new(dst: Vec<Ipv4Addr>, ready: Arc<AtomicBool>) -> Self {
        Self { dst, ready }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let udp = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;

        loop {
            if Platform::get().new_network_interface().is_online() && self.ready.load(Ordering::SeqCst) {
                for addr in &self.dst {
                    let _ = udp.send_to(SCV_COMPLIANCE_DATA, (*addr, SCV_PORT)).await;
                }
                tokio::time::sleep(SCV_INTERVAL).await;
            } else {
                tokio::time::sleep(LOOP_INTERVAL).await;
            }
        }
    }
}
