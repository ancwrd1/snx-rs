use std::{net::SocketAddr, time::Duration};

use anyhow::anyhow;
use bytes::Bytes;
use http_body_util::Empty;
use hyper::{Method, Request, Response, server::conn::http1, service::service_fn};
use hyper_util::rt::{TokioIo, TokioTimer};
use i18n::tr;
use tokio::{
    net::{TcpListener, ToSocketAddrs},
    sync::mpsc,
};
use tracing::{debug, warn};

const OTP_TIMEOUT: Duration = Duration::from_secs(120);

async fn otp_handler(
    req: Request<impl hyper::body::Body>,
    sender: mpsc::Sender<String>,
) -> anyhow::Result<Response<Empty<Bytes>>> {
    match *req.method() {
        Method::GET | Method::OPTIONS => {
            let otp = req.uri().path().trim_start_matches('/');
            if otp.is_empty() {
                warn!("OTP not present in the request");
            } else {
                debug!("Successfully received OTP from the browser");
                let _ = sender.send(otp.to_owned()).await;
            }

            Ok(Response::builder()
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Allow-Methods", "GET, OPTIONS")
                .header("Connection", "close")
                .body(Empty::new())?)
        }

        _ => {
            warn!("Received unsupported request: {}", req.method());

            Ok(Response::builder().status(400).body(Empty::new())?)
        }
    }
}

pub struct OtpListener {
    tcp: TcpListener,
}

impl OtpListener {
    pub async fn new() -> anyhow::Result<Self> {
        Self::with_address("127.0.0.1:7779").await
    }

    pub async fn with_address<A>(address: A) -> anyhow::Result<Self>
    where
        A: ToSocketAddrs,
    {
        let tcp = TcpListener::bind(address).await?;
        Ok(Self { tcp })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.tcp.local_addr().unwrap()
    }

    pub async fn acquire_otp(&self) -> anyhow::Result<String> {
        let (stream, _) = self.tcp.accept().await?;
        let (sender, mut receiver) = mpsc::channel(1);

        tokio::spawn(async move {
            http1::Builder::new()
                .timer(TokioTimer::new())
                .serve_connection(TokioIo::new(stream), service_fn(|req| otp_handler(req, sender.clone())))
                .await?;

            Ok::<_, anyhow::Error>(())
        });

        match tokio::time::timeout(OTP_TIMEOUT, receiver.recv()).await {
            Ok(Some(otp)) => Ok(otp),
            _ => Err(anyhow!(tr!("error-otp-browser-failed"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, time::Duration};

    use reqwest::Method;

    use super::OtpListener;

    async fn send_req(addr: SocketAddr, method: Method, otp: &str) -> anyhow::Result<()> {
        let req = reqwest::Request::new(method.clone(), format!("http://{}/{}", addr, otp).parse()?);
        reqwest::Client::new().execute(req).await?.error_for_status()?;
        Ok(())
    }

    async fn test_otp_listener(method: Method) -> anyhow::Result<()> {
        let expected_otp = "1234567890";

        let listener = OtpListener::with_address("127.0.0.1:0").await?;
        let addr = listener.local_addr();

        tokio::spawn(async move { send_req(addr, method, expected_otp).await });

        let otp = tokio::time::timeout(Duration::from_secs(1), listener.acquire_otp()).await??;

        assert_eq!(otp, expected_otp);

        Ok(())
    }

    #[tokio::test]
    async fn test_otp_lister_get() {
        test_otp_listener(Method::GET).await.unwrap();
    }

    #[tokio::test]
    async fn test_otp_lister_options() {
        test_otp_listener(Method::OPTIONS).await.unwrap();
    }
}
