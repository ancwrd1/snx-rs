use std::{net::Ipv4Addr, time::Duration};

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
    pub esp_in: EspCryptMaterial,
    pub esp_out: EspCryptMaterial,
}

impl Default for IpsecSession {
    fn default() -> Self {
        Self {
            lifetime: Default::default(),
            address: Ipv4Addr::new(0, 0, 0, 0),
            netmask: Ipv4Addr::new(0, 0, 0, 0),
            dns: Vec::new(),
            domains: Vec::new(),
            esp_in: Default::default(),
            esp_out: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CccSession {
    pub session_id: String,
    pub ipsec_session: Option<IpsecSession>,
    pub state: SessionState,
}

impl CccSession {
    pub fn empty() -> Self {
        Self {
            session_id: String::new(),
            ipsec_session: None,
            state: Default::default(),
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
    UserInput,
    SamlSso,
}

impl MfaType {
    pub fn from_id(id: &str) -> Self {
        if id == "CPSC_SP_URL" {
            Self::SamlSso
        } else {
            Self::UserInput
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, PartialOrd)]
pub struct MfaChallenge {
    pub mfa_type: MfaType,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, PartialOrd)]
pub struct ConnectionStatus {
    pub connected_since: Option<DateTime<Local>>,
    pub mfa: Option<MfaChallenge>,
}

impl ConnectionStatus {
    pub fn connected() -> Self {
        Self {
            connected_since: Some(Local::now()),
            ..Default::default()
        }
    }

    pub fn disconnected() -> Self {
        Self::default()
    }

    pub fn mfa(challenge: MfaChallenge) -> Self {
        Self {
            mfa: Some(challenge),
            ..Default::default()
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
