use std::sync::Arc;

use futures::{
    channel::mpsc::{self},
    SinkExt, StreamExt, TryStreamExt,
};
use isakmp::transport::{
    tcpt::{TcptHandshaker, TcptTransportCodec},
    TcptDataType,
};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    model::{
        params::{TransportType, TunnelParams},
        VpnSession,
    },
    tunnel::{
        ipsec::imp::tun::{PacketReceiver, PacketSender, TunIpsecTunnel},
        TunnelCommand, TunnelEvent, VpnTunnel,
    },
};

const CHANNEL_SIZE: usize = 1024;

fn make_channel<S>(stream: S) -> (PacketSender, PacketReceiver)
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let framed = tokio_util::codec::Framed::new(stream, TcptTransportCodec::new(TcptDataType::Esp));

    let (tx_in, rx_in) = mpsc::channel(CHANNEL_SIZE);
    let (tx_out, rx_out) = mpsc::channel(CHANNEL_SIZE);

    let channel = async move {
        let (mut sink, stream) = framed.split();

        let mut rx = rx_out.map(Ok::<_, anyhow::Error>);
        let to_wire = sink.send_all(&mut rx);

        let mut tx = tx_in.sink_map_err(anyhow::Error::from);
        let from_wire = stream.map_err(Into::into).forward(&mut tx);

        futures::future::select(to_wire, from_wire).await;
    };

    tokio::spawn(channel);

    (tx_out, rx_in)
}

pub(crate) struct TcptIpsecTunnel(Box<TunIpsecTunnel>);

impl TcptIpsecTunnel {
    pub(crate) async fn create(params: Arc<TunnelParams>, session: Arc<VpnSession>) -> anyhow::Result<Self> {
        let mut tcp = tokio::net::TcpStream::connect((params.server_name.as_str(), 443)).await?;

        tcp.handshake(TcptDataType::Esp).await?;

        let (sender, receiver) = make_channel(tcp);

        Ok(Self(Box::new(
            TunIpsecTunnel::create(params, session, sender, receiver, TransportType::Tcpt).await?,
        )))
    }
}

#[async_trait::async_trait]
impl VpnTunnel for TcptIpsecTunnel {
    async fn run(
        mut self: Box<Self>,
        command_receiver: tokio::sync::mpsc::Receiver<TunnelCommand>,
        event_sender: tokio::sync::mpsc::Sender<TunnelEvent>,
    ) -> anyhow::Result<()> {
        self.0.run(command_receiver, event_sender).await
    }
}
