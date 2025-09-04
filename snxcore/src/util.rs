use std::{collections::HashMap, ffi::OsStr, fmt, future::Future, net::Ipv4Addr, path::Path, process::Output};

use anyhow::anyhow;
use cached::proc_macro::cached;
use ipnet::{Ipv4Net, Ipv4Subnets};
use itertools::Itertools;
use tokio::process::Command;
use tracing::trace;
use uuid::Uuid;

use crate::{
    model::{
        params::TunnelParams,
        proto::{LoginOption, NetworkRange},
    },
    platform::{Platform, PlatformAccess},
    server_info,
};

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
    ranges
        .iter()
        // skip default route
        .filter(|r| r.from != Ipv4Addr::new(0, 0, 0, 1))
        .flat_map(|r| Ipv4Subnets::new(r.from, r.to, 0))
}

pub async fn print_login_options(params: &TunnelParams) -> anyhow::Result<()> {
    let info = server_info::get(params).await?;

    let mut values = vec![
        ("login-options-server-address".to_owned(), params.server_name.clone()),
        (
            "login-options-server-ip".to_owned(),
            info.connectivity_info.server_ip.to_string(),
        ),
        (
            "login-options-client-enabled".to_owned(),
            info.connectivity_info.client_enabled.to_string(),
        ),
        (
            "login-options-supported-protocols".to_owned(),
            info.connectivity_info.supported_data_tunnel_protocols.join(", "),
        ),
        (
            "login-options-preferred-protocol".to_owned(),
            info.connectivity_info.connectivity_type,
        ),
        (
            "login-options-tcpt-port".to_owned(),
            info.connectivity_info.tcpt_port.to_string(),
        ),
        (
            "login-options-natt-port".to_owned(),
            info.connectivity_info.natt_port.to_string(),
        ),
    ];

    for fingerprint in info.connectivity_info.internal_ca_fingerprint.values() {
        values.push((
            "login-options-internal-ca-fingerprint".to_owned(),
            String::from_utf8_lossy(&snx_decrypt(fingerprint.as_bytes())?).into_owned(),
        ));
    }

    let mut options_list = info
        .login_options_data
        .map(|data| data.login_options_list)
        .unwrap_or_default();

    if options_list.is_empty() {
        options_list.insert(String::new(), LoginOption::unspecified());
    }

    for opt in options_list.into_values().filter(|opt| opt.show_realm != 0) {
        let factors = opt.factors.into_values().map(|factor| factor.factor_type).join(", ");
        values.push((format!("[{}]", opt.display_name), format!("{} ({})", opt.id, factors)));
    }

    let label_width = values
        .iter()
        .map(|(label, _)| {
            if label.starts_with("[") {
                label.chars().count()
            } else {
                i18n::translate(label).chars().count()
            }
        })
        .max()
        .unwrap_or_default();
    let mut result = String::new();
    for (index, (key, value)) in values.iter().enumerate() {
        let key_str = if key.starts_with("[") {
            key.clone()
        } else {
            i18n::translate(key)
        };
        result.push_str(&format!("{key_str:>label_width$}: {value}"));
        if index < values.len() - 1 {
            result.push('\n');
        }
    }

    println!("{result}");
    Ok(())
}

#[cached]
pub fn get_device_id() -> String {
    let machine_uuid = Platform::get().get_machine_uuid().unwrap_or_else(|_| Uuid::new_v4());
    Uuid::new_v5(&Uuid::NAMESPACE_OID, machine_uuid.as_bytes())
        .braced()
        .encode_upper(&mut Uuid::encode_buffer())
        .to_owned()
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

pub fn parse_ipv4_or_subnet(s: &str) -> anyhow::Result<Ipv4Net> {
    if let Ok(ip) = s.parse::<Ipv4Net>() {
        Ok(ip)
    } else {
        Ok(Ipv4Net::new(s.parse::<Ipv4Addr>()?, 32)?)
    }
}

pub fn ipv4net_to_string(ip: Ipv4Net) -> String {
    if ip.prefix_len() == 32 {
        ip.addr().to_string()
    } else {
        ip.to_string()
    }
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

    #[test]
    fn test_parse_range() {
        let ipaddr = "10.0.10.10".parse::<Ipv4Addr>().unwrap();
        let range = NetworkRange {
            from: "10.0.0.0".parse().unwrap(),
            to: "10.255.255.255".parse().unwrap(),
        };

        let subnets = Ipv4Subnets::new(range.from, range.to, 0);
        assert!(subnets.clone().any(|s| s.contains(&ipaddr)));

        for subnet in subnets {
            assert_eq!(subnet.to_string(), "10.0.0.0/8");
        }
    }

    #[test]
    fn test_parse_ipv4_or_subnet() {
        assert_eq!(
            parse_ipv4_or_subnet("10.0.0.1/8").unwrap(),
            Ipv4Net::new([10, 0, 0, 1].into(), 8).unwrap()
        );
        assert_eq!(
            parse_ipv4_or_subnet("10.0.0.2").unwrap(),
            Ipv4Net::new([10, 0, 0, 2].into(), 32).unwrap()
        );
    }

    #[test]
    fn test_ipv4net_to_string() {
        assert_eq!(
            ipv4net_to_string(Ipv4Net::new([10, 0, 0, 1].into(), 8).unwrap()),
            "10.0.0.1/8"
        );
        assert_eq!(
            ipv4net_to_string(Ipv4Net::new([10, 0, 0, 2].into(), 32).unwrap()),
            "10.0.0.2"
        );
    }
}
