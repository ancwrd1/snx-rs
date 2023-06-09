use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::anyhow;
use clap::Parser;
use serde::{Deserialize, Serialize};
use tracing::{metadata::LevelFilter, warn};

use crate::model::LoginType;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum OperationMode {
    #[default]
    Standalone,
    Command,
    Info,
}

impl FromStr for OperationMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "standalone" => Ok(Self::Standalone),
            "command" => Ok(Self::Command),
            "info" => Ok(Self::Info),
            _ => Err(anyhow!("Invalid operation mode!")),
        }
    }
}

#[derive(Parser)]
#[clap(about = "VPN client for Checkpoint security gateway", name = "snx-rs")]
pub struct CmdlineParams {
    #[clap(long = "server-name", short = 's', help = "Server name")]
    pub server_name: Option<String>,

    #[clap(
        long = "mode",
        short = 'm',
        default_value = "standalone",
        help = "Operation mode, one of: standalone, command, info"
    )]
    pub mode: OperationMode,

    #[clap(long = "user-name", short = 'u', help = "User name")]
    pub user_name: Option<String>,

    #[clap(long = "password", short = 'p', help = "Password in base64-encoded form")]
    pub password: Option<String>,

    #[clap(long = "config-file", short = 'c', help = "Read parameters from config file")]
    pub config_file: Option<PathBuf>,

    #[clap(
        long = "log-level",
        short = 'l',
        help = "Enable logging to stdout, one of: off, info, warn, error, debug, trace"
    )]
    pub log_level: Option<LevelFilter>,

    #[clap(
        long = "reauth",
        short = 'r',
        help = "Enable automatic re-authentication, SSL tunnel only"
    )]
    pub reauth: Option<bool>,

    #[clap(long = "search-domains", short = 'd', help = "Additional search domains")]
    pub search_domains: Vec<String>,

    #[clap(
        long = "default-route",
        short = 't',
        help = "Set the default route through the tunnel"
    )]
    pub default_route: Option<bool>,

    #[clap(long = "no-routing", short = 'n', help = "Do not change routing table")]
    pub no_routing: Option<bool>,

    #[clap(long = "no-dns", short = 'N', help = "Do not change DNS resolver configuration")]
    pub no_dns: Option<bool>,

    #[clap(
        long = "no-cert-check",
        short = 'H',
        help = "Do not validate server common name in the certificate"
    )]
    pub no_cert_check: Option<bool>,

    #[clap(long = "tunnel-type", short = 'e', help = "Tunnel type, one of: ssl, ipsec")]
    pub tunnel_type: Option<TunnelType>,

    #[clap(long = "ca-cert", short = 'k', help = "Custom CA cert file in PEM or DER format")]
    pub ca_cert: Option<PathBuf>,

    #[clap(
        long = "login-type",
        short = 'o',
        help = "Login type, one of: password, password-mfa, password-ms-auth (default), emergency-access, sso-azure"
    )]
    pub login_type: Option<LoginType>,
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum TunnelType {
    #[default]
    Ssl,
    Ipsec,
}

impl TunnelType {
    pub fn as_client_type(&self) -> &'static str {
        match self {
            TunnelType::Ssl => "SYMBIAN",
            TunnelType::Ipsec => "SYMBIAN",
        }
    }
}

impl FromStr for TunnelType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ssl" => Ok(TunnelType::Ssl),
            "ipsec" => Ok(TunnelType::Ipsec),
            _ => Err(anyhow!("Invalid tunnel type!")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelParams {
    pub server_name: String,
    pub user_name: String,
    pub password: String,
    pub log_level: String,
    pub reauth: bool,
    pub search_domains: Vec<String>,
    pub default_route: bool,
    pub no_routing: bool,
    pub no_dns: bool,
    pub no_cert_check: bool,
    pub tunnel_type: TunnelType,
    pub ca_cert: Option<PathBuf>,
    pub login_type: LoginType,
}

impl Default for TunnelParams {
    fn default() -> Self {
        Self {
            server_name: String::new(),
            user_name: String::new(),
            password: String::new(),
            log_level: "off".to_owned(),
            reauth: false,
            search_domains: Vec::new(),
            default_route: false,
            no_routing: false,
            no_dns: false,
            no_cert_check: false,
            tunnel_type: TunnelType::Ssl,
            ca_cert: None,
            login_type: LoginType::default(),
        }
    }
}

impl TunnelParams {
    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let mut params = Self::default();
        let data = std::fs::read_to_string(path)?;
        for line in data.lines() {
            if !line.trim().starts_with('#') {
                let parts = line
                    .split_once('=')
                    .map(|(k, v)| (k.trim(), v.trim().trim_matches('"')));

                if let Some((k, v)) = parts {
                    match k {
                        "server-name" => params.server_name = v.to_string(),
                        "user-name" => params.user_name = v.to_string(),
                        "password" => params.password = v.to_string(),
                        "log-level" => params.log_level = v.to_string(),
                        "reauth" => params.reauth = v.parse().unwrap_or_default(),
                        "search-domains" => params.search_domains = v.split(',').map(|s| s.trim().to_owned()).collect(),
                        "default-route" => params.default_route = v.parse().unwrap_or_default(),
                        "no-routing" => params.no_routing = v.parse().unwrap_or_default(),
                        "no-dns" => params.no_dns = v.parse().unwrap_or_default(),
                        "no-cert-check" => params.no_cert_check = v.parse().unwrap_or_default(),
                        "tunnel-type" => params.tunnel_type = v.parse().unwrap_or_default(),
                        "ca-cert" => params.ca_cert = Some(v.into()),
                        "login-type" => params.login_type = v.parse().ok().unwrap_or_default(),
                        other => {
                            warn!("Ignoring unknown option: {}", other);
                        }
                    }
                }
            }
        }
        Ok(params)
    }

    pub fn merge(&mut self, other: CmdlineParams) {
        if let Some(server_name) = other.server_name {
            self.server_name = server_name;
        }

        if let Some(user_name) = other.user_name {
            self.user_name = user_name;
        }

        if let Some(password) = other.password {
            self.password = password;
        }

        if let Some(reauth) = other.reauth {
            self.reauth = reauth;
        }

        if let Some(log_level) = other.log_level {
            self.log_level = log_level.to_string();
        }

        if !other.search_domains.is_empty() {
            self.search_domains = other.search_domains;
        }

        if let Some(default_route) = other.default_route {
            self.default_route = default_route;
        }

        if let Some(no_routing) = other.no_routing {
            self.no_routing = no_routing;
        }

        if let Some(no_dns) = other.no_dns {
            self.no_dns = no_dns;
        }

        if let Some(tunnel_type) = other.tunnel_type {
            self.tunnel_type = tunnel_type;
        }

        if let Some(ca_cert) = other.ca_cert {
            self.ca_cert = Some(ca_cert);
        }

        if let Some(no_cert_check) = other.no_cert_check {
            self.no_cert_check = no_cert_check;
        }

        if let Some(login_type) = other.login_type {
            self.login_type = login_type;
        }
    }
}
