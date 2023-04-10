use std::time::{Duration, Instant};

use anyhow::anyhow;
use futures::{
    channel::mpsc::{self, Receiver, Sender},
    future, SinkExt, StreamExt, TryStreamExt,
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_native_tls::native_tls::TlsConnector;
use tracing::{debug, trace, warn};
use tun::{Device, TunPacket};

use crate::{auth::SnxHttpAuthenticator, codec::SnxCodec, device::TunDevice, model::*, util};

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
    reauth: bool,
}

impl SnxClientBuilder {
    fn new() -> Self {
        Self {
            server_name: None,
            auth: None,
            reauth: false,
        }
    }

    pub fn auth<U, P>(mut self, user_name: U, password: P) -> Self
    where
        U: AsRef<str>,
        P: AsRef<str>,
    {
        self.auth = Some((user_name.as_ref().to_owned(), password.as_ref().to_owned()));
        self
    }

    pub fn server_name<S>(mut self, server_name: S) -> Self
    where
        S: AsRef<str>,
    {
        self.server_name = Some(server_name.as_ref().to_owned());
        self
    }

    pub fn reauth(mut self, flag: bool) -> Self {
        self.reauth = flag;
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

    pub(crate) async fn authenticate(
        &self,
        session_id: Option<&str>,
    ) -> anyhow::Result<(String, String)> {
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
        let client = SnxHttpAuthenticator::new(server_name.clone(), auth.clone());

        let server_response = client.authenticate(session_id).await?;

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

        debug!("Authentication OK, session id: {}", session_id,);

        Ok((session_id, cookie))
    }

    pub async fn connect(&self) -> anyhow::Result<SnxTunnel> {
        let (session_id, cookie) = self.authenticate(None).await?;

        debug!("Creating TLS tunnel");

        let server_name = self.0.server_name.as_deref().unwrap();

        let tcp = tokio::net::TcpStream::connect((server_name, 443)).await?;

        let tls: tokio_native_tls::TlsConnector = TlsConnector::builder().build()?.into();
        let stream = tls.connect(&server_name, tcp).await?;

        let (sender, receiver) = make_channel(stream);

        debug!("Tunnel connected");

        Ok(SnxTunnel {
            server_name: server_name.to_owned(),
            auth: self.0.auth.clone().unwrap(),
            cookie,
            session_id,
            auth_timeout: Duration::default(),
            keepalive: Duration::default(),
            ip_address: "0.0.0.0".to_string(),
            sender,
            receiver: Some(receiver),
            reauth: self.0.reauth,
        })
    }
}

pub struct SnxTunnel {
    server_name: String,
    auth: (String, String),
    cookie: String,
    session_id: String,
    auth_timeout: Duration,
    keepalive: Duration,
    ip_address: String,
    sender: SnxPacketSender,
    receiver: Option<SnxPacketReceiver>,
    reauth: bool,
}

impl SnxTunnel {
    fn new_hello_request(&self, keep_address: bool) -> ClientHello {
        ClientHello {
            client_version: "1".to_string(),
            protocol_version: "1".to_string(),
            protocol_minor_version: "1".to_string(),
            office_mode: OfficeMode {
                ipaddr: self.ip_address.clone(),
                keep_address: Some(keep_address.to_string()),
                dns_servers: None,
                dns_suffix: None,
            },
            optional: Some(OptionalRequest {
                client_type: "4".to_string(),
            }),
            cookie: self.cookie.clone(),
        }
    }

    pub async fn client_hello(&mut self) -> anyhow::Result<HelloReply> {
        let req = self.new_hello_request(false);
        self.send(req).await?;

        let receiver = self.receiver.as_mut().unwrap();

        let reply = receiver
            .next()
            .await
            .ok_or_else(|| anyhow!("Channel closed!"))?;

        let reply = match reply {
            SnxPacket::Control(name, value) if name == HelloReply::NAME => {
                let result = serde_json::from_value::<HelloReply>(value)?;
                self.ip_address = result.office_mode.ipaddr.clone();
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

    async fn keepalive(&mut self) -> anyhow::Result<()> {
        let req = KeepaliveRequest {
            id: "0".to_string(),
        };

        self.send(req).await?;

        Ok(())
    }

    async fn reauth(&mut self) -> anyhow::Result<()> {
        let client = SnxClient::builder()
            .server_name(&self.server_name)
            .auth(&self.auth.0, &self.auth.1)
            .build();

        let (session_id, cookie) = client.authenticate(Some(&self.session_id)).await?;

        self.session_id = session_id;
        self.cookie = cookie;

        let req = self.new_hello_request(true);
        self.send(req).await?;

        Ok(())
    }

    async fn send<P>(&mut self, packet: P) -> anyhow::Result<()>
    where
        P: Into<SnxPacket>,
    {
        self.sender.send(packet.into()).await?;
        Ok(())
    }

    pub async fn run(mut self, tun: TunDevice) -> anyhow::Result<()> {
        debug!("Running tunnel for session {}", self.session_id);

        let dev_name = tun.inner.get_ref().name().to_owned();

        let (mut tun_sender, mut tun_receiver) = tun.inner.into_framed().split();

        let mut snx_receiver = self.receiver.take().unwrap();

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

        let mut now = Instant::now();

        loop {
            tokio::select! {
                _ = tokio::time::sleep(self.keepalive) => {
                    self.keepalive().await?;
                }

                result = tun_receiver.next() => {
                    if let Some(Ok(item)) = result {
                        let data = item.into_bytes().to_vec();
                        trace!("{} => snx: {}", dev_name, data.len());
                        self.send(data).await?;
                    } else {
                        break;
                    }
                }
            }

            if self.reauth && (Instant::now() - now) > self.auth_timeout {
                self.reauth().await?;
                now = Instant::now();
            }
        }

        Ok(())
    }
}
