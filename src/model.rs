use chrono::{DateTime, Local};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::model::params::TunnelParams;

pub mod newtype;
pub mod params;
pub mod snx;

#[derive(Default, Debug, Clone, PartialEq)]
pub struct SnxSession {
    pub session_id: String,
    pub cookie: String,
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum EncryptionAlgorithm {
    #[default]
    Aes256Cbc,
}

impl EncryptionAlgorithm {
    pub fn as_xfrm_name(&self) -> &'static str {
        match self {
            Self::Aes256Cbc => "aes",
        }
    }
}

impl<'de> Deserialize<'de> for EncryptionAlgorithm {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match String::deserialize(deserializer)?.as_str() {
            "AES-256" => Ok(Self::Aes256Cbc),
            _ => Err(serde::de::Error::custom("Unsupported encryption algorithm!")),
        }
    }
}

impl Serialize for EncryptionAlgorithm {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Aes256Cbc => String::from("AES-256").serialize(serializer),
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum AuthenticationAlgorithm {
    #[default]
    HmacSha256,
}

impl AuthenticationAlgorithm {
    pub fn as_xfrm_name(&self) -> &'static str {
        match self {
            Self::HmacSha256 => "sha256",
        }
    }

    pub fn trunc_length(&self) -> u32 {
        match self {
            Self::HmacSha256 => 128,
        }
    }
}

impl<'de> Deserialize<'de> for AuthenticationAlgorithm {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match String::deserialize(deserializer)?.as_str() {
            "SHA256" => Ok(Self::HmacSha256),
            _ => Err(serde::de::Error::custom("Unsupported authentication algorithm!")),
        }
    }
}

impl Serialize for AuthenticationAlgorithm {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::HmacSha256 => String::from("SHA256").serialize(serializer),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub connected_since: Option<DateTime<Local>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TunnelServiceRequest {
    Connect(TunnelParams),
    Disconnect,
    GetStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TunnelServiceResponse {
    Ok,
    Error(String),
    ConnectionStatus(ConnectionStatus),
}
