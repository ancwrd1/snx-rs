use std::io::{IsTerminal, Write, stderr, stdin};

use anyhow::anyhow;
use i18n::tr;
use secrecy::{ExposeSecret, SecretString};

use crate::model::{
    MfaChallenge, PromptInfo,
    params::{NotificationLevel, TunnelParams},
};

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

/// Input for a password-type challenge, `factor` being its 1-based position in the login.
/// A password from the config or `stored_password` (the keychain) answers the password factor,
/// `mfa-code` answers the others; the prompt is skipped for them unless the username is needed.
pub async fn get_password_factor_input<P: SecurePrompt>(
    secure_prompt: &P,
    params: &TunnelParams,
    challenge: &MfaChallenge,
    mfa_prompts: &[PromptInfo],
    factor: usize,
    stored_password: &SecretString,
) -> anyhow::Result<CombinedInput> {
    let is_password_factor = factor == params.password_factor;

    let known_input = if is_password_factor {
        [&params.password, stored_password]
            .into_iter()
            .find(|p| !p.expose_secret().is_empty())
            .cloned()
    } else {
        params.mfa_code.as_deref().map(SecretString::from)
    };

    let ask_user_name = challenge.user_name_required && params.user_name.is_empty();

    if !ask_user_name && let Some(secret) = known_input {
        return Ok(CombinedInput {
            secret,
            ..Default::default()
        });
    }

    let mut prompt = mfa_prompts
        .get(factor - 1)
        .cloned()
        .unwrap_or_else(|| PromptInfo::new("", &challenge.prompt));

    let user_name_prompt = ask_user_name.then(|| {
        if prompt.header.is_empty() {
            prompt.header = tr!("label-username-required");
        }
        let mut user_name_prompt = PromptInfo::new("", tr!("label-username"));
        user_name_prompt.default_entry = std::env::var("USER").ok();
        user_name_prompt
    });

    let save = is_password_factor.then(|| SaveOffer {
        target: if params.keychain {
            SaveTarget::Keychain
        } else {
            SaveTarget::Profile
        },
        checked: params.keychain || !params.password.expose_secret().is_empty(),
    });

    secure_prompt.get_combined_input(prompt, user_name_prompt, save).await
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

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::model::MfaType;

    type Asked = (PromptInfo, Option<PromptInfo>, Option<SaveOffer>);

    #[derive(Default)]
    struct RecordingPrompt {
        asked: Mutex<Option<Asked>>,
    }

    impl SecurePrompt for RecordingPrompt {
        async fn get_secure_input(&self, _prompt: PromptInfo) -> anyhow::Result<String> {
            unreachable!()
        }

        async fn get_plain_input(&self, _prompt: PromptInfo) -> anyhow::Result<String> {
            unreachable!()
        }

        async fn get_combined_input(
            &self,
            prompt: PromptInfo,
            user_name_prompt: Option<PromptInfo>,
            save: Option<SaveOffer>,
        ) -> anyhow::Result<CombinedInput> {
            let user_name = user_name_prompt.as_ref().map(|_| "typed-user".to_owned());
            *self.asked.lock().unwrap() = Some((prompt, user_name_prompt, save));
            Ok(CombinedInput {
                user_name,
                secret: "typed".into(),
                save: None,
            })
        }

        async fn show_notification(
            &self,
            _summary: &str,
            _message: &str,
            _category: NotificationCategory,
            _level: NotificationLevel,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    fn challenge(user_name_required: bool) -> MfaChallenge {
        MfaChallenge {
            mfa_type: MfaType::PasswordInput,
            prompt: "Challenge".to_owned(),
            user_name_required,
        }
    }

    fn params(user_name: &str, password: &str, keychain: bool) -> TunnelParams {
        TunnelParams {
            user_name: user_name.to_owned(),
            password: password.into(),
            keychain,
            mfa_code: Some("123456".to_owned()),
            ..Default::default()
        }
    }

    async fn run(
        params: &TunnelParams,
        challenge: &MfaChallenge,
        factor: usize,
        stored: &str,
    ) -> (CombinedInput, Option<Asked>) {
        let prompt = RecordingPrompt::default();
        let prompts = [PromptInfo::new("Header", "Password: "), PromptInfo::new("", "Code: ")];
        let input = get_password_factor_input(&prompt, params, challenge, &prompts, factor, &stored.into())
            .await
            .unwrap();
        (input, prompt.asked.into_inner().unwrap())
    }

    #[tokio::test]
    async fn known_password_skips_prompt() {
        let (input, asked) = run(&params("user", "secret", false), &challenge(false), 1, "").await;
        assert!(asked.is_none());
        assert_eq!(input.secret.expose_secret(), "secret");
        assert_eq!(input.user_name, None);
    }

    #[tokio::test]
    async fn stored_password_skips_prompt() {
        let (input, asked) = run(&params("user", "", true), &challenge(false), 1, "stored").await;
        assert!(asked.is_none());
        assert_eq!(input.secret.expose_secret(), "stored");
    }

    #[tokio::test]
    async fn mfa_code_answers_other_factors() {
        let (input, asked) = run(&params("user", "secret", false), &challenge(false), 2, "").await;
        assert!(asked.is_none());
        assert_eq!(input.secret.expose_secret(), "123456");
    }

    #[tokio::test]
    async fn missing_user_name_prompts_despite_known_password() {
        let (input, asked) = run(&params("", "secret", false), &challenge(true), 1, "").await;
        let (prompt, user_name_prompt, save) = asked.unwrap();
        assert_eq!(prompt.prompt, "Password: ");
        assert!(user_name_prompt.is_some());
        assert_eq!(
            save,
            Some(SaveOffer {
                target: SaveTarget::Profile,
                checked: true
            })
        );
        assert_eq!(input.user_name.as_deref(), Some("typed-user"));
        assert_eq!(input.secret.expose_secret(), "typed");
    }

    #[tokio::test]
    async fn user_name_prompt_gets_default_header() {
        let mut params = params("", "", true);
        params.mfa_code = None;
        let (_, asked) = run(&params, &challenge(true), 2, "").await;
        let (prompt, user_name_prompt, save) = asked.unwrap();
        assert_eq!(prompt.prompt, "Code: ");
        assert_eq!(prompt.header, tr!("label-username-required"));
        assert!(user_name_prompt.is_some());
        assert_eq!(save, None);
    }

    #[tokio::test]
    async fn configured_user_name_is_not_asked() {
        let (_, asked) = run(&params("user", "", true), &challenge(true), 1, "").await;
        let (prompt, user_name_prompt, save) = asked.unwrap();
        assert_eq!(prompt.header, "Header");
        assert!(user_name_prompt.is_none());
        assert_eq!(
            save,
            Some(SaveOffer {
                target: SaveTarget::Keychain,
                checked: true
            })
        );
    }

    #[tokio::test]
    async fn falls_back_to_challenge_prompt() {
        let mut params = params("user", "", false);
        params.mfa_code = None;
        let (_, asked) = run(&params, &challenge(false), 3, "").await;
        let (prompt, _, save) = asked.unwrap();
        assert_eq!(prompt.prompt, "Challenge");
        assert_eq!(save, None);
    }
}
