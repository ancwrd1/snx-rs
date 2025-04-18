use crate::model::PromptInfo;
use anyhow::anyhow;
use std::io::Write;
use std::io::{stderr, stdin, IsTerminal};

pub trait SecurePrompt {
    fn get_secure_input(&self, prompt: &PromptInfo) -> anyhow::Result<String>;

    fn get_plain_input(&self, prompt: &PromptInfo) -> anyhow::Result<String>;

    fn show_notification(&self, summary: &str, message: &str) -> anyhow::Result<()>;
}

pub struct TtyPrompt;

impl SecurePrompt for TtyPrompt {
    fn get_secure_input(&self, prompt: &PromptInfo) -> anyhow::Result<String> {
        if stdin().is_terminal() && stderr().is_terminal() {
            if !prompt.header.is_empty() {
                println!("{}", prompt.header);
            }
            Ok(passterm::prompt_password_stdin(
                Some(&prompt.prompt),
                passterm::Stream::Stderr,
            )?)
        } else {
            Err(anyhow!("No attached TTY to get user input!"))
        }
    }

    fn get_plain_input(&self, prompt: &PromptInfo) -> anyhow::Result<String> {
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
            Err(anyhow!("No attached TTY to get user input!"))
        }
    }

    fn show_notification(&self, summary: &str, message: &str) -> anyhow::Result<()> {
        println!("{summary}: {message}");
        Ok(())
    }
}
