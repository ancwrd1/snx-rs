use std::io::{stderr, stdin, IsTerminal};
use std::time::Duration;

use anyhow::anyhow;
use once_cell::sync::Lazy;
use regex::Regex;
use tokio::sync::oneshot;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

pub const OTP_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum PromptSource {
    #[default]
    Tty,
    Gui,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SecurePrompt {
    source: PromptSource,
}

impl SecurePrompt {
    pub fn tty() -> Self {
        Self {
            source: PromptSource::Tty,
        }
    }

    pub fn gui() -> Self {
        Self {
            source: PromptSource::Gui,
        }
    }

    pub fn get_secure_input(&self, prompt: &str) -> anyhow::Result<String> {
        match self.source {
            PromptSource::Tty => get_input_from_tty(prompt),
            PromptSource::Gui => get_input_from_gui(prompt),
        }
    }

    pub fn show_notification(&self, summary: &str, message: &str) -> anyhow::Result<()> {
        match self.source {
            PromptSource::Tty => show_notification_tty(summary, message),
            PromptSource::Gui => show_notification_gui(summary, message),
        }
    }
}

fn get_input_from_tty(prompt: &str) -> anyhow::Result<String> {
    if stdin().is_terminal() && stderr().is_terminal() {
        Ok(passterm::prompt_password_stdin(Some(prompt), passterm::Stream::Stderr)?)
    } else {
        Err(anyhow!("No attached TTY to get user input!"))
    }
}

#[cfg(not(feature = "tray-icon"))]
fn get_input_from_gui(_prompt: &str) -> anyhow::Result<String> {
    Err(anyhow!("Not implemented"))
}

#[cfg(feature = "tray-icon")]
fn get_input_from_gui(prompt: &str) -> anyhow::Result<String> {
    let (cmd, args) = if let Ok(cmd) = which::which("zenity") {
        (
            cmd,
            vec![
                "--forms",
                "--add-password",
                prompt,
                "--text",
                "snx-rs: user input required",
            ],
        )
    } else if let Ok(cmd) = which::which("kdialog") {
        (cmd, vec!["--password", prompt, "snx-rs: user input required"])
    } else {
        return Err(anyhow!("No GUI prompts found!"));
    };

    tracing::debug!("Running command: {} with args: {:?}", cmd.display(), args);

    let output = std::process::Command::new(cmd).args(args).output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(anyhow!("Password not acquired"))
    }
}

fn show_notification_tty(summary: &str, message: &str) -> anyhow::Result<()> {
    println!("{}: {}", summary, message);
    Ok(())
}

#[cfg(feature = "tray-icon")]
fn show_notification_gui(summary: &str, message: &str) -> anyhow::Result<()> {
    Ok(std::thread::scope(|s| {
        s.spawn(|| crate::util::block_on(crate::platform::send_notification(summary, message)))
            .join()
            .unwrap()
    })?)
}

#[cfg(not(feature = "tray-icon"))]
fn show_notification_gui(_summary: &str, _message: &str) -> anyhow::Result<()> {
    Err(anyhow!("Not implemented"))
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
        }
    }
    Err(anyhow!("No OTP acquired!"))
}
