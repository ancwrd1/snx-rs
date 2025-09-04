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

async fn parse_otp<R>(mut stream: R) -> anyhow::Result<Option<String>>
where
    R: AsyncBufReadExt + AsyncWriteExt + Unpin,
{
    static OTP_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(?<method>[A-Z]+) /(?<otp>\S+).*").unwrap());

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
                return Ok(None);
            }
            Some(method) if method.as_str() == "GET" => {
                debug!("OTP acquired from the browser");
                return Ok(Some(otp.as_str().to_owned()));
            }
            _ => {}
        }
    }
    Err(anyhow!(i18n::tr!("error-invalid-otp-reply")))
}

pub fn spawn_otp_listener(cancel_receiver: oneshot::Receiver<()>) -> oneshot::Receiver<anyhow::Result<String>> {
    let (sender, receiver) = oneshot::channel();

    let fut = async move {
        let tcp = TcpListener::bind("127.0.0.1:7779").await?;
        loop {
            let (stream, _) = tcp.accept().await?;

            if let Some(otp) = parse_otp(tokio::io::BufReader::new(stream)).await? {
                return Ok(otp);
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use tokio::io::{AsyncReadExt, BufReader};
    use tokio::net::TcpStream;

    async fn send_req(addr: SocketAddr, req: &[u8]) -> anyhow::Result<Vec<u8>> {
        let mut stream = TcpStream::connect(addr).await?;
        stream.write_all(req).await?;
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await?;

        Ok(buf)
    }

    #[tokio::test]
    async fn test_parse_otp() {
        let expected_otp = "b8ca70b1a762f044c2938f9ea2b5ff3db36807a53e0c6fcd3a938a7b7791".to_owned();
        let options_req = format!("OPTIONS /{} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n", expected_otp).into_bytes();
        let otp_req = format!("GET /{} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n", expected_otp).into_bytes();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let reply = String::from_utf8_lossy(&send_req(addr, &options_req).await.unwrap()).into_owned();
            assert!(reply.contains("Access-Control-Allow-Origin: *"));
            println!("{}", reply);

            let reply = String::from_utf8_lossy(&send_req(addr, &otp_req).await.unwrap()).into_owned();
            assert!(reply.contains("Access-Control-Allow-Origin: *"));
            println!("{}", reply);
        });

        let stream = listener.accept().await.unwrap().0;
        let otp = parse_otp(BufReader::new(stream)).await.unwrap();
        assert_eq!(otp, None);

        let mut stream = listener.accept().await.unwrap().0;
        let otp = parse_otp(BufReader::new(&mut stream)).await.unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await.unwrap();

        assert_eq!(otp, Some(expected_otp));
    }
}
