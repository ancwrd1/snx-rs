use anyhow::{anyhow, Context};
use ipnet::{Ipv4Net, Ipv4Subnets};
use std::collections::HashMap;
use std::{
    ffi::OsStr,
    fmt,
    future::Future,
    net::{IpAddr, Ipv4Addr, ToSocketAddrs},
    path::Path,
    process::Output,
};
use tokio::process::Command;
use tracing::trace;
use uuid::Uuid;

use crate::model::params::TunnelParams;
use crate::model::proto::LoginDisplayLabelSelect;
use crate::{model::proto::NetworkRange, server_info};

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

fn process_output(output: &Output) -> anyhow::Result<String> {
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(anyhow!(if stderr.is_empty() {
            output.status.to_string()
        } else {
            stderr
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

    process_output(&command.output().await?)
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
    ranges.iter().flat_map(|r| Ipv4Subnets::new(r.from, r.to, 0))
}

pub async fn print_login_options(params: &TunnelParams) -> anyhow::Result<()> {
    let info = server_info::get(params).await?;

    println!("Server address: {}", params.server_name);
    println!("Server IP: {}", info.connectivity_info.server_ip);
    println!(
        "Supported tunnel protocols: {}",
        info.connectivity_info.supported_data_tunnel_protocols.join(", ")
    );

    println!("Connectivity type: {}", info.connectivity_info.connectivity_type);
    println!("TCPT port: {}", info.connectivity_info.tcpt_port);
    println!("NATT port: {}", info.connectivity_info.natt_port);

    println!(
        "Internal CA fingerprint: {}",
        String::from_utf8_lossy(&snx_decrypt(
            info.connectivity_info
                .internal_ca_fingerprint
                .values()
                .cloned()
                .collect::<Vec<String>>()
                .join(" ")
                .as_bytes()
        )?)
    );

    if let Some(login_options_data) = info.login_options_data {
        println!("Available login types:");

        for opt in login_options_data.login_options_list.values() {
            println!("\t{} ({})", opt.id, opt.display_name);

            for (index, factor) in opt.factors.values().enumerate() {
                if let LoginDisplayLabelSelect::LoginDisplayLabel(ref labels) = factor.custom_display_labels {
                    let prompt = labels
                        .get("password")
                        .map(|p| format!(", prompt = \"{}\"", p))
                        .unwrap_or_default();

                    println!("\t\tfactor {}: type = {}{}", index + 1, factor.factor_type, prompt);
                } else {
                    println!("\t\tfactor {}: type = {}", index + 1, factor.factor_type);
                }
            }
        }
    }

    Ok(())
}

pub fn get_device_id() -> String {
    let machine_uuid = crate::platform::get_machine_uuid().unwrap_or_else(|_| Uuid::new_v4());
    Uuid::new_v5(&Uuid::NAMESPACE_OID, machine_uuid.as_bytes())
        .braced()
        .encode_upper(&mut Uuid::encode_buffer())
        .to_owned()
}

pub fn resolve_ipv4_host(server_name: &str) -> anyhow::Result<Ipv4Addr> {
    let address = server_name
        .to_socket_addrs()?
        .find_map(|addr| match addr.ip() {
            IpAddr::V4(v4) => Some(v4),
            IpAddr::V6(_) => None,
        })
        .context(format!("Cannot resolve {}", server_name))?;

    Ok(address)
}

pub fn parse_config<S: AsRef<str>>(config: S) -> anyhow::Result<HashMap<String, String>> {
    let mut result = HashMap::new();

    for line in config.as_ref().lines() {
        let (line, _) = line.split_once('#').unwrap_or((line, ""));

        let parts = line
            .split_once('=')
            .map(|(k, v)| (k.trim(), v.trim_matches(|c: char| c == '"' || c.is_whitespace())))
            .and_then(|(k, v)| if v.is_empty() { None } else { Some((k, v)) });

        if let Some((k, v)) = parts {
            result.insert(k.to_owned(), v.to_owned());
        };
    }

    Ok(result)
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

    #[test]
    fn test_parse_config() {
        let config = "# comment 1\nfoo = bar #comment 2\nbaz # = bar\nnoparam\npar1 = val1";
        let parsed = parse_config(config).unwrap();
        assert_eq!(
            parsed,
            HashMap::from([
                ("foo".to_owned(), "bar".to_owned()),
                ("par1".to_owned(), "val1".to_owned())
            ])
        );
    }
}
