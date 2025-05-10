use std::time::Duration;

use anyhow::anyhow;
use once_cell::sync::Lazy;
use regex::Regex;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
};

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
    static OTP_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^GET /(?<otp>[0-9a-f]{60}|[0-9A-F]{60}).*").unwrap());

    let (sender, receiver) = oneshot::channel();

    let fut = async move {
        let tcp = TcpListener::bind("127.0.0.1:7779").await?;
        let (mut stream, _) = tcp.accept().await?;

        let mut buf = [0u8; 65];
        stream.read_exact(&mut buf).await?;

        let mut data = String::from_utf8_lossy(&buf).into_owned();

        while stream.read(&mut buf[0..1]).await.is_ok() && buf[0] != b'\n' && buf[0] != b'\r' {
            data.push(buf[0].into());
        }

        let _ = stream.shutdown().await;

        if let Some(captures) = OTP_RE.captures(&data) {
            if let Some(otp) = captures.name("otp") {
                return Ok(otp.as_str().to_owned());
            }
        }
        Err(anyhow!(i18n::tr!("error-invalid-otp-reply")))
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
