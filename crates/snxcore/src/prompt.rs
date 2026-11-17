use std::io::{IsTerminal, Write, stderr, stdin};

use anyhow::anyhow;
use secrecy::SecretString;

use crate::model::{PromptInfo, params::NotificationLevel};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum NotificationCategory {
    #[default]
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveTarget {
    Keychain,
    /// The profile file, where the password is only base64-encoded.
    Profile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveOffer {
    pub target: SaveTarget,
    pub checked: bool,
}

#[derive(Debug, Clone, Default)]
pub struct CombinedInput {
    pub user_name: Option<String>,
    pub secret: SecretString,
    /// The user's answer to the "save credentials" choice, `None` if the prompt did not offer one.
    pub save: Option<bool>,
}

pub trait SecurePrompt {
    fn get_secure_input(&self, prompt: PromptInfo) -> impl Future<Output = anyhow::Result<String>> + Send;

    fn get_plain_input(&self, prompt: PromptInfo) -> impl Future<Output = anyhow::Result<String>> + Send;

    /// Asks for a secret, preceded by a username when `user_name_prompt` is given.
    /// `save` offers the "save credentials" choice: where the password would go and the initial state.
    /// `None` means it is not offered.
    /// Prompts that cannot offer the choice ignore it.
    fn get_combined_input(
        &self,
        prompt: PromptInfo,
        user_name_prompt: Option<PromptInfo>,
        save: Option<SaveOffer>,
    ) -> impl Future<Output = anyhow::Result<CombinedInput>> + Send;

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

    async fn get_combined_input(
        &self,
        prompt: PromptInfo,
        user_name_prompt: Option<PromptInfo>,
        _save: Option<SaveOffer>,
    ) -> anyhow::Result<CombinedInput> {
        if !prompt.header.is_empty() && stdin().is_terminal() && stderr().is_terminal() {
            println!("{}", prompt.header);
        }

        let user_name = match user_name_prompt {
            Some(user_name_prompt) => Some(
                self.get_plain_input(PromptInfo {
                    header: String::new(),
                    ..user_name_prompt
                })
                .await?,
            ),
            None => None,
        };

        let secret = self
            .get_secure_input(PromptInfo {
                header: String::new(),
                ..prompt
            })
            .await?;

        Ok(CombinedInput {
            user_name,
            secret: secret.into(),
            save: None,
        })
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
