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
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VpnSession {
    pub ccc_session_id: String,
    pub ipsec_session: Option<IpsecSession>,
    pub state: SessionState,
}

impl VpnSession {
    pub fn empty() -> Self {
        Self {
            ccc_session_id: String::new(),
            ipsec_session: None,
            state: SessionState::default(),
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
    pub since: DateTime<Local>,
    pub server_name: String,
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
    pub fn print(&self) -> String {
        format!(
            "Connected since:\t{}
Server name:\t\t{}
Tunnel type:\t\t{}
Transport type:\t\t{}
IP address:\t\t{}
DNS servers:\t\t{:?}
Search domains:\t\t{:?}
Interface:\t\t{}
DNS configured:\t\t{}
Routing configured:\t{}
Default route:\t\t{}",
            self.since.format("%Y-%m-%d %H:%M:%S"),
            self.server_name,
            self.tunnel_type,
            self.transport_type,
            self.ip_address,
            self.dns_servers,
            self.search_domains,
            self.interface_name,
            self.dns_configured,
            self.routing_configured,
            self.default_route,
        )
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
            ConnectionStatus::Disconnected => write!(f, "Disconnected"),
            ConnectionStatus::Connecting => write!(f, "Connecting in progress"),
            ConnectionStatus::Connected(info) => {
                write!(f, "Connected since: {}", info.since.format("%Y-%m-%d %H:%M:%S"))
            }
            ConnectionStatus::Mfa(mfa) => write!(f, "MFA pending: {:?}", mfa.mfa_type),
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
        }
    }
}
