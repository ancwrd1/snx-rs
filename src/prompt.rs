use std::io::{stderr, stdin, IsTerminal};

use anyhow::anyhow;

pub fn get_input_from_tty(prompt: &str) -> anyhow::Result<String> {
    if stdin().is_terminal() && stderr().is_terminal() {
        Ok(passterm::prompt_password_stdin(Some(prompt), passterm::Stream::Stderr)?)
    } else {
        Err(anyhow!("No attached TTY to get user input!"))
    }
}
