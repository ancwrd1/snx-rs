use std::fmt;

use serde::{Deserialize, Serialize};

pub enum SnxPacket {
    Control(String, serde_json::Value),
    Data(Vec<u8>),
}

impl fmt::Debug for SnxPacket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SnxPacket::Control(name, _) => write!(f, "CONTROL: {}", name),
            SnxPacket::Data(data) => write!(f, "DATA: {} bytes", data.len()),
        }
    }
}

impl SnxPacket {
    pub fn control<S, T>(name: S, data: T) -> Self
    where
        S: AsRef<str>,
        T: Serialize + Default,
    {
        let value = serde_json::to_value(data).unwrap_or_default();
        SnxPacket::Control(name.as_ref().to_owned(), value)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfficeMode {
    pub ipaddr: String,
    pub keep_address: Option<String>,
    pub dns_servers: Option<Vec<String>>,
    pub dns_suffix: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionalRequest {
    pub client_type: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientHello {
    pub client_version: String,
    pub protocol_version: String,
    pub protocol_minor_version: String,
    #[serde(rename = "OM")]
    pub office_mode: OfficeMode,
    pub optional: Option<OptionalRequest>,
    pub cookie: String,
}

impl ClientHello {
    pub const NAME: &'static str = "client_hello";
}

impl From<ClientHello> for SnxPacket {
    fn from(value: ClientHello) -> Self {
        SnxPacket::control(ClientHello::NAME, value)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HelloReply {
    pub version: String,
    pub protocol_version: String,
    #[serde(rename = "OM")]
    pub office_mode: OfficeMode,
    pub range: Vec<NetworkRange>,
    pub timeouts: Timeouts,
    pub optional: Option<OptionalResponse>,
}

impl HelloReply {
    pub const NAME: &'static str = "hello_reply";
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkRange {
    pub from: String,
    pub to: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Timeouts {
    pub authentication: String,
    pub keepalive: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionalResponse {
    pub subnet: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CccClientRequest {
    #[serde(rename = "RequestHeader")]
    pub header: RequestHeader,
    #[serde(rename = "RequestData")]
    pub data: RequestData,
}

impl CccClientRequest {
    pub const NAME: &'static str = "CCCclientRequest";
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CccServerResponse {
    #[serde(rename = "ResponseHeader")]
    pub header: ResponseHeader,
    #[serde(rename = "ResponseData")]
    pub data: ResponseData,
}

impl CccServerResponse {
    pub const NAME: &'static str = "CCCserverResponse";
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestHeader {
    pub id: String,
    #[serde(rename = "type")]
    pub request_type: String,
    pub session_id: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestData {
    pub client_type: String,
    pub endpoint_os: String,
    pub username: String,
    pub password: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseHeader {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub session_id: String,
    pub return_code: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseData {
    pub authn_status: String,
    pub is_authenticated: String,
    pub active_key: Option<String>,
    pub server_fingerprint: Option<String>,
    pub server_cn: Option<String>,
    pub session_id: Option<String>,
    pub active_key_timeout: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeepaliveRequest {
    pub id: String,
}

impl KeepaliveRequest {
    pub const NAME: &'static str = "keepalive";
}

impl From<KeepaliveRequest> for SnxPacket {
    fn from(value: KeepaliveRequest) -> Self {
        SnxPacket::control(KeepaliveRequest::NAME, value)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisconnectRequest {
    pub code: String,
    pub message: Option<String>,
}

impl DisconnectRequest {
    pub const NAME: &'static str = "disconnect";
}

impl From<DisconnectRequest> for SnxPacket {
    fn from(value: DisconnectRequest) -> Self {
        SnxPacket::control(DisconnectRequest::NAME, value)
    }
}
