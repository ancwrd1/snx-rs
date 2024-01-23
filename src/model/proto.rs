use std::net::Ipv4Addr;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use crate::model::{
    wrappers::{HexKey, Maybe, QuotedString, SecretKey},
    AuthenticationAlgorithm, EncryptionAlgorithm,
};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfficeMode {
    pub ipaddr: String,
    pub keep_address: Option<bool>,
    pub dns_servers: Option<Vec<String>>,
    pub dns_suffix: Option<QuotedString>,
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
    pub fn into_data(self) -> anyhow::Result<ResponseData> {
        match self.data {
            ResponseData::Generic(v) if v.as_str().is_some_and(|s| s.is_empty()) => {
                Err(anyhow!("Request failed, error code: {}", self.header.return_code))
            }
            other => Ok(other),
        }
    }
}

impl CccServerResponse {
    pub const NAME: &'static str = "CCCserverResponse";
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
    pub username: Option<SecretKey>,
    pub password: Option<SecretKey>,
    pub client_logging_data: Option<ClientLoggingData>,
    #[serde(rename = "selectedLoginOption")]
    pub selected_login_option: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiChallengeRequest {
    pub client_type: String,
    pub auth_session_id: String,
    pub user_input: SecretKey,
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
pub struct KeyManagementRequest {
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
    pub nemo_client_1: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocationAwarenessRequest {
    pub source_ip: Ipv4Addr,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum RequestData {
    Auth(AuthRequest),
    MultiChallenge(MultiChallengeRequest),
    KeyManagement(KeyManagementRequest),
    LocationAwareness(LocationAwarenessRequest),
    ClientHello { client_info: ClientInfo },
    Custom(String),
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
    KeyManagement(KeyManagementResponse),
    ClientSettings(ClientSettingsResponse),
    LocationAwareness(LocationAwarenessResponse),
    ServerInfo(ServerInfoResponse),
    Generic(serde_json::Value),
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthResponse {
    pub authn_status: String,
    pub is_authenticated: Option<bool>,
    pub active_key: Option<SecretKey>,
    pub server_fingerprint: Option<String>,
    pub server_cn: Option<String>,
    pub session_id: Option<String>,
    pub active_key_timeout: Option<u64>,
    pub error_message: Option<SecretKey>,
    pub error_id: Option<SecretKey>,
    pub error_code: Option<u32>,
    pub prompt: Option<SecretKey>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyManagementResponse {
    pub client_encsa: IpsecSA,
    pub client_decsa: IpsecSA,
    pub om_addr: u32,
    pub om_subnet_mask: u32,
    pub om_nbns0: Option<u32>,
    pub om_nbns1: Option<u32>,
    pub om_nbns2: Option<u32>,
    pub om_dns0: Option<u32>,
    pub om_dns1: Option<u32>,
    pub om_dns2: Option<u32>,
    pub om_domain_name: Option<QuotedString>,
    pub lifetime: Option<u64>,
    pub encalg: EncryptionAlgorithm,
    pub authalg: AuthenticationAlgorithm,
    pub nattport: Option<u16>,
    pub udpencapsulation: Option<bool>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IpsecSA {
    pub enckey: HexKey,
    pub authkey: HexKey,
    pub spi: u32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientSettingsResponse {
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
pub struct LocationAwarenessResponse {
    pub location: String,
    pub source_ip: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeepaliveRequest {
    pub id: String,
}

impl KeepaliveRequest {
    pub const NAME: &'static str = "keepalive";
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisconnectRequest {
    pub code: String,
    pub message: Option<String>,
}

impl DisconnectRequest {
    pub const NAME: &'static str = "disconnect";
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
    pub end_point_security: EndPointSecurity,
    pub login_options_data: LoginOptionsData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub protocol_version: u32,
    pub features: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpgradeConfiguration {
    pub available_client_version: u32,
    pub client_upgrade_url: QuotedString,
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
    pub connect_with_certificate_url: QuotedString,
    pub cookie_name: String,
    pub internal_ca_fingerprint: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EndPointSecurity {
    pub ics: IcsInfo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IcsInfo {
    pub run_ics: bool,
    pub ics_base_url: QuotedString,
    pub ics_version: u32,
    pub ics_upgrade_url: QuotedString,
    pub ics_images_url: QuotedString,
    pub ics_images_ver: u32,
    pub ics_cab_url: QuotedString,
    pub ics_cab_version: QuotedString,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoginOptionsData {
    pub login_options_list: Vec<LoginOption>,
    pub login_options_md5: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoginOption {
    pub id: String,
    pub secondary_realm_hash: String,
    pub display_name: QuotedString,
    pub show_realm: u32,
    pub factors: Vec<LoginFactor>,
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
    LoginDisplayLabel(LoginDisplayLabel),
    Empty(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoginDisplayLabel {
    pub header: QuotedString,
    pub username: QuotedString,
    pub password: QuotedString,
}
