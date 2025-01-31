use std::{
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
    time::Duration,
};

use anyhow::anyhow;
use bytes::Bytes;
use isakmp::{
    ikev1::{service::Ikev1Service, session::Ikev1Session},
    model::Identity,
    session::IsakmpSession,
    transport::UdpTransport,
};
use tokio::{
    net::UdpSocket,
    sync::{mpsc, oneshot},
};
use tracing::debug;

use crate::{model::params::TunnelParams, platform::UdpSocketExt, tunnel::TunnelEvent};

const MAX_NATT_PROBES: usize = 3;

pub struct NattProber {
    params: Arc<TunnelParams>,
    port: u16,
}

impl NattProber {
    pub fn new(params: Arc<TunnelParams>) -> Self {
        Self { params, port: 4500 }
    }

    pub async fn probe(&self) -> anyhow::Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect(format!("{}:{}", self.params.server_name, 500)).await?;

        let IpAddr::V4(gateway_address) = socket.peer_addr()?.ip() else {
            anyhow::bail!("No IPv4 address for {}", self.params.server_name);
        };

        if self.send_probe(gateway_address).await.is_err() {
            // As reported by some users, CP gateway may not respond to the probe unless there is traffic on port 500.
            // So we try the SA exchange first to unblock port 4500.
            debug!("Sending dummy SA proposal to port 500");
            let ikev1_session = Box::new(Ikev1Session::new(Identity::None)?);
            let transport = Box::new(UdpTransport::new(socket, ikev1_session.new_codec()));
            let mut service = Ikev1Service::new(transport, ikev1_session)?;
            service.do_sa_proposal(Duration::from_secs(5)).await?;
            debug!("SA proposal succeeded");
        }

        for _ in 0..MAX_NATT_PROBES {
            if self.send_probe(gateway_address).await.is_ok() {
                return Ok(());
            }
        }
        Err(anyhow!("Probing failed, server is not reachable via ESPinUDP tunnel!"))
    }

    async fn send_probe(&self, address: Ipv4Addr) -> anyhow::Result<()> {
        debug!("Sending NAT-T probe to {}", address);
        let udp = UdpSocket::bind("0.0.0.0:0").await?;
        udp.connect(format!("{}:{}", address, self.port)).await?;

        let data = vec![0u8; 32];

        let result = udp.send_receive(&data, Duration::from_secs(5)).await;

        match result {
            Ok(reply) if reply.len() == 32 => {
                let srcport: [u8; 4] = reply[8..12].try_into().unwrap();
                let dstport: [u8; 4] = reply[12..16].try_into().unwrap();
                debug!(
                    "Received NAT-T reply from {}: srcport: {}, dstport: {}, hash: {}",
                    address,
                    u32::from_be_bytes(srcport),
                    u32::from_be_bytes(dstport),
                    hex::encode(&reply[reply.len() - 16..reply.len()])
                );
                Ok(())
            }
            _ => Err(anyhow!("No NAT-T reply!")),
        }
    }
}

// start a dummy UDP listener with UDP_ENCAP option.
// this is necessary in order to perform automatic decapsulation of incoming ESP packets
pub async fn start_natt_listener(
    socket: Arc<UdpSocket>,
    sender: mpsc::Sender<TunnelEvent>,
) -> anyhow::Result<oneshot::Sender<()>> {
    let (tx, mut rx) = oneshot::channel();

    debug!("Listening for NAT-T packets on port {}", socket.local_addr()?);

    tokio::spawn(async move {
        let mut buf = [0u8; 1024];

        loop {
            tokio::select! {
                result = socket.recv_from(&mut buf) => {
                    if let Ok((size, _)) = result {
                        let data = Bytes::copy_from_slice(&buf[0..size]);
                        let _ = sender.send(TunnelEvent::RemoteControlData(data)).await;
                    }
                }
                _ = &mut rx => {
                    break;
                }
            }
        }
        debug!("NAT-T listener stopped");
    });

    Ok(tx)
}
