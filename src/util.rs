use std::{ffi::OsStr, fmt, path::Path, process::Output};

use anyhow::anyhow;
use tokio::process::Command;
use tracing::trace;

// reverse engineered from vendor snx utility
const TABLE: &[u8] = b"-ODIFIED&W0ROPERTY3HEET7ITH/+4HE3HEET)$3?,$!0?!5?02/0%24)%3.5,,\x10&7?70?/\"*%#43";

fn translate_byte(i: usize, c: u8) -> u8 {
    let mut c = if c == 0xff { 0 } else { c };
    c ^= TABLE[i % 77];

    if c == 0 {
        0xff
    } else {
        c
    }
}

fn translate<P: AsRef<[u8]>>(data: P) -> Vec<u8> {
    data.as_ref()
        .iter()
        .enumerate()
        .rev()
        .map(|(i, c)| translate_byte(i, *c))
        .collect::<Vec<u8>>()
}

pub fn snx_encrypt<P: AsRef<[u8]>>(data: P) -> String {
    hex::encode(translate(data))
}

pub fn snx_decrypt<D: AsRef<[u8]>>(data: D) -> anyhow::Result<Vec<u8>> {
    let mut unhexed = hex::decode(data)?;
    unhexed.reverse();

    let mut decoded = translate(unhexed);
    decoded.reverse();

    Ok(decoded)
}

fn process_output(output: Output) -> anyhow::Result<String> {
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(anyhow!(if !stderr.is_empty() {
            stderr
        } else {
            output.status.to_string()
        }))
    }
}

pub async fn run_command<C, I, T>(command: C, args: I) -> anyhow::Result<String>
where
    C: AsRef<Path> + fmt::Debug,
    I: IntoIterator<Item = T> + fmt::Debug,
    T: AsRef<OsStr>,
{
    trace!("Exec: {:?} {:?}", command, args);

    let mut command = Command::new(command.as_ref().as_os_str());
    command.envs(vec![("LANG", "C"), ("LC_ALL", "C")]).args(args);

    // call setuid on macOS for privileged commands
    #[cfg(target_os = "macos")]
    {
        if unsafe { libc::geteuid() == 0 } {
            command.uid(0);
        }
    }

    process_output(command.output().await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let username = "testuser";
        let secret = snx_encrypt(username.as_bytes());
        assert_eq!(secret, "36203a333d372a59");

        let decoded = snx_decrypt(secret.as_bytes()).unwrap();
        assert_eq!(decoded, b"testuser");
    }
}
