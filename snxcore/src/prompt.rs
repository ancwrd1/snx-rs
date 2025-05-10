use std::io::{IsTerminal, Write, stderr, stdin};

use anyhow::anyhow;

use crate::model::PromptInfo;

#[async_trait::async_trait]
pub trait SecurePrompt {
    async fn get_secure_input(&self, prompt: PromptInfo) -> anyhow::Result<String>;

    async fn get_plain_input(&self, prompt: PromptInfo) -> anyhow::Result<String>;

    async fn show_notification(&self, summary: &str, message: &str) -> anyhow::Result<()>;
}

pub struct TtyPrompt;

#[async_trait::async_trait]
impl SecurePrompt for TtyPrompt {
    async fn get_secure_input(&self, prompt: PromptInfo) -> anyhow::Result<String> {
        Ok(tokio::task::spawn_blocking(move || {
            if stdin().is_terminal() && stderr().is_terminal() {
                if !prompt.header.is_empty() {
                    println!("{}", prompt.header);
                }
                Ok(passterm::prompt_password_stdin(
                    Some(&prompt.prompt),
                    passterm::Stream::Stderr,
                )?)
            } else {
                Err(anyhow!(i18n::tr!("error-no-tty")))
            }
        })
        .await??)
    }

    async fn get_plain_input(&self, prompt: PromptInfo) -> anyhow::Result<String> {
        Ok(tokio::task::spawn_blocking(move || {
            if stdin().is_terminal() && stderr().is_terminal() {
                if !prompt.header.is_empty() {
                    println!("{}", prompt.header);
                }
                eprint!("{}", prompt.prompt);
                stderr().flush()?;
                let mut line = String::new();
                stdin().read_line(&mut line)?;
                Ok(line.trim().to_owned())
            } else {
                Err(anyhow!(i18n::tr!("error-no-tty")))
            }
        })
        .await??)
    }

    async fn show_notification(&self, summary: &str, message: &str) -> anyhow::Result<()> {
        println!("{summary}: {message}");
        Ok(())
    }
}
