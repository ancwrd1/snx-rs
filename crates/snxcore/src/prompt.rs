use std::io::{IsTerminal, Write, stderr, stdin};

use anyhow::anyhow;

use crate::model::{PromptInfo, params::NotificationLevel};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum NotificationCategory {
    #[default]
    Info,
    Warning,
    Error,
}

pub trait SecurePrompt {
    fn get_secure_input(&self, prompt: PromptInfo) -> impl Future<Output = anyhow::Result<String>> + Send;

    fn get_plain_input(&self, prompt: PromptInfo) -> impl Future<Output = anyhow::Result<String>> + Send;

    fn show_notification(
        &self,
        summary: &str,
        message: &str,
        category: NotificationCategory,
        level: NotificationLevel,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}

pub struct TtyPrompt;

impl SecurePrompt for TtyPrompt {
    async fn get_secure_input(&self, prompt: PromptInfo) -> anyhow::Result<String> {
        tokio::task::spawn_blocking(move || {
            if stdin().is_terminal() && stderr().is_terminal() {
                if !prompt.header.is_empty() {
                    println!("{}", prompt.header);
                }

                Ok(passterm::prompt_password_stdin(
                    Some(&prompt.prompt_with_colon()),
                    passterm::Stream::Stderr,
                )?)
            } else {
                Err(anyhow!(i18n::tr!("error-no-tty")))
            }
        })
        .await?
    }

    async fn get_plain_input(&self, prompt: PromptInfo) -> anyhow::Result<String> {
        tokio::task::spawn_blocking(move || {
            if stdin().is_terminal() && stderr().is_terminal() {
                if !prompt.header.is_empty() {
                    println!("{}", prompt.header);
                }
                eprint!("{}", prompt.prompt_with_colon());
                stderr().flush()?;
                let mut line = String::new();
                stdin().read_line(&mut line)?;
                Ok(line.trim().to_owned())
            } else {
                Err(anyhow!(i18n::tr!("error-no-tty")))
            }
        })
        .await?
    }

    async fn show_notification(
        &self,
        summary: &str,
        message: &str,
        _category: NotificationCategory,
        _level: NotificationLevel,
    ) -> anyhow::Result<()> {
        println!("{summary}: {message}");
        Ok(())
    }
}
