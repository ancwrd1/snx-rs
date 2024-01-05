use std::io::{stderr, stdin, IsTerminal, Write};

use anyhow::anyhow;

pub fn get_input_from_tty() -> anyhow::Result<String> {
    let stdin = stdin();
    let mut stderr = stderr();
    if stdin.is_terminal() && stderr.is_terminal() {
        eprint!("Enter challenge code: ");
        let _ = stderr.flush();
        let mut line = String::new();
        stdin.read_line(&mut line)?;
        Ok(line.trim().to_owned())
    } else {
        Err(anyhow!("No attached TTY to get user input!"))
    }
}
