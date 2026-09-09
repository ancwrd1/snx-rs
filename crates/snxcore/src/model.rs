use std::{borrow::Cow, fmt, net::Ipv4Addr, sync::Arc, time::Duration};

use chrono::{DateTime, Local};
use ipnet::Ipv4Net;
use isakmp::model::{EspAuthAlgorithm, EspCryptMaterial, TransformId};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    model::{
        params::{TransportType, TunnelParams, TunnelType},
        wrappers::SessionId,
    },
    platform::SearchDomain,
    util,
};

pub mod params;
pub mod proto;
pub mod wrappers;

#[derive(Debug, Clone, PartialEq)]
pub enum AuthenticatedSession {
    SslSessionKey(String),
    IPsecSession(IPsecSession),
}

#[derive(Debug, Default, Clone, PartialEq)]
pub enum SessionState {
    #[default]
    NoState,
    Authenticated(AuthenticatedSession),
    PendingChallenge(MfaChallenge),
}

#[derive(Debug, Clone, PartialEq)]
pub struct IPsecSession {
    pub initiator_spi: u64,
    pub responder_spi: u64,
    pub lifetime: Duration,
    pub address: Ipv4Addr,
    pub netmask: Ipv4Addr,
    pub dns: Vec<Ipv4Addr>,
    pub domains: Vec<String>,
    pub esp_in: Arc<EspCryptMaterial>,
    pub esp_out: Arc<EspCryptMaterial>,
    pub transport_type: TransportType,
    pub address_lifetime: Duration,
    pub ike_lifetime: Duration,
    pub ike_timestamp: DateTime<Local>,
}

impl Default for IPsecSession {
    fn default() -> Self {
        Self {
            initiator_spi: 0,
            responder_spi: 0,
            lifetime: Duration::default(),
            address: Ipv4Addr::new(0, 0, 0, 0),
            netmask: Ipv4Addr::new(0, 0, 0, 0),
            dns: Vec::new(),
            domains: Vec::new(),
            esp_in: Arc::default(),
            esp_out: Arc::default(),
            transport_type: TransportType::default(),
            address_lifetime: Duration::default(),
            ike_lifetime: Duration::default(),
            ike_timestamp: Local::now(),
        }
    }
}

impl IPsecSession {
    pub fn ipv4net_address(&self) -> Ipv4Net {
        Ipv4Net::with_netmask(self.address, self.netmask).unwrap_or_else(|_| Ipv4Net::from(self.address))
    }

