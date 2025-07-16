use std::{net::SocketAddr, sync::Arc};

use futures::{
    SinkExt, StreamExt, TryStreamExt,
    channel::mpsc::{self},
};
use tokio::net::UdpSocket;

use crate::{
    model::{
        VpnSession,
        params::{TransportType, TunnelParams},
    },
    server_info,
    tunnel::{
        TunnelCommand, TunnelEvent, VpnTunnel,
        ipsec::imp::tun::{PacketReceiver, PacketSender, TunIpsecTunnel},
    },
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

        futures::future::select(to_wire, from_wire).await;
    };

    tokio::spawn(channel);

    (tx_out, rx_in)
}

pub(crate) struct UdpIpsecTunnel(Box<TunIpsecTunnel>);

impl UdpIpsecTunnel {
    pub(crate) async fn create(params: Arc<TunnelParams>, session: Arc<VpnSession>) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        let server_info = server_info::get(&params).await?;

        let address = params.server_name_with_port(server_info.connectivity_info.natt_port);

        socket.connect(address.as_ref()).await?;

        let address = socket.peer_addr()?;
        let (sender, receiver) = make_channel(socket, address);

        Ok(Self(Box::new(
            TunIpsecTunnel::create(params, session, sender, receiver, TransportType::Udp).await?,
        )))
    }
}

#[async_trait::async_trait]
impl VpnTunnel for UdpIpsecTunnel {
    async fn run(
        mut self: Box<Self>,
        command_receiver: tokio::sync::mpsc::Receiver<TunnelCommand>,
        event_sender: tokio::sync::mpsc::Sender<TunnelEvent>,
    ) -> anyhow::Result<()> {
        self.0.run(command_receiver, event_sender).await
    }
}
