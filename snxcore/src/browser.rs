use anyhow::anyhow;
use once_cell::sync::Lazy;
use regex::Regex;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
};

pub trait BrowserController {
    fn open(&self, url: &str) -> anyhow::Result<()>;
    fn close(&self);
}

pub async fn run_otp_listener(sender: oneshot::Sender<String>) -> anyhow::Result<()> {
    static OTP_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"^GET /(?<otp>[0-9a-f]{60}|[0-9A-F]{60}).*"#).unwrap());

    let tcp = TcpListener::bind("127.0.0.1:7779").await?;
    let (mut stream, _) = tcp.accept().await?;

    let mut buf = [0u8; 65];
    stream.read_exact(&mut buf).await?;

    let mut data = String::from_utf8_lossy(&buf).into_owned();

    while stream.read(&mut buf[0..1]).await.is_ok() && buf[0] != b'\n' && buf[0] != b'\r' {
        data.push(buf[0].into());
    }

    let _ = stream.shutdown().await;
    drop(stream);
    drop(tcp);

    if let Some(captures) = OTP_RE.captures(&data) {
        if let Some(otp) = captures.name("otp") {
            let _ = sender.send(otp.as_str().to_owned());
            return Ok(());
        }
    }
    Err(anyhow!("No OTP acquired!"))
}
