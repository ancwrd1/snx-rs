use std::time::Duration;

use anyhow::anyhow;
use futures::future::Either;
use futures::{
    channel::mpsc::{self, Receiver, Sender},
    future, pin_mut, SinkExt, StreamExt, TryStreamExt,
};
use log::{debug, trace, warn};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_native_tls::native_tls::TlsConnector;
use tun::{Device, TunPacket};

use crate::auth::SnxHttpAuthenticator;
use crate::device::TunDevice;
use crate::{codec::SnxCodec, model::*, util};

pub type SnxPacketSender = Sender<SnxPacket>;
pub type SnxPacketReceiver = Receiver<SnxPacket>;

const CHANNEL_SIZE: usize = 1024;

fn make_channel<S>(stream: S) -> (SnxPacketSender, SnxPacketReceiver)
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let framed = tokio_util::codec::Framed::new(stream, SnxCodec);

    let (tx_in, rx_in) = mpsc::channel(CHANNEL_SIZE);
    let (tx_out, rx_out) = mpsc::channel(CHANNEL_SIZE);

    let channel = async move {
        let (mut sink, stream) = framed.split();

        let mut rx = rx_out.map(Ok::<_, anyhow::Error>);
        let to_wire = sink.send_all(&mut rx);

        let mut tx = tx_in.sink_map_err(anyhow::Error::from);
        let from_wire = stream.map_err(Into::into).forward(&mut tx);

        future::select(to_wire, from_wire).await;
    };

    tokio::spawn(channel);

    (tx_out, rx_in)
}

pub struct SnxClientBuilder {
    server_name: Option<String>,
    auth: Option<(String, String)>,
}

impl SnxClientBuilder {
    fn new() -> Self {
        Self {
            server_name: None,
            auth: None,
        }
    }

    pub fn auth<U, P>(&mut self, user_name: U, password: P) -> &mut Self
    where
        U: AsRef<str>,
        P: AsRef<str>,
    {
        self.auth = Some((user_name.as_ref().to_owned(), password.as_ref().to_owned()));
        self
    }

    pub fn server_name<S>(&mut self, server_name: S) -> &mut Self
    where
        S: AsRef<str>,
    {
        self.server_name = Some(server_name.as_ref().to_owned());
        self
    }

    pub fn build(self) -> SnxClient {
        SnxClient(self)
    }
}

pub struct SnxClient(SnxClientBuilder);

impl SnxClient {
    pub fn builder() -> SnxClientBuilder {
        SnxClientBuilder::new()
    }

    pub async fn connect(&self) -> anyhow::Result<SnxTunnel> {
        let server_name = self
            .0
            .server_name
            .clone()
            .ok_or_else(|| anyhow!("No server name specified!"))?;

        let auth = self
            .0
            .auth
            .clone()
            .ok_or_else(|| anyhow!("No authentication specified!"))?;

        debug!("Connecting to http endpoint: {}", server_name);
        let client = SnxHttpAuthenticator::new(server_name.clone(), auth);

        let server_response = client.connect().await?;

        let active_key = match (
            server_response.data.is_authenticated.as_str(),
            server_response.data.active_key,
        ) {
            ("true", Some(ref key)) => key.clone(),
            _ => {
                warn!("Authentication failed!");
                return Err(anyhow!("Authentication failed!"));
            }
        };

        let session_id = server_response.data.session_id.clone().unwrap_or_default();
        let cookie = util::decode_from_hex(active_key.as_bytes())?;
        let cookie = String::from_utf8_lossy(&cookie).into_owned();

        debug!(
            "Authentication OK, session id: {:?}, opening tunnel connection",
            session_id,
        );

        let tcp = tokio::net::TcpStream::connect((server_name.as_str(), 443)).await?;

        let tls: tokio_native_tls::TlsConnector = TlsConnector::builder().build()?.into();
        let stream = tls.connect(&server_name, tcp).await?;

        let (sender, receiver) = make_channel(stream);

        debug!("Tunnel connected");

        Ok(SnxTunnel {
            cookie,
            session_id,
            auth_timeout: Duration::default(),
            keepalive: Duration::default(),
            sender,
            receiver,
        })
    }
}

