use std::{
    io::{stderr, stdin, IsTerminal},
    time::Duration,
};

use anyhow::anyhow;

pub const OTP_TIMEOUT: Duration = Duration::from_secs(120);

pub trait SecurePrompt {
    fn get_secure_input(&self, prompt: &str) -> anyhow::Result<String>;
    fn show_notification(&self, summary: &str, message: &str) -> anyhow::Result<()>;
}

pub struct TtyPrompt;

impl SecurePrompt for TtyPrompt {
    fn get_secure_input(&self, prompt: &str) -> anyhow::Result<String> {
        if stdin().is_terminal() && stderr().is_terminal() {
            Ok(passterm::prompt_password_stdin(Some(prompt), passterm::Stream::Stderr)?)
        } else {
            Err(anyhow!("No attached TTY to get user input!"))
        }
    }

    fn show_notification(&self, summary: &str, message: &str) -> anyhow::Result<()> {
        println!("{summary}: {message}");
        Ok(())
    }
}
