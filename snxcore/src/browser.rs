use std::time::Duration;

use anyhow::anyhow;
use bytes::Bytes;
use http_body_util::Empty;
use hyper::{Method, Request, Response, server::conn::http1, service::service_fn};
use hyper_util::rt::{TokioIo, TokioTimer};
use i18n::tr;
use tokio::{
    net::TcpListener,
    sync::{mpsc, oneshot},
};
use tracing::{debug, warn};

const OTP_TIMEOUT: Duration = Duration::from_secs(120);

pub trait BrowserController {
    fn open(&self, url: &str) -> anyhow::Result<()>;
    fn close(&self);
}

pub struct SystemBrowser;

impl BrowserController for SystemBrowser {
    fn open(&self, url: &str) -> anyhow::Result<()> {
        Ok(opener::open(url)?)
    }

    fn close(&self) {}
}

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
                .body(Empty::new())?)
        }

        _ => {
            warn!("Received unsupported request: {}", req.method());

            Ok(Response::builder().status(400).body(Empty::new())?)
        }
    }
}

async fn await_otp_internal(cancel_receiver: oneshot::Receiver<()>, tcp: TcpListener) -> anyhow::Result<String> {
    let (sender, mut receiver) = mpsc::channel(1);

    let fut = async move {
        let (stream, _) = tcp.accept().await?;

        http1::Builder::new()
            .timer(TokioTimer::new())
            .serve_connection(
                TokioIo::new(stream),
                service_fn(move |req| otp_handler(req, sender.clone())),
            )
            .await?;

        Ok::<_, anyhow::Error>(())
    };

    tokio::select! {
        _ = cancel_receiver => {
            warn!("OTP listener cancelled");
        }
        _ = fut => {
            warn!("OTP listener finished without receiving OTP");
        }
        result = tokio::time::timeout(OTP_TIMEOUT, receiver.recv()) => {
            if let Ok(Some(otp)) = result {
                return Ok(otp);
            }
        }
    }

    Err(anyhow!(tr!("error-otp-browser-failed")))
}

pub async fn await_otp(cancel_receiver: oneshot::Receiver<()>) -> anyhow::Result<String> {
    let tcp = TcpListener::bind("127.0.0.1:7779").await?;
    await_otp_internal(cancel_receiver, tcp).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    async fn send_req(addr: SocketAddr, method: Method, otp: &str) -> anyhow::Result<()> {
        let req = reqwest::Request::new(method.clone(), format!("http://{}/{}", addr, otp).parse()?);
        reqwest::Client::new().execute(req).await?.error_for_status()?;
        Ok(())
    }

    async fn test_otp_listener(method: Method) -> anyhow::Result<()> {
        let expected_otp = "1234567890";

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        let (_cancel_sender, cancel_receiver) = oneshot::channel();

        tokio::spawn(async move { send_req(addr, method, expected_otp).await });

        let otp = tokio::time::timeout(Duration::from_secs(1), await_otp_internal(cancel_receiver, listener)).await??;

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
