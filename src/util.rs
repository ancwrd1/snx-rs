use std::{ffi::OsStr, fmt, future::Future, path::Path, process::Output};

use anyhow::anyhow;
use ipnet::{Ipv4Net, Ipv4Subnets};
use tokio::process::Command;
use tracing::trace;

use crate::model::proto::NetworkRange;

// reverse engineered from vendor snx utility
const XOR_TABLE: &[u8] = b"-ODIFIED&W0ROPERTY3HEET7ITH/+4HE3HEET)$3?,$!0?!5?02/0%24)%3.5,,\x10&7?70?/\"*%#43";

#[inline]
fn translate_byte(i: usize, c: u8) -> u8 {
    match (c % 255) ^ XOR_TABLE[i % XOR_TABLE.len()] {
        0 => 255,
        v => v,
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

    process_output(command.output().await?)
}

pub fn block_on<F, O>(f: F) -> O
where
    F: Future<Output = O>,
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(f)
}

pub fn ranges_to_subnets(ranges: &[NetworkRange]) -> impl Iterator<Item = Ipv4Net> + '_ {
    ranges.iter().map(|r| Ipv4Subnets::new(r.from, r.to, 0)).flatten()
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
