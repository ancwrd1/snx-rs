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
    sender: mpsc::Sender<anyhow::Result<String>>,
) -> anyhow::Result<Response<Empty<Bytes>>> {
    match *req.method() {
        Method::GET | Method::OPTIONS => {
            let otp = req.uri().path().trim_start_matches('/');
            if otp.is_empty() {
                warn!("OTP not present in the request");
            } else {
                debug!("Successfully received OTP from the browser");
                let _ = sender.send(Ok(otp.to_owned())).await;
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

fn spawn_otp_listener_internal(
    cancel_receiver: oneshot::Receiver<()>,
    tcp: TcpListener,
) -> mpsc::Receiver<anyhow::Result<String>> {
    let (sender, receiver) = mpsc::channel(1);
    let sender_copy = sender.clone();

    let fut = async move {
        let (stream, _) = tcp.accept().await?;

        let sender = sender.clone();

        http1::Builder::new()
            .timer(TokioTimer::new())
            .serve_connection(
                TokioIo::new(stream),
                service_fn(move |req| otp_handler(req, sender.clone())),
            )
            .await?;

        Ok::<_, anyhow::Error>(())
    };

    tokio::spawn(async move {
        tokio::select! {
            _ = cancel_receiver => {
                debug!("OTP listener cancelled");
            },
            _ = tokio::time::timeout(OTP_TIMEOUT, fut) => {
                debug!("OTP listener finished");
            }
        }
        let _ = sender_copy.send(Err(anyhow!(tr!("error-otp-browser-failed")))).await;
    });

    receiver
}

pub async fn spawn_otp_listener(
    cancel_receiver: oneshot::Receiver<()>,
) -> anyhow::Result<mpsc::Receiver<anyhow::Result<String>>> {
    let tcp = TcpListener::bind("127.0.0.1:7779").await?;
    Ok(spawn_otp_listener_internal(cancel_receiver, tcp))
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

        let mut receiver = spawn_otp_listener_internal(cancel_receiver, listener);

        send_req(addr, method, expected_otp).await?;

        let otp = tokio::time::timeout(Duration::from_secs(1), receiver.recv())
            .await?
            .unwrap()?;

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
