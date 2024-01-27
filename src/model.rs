use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::model::params::TunnelParams;

pub mod params;
pub mod proto;
pub mod wrappers;

#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    Authenticated { active_key: String },
    Pending { prompt: Option<String> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CccSession {
    pub session_id: String,
    pub state: SessionState,
}

impl CccSession {
    pub fn active_key(&self) -> &str {
        match self.state {
            SessionState::Authenticated { ref active_key } => active_key.as_str(),
            SessionState::Pending { .. } => "",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, PartialOrd)]
pub struct ConnectionStatus {
    pub connected_since: Option<DateTime<Local>>,
    pub mfa_pending: bool,
    pub mfa_prompt: Option<String>,
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
