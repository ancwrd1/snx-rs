use std::{
    borrow::Cow,
    fmt, fs,
    io::{Cursor, Write},
    net::{IpAddr, Ipv4Addr, ToSocketAddrs},
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

use anyhow::{Context, anyhow};
use base64::Engine;
use directories_next::ProjectDirs;
use i18n::tr;
use ipnet::Ipv4Net;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::util::{self, ipv4net_to_string, parse_ipv4_or_subnet};

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
            _ => Err(anyhow!(tr!("error-invalid-operation-mode"))),
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
        "TRAC"
    }

    pub fn as_client_mode(&self) -> &'static str {
        "secure_connect"
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
            _ => Err(anyhow!(tr!("error-invalid-tunnel-type"))),
        }
    }
}

impl fmt::Display for TunnelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ssl => write!(f, "SSL"),
            Self::Ipsec => write!(f, "IPSec"),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum CertType {
    #[default]
    None,
    Pkcs12,
    Pkcs8,
    Pkcs11,
}

impl CertType {
    pub fn as_u32(&self) -> u32 {
        match self {
            Self::None => 0,
            Self::Pkcs12 => 1,
            Self::Pkcs8 => 2,
            Self::Pkcs11 => 3,
        }
    }
}

impl From<u32> for CertType {
    fn from(value: u32) -> Self {
        match value {
            1 => Self::Pkcs12,
            2 => Self::Pkcs8,
            3 => Self::Pkcs11,
            _ => Self::None,
        }
    }
}

impl fmt::Display for CertType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::None => "none",
            Self::Pkcs12 => "pkcs12",
            Self::Pkcs8 => "pkcs8",
            Self::Pkcs11 => "pkcs11",
        };
        write!(f, "{s}")
    }
}

impl FromStr for CertType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(CertType::None),
            "pkcs12" => Ok(CertType::Pkcs12),
            "pkcs8" => Ok(CertType::Pkcs8),
            "pkcs11" => Ok(CertType::Pkcs11),
            _ => Err(anyhow!(tr!("error-invalid-cert-type"))),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum IconTheme {
    #[default]
    Auto,
    Dark,
    Light,
}

impl IconTheme {
    pub fn as_u32(&self) -> u32 {
        match self {
            Self::Auto => 0,
            Self::Dark => 1,
            Self::Light => 2,
        }
    }
}

impl From<u32> for IconTheme {
    fn from(value: u32) -> Self {
        match value {
            1 => Self::Dark,
            2 => Self::Light,
            _ => Self::Auto,
        }
    }
}

impl fmt::Display for IconTheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Auto => "auto",
            Self::Dark => "dark",
            Self::Light => "light",
        };
        write!(f, "{s}")
    }
}