    pub fn to_ike_state(&self) -> IkeState {
        IkeState {
            initiator_spi: self.initiator_spi,
            responder_spi: self.responder_spi,
            lifetime: self.ike_lifetime,
            timestamp: self.ike_timestamp,
            esp_in: EspState {
                spi: self.esp_in.spi,
                enc_algorithm: self.esp_in.transform_id.into(),
                auth_algorithm: self.esp_in.auth_algorithm.into(),
            },
            esp_out: EspState {
                spi: self.esp_out.spi,
                enc_algorithm: self.esp_out.transform_id.into(),
                auth_algorithm: self.esp_out.auth_algorithm.into(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TunnelSession {
    pub session_id: SessionId,
    pub state: SessionState,
    pub username: Option<String>,
}

impl TunnelSession {
    pub fn empty() -> Self {
        Self {
            session_id: SessionId::default(),
            state: SessionState::default(),
            username: None,
        }
    }

    pub fn ssl_session_key(&self) -> Option<&str> {
        match self.state {
            SessionState::Authenticated(AuthenticatedSession::SslSessionKey(ref active_key)) => {
                Some(active_key.as_str())
            }
            _ => None,
        }
    }

    pub fn ipsec_session(&self) -> Option<&IPsecSession> {
        match self.state {
            SessionState::Authenticated(AuthenticatedSession::IPsecSession(ref ipsec_session)) => Some(ipsec_session),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, PartialOrd)]
pub enum MfaType {
    #[default]
    PasswordInput,
    IdentityProvider,
    UserNameInput,
    MobileAccess,
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

fn format_bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = n as f64;
    let mut idx = 0;
    while value >= 1024.0 && idx < UNITS.len() - 1 {
        value /= 1024.0;
        idx += 1;
    }
    if idx == 0 {
        format!("{n} {}", UNITS[0])
    } else {
        format!("{value:.2} {}", UNITS[idx])
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct LiveStats {
    pub last_rtt_ms: Option<u64>,
    pub bytes_rx: u64,
    pub bytes_tx: u64,
    pub packets_rx: u64,
    pub packets_tx: u64,
    pub errors_rx: u64,
    pub errors_tx: u64,
    pub bps_rx: u64,
    pub bps_tx: u64,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub enum EncryptionAlgorithm {
    #[default]
    EspAesCbc,
    Esp3Des,
}

impl fmt::Display for EncryptionAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncryptionAlgorithm::EspAesCbc => f.write_str("AES-CBC"),
            EncryptionAlgorithm::Esp3Des => f.write_str("3DES"),
        }
    }
}

impl From<TransformId> for EncryptionAlgorithm {
    fn from(id: TransformId) -> Self {
        match id {
            TransformId::Esp3Des => Self::Esp3Des,
            _ => Self::EspAesCbc,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub enum AuthenticationAlgorithm {
    HmacSha96,
    HmacSha160,
    #[default]
    HmacSha256,
    HmacSha256v2,
}

impl fmt::Display for AuthenticationAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthenticationAlgorithm::HmacSha96 => f.write_str("HMAC-SHA96"),
            AuthenticationAlgorithm::HmacSha160 => f.write_str("HMAC-SHA160"),
            AuthenticationAlgorithm::HmacSha256 => f.write_str("HMAC-SHA256"),
            AuthenticationAlgorithm::HmacSha256v2 => f.write_str("HMAC-SHA256V2"),
        }
    }
}

impl From<EspAuthAlgorithm> for AuthenticationAlgorithm {
    fn from(algo: EspAuthAlgorithm) -> Self {
        match algo {
            EspAuthAlgorithm::HmacSha96 => Self::HmacSha96,
            EspAuthAlgorithm::HmacSha160 => Self::HmacSha160,
            EspAuthAlgorithm::HmacSha256v2 => Self::HmacSha256v2,
            _ => Self::HmacSha256,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EspState {
    pub spi: u32,
    pub enc_algorithm: EncryptionAlgorithm,
    pub auth_algorithm: AuthenticationAlgorithm,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct IkeState {
    pub initiator_spi: u64,
    pub responder_spi: u64,
    pub lifetime: Duration,
    pub timestamp: DateTime<Local>,
    pub esp_in: EspState,
    pub esp_out: EspState,
}

impl IkeState {
    pub fn print(&self) -> String {
        let values = [
            ("info-ike-initiator-spi", format!("{:016x}", self.initiator_spi)),
            ("info-ike-responder-spi", format!("{:016x}", self.responder_spi)),
            ("info-ike-lifetime", self.lifetime.as_secs().to_string()),
            ("info-ike-timestamp", self.timestamp.to_rfc3339()),
            ("info-ike-expiration", (self.timestamp + self.lifetime).to_rfc3339()),
            ("info-esp-spi-in", format!("{:08x}", self.esp_in.spi)),
            ("info-esp-spi-out", format!("{:08x}", self.esp_out.spi)),
            ("info-esp-encryption-in", self.esp_in.enc_algorithm.to_string()),
            ("info-esp-authentication-in", self.esp_in.auth_algorithm.to_string()),
            ("info-esp-encryption-out", self.esp_out.enc_algorithm.to_string()),
            ("info-esp-authentication-out", self.esp_out.auth_algorithm.to_string()),
        ];
        util::format_values(&values)
    }
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
    pub search_domains: Vec<SearchDomain>,
    pub interface_name: String,
    pub dns_configured: bool,
    pub routing_configured: bool,
    pub default_route: bool,
    pub profile_id: Uuid,
    pub profile_name: String,
    #[serde(default)]
    pub live: LiveStats,
    #[serde(default)]
    pub ike_state: Option<IkeState>,
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

    pub fn to_values(&self, with_stats: bool) -> Vec<(&'static str, String)> {
        let mut result = vec![
            (
                "info-connected-since",
                if let Some(ref since) = self.since {
                    since.format("%Y-%m-%d %H:%M:%S").to_string()
                } else {
                    String::new()
                },
            ),
            ("info-connection-profile", self.or_empty(|| self.profile_name.clone())),
            ("info-server-name", self.or_empty(|| self.server_name.clone())),
            ("info-user-name", self.or_empty(|| self.username.clone())),
            ("info-login-type", self.or_empty(|| self.login_type.clone())),
            ("info-tunnel-type", self.or_empty(|| self.tunnel_type.to_string())),
        ];

        if self.tunnel_type == TunnelType::IPsec {
            result.push(("info-transport-type", self.or_empty(|| self.transport_type.as_i18n())));
        }

        result.extend([
            ("info-ip-address", self.or_empty(|| self.ip_address.to_string())),
            ("info-dns-servers", self.or_empty(|| format!("{:?}", self.dns_servers))),
            (
                "info-search-domains",
                self.or_empty(|| format!("[{}]", self.search_domains.iter().map(|d| d.to_string()).join(", "))),
            ),
            ("info-interface", self.or_empty(|| self.interface_name.clone())),
            ("info-dns-configured", self.or_empty(|| self.dns_configured.to_string())),
            (
                "info-routing-configured",
                self.or_empty(|| self.routing_configured.to_string()),
            ),
            ("info-default-route", self.or_empty(|| self.default_route.to_string())),
        ]);

        if with_stats {
            if self.tunnel_type == TunnelType::IPsec {
                result.push((
                    "info-rtt",
                    self.or_empty(|| match self.live.last_rtt_ms {
                        Some(ms) => format!("{ms} ms"),
                        None => "—".to_string(),
                    }),
                ));
            }

            result.extend([
                (
                    "info-bytes-received",
                    self.or_empty(|| format_bytes(self.live.bytes_rx)),
                ),
                ("info-bytes-sent", self.or_empty(|| format_bytes(self.live.bytes_tx))),
                (
                    "info-rate-received",
                    self.or_empty(|| format!("{}/s", format_bytes(self.live.bps_rx))),
                ),
                (
                    "info-rate-sent",
                    self.or_empty(|| format!("{}/s", format_bytes(self.live.bps_tx))),
                ),
                (
                    "info-packets-received",
                    self.or_empty(|| self.live.packets_rx.to_string()),
                ),
                ("info-packets-sent", self.or_empty(|| self.live.packets_tx.to_string())),
            ]);
        }
        result
    }

    pub fn print(&self, with_stats: bool) -> String {
        util::format_values(&self.to_values(with_stats))
    }

    pub fn without_stats(&self) -> Self {
        Self {
            live: LiveStats::default(),
            ..self.clone()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected(Box<ConnectionInfo>),
    Mfa(MfaChallenge),
}

impl ConnectionStatus {
    pub fn connected(info: ConnectionInfo) -> Self {
        Self::Connected(Box::new(info))
    }

    pub fn mfa(challenge: MfaChallenge) -> Self {
        Self::Mfa(challenge)
    }

    pub fn print(&self) -> String {
        match self {
            Self::Connected(info) => info.print(true),
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
    Rekey,
}

impl TunnelServiceRequest {
    pub fn is_polling(&self) -> bool {
        matches!(self, Self::GetStatus)
    }
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

    pub fn prompt_with_colon(&self) -> Cow<'_, str> {
        if self.prompt.trim().ends_with(':') {
            Cow::Borrowed(self.prompt.as_ref())
        } else {
            Cow::Owned(format!("{}: ", self.prompt.trim()))
        }
    }
}
