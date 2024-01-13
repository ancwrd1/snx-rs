use std::io::{stderr, stdin, IsTerminal};

use anyhow::anyhow;

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
