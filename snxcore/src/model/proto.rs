use std::collections::BTreeMap;
use std::net::Ipv4Addr;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use crate::model::wrappers::*;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfficeMode {
    pub ipaddr: String,
    pub keep_address: Option<bool>,
    pub dns_servers: Option<Vec<Ipv4Addr>>,
    pub dns_suffix: Option<StringList>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionalRequest {
    pub client_type: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientHello {
    #[serde(rename = "(client_hello")]
    pub data: ClientHelloData,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientHelloData {
    pub client_version: u32,
    pub protocol_version: u32,
    pub protocol_minor_version: u32,
    #[serde(rename = "OM")]
    pub office_mode: OfficeMode,
    pub optional: Option<OptionalRequest>,
    pub cookie: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HelloReply {
    #[serde(rename = "(hello_reply")]
    pub data: HelloReplyData,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HelloReplyData {
    pub version: u32,
    pub protocol_version: u32,
    #[serde(rename = "OM")]
    pub office_mode: OfficeMode,
    pub range: Vec<NetworkRange>,
    pub timeouts: Timeouts,
    pub optional: Option<OptionalResponse>,
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
    #[serde(rename = "(CCCclientRequest")]
    pub data: CccClientRequestData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CccClientRequestData {
    #[serde(rename = "RequestHeader")]
    pub header: RequestHeader,
    #[serde(rename = "RequestData")]
    pub data: RequestData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CccServerResponse {
    #[serde(rename = "(CCCserverResponse")]
    pub data: CccServerResponseData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CccServerResponseData {
    #[serde(rename = "ResponseHeader")]
    pub header: ResponseHeader,
    #[serde(rename = "ResponseData")]
    pub data: ResponseData,
}

impl CccServerResponseData {
    pub fn into_data(self) -> anyhow::Result<ResponseData> {
        match self.data {
            ResponseData::Generic(v) if v.as_str().is_some_and(|s| s.is_empty()) => {
                Err(anyhow!("Request failed, error code: {}", self.header.return_code))
            }
            other => Ok(other),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestHeader {
    pub id: u32,
    #[serde(rename = "type")]
    pub request_type: String,
    pub session_id: Option<String>,
    pub protocol_version: Option<u32>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthRequest {
    pub client_type: String,
    pub endpoint_os: Option<String>,
    pub username: Option<EncryptedString>,
    pub password: Option<EncryptedString>,
    pub client_logging_data: Option<ClientLoggingData>,
    #[serde(rename = "selectedLoginOption")]
    pub selected_login_option: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiChallengeRequest {
    pub client_type: String,
    pub auth_session_id: String,
    pub user_input: EncryptedString,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientLoggingData {
    pub client_name: Option<String>,
    pub client_ver: Option<String>,
    pub client_build_number: Option<String>,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub device_type: Option<String>,
    pub hardware_model: Option<String>,
    pub machine_name: Option<String>,
    pub device_id: Option<String>,
    pub mac_address: Option<StringList>,
    pub physical_ip: Option<Ipv4Addr>,
    pub is_compliant: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientSettingsRequest {
    #[serde(rename = "(ClientSettings")]
    pub data: ClientSettingsData,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientSettingsData {
    pub requested_policies_and_current_versions: PoliciesAndVersions,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignoutRequest {}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoliciesAndVersions {
    pub range: Vec<NetworkRange>,
    pub nemo_client_1: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum RequestData {
    Auth(AuthRequest),
    MultiChallenge(MultiChallengeRequest),
    ClientHello { client_info: ClientInfo },
    ClientSettings(ClientSettingsRequest),
    Signout(SignoutRequest),
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseHeader {
    pub id: Maybe<u32>,
    #[serde(rename = "type")]
    pub response_type: String,
    pub session_id: String,
    pub return_code: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum ResponseData {
    Auth(AuthResponse),
    ClientSettings(ClientSettingsResponse),
    ServerInfo(ServerInfoResponse),
    Generic(serde_json::Value),
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthResponse {
    pub authn_status: String,
    pub is_authenticated: Option<bool>,
    pub active_key: Option<EncryptedString>,
    pub server_fingerprint: Option<String>,
    pub server_cn: Option<String>,
    pub session_id: Option<String>,
    pub active_key_timeout: Option<u64>,
    pub error_message: Option<EncryptedString>,
    pub error_id: Option<EncryptedString>,
    pub error_code: Option<u32>,
    pub prompt: Option<EncryptedString>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientSettingsResponse {
    pub gw_internal_ip: Ipv4Addr,
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
pub struct KeepaliveRequest {
    #[serde(rename = "(keepalive")]
    pub data: KeepaliveRequestData,
}
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeepaliveRequestData {
    pub id: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisconnectRequest {
    #[serde(rename = "(disconnect")]
    pub data: DisconnectRequestData,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisconnectRequestData {
    pub code: String,
    pub message: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientInfo {
    pub client_type: String,
    pub client_version: u32,
    pub client_support_saml: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerInfoResponse {
    pub protocol_version: ProtocolVersion,
    pub upgrade_configuration: UpgradeConfiguration,
    pub connectivity_info: ConnectivityInfo,
    pub login_options_data: Option<LoginOptionsData>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub protocol_version: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpgradeConfiguration {
    pub available_client_version: u32,
    pub client_upgrade_url: String,
    pub upgrade_mode: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectivityInfo {
    pub default_authentication_method: String,
    pub client_enabled: bool,
    pub supported_data_tunnel_protocols: Vec<String>,
    pub connectivity_type: String,
    pub server_ip: Ipv4Addr,
    pub ipsec_transport: String,
    pub tcpt_port: u16,
    pub natt_port: u16,
    pub connect_with_certificate_url: String,
    pub cookie_name: String,
    pub internal_ca_fingerprint: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoginOptionsData {
    pub login_options_list: BTreeMap<String, LoginOption>,
    pub login_options_md5: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoginOption {
    pub id: String,
    pub secondary_realm_hash: String,
    pub display_name: String,
    pub show_realm: u32,
    pub factors: BTreeMap<String, LoginFactor>,
}

impl LoginOption {
    pub fn unspecified() -> Self {
        Self {
            id: "vpn_Username_Password".to_string(),
            secondary_realm_hash: String::new(),
            display_name: "Username and password".into(),
            show_realm: 0,
            factors: BTreeMap::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoginFactor {
    pub factor_type: String,
    pub securid_card_type: String,
    pub certificate_storage_type: String,
    pub custom_display_labels: LoginDisplayLabelSelect,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LoginDisplayLabelSelect {
    LoginDisplayLabel(BTreeMap<String, String>),
    Empty(String),
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthenticationRealm {
    #[serde(rename = "clientType")]
    pub client_type: String,
    #[serde(rename = "oldSessionId")]
    pub old_session_id: String,
    #[serde(rename = "protocolVersion")]
    pub protocol_version: u32,
    pub client_mode: String,
    pub selected_realm_id: String,
    pub secondary_realm_hash: Option<String>,
    pub client_logging_data: Option<ClientLoggingData>,
}
