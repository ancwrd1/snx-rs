use anyhow::anyhow;
use std::str::FromStr;
use std::{fmt, net::Ipv4Addr};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

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

impl From<Vec<u8>> for SnxPacket {
    fn from(value: Vec<u8>) -> Self {
        SnxPacket::Data(value)
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct QuotedString(pub String);

impl Serialize for QuotedString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format!("\"{}\"", self.0).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for QuotedString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self(String::deserialize(deserializer)?.trim_matches('"').to_owned()))
    }
}

impl From<String> for QuotedString {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<QuotedString> for String {
    fn from(value: QuotedString) -> Self {
        value.0
    }
}

impl<'a> From<&'a str> for QuotedString {
    fn from(value: &'a str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct EncryptedString(pub String);

impl Serialize for EncryptedString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        crate::util::snx_encrypt(self.0.as_bytes()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EncryptedString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let decrypted = crate::util::snx_decrypt(s.as_bytes()).map_err(|e| serde::de::Error::custom(e))?;
        Ok(Self(String::from_utf8_lossy(&decrypted).into_owned()))
    }
}

impl From<String> for EncryptedString {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<EncryptedString> for String {
    fn from(value: EncryptedString) -> Self {
        value.0
    }
}

impl<'a> From<&'a str> for EncryptedString {
    fn from(value: &'a str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct SnxSession {
    pub session_id: String,
    pub cookie: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfficeMode {
    pub ipaddr: String,
    pub keep_address: Option<bool>,
    pub dns_servers: Option<Vec<String>>,
    pub dns_suffix: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionalRequest {
    pub client_type: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientHello {
    pub client_version: u32,
    pub protocol_version: u32,
    pub protocol_minor_version: u32,
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
    pub version: u32,
    pub protocol_version: u32,
    #[serde(rename = "OM")]
    pub office_mode: OfficeMode,
    pub range: Vec<NetworkRange>,
    pub timeouts: Timeouts,
    pub optional: Option<OptionalResponse>,
}

impl HelloReply {
    pub const NAME: &'static str = "hello_reply";
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkRange {
    pub from: Ipv4Addr,
    pub to: Ipv4Addr,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Timeouts {
    pub authentication: u64,
    pub keepalive: u64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionalResponse {
    pub subnet: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CccClientRequest {
    #[serde(rename = "RequestHeader")]
    pub header: RequestHeader,
    #[serde(rename = "RequestData")]
    pub data: RequestData,
}

impl CccClientRequest {
    pub const NAME: &'static str = "CCCclientRequest";
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub id: u32,
    #[serde(rename = "type")]
    pub request_type: String,
    pub session_id: String,
    pub protocol_version: Option<u32>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PasswordData {
    pub client_type: String,
    pub endpoint_os: Option<String>,
    pub username: EncryptedString,
    pub password: EncryptedString,
    pub client_logging_data: Option<ClientLoggingData>,
    #[serde(rename = "selectedLoginOption")]
    pub selected_login_option: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientLoggingData {
    pub client_name: Option<String>,
    pub client_ver: Option<String>,
    pub client_build_number: Option<String>,
    pub os_name: Option<QuotedString>,
    pub os_version: Option<String>,
    pub device_type: Option<String>,
    pub hardware_model: Option<String>,
    pub machine_name: Option<String>,
    pub device_id: Option<String>,
    pub mac_address: Option<String>,
    pub physical_ip: Option<String>,
    pub is_compliant: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IpsecData {
    #[serde(rename = "SPI")]
    pub spi: u32,
    pub rekey: bool,
    pub req_om_addr: u32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientSettingsData {
    pub requested_policies_and_current_versions: PoliciesAndVersions,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoliciesAndVersions {
    pub range: Vec<NetworkRange>,
    pub nemo_client_1: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocationAwarenessData {
    pub source_ip: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum RequestData {
    Password(PasswordData),
    Ipsec(IpsecData),
    LocationAwareness(LocationAwarenessData),
    Wrapped(String),
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseHeader {
    pub id: u32,
    #[serde(rename = "type")]
    pub response_type: String,
    pub session_id: String,
    pub return_code: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResponseData {
    Auth(AuthResponseData),
    Ipsec(IpsecResponseData),
    ClientSettings(ClientSettingsResponseData),
    LocationAwareness(LocationAwarenessResponseData),
    Other(String),
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthResponseData {
    pub authn_status: String,
    pub is_authenticated: bool,
    pub active_key: Option<EncryptedString>,
    pub server_fingerprint: Option<String>,
    pub server_cn: Option<String>,
    pub session_id: Option<String>,
    pub active_key_timeout: Option<u64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IpsecResponseData {
    pub client_encsa: IpsecKey,
    pub client_decsa: IpsecKey,
    pub om_addr: u32,
    pub om_subnet_mask: u32,
    pub om_nbns0: u32,
    pub om_nbns1: u32,
    pub om_nbns2: u32,
    pub om_dns0: u32,
    pub om_dns1: u32,
    pub om_dns2: u32,
    pub om_domain_name: String,
    pub lifetime: u64,
    pub encalg: String,
    pub authalg: String,
    pub nattport: u16,
    pub udpencapsulation: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IpsecKey {
    pub enckey: String,
    pub authkey: String,
    pub spi: u32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientSettingsResponseData {
    pub gw_internal_ip: String,
    pub updated_policies: UpdatedPolicies,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdatedPolicies {
    pub range: Range,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Range {
    pub settings: Vec<NetworkRange>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocationAwarenessResponseData {
    pub location: String,
    pub source_ip: String,
}

impl IpsecKey {
    pub fn decode(&self) -> Self {
        let mut enckey = hex::decode(&self.enckey).unwrap_or_default();
        enckey.reverse();
        let enckey = format!("0x{}", hex::encode(enckey));

        let mut authkey = hex::decode(&self.authkey).unwrap_or_default();
        authkey.reverse();
        let authkey = format!("0x{}", hex::encode(authkey));

        Self {
            enckey,
            authkey,
            spi: self.spi,
        }
    }
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

#[derive(Default, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LoginType {
    Password,
    PasswordWithMfa,
    #[default]
    PasswordWithMsAuth,
    EmergencyAccess,
    SsoAzure,
}

impl LoginType {
    pub fn as_login_option(&self) -> &'static str {
        match self {
            Self::Password => "vpn_Username_Password",
            Self::PasswordWithMfa => "vpn",
            Self::PasswordWithMsAuth => "vpn_Microsoft_Authenticator",
            Self::EmergencyAccess => "vpn_Emergency_Access",
            Self::SsoAzure => "vpn_Azure_Authentication",
        }
    }
}

impl FromStr for LoginType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "password" => Ok(Self::Password),
            "password-mfa" => Ok(Self::PasswordWithMfa),
            "password-ms-auth" => Ok(Self::PasswordWithMsAuth),
            "emergency-access" => Ok(Self::EmergencyAccess),
            "sso-azure" => Ok(Self::SsoAzure),
            other => Err(anyhow!("Unknown login type: {}", other)),
        }
    }
}
