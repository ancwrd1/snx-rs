use std::{
    io::{Cursor, Write},
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

use anyhow::anyhow;
use base64::Engine;
use directories_next::ProjectDirs;
use ipnet::Ipv4Net;
use serde::{Deserialize, Serialize};
use tracing::warn;

const DEFAULT_ESP_LIFETIME: Duration = Duration::from_secs(3600);
const DEFAULT_IKE_LIFETIME: Duration = Duration::from_secs(28800);

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

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum TunnelType {
    #[default]
    Ipsec,
    Ssl,
}

impl TunnelType {
    pub fn as_client_type(&self) -> &'static str {
        "SYMBIAN"
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            TunnelType::Ipsec => "ipsec",
            TunnelType::Ssl => "ssl",
        }
    }
}

impl FromStr for TunnelType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ipsec" => Ok(TunnelType::Ipsec),
            "ssl" => Ok(TunnelType::Ssl),
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
    pub search_domains: Vec<String>,
    pub ignore_search_domains: Vec<String>,
    pub default_route: bool,
    pub no_routing: bool,
    pub add_routes: Vec<Ipv4Net>,
    pub ignore_routes: Vec<Ipv4Net>,
    pub no_dns: bool,
    pub no_cert_check: bool,
    pub ignore_server_cert: bool,
    pub tunnel_type: TunnelType,
    pub ca_cert: Option<PathBuf>,
    pub login_type: String,
    pub client_cert: Option<PathBuf>,
    pub cert_password: Option<String>,
    pub if_name: Option<String>,
    pub no_keychain: bool,
    pub server_prompt: bool,
    pub esp_lifetime: Duration,
    pub ike_lifetime: Duration,
    pub config_file: PathBuf,
}

impl Default for TunnelParams {
    fn default() -> Self {
        Self {
            server_name: String::new(),
            user_name: String::new(),
            password: String::new(),
            log_level: "off".to_owned(),
            search_domains: Vec::new(),
            ignore_search_domains: Vec::new(),
            default_route: false,
            no_routing: false,
            add_routes: Vec::new(),
            ignore_routes: Vec::new(),
            no_dns: false,
            no_cert_check: false,
            ignore_server_cert: false,
            tunnel_type: Default::default(),
            ca_cert: None,
            login_type: String::new(),
            client_cert: None,
            cert_password: None,
            if_name: None,
            no_keychain: false,
            server_prompt: true,
            esp_lifetime: DEFAULT_ESP_LIFETIME,
            ike_lifetime: DEFAULT_IKE_LIFETIME,
            config_file: Self::default_config_path(),
        }
    }
}

