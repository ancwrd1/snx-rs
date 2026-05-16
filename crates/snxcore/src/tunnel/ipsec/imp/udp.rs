use std::{net::SocketAddr, sync::Arc};

use anyhow::anyhow;
use futures::{
    SinkExt, StreamExt, TryStreamExt,
    channel::mpsc::{self},
};
use tokio::net::{UdpSocket, lookup_host};
use tracing::warn;

use crate::{
    model::{
        TunnelSession,
        params::{TransportType, TunnelParams},
    },
    tunnel::{
        GatewayConnector, TunnelCommand, TunnelEvent, VpnTunnel,
        ipsec::imp::tun::{PacketReceiver, PacketSender, TunIPsecTunnel},
    },
    util,
};

const CHANNEL_SIZE: usize = 1024;

fn make_channel(socket: UdpSocket, address: SocketAddr) -> (PacketSender, PacketReceiver) {
    let framed = tokio_util::udp::UdpFramed::new(socket, tokio_util::codec::BytesCodec::new());

    let (tx_in, rx_in) = mpsc::channel(CHANNEL_SIZE);
    let (tx_out, rx_out) = mpsc::channel(CHANNEL_SIZE);

    let channel = async move {
        let (mut sink, stream) = framed.split();

        let mut rx = rx_out.map(|v| Ok::<_, std::io::Error>((v, address)));
        let to_wire = sink.send_all(&mut rx);

        let mut tx = tx_in.sink_map_err(anyhow::Error::from);
        let from_wire = stream
            .map(|v| v.map(|v| v.0.freeze()))
            .map_err(Into::into)
            .forward(&mut tx);

        match futures::future::select(to_wire, from_wire).await {
            futures::future::Either::Left((res, _)) => {
                if let Err(e) = res {
                    warn!("IPsec UDP tx channel terminated: {e}");
                }
            }
            futures::future::Either::Right((res, _)) => {
                if let Err(e) = res {
                    warn!("IPsec UDP rx channel terminated: {e}");
                }
            }
        }
    };

    tokio::spawn(channel);

    (tx_out, rx_in)
}

pub(crate) struct UdpIPsecTunnel(TunIPsecTunnel);

impl UdpIPsecTunnel {
    pub(crate) async fn create(
        params: Arc<TunnelParams>,
        session: Arc<TunnelSession>,
        gateway_connector: Arc<dyn GatewayConnector + Send + Sync>,
    ) -> anyhow::Result<Self> {
        let socket = {
            use crate::platform::{NetworkInterface, Platform, PlatformAccess};
            let local_ip = Platform::get().new_network_interface().get_default_ipv4().await?;
            UdpSocket::bind(SocketAddr::from((local_ip, 0))).await?
        };

        let gateway_information = gateway_connector.get_gateway_information().await?;

        let address_str =
            util::server_name_with_port(&params.server_name, gateway_information.connectivity_info.natt_port);

        let address: SocketAddr = lookup_host(address_str.as_ref())
            .await?
            .next()
            .ok_or_else(|| anyhow!("Failed to resolve {}", address_str.as_ref()))?;

        socket.connect(address).await?;

        let (sender, receiver) = make_channel(socket, address);

        Ok(Self(
            TunIPsecTunnel::create(params, session, sender, receiver, TransportType::Udp, gateway_connector).await?,
        ))
    }
}

#[async_trait::async_trait]
impl VpnTunnel for UdpIPsecTunnel {
    async fn run(
        &mut self,
        command_receiver: tokio::sync::mpsc::Receiver<TunnelCommand>,
        event_sender: tokio::sync::mpsc::Sender<TunnelEvent>,
    ) -> anyhow::Result<()> {
        self.0.run(command_receiver, event_sender).await
    }
}
