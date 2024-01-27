use std::{net::Ipv4Addr, time::Duration};

use anyhow::anyhow;
use tokio::net::UdpSocket;
use tracing::debug;

use crate::platform::UdpSocketExt;

const MAX_ISAKMP_PROBES: usize = 5;

pub struct Isakmp {
    address: Ipv4Addr,
    port: u16,
}

impl Isakmp {
    pub fn new(address: Ipv4Addr, port: u16) -> Self {
        Self { address, port }
    }

    pub async fn probe(&self) -> anyhow::Result<()> {
        for _ in 0..MAX_ISAKMP_PROBES {
            if self.send_probe().await.is_ok() {
                return Ok(());
            }
        }
        Err(anyhow!("Probing failed, server is not reachable via ESPinUDP tunnel!"))
    }

    async fn send_probe(&self) -> anyhow::Result<()> {
        debug!("Sending isakmp probe to {}", self.address);
        let udp = UdpSocket::bind("0.0.0.0:0").await?;
        udp.connect(format!("{}:{}", self.address, self.port)).await?;

        let data = vec![0u8; 32];

        let result = udp.send_receive(&data, Duration::from_secs(5)).await;

        match result {
            Ok(reply) if reply.len() == 32 => {
                let srcport: [u8; 4] = reply[8..12].try_into().unwrap();
                let dstport: [u8; 4] = reply[12..16].try_into().unwrap();
                debug!(
                    "Received isakmp reply from {}: srcport: {}, dstport: {}, hash: {}",
                    self.address,
                    u32::from_be_bytes(srcport),
                    u32::from_be_bytes(dstport),
                    hex::encode(&reply[reply.len() - 16..reply.len()])
                );
                Ok(())
            }
            _ => Err(anyhow!("No isakmp reply!")),
        }
    }
}