impl TunnelParams {
    pub const IPSEC_KEEPALIVE_PORT: u16 = 18234;
    pub const DEFAULT_IPSEC_IF_NAME: &'static str = "snx-xfrm";
    pub const DEFAULT_SSL_IF_NAME: &'static str = "snx-tun";

    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let mut params = Self::default();
        let data = std::fs::read_to_string(&path)?;
        for line in data.lines() {
            if !line.trim().starts_with('#') {
                let parts = line
                    .split_once('=')
                    .map(|(k, v)| (k.trim(), v.trim_matches(|c: char| c == '"' || c.is_whitespace())))
                    .and_then(|(k, v)| if v.is_empty() { None } else { Some((k, v)) });

                if let Some((k, v)) = parts {
                    let v = v.to_owned();
                    match k {
                        "server-name" => params.server_name = v,
                        "user-name" => params.user_name = v,
                        "password" => params.password = v,
                        "log-level" => params.log_level = v,
                        "search-domains" => params.search_domains = v.split(',').map(|s| s.trim().to_owned()).collect(),
                        "ignore-search-domains" => {
                            params.ignore_search_domains = v.split(',').map(|s| s.trim().to_owned()).collect()
                        }
                        "default-route" => params.default_route = v.parse().unwrap_or_default(),
                        "no-routing" => params.no_routing = v.parse().unwrap_or_default(),
                        "add-routes" => params.add_routes = v.split(',').flat_map(|s| s.trim().parse().ok()).collect(),
                        "ignore-routes" => {
                            params.ignore_routes = v.split(',').flat_map(|s| s.trim().parse().ok()).collect()
                        }
                        "no-dns" => params.no_dns = v.parse().unwrap_or_default(),
                        "no-cert-check" => params.no_cert_check = v.parse().unwrap_or_default(),
                        "ignore-server-cert" => params.ignore_server_cert = v.parse().unwrap_or_default(),
                        "tunnel-type" => params.tunnel_type = v.parse().unwrap_or_default(),
                        "ca-cert" => params.ca_cert = Some(v.into()),
                        "login-type" => params.login_type = v,
                        "client-cert" => params.client_cert = Some(v.into()),
                        "cert-password" => params.cert_password = Some(v),
                        "if-name" => params.if_name = Some(v),
                        "no-keychain" => params.no_keychain = v.parse().unwrap_or_default(),
                        "server-prompt" => params.server_prompt = v.parse().unwrap_or_default(),
                        "esp-lifetime" => {
                            params.esp_lifetime = v
                                .parse::<u64>()
                                .ok()
                                .map(Duration::from_secs)
                                .unwrap_or(DEFAULT_ESP_LIFETIME)
                        }
                        "ike-lifetime" => {
                            params.ike_lifetime = v
                                .parse::<u64>()
                                .ok()
                                .map(Duration::from_secs)
                                .unwrap_or(DEFAULT_IKE_LIFETIME)
                        }
                        other => {
                            warn!("Ignoring unknown option: {}", other);
                        }
                    }
                }
            }
        }
        params.config_file = path.as_ref().to_owned();
        Ok(params)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let mut buf = Cursor::new(Vec::new());
        writeln!(buf, "server-name={}", self.server_name)?;
        writeln!(buf, "user-name={}", self.user_name)?;
        writeln!(
            buf,
            "password={}",
            base64::engine::general_purpose::STANDARD.encode(&self.password)
        )?;
        writeln!(buf, "search-domains={}", self.search_domains.join(","))?;
        writeln!(buf, "ignore-search-domains={}", self.ignore_search_domains.join(","))?;
        writeln!(buf, "default-route={}", self.default_route)?;
        writeln!(buf, "no-routing={}", self.no_routing)?;
        writeln!(
            buf,
            "add-routes={}",
            self.add_routes
                .iter()
                .map(|r| r.to_string())
                .collect::<Vec<_>>()
                .join(",")
        )?;
        writeln!(
            buf,
            "ignore-routes={}",
            self.ignore_routes
                .iter()
                .map(|r| r.to_string())
                .collect::<Vec<_>>()
                .join(",")
        )?;
        writeln!(buf, "no-dns={}", self.no_dns)?;
        writeln!(buf, "no-cert-check={}", self.no_cert_check)?;
        writeln!(buf, "ignore-server-cert={}", self.ignore_server_cert)?;
        writeln!(buf, "tunnel-type={}", self.tunnel_type.as_str())?;
        if let Some(ref ca_cert) = self.ca_cert {
            writeln!(buf, "ca-cert={}", ca_cert.display())?;
        }
        writeln!(buf, "login-type={}", self.login_type)?;
        if let Some(ref client_cert) = self.client_cert {
            writeln!(buf, "client-cert={}", client_cert.display())?;
        }
        if let Some(ref cert_password) = self.cert_password {
            writeln!(buf, "cert-password={}", cert_password)?;
        }
        if let Some(ref if_name) = self.if_name {
            writeln!(buf, "if-name={}", if_name)?;
        }
        writeln!(buf, "no-keychain={}", self.no_keychain)?;
        writeln!(buf, "server-prompt={}", self.server_prompt)?;
        writeln!(buf, "esp-lifetime={}", self.esp_lifetime.as_secs())?;
        writeln!(buf, "ike-lifetime={}", self.ike_lifetime.as_secs())?;

        std::fs::write(&self.config_file, buf.into_inner())?;

        Ok(())
    }

    pub fn decode_password(&mut self) -> anyhow::Result<()> {
        if !self.password.is_empty() {
            self.password = String::from_utf8_lossy(&base64::engine::general_purpose::STANDARD.decode(&self.password)?)
                .into_owned();
        }
        Ok(())
    }

    pub fn default_config_path() -> PathBuf {
        let dir = ProjectDirs::from("", "", "snx-rs").expect("No home directory!");
        dir.config_dir().join("snx-rs.conf")
    }
}