pub struct SnxTunnel {
    cookie: String,
    session_id: String,
    auth_timeout: Duration,
    keepalive: Duration,
    sender: SnxPacketSender,
    receiver: SnxPacketReceiver,
}

impl SnxTunnel {
    pub async fn client_hello(&mut self) -> anyhow::Result<HelloReply> {
        let req = ClientHello {
            client_version: "1".to_string(),
            protocol_version: "1".to_string(),
            protocol_minor_version: "1".to_string(),
            office_mode: OfficeMode {
                ipaddr: "0.0.0.0".to_string(),
                keep_address: Some("true".to_string()),
                dns_servers: None,
                dns_suffix: None,
            },
            optional: Some(OptionalRequest {
                client_type: "4".to_string(),
            }),
            cookie: self.cookie.clone(),
        };

        self.send(req).await?;

        let reply = self
            .receiver
            .next()
            .await
            .ok_or_else(|| anyhow!("Channel closed!"))?;

        let reply = match reply {
            SnxPacket::Control(name, value) if name == HelloReply::NAME => {
                let result = serde_json::from_value::<HelloReply>(value)?;
                self.auth_timeout = result
                    .timeouts
                    .authentication
                    .parse::<u64>()
                    .ok()
                    .map(Duration::from_secs)
                    .ok_or_else(|| anyhow!("Invalid auth timeout!"))?;
                self.keepalive = result
                    .timeouts
                    .keepalive
                    .parse::<u64>()
                    .ok()
                    .map(Duration::from_secs)
                    .ok_or_else(|| anyhow!("Invalid keepalive timeout!"))?;
                result
            }
            _ => return Err(anyhow!("Unexpected reply")),
        };

        Ok(reply)
    }

    async fn send<P>(&mut self, packet: P) -> anyhow::Result<()>
    where
        P: Into<SnxPacket>,
    {
        self.sender.send(packet.into()).await?;
        Ok(())
    }

    pub async fn run(self, tun: TunDevice) -> anyhow::Result<()> {
        debug!("Running tunnel for session {}", self.session_id);

        let dev_name = tun.inner.get_ref().name().to_owned();

        let (mut tun_sender, mut tun_receiver) = tun.inner.into_framed().split();

        let mut snx_receiver = self.receiver;

        let dev_name2 = dev_name.clone();

        tokio::spawn(async move {
            while let Some(item) = snx_receiver.next().await {
                match item {
                    SnxPacket::Control(name, _) => {
                        debug!("Control packet received: {}", name);
                    }
                    SnxPacket::Data(data) => {
                        trace!("snx => {}: {}", data.len(), dev_name2);
                        let tun_packet = TunPacket::new(data);
                        tun_sender.send(tun_packet).await?;
                    }
                }
            }
            Ok::<_, anyhow::Error>(())
        });

        let mut snx_sender = self.sender;

        loop {
            let ka_fut = tokio::time::sleep(self.keepalive);
            pin_mut!(ka_fut);

            let recv_fut = tun_receiver.next();
            pin_mut!(recv_fut);

            match futures::future::select(ka_fut, recv_fut).await {
                Either::Left((_, _)) => {
                    let req = KeepaliveRequest {
                        id: "0".to_string(),
                    };

                    snx_sender.send(req.into()).await?;
                }
                Either::Right((result, _)) => {
                    if let Some(Ok(item)) = result {
                        let data = item.into_bytes().to_vec();
                        trace!("{} => snx: {}", dev_name, data.len());
                        snx_sender.send(SnxPacket::Data(data)).await?;
                    } else {
                        break;
                    }
                }
            }
        }

        Ok(())
    }
}
