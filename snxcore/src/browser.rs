use std::time::Duration;

use anyhow::anyhow;
use once_cell::sync::Lazy;
use regex::Regex;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
};
use tracing::debug;

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

pub fn spawn_otp_listener(cancel_receiver: oneshot::Receiver<()>) -> oneshot::Receiver<anyhow::Result<String>> {
    static OTP_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(?<method>[A-Z]+) /(?<otp>\S+).*").unwrap());

    let (sender, receiver) = oneshot::channel();

    let fut = async move {
        let tcp = TcpListener::bind("127.0.0.1:7779").await?;
        loop {
            let (stream, _) = tcp.accept().await?;
            let mut stream = tokio::io::BufReader::new(stream);

            let mut data = String::new();
            tokio::time::timeout(Duration::from_secs(5), stream.read_line(&mut data)).await??;

            if let Some(captures) = OTP_RE.captures(&data)
                && let Some(otp) = captures.name("otp")
            {
                let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, OPTIONS\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;
                let _ = stream.shutdown().await;

                match captures.name("method") {
                    Some(method) if method.as_str() == "OPTIONS" => {
                        debug!("Browser CORS preflight check detected");
                        continue;
                    }
                    Some(method) if method.as_str() == "GET" => {
                        debug!("OTP acquired from the browser");
                        return Ok(otp.as_str().to_owned());
                    }
                    _ => {}
                }
            }
            break Err(anyhow!(i18n::tr!("error-invalid-otp-reply")));
        }
    };

    tokio::spawn(async move {
        tokio::select! {
            _ = cancel_receiver => {},
            result = tokio::time::timeout(OTP_TIMEOUT, fut) => {
                let _ = sender.send(result.unwrap_or_else(|e| Err(e.into())));
            }
        }
    });

    receiver
}
