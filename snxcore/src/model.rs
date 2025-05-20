use std::{fmt, net::Ipv4Addr, sync::Arc, time::Duration};

use chrono::{DateTime, Local};
use ipnet::Ipv4Net;
use isakmp::model::EspCryptMaterial;
use serde::{Deserialize, Serialize};

use crate::model::params::{TransportType, TunnelParams, TunnelType};

pub mod params;
pub mod proto;
pub mod wrappers;

#[derive(Debug, Default, Clone, PartialEq)]
pub enum SessionState {
    #[default]
    NoState,
    Authenticated(String),
    PendingChallenge(MfaChallenge),
}

#[derive(Debug, Clone, PartialEq)]
pub struct IpsecSession {
    pub lifetime: Duration,
    pub address: Ipv4Addr,
    pub netmask: Ipv4Addr,
    pub dns: Vec<Ipv4Addr>,
    pub domains: Vec<String>,
    pub esp_in: Arc<EspCryptMaterial>,
    pub esp_out: Arc<EspCryptMaterial>,
    pub transport_type: TransportType,
    pub address_lifetime: Duration,
}

impl Default for IpsecSession {
    fn default() -> Self {
        Self {
            lifetime: Duration::default(),
            address: Ipv4Addr::new(0, 0, 0, 0),
            netmask: Ipv4Addr::new(0, 0, 0, 0),
            dns: Vec::new(),
            domains: Vec::new(),
            esp_in: Arc::default(),
            esp_out: Arc::default(),
            transport_type: TransportType::default(),
            address_lifetime: Duration::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VpnSession {
    pub ccc_session_id: String,
    pub ipsec_session: Option<IpsecSession>,
    pub state: SessionState,
    pub username: Option<String>,
}

impl VpnSession {
    pub fn empty() -> Self {
        Self {
            ccc_session_id: String::new(),
            ipsec_session: None,
            state: SessionState::default(),
            username: None,
        }
    }

    pub fn active_key(&self) -> &str {
        match self.state {
            SessionState::Authenticated(ref active_key) => active_key.as_str(),
            _ => "",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, PartialOrd)]
pub enum MfaType {
    #[default]
    PasswordInput,
    IdentityProvider,
    UserNameInput,
}

impl MfaType {
    pub fn from_id(id: &str) -> Self {
        if id == "CPSC_SP_URL" {
            Self::IdentityProvider
        } else {
            Self::PasswordInput
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, PartialOrd)]
pub struct MfaChallenge {
    pub mfa_type: MfaType,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ConnectionInfo {
    pub since: Option<DateTime<Local>>,
    pub server_name: String,
    pub username: String,
    pub login_type: String,
    pub tunnel_type: TunnelType,
    pub transport_type: TransportType,
    pub ip_address: Ipv4Net,
    pub dns_servers: Vec<Ipv4Addr>,
    pub search_domains: Vec<String>,
    pub interface_name: String,
    pub dns_configured: bool,
    pub routing_configured: bool,
    pub default_route: bool,
}

impl ConnectionInfo {
    fn is_connected(&self) -> bool {
        self.since.is_some()
    }

    fn or_empty<F>(&self, f: F) -> String
    where
        F: Fn() -> String,
    {
        if self.is_connected() { f() } else { String::new() }
    }

    pub fn to_values(&self) -> Vec<(&'static str, String)> {
        vec![
            (
                "info-connected-since",
                if let Some(ref since) = self.since {
                    since.format("%Y-%m-%d %H:%M:%S").to_string()
                } else {
                    String::new()
                },
            ),
            ("info-server-name", self.or_empty(|| self.server_name.clone())),
            ("info-user-name", self.or_empty(|| self.username.clone())),
            ("info-login-type", self.or_empty(|| self.login_type.clone())),
            ("info-tunnel-type", self.or_empty(|| self.tunnel_type.to_string())),
            ("info-transport-type", self.or_empty(|| self.transport_type.to_string())),
            ("info-ip-address", self.or_empty(|| self.ip_address.to_string())),
            ("info-dns-servers", self.or_empty(|| format!("{:?}", self.dns_servers))),
            (
                "info-search-domains",
                self.or_empty(|| format!("[{}]", self.search_domains.join(", "))),
            ),
            ("info-interface", self.or_empty(|| self.interface_name.clone())),
            ("info-dns-configured", self.or_empty(|| self.dns_configured.to_string())),
            (
                "info-routing-configured",
                self.or_empty(|| self.routing_configured.to_string()),
            ),
            ("info-default-route", self.or_empty(|| self.default_route.to_string())),
        ]
    }

    pub fn print(&self) -> String {
        let values = self.to_values();
        let label_width = values
            .iter()
            .map(|(label, _)| i18n::translate(label).chars().count())
            .max()
            .unwrap_or_default();
        let mut result = String::new();
        for (index, (key, value)) in values.iter().enumerate() {
            result.push_str(&format!("{:>label_width$}: {}", i18n::translate(key), value));
            if index < values.len() - 1 {
                result.push('\n');
            }
        }
        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected(ConnectionInfo),
    Mfa(MfaChallenge),
}

impl ConnectionStatus {
    pub fn connected(info: ConnectionInfo) -> Self {
        Self::Connected(info)
    }

    pub fn mfa(challenge: MfaChallenge) -> Self {
        Self::Mfa(challenge)
    }

    pub fn print(&self) -> String {
        match self {
            Self::Connected(info) => info.print(),
            other => other.to_string(),
        }
    }
}

impl fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionStatus::Disconnected => write!(f, "{}", i18n::tr!("connection-status-disconnected")),
            ConnectionStatus::Connecting => write!(f, "{}", i18n::tr!("connection-status-connecting")),
            ConnectionStatus::Connected(info) => {
                write!(
                    f,
                    "{}",
                    i18n::tr!(
                        "connection-status-connected-since",
                        since = info.since.unwrap_or_default().format("%Y-%m-%d %H:%M:%S").to_string()
                    )
                )
            }
            ConnectionStatus::Mfa(mfa) => write!(
                f,
                "{}",
                i18n::tr!(
                    "connection-status-mfa-pending",
                    mfa_type = format!("{:?}", mfa.mfa_type)
                )
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TunnelServiceRequest {
    Connect(TunnelParams),
    ChallengeCode(String, TunnelParams),
    Disconnect,
    GetStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TunnelServiceResponse {
    Ok,
    Error(String),
    ConnectionStatus(ConnectionStatus),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PromptInfo {
    pub header: String,
    pub prompt: String,
    pub default_entry: Option<String>,
}

impl PromptInfo {
    pub fn new<H, S>(header: H, prompt: S) -> Self
    where
        H: AsRef<str>,
        S: AsRef<str>,
    {
        Self {
            header: header.as_ref().to_owned(),
            prompt: prompt.as_ref().to_owned(),
            default_entry: None,
        }
    }
}
