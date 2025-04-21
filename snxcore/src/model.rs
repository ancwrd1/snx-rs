use std::{fmt, net::Ipv4Addr, sync::Arc, time::Duration};

use chrono::{DateTime, Local};
use isakmp::model::EspCryptMaterial;
use serde::{Deserialize, Serialize};

use crate::model::params::TunnelParams;

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
    SamlSso,
    UserNameInput,
}

impl MfaType {
    pub fn from_id(id: &str) -> Self {
        if id == "CPSC_SP_URL" {
            Self::SamlSso
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, PartialOrd)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected(DateTime<Local>),
    Mfa(MfaChallenge),
}

impl ConnectionStatus {
    pub fn connected() -> Self {
        Self::Connected(Local::now())
    }

    pub fn mfa(challenge: MfaChallenge) -> Self {
        Self::Mfa(challenge)
    }
}

impl fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionStatus::Disconnected => write!(f, "Disconnected"),
            ConnectionStatus::Connecting => write!(f, "Connecting in progress"),
            ConnectionStatus::Connected(since) => write!(f, "Connected since {}", since),
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