impl FromStr for IconTheme {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(IconTheme::Auto),
            "dark" => Ok(IconTheme::Dark),
            "light" => Ok(IconTheme::Light),
            _ => Err(anyhow!(tr!("error-invalid-icon-theme"))),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum TransportType {
    #[default]
    Native,
    Udp,
    Tcpt,
}

impl fmt::Display for TransportType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Native => write!(f, "Native"),
            Self::Tcpt => write!(f, "TCPT"),
            Self::Udp => write!(f, "UDP"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelParams {
    pub server_name: String,
    pub user_name: String,
    pub password: String,
    pub password_factor: usize,
    pub log_level: String,
    pub search_domains: Vec<String>,
    pub ignore_search_domains: Vec<String>,
    pub dns_servers: Vec<Ipv4Addr>,
    pub ignore_dns_servers: Vec<Ipv4Addr>,
    pub default_route: bool,
    pub no_routing: bool,
    pub add_routes: Vec<Ipv4Net>,
    pub ignore_routes: Vec<Ipv4Net>,
    pub no_dns: bool,
    pub ignore_server_cert: bool,
    pub tunnel_type: TunnelType,
    pub ca_cert: Vec<PathBuf>,
    pub login_type: String,
    pub cert_type: CertType,
    pub cert_path: Option<PathBuf>,
    pub cert_password: Option<String>,
    pub cert_id: Option<String>,
    pub if_name: Option<String>,
    pub no_keychain: bool,
    pub ike_lifetime: Duration,
    pub ike_persist: bool,
    pub client_mode: String,
    pub no_keepalive: bool,
    pub icon_theme: IconTheme,
    pub set_routing_domains: bool,
    pub port_knock: bool,
    pub locale: Option<String>,
    pub auto_connect: bool,
    pub ip_lease_time: Option<Duration>,
    #[serde(skip)]
    pub config_file: PathBuf,
}

impl Default for TunnelParams {
    fn default() -> Self {
        Self {
            server_name: String::new(),
            user_name: String::new(),
            password: String::new(),
            password_factor: 1,
            log_level: "off".to_owned(),
            search_domains: Vec::new(),
            ignore_search_domains: Vec::new(),
            dns_servers: Vec::new(),
            ignore_dns_servers: Vec::new(),
            default_route: false,
            no_routing: false,
            add_routes: Vec::new(),
            ignore_routes: Vec::new(),
            no_dns: false,
            ignore_server_cert: false,
            tunnel_type: TunnelType::default(),
            ca_cert: Vec::new(),
            login_type: String::new(),
            cert_type: CertType::default(),
            cert_path: None,
            cert_password: None,
            cert_id: None,
            if_name: None,
            no_keychain: true,
            ike_lifetime: DEFAULT_IKE_LIFETIME,
            ike_persist: false,
            client_mode: TunnelType::Ipsec.as_client_mode().to_owned(),
            no_keepalive: false,
            icon_theme: IconTheme::default(),
            set_routing_domains: false,
            port_knock: false,
            locale: None,
            auto_connect: false,
            ip_lease_time: None,
            config_file: Self::default_config_path(),
        }
    }
}

impl TunnelParams {
    pub const IPSEC_KEEPALIVE_PORT: u16 = 18234;
    pub const DEFAULT_IPSEC_IF_NAME: &'static str = "snx-xfrm";
    pub const DEFAULT_SSL_IF_NAME: &'static str = "snx-tun";
    pub const DEFAULT_MTU: u16 = 1350;

    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let mut params = Self::default();
        let data = fs::read_to_string(&path)?;
        let config = util::parse_config(data)?;

        for (k, v) in config.into_iter() {
            match k.as_str() {
                "server-name" => params.server_name = v,
                "user-name" => params.user_name = v,
                "password" => params.password = v,
                "password-factor" => params.password_factor = v.parse().unwrap_or(1),
                "log-level" => params.log_level = v,
                "search-domains" => params.search_domains = v.split(',').map(|s| s.trim().to_owned()).collect(),
                "ignore-search-domains" => {
                    params.ignore_search_domains = v.split(',').map(|s| s.trim().to_owned()).collect();
                }
                "dns-servers" => params.dns_servers = v.split(',').flat_map(|s| s.trim().parse().ok()).collect(),
                "ignore-dns-servers" => {
                    params.ignore_dns_servers = v.split(',').flat_map(|s| s.trim().parse().ok()).collect();
                }
                "default-route" => params.default_route = v.parse().unwrap_or_default(),
                "no-routing" => params.no_routing = v.parse().unwrap_or_default(),
                "add-routes" => params.add_routes = v.split(',').flat_map(|s| parse_ipv4_or_subnet(s).ok()).collect(),
                "ignore-routes" => {
                    params.ignore_routes = v.split(',').flat_map(|s| parse_ipv4_or_subnet(s).ok()).collect();
                }
                "no-dns" => params.no_dns = v.parse().unwrap_or_default(),
                "ignore-server-cert" => params.ignore_server_cert = v.parse().unwrap_or_default(),
                "tunnel-type" => params.tunnel_type = v.parse().unwrap_or_default(),
                "ca-cert" => params.ca_cert = v.split(',').map(|s| s.trim().into()).collect(),
                "login-type" => params.login_type = v,
                "cert-type" => params.cert_type = v.parse().unwrap_or_default(),
                "cert-path" => params.cert_path = Some(v.into()),
                "cert-password" => params.cert_password = Some(v),
                "cert-id" => params.cert_id = Some(v),
                "if-name" => params.if_name = Some(v),
                "no-keychain" => params.no_keychain = v.parse().unwrap_or_default(),
                "ike-lifetime" => {
                    params.ike_lifetime = v.parse::<u64>().ok().map_or(DEFAULT_IKE_LIFETIME, Duration::from_secs);
                }
                "ike-persist" => params.ike_persist = v.parse().unwrap_or_default(),
                "no-keepalive" => params.no_keepalive = v.parse().unwrap_or_default(),
                "icon-theme" => params.icon_theme = v.parse().unwrap_or_default(),
                "client-mode" => params.client_mode = v,
                "set-routing-domains" => params.set_routing_domains = v.parse().unwrap_or_default(),
                "port-knock" => params.port_knock = v.parse().unwrap_or_default(),
                "locale" => params.locale = Some(v),
                "auto-connect" => params.auto_connect = v.parse().unwrap_or_default(),
                "ip-lease-time" => {
                    params.ip_lease_time = if !v.trim().is_empty() {
                        v.parse::<u64>().ok().map(Duration::from_secs)
                    } else {
                        None
                    };
                }
                other => {
                    warn!("Ignoring unknown option: {}", other);
                }
            }
        }
        path.as_ref().clone_into(&mut params.config_file);
        params.decode_password()?;

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
        writeln!(buf, "password-factor={}", self.password_factor)?;
        writeln!(buf, "search-domains={}", self.search_domains.join(","))?;
        writeln!(buf, "ignore-search-domains={}", self.ignore_search_domains.join(","))?;
        writeln!(
            buf,
            "dns-servers={}",
            self.dns_servers
                .iter()
                .map(|r| r.to_string())
                .collect::<Vec<_>>()
                .join(",")
        )?;
        writeln!(
            buf,
            "ignore-dns-servers={}",
            self.ignore_dns_servers
                .iter()
                .map(|r| r.to_string())
                .collect::<Vec<_>>()
                .join(",")
        )?;
        writeln!(buf, "default-route={}", self.default_route)?;
        writeln!(buf, "no-routing={}", self.no_routing)?;
        writeln!(
            buf,
            "add-routes={}",
            self.add_routes
                .iter()
                .map(|r| ipv4net_to_string(*r))
                .collect::<Vec<_>>()
                .join(",")
        )?;
        writeln!(
            buf,
            "ignore-routes={}",
            self.ignore_routes
                .iter()
                .map(|r| ipv4net_to_string(*r))
                .collect::<Vec<_>>()
                .join(",")
        )?;
        writeln!(buf, "no-dns={}", self.no_dns)?;
        writeln!(buf, "ignore-server-cert={}", self.ignore_server_cert)?;
        writeln!(buf, "tunnel-type={}", self.tunnel_type.as_str())?;
        writeln!(
            buf,
            "ca-cert={}",
            self.ca_cert
                .iter()
                .map(|r| format!("{}", r.display()))
                .collect::<Vec<_>>()
                .join(",")
        )?;
        writeln!(buf, "login-type={}", self.login_type)?;
        writeln!(buf, "cert-type={}", self.cert_type)?;
        if let Some(ref cert_path) = self.cert_path {
            writeln!(buf, "cert-path={}", cert_path.display())?;
        }
        if let Some(ref cert_password) = self.cert_password {
            writeln!(buf, "cert-password={cert_password}")?;
        }
        if let Some(ref cert_id) = self.cert_id {
            writeln!(buf, "cert-id={cert_id}")?;
        }
        if let Some(ref if_name) = self.if_name {
            writeln!(buf, "if-name={if_name}")?;
        }
        writeln!(buf, "no-keychain={}", self.no_keychain)?;
        writeln!(buf, "ike-lifetime={}", self.ike_lifetime.as_secs())?;
        writeln!(buf, "ike-persist={}", self.ike_persist)?;
        writeln!(buf, "log-level={}", self.log_level)?;
        writeln!(buf, "client-mode={}", self.client_mode)?;
        writeln!(buf, "no-keepalive={}", self.no_keepalive)?;
        writeln!(buf, "icon-theme={}", self.icon_theme)?;
        writeln!(buf, "set-routing-domains={}", self.set_routing_domains)?;
        writeln!(buf, "port-knock={}", self.port_knock)?;

        if let Some(ref locale) = self.locale {
            writeln!(buf, "locale={locale}")?;
        }

        writeln!(buf, "auto-connect={}", self.auto_connect)?;
        writeln!(
            buf,
            "ip-lease-time={}",
            self.ip_lease_time.map(|v| v.as_secs().to_string()).unwrap_or_default()
        )?;

        PathBuf::from(&self.config_file).parent().iter().for_each(|dir| {
            let _ = fs::create_dir_all(dir);
        });
        fs::write(&self.config_file, buf.into_inner())?;

        Ok(())
    }

    pub fn decode_password(&mut self) -> anyhow::Result<()> {
        if !self.password.is_empty() {
            self.password = String::from_utf8_lossy(&base64::engine::general_purpose::STANDARD.decode(&self.password)?)
                .into_owned();
        }
        Ok(())
    }

    pub fn default_config_dir() -> PathBuf {
        ProjectDirs::from("", "", "snx-rs")
            .expect("No home directory!")
            .config_dir()
            .to_owned()
    }

    pub fn default_config_path() -> PathBuf {
        Self::default_config_dir().join("snx-rs.conf")
    }

    pub fn server_name_with_port(&self, port: u16) -> Cow<'_, str> {
        if self.server_name.contains(':') {
            Cow::Borrowed(self.server_name.as_str())
        } else {
            Cow::Owned(format!("{}:{}", self.server_name, port))
        }
    }

    pub fn server_name_to_ipv4(&self, port: u16) -> anyhow::Result<Ipv4Addr> {
        let address = self
            .server_name_with_port(port)
            .to_socket_addrs()?
            .find_map(|addr| match addr.ip() {
                IpAddr::V4(v4) => Some(v4),
                IpAddr::V6(_) => None,
            })
            .context(format!("Cannot resolve {}", self.server_name))?;

        Ok(address)
    }
}
