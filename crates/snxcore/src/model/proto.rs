use std::{collections::BTreeMap, net::Ipv4Addr};

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::{
    model::{PromptInfo, wrappers::*},
    util::snx_deobfuscate,
};

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
    pub protocol_minor_version: Option<u32>,
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
    pub range: Option<Vec<NetworkRange>>,
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
            ResponseData::Generic(v) if v.as_str().is_some_and(|s| s.is_empty()) => anyhow::bail!(i18n::tr!(
                "error-request-failed-error-code",
                error_code = self.header.return_code
            )),
            other => Ok(other),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestHeader {
    pub id: u32,
    #[serde(rename = "type")]
    pub request_type: String,
    pub session_id: Option<SessionId>,
    pub protocol_version: Option<u32>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthRequest {
    pub client_type: String,
    pub endpoint_os: Option<String>,
    pub username: Option<ObfuscatedString>,
    pub password: Option<ObfuscatedString>,
    pub client_logging_data: Option<ClientLoggingData>,
    #[serde(rename = "selectedLoginOption")]
    pub selected_login_option: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiChallengeRequest {
    pub client_type: String,
    pub auth_session_id: SessionId,
    pub user_input: ObfuscatedString,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CertEnrollmentRequest {
    pub regkey: ObfuscatedString,
    pub password: ObfuscatedString,
    pub device_type: String,
    pub device_id: String,
    pub device_name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CertRenewalRequest {
    pub binary: String,
    pub password: ObfuscatedString,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientLoggingData {
    pub client_name: Option<String>,
    pub client_ver: Option<String>,
    pub client_build_number: Option<String>,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub os_edition: Option<String>,
    pub os_service_pack: Option<String>,
    pub os_build: Option<String>,
    pub os_bits: Option<String>,
    pub device_type: Option<String>,
    pub hardware_model: Option<String>,
    pub machine_name: Option<String>,
    pub machine_domain: Option<String>,
    pub device_id: Option<String>,
    pub mac_address: Option<StringList>,
    pub physical_ip: Option<Ipv4Addr>,
    pub is_compliant: Option<String>,
}

impl ClientLoggingData {
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
        let data = std::fs::read(path)?;
        let data: ClientLoggingData = serde_json::from_slice(&data)?;
        Ok(data)
    }
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
pub struct SignOutRequest {}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoliciesAndVersions {
    pub range: Vec<NetworkRange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum RequestData {
    Auth(AuthRequest),
    MultiChallenge(MultiChallengeRequest),
    ClientHello { client_info: ClientInfo },
    ClientSettings(ClientSettingsRequest),
    SignOut(SignOutRequest),
    CertEnrollment(CertEnrollmentRequest),
    CertRenewal(CertRenewalRequest),
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseHeader {
    pub id: Maybe<u32>,
    #[serde(rename = "type")]
    pub response_type: String,
    pub session_id: SessionId,
    pub return_code: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum ResponseData {
    Auth(AuthResponse),
    ClientSettings(ClientSettingsResponse),
    ServerInfo(GatewayInformation),
    Certificate(CertificateResponse),
    Generic(serde_json::Value),
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthResponse {
    pub authn_status: String,
    pub is_authenticated: Option<bool>,
    pub active_key: Option<ObfuscatedString>,
    pub server_fingerprint: Option<String>,
    pub server_cn: Option<String>,
    pub session_id: Option<SessionId>,
    pub active_key_timeout: Option<u64>,
    pub error_message: Option<ObfuscatedString>,
    pub error_id: Option<ObfuscatedString>,
    pub error_code: Option<u32>,
    pub prompt: Option<ObfuscatedString>,
    pub username: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientSettingsResponse {
    pub gw_internal_ip: Ipv4Addr,
    pub updated_policies: UpdatedPolicies,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CertificateResponse {
    pub error_code: u32,
    pub binary: Option<String>,
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
pub struct GatewayInformation {
    pub protocol_version: ProtocolVersion,
    pub connectivity_info: ConnectivityInfo,
    pub login_options_data: Option<LoginOptionsData>,
}

impl GatewayInformation {
    pub fn get_login_prompts(&self, login_type: &str) -> Vec<PromptInfo> {
        let Some(login_option) = self.get_login_option(login_type) else {
            return Vec::new();
        };

        let result = login_option
            .factors
            .values()
            .filter_map(|factor| match factor.custom_display_labels {
                LoginDisplayLabelSelect::LoginDisplayLabel(ref map) => map.get("password").map(|label| {
                    PromptInfo::new(
                        map.get("header").map(ToOwned::to_owned).unwrap_or_default(),
                        format!("{label}: "),
                    )
                }),
                LoginDisplayLabelSelect::Empty(_) => None,
            })
            .collect();

        trace!("Retrieved server prompts: {:?}", result);

        result
    }

    pub fn get_login_option(&self, login_type: &str) -> Option<&LoginOption> {
        self.login_options_data
            .as_ref()
            .and_then(|data| data.login_options_list.values().find(|option| option.id == login_type))
    }

    pub fn is_multi_factor_login_type(&self, login_type: &str) -> bool {
        self.get_login_option(login_type)
            .map(|opt| opt.is_multi_factor())
            .unwrap_or(true)
    }

    pub fn is_certificate_login_type(&self, login_type: &str) -> bool {
        self.get_login_option(login_type)
            .map(|opt| opt.is_certificate())
            .unwrap_or(true)
    }

    pub fn print_login_options(&self, server_address: &str) {
        let mut values = vec![
            ("login-options-server-address".to_owned(), server_address.to_owned()),
            (
                "login-options-server-ip".to_owned(),
                self.connectivity_info.server_ip.to_string(),
            ),
            (
                "login-options-client-enabled".to_owned(),
                self.connectivity_info.client_enabled.to_string(),
            ),
            (
                "login-options-supported-protocols".to_owned(),
                self.connectivity_info.supported_data_tunnel_protocols.join(", "),
            ),
            (
                "login-options-preferred-protocol".to_owned(),
                self.connectivity_info.connectivity_type.clone(),
            ),
            (
                "login-options-tcpt-port".to_owned(),
                self.connectivity_info.tcpt_port.to_string(),
            ),
            (
                "login-options-natt-port".to_owned(),
                self.connectivity_info.natt_port.to_string(),
            ),
        ];

        for fingerprint in self.connectivity_info.internal_ca_fingerprint.values() {
            values.push((
                "login-options-internal-ca-fingerprint".to_owned(),
                String::from_utf8_lossy(&snx_deobfuscate(fingerprint).unwrap_or_default()).into_owned(),
            ));
        }

        let mut options_list = self
            .login_options_data
            .clone()
            .map(|data| data.login_options_list)
            .unwrap_or_default();

        if options_list.is_empty() {
            options_list.insert(String::new(), LoginOption::unspecified());
        }

        for opt in options_list.into_values().filter(|opt| opt.show_realm != 0) {
            let factors = opt.factors.into_values().map(|factor| factor.factor_type).join(", ");
            values.push((format!("[{}]", opt.display_name), format!("{} ({})", opt.id, factors)));
        }

        let label_width = values
            .iter()
            .map(|(label, _)| {
                if label.starts_with("[") {
                    label.chars().count()
                } else {
                    i18n::translate(label).chars().count()
                }
            })
            .max()
            .unwrap_or_default();
        let mut result = String::new();
        for (index, (key, value)) in values.iter().enumerate() {
            let key_str = if key.starts_with("[") {
                key.clone()
            } else {
                i18n::translate(key)
            };
            result.push_str(&format!("{key_str:>label_width$}: {value}"));
            if index < values.len() - 1 {
                result.push('\n');
            }
        }

        println!("{result}");
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub protocol_version: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectivityInfo {
    pub default_authentication_method: Option<String>,
    pub client_enabled: bool,
    pub supported_data_tunnel_protocols: Vec<String>,
    pub connectivity_type: String,
    pub server_ip: Ipv4Addr,
    pub ipsec_transport: String,
    pub tcpt_port: u16,
    pub natt_port: u16,
    pub connect_with_certificate_url: String,
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
    pub const MOBILE_ACCESS_ID: &'static str = "ma";

    pub fn unspecified() -> Self {
        Self {
            id: "vpn_Username_Password".to_string(),
            secondary_realm_hash: String::new(),
            display_name: i18n::tr!("label-username-password"),
            show_realm: 1,
            factors: BTreeMap::from([(
                "1".to_owned(),
                LoginFactor {
                    factor_type: "password".to_owned(),
                    custom_display_labels: LoginDisplayLabelSelect::Empty(String::new()),
                },
            )]),
        }
    }

    pub fn mobile_access() -> Self {
        Self {
            id: Self::MOBILE_ACCESS_ID.to_string(),
            secondary_realm_hash: String::new(),
            display_name: i18n::tr!("label-mobile-access"),
            show_realm: 1,
            factors: BTreeMap::from([(
                "1".to_owned(),
                LoginFactor {
                    factor_type: "mobile_access".to_owned(),
                    custom_display_labels: LoginDisplayLabelSelect::Empty(String::new()),
                },
            )]),
        }
    }

    pub fn is_multi_factor(&self) -> bool {
        self.factors.values().any(|v| v.factor_type != "certificate")
    }

    pub fn is_certificate(&self) -> bool {
        self.factors.values().any(|v| v.factor_type == "certificate")
    }

    pub fn is_mobile_access(&self) -> bool {
        self.id == "ma"
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoginFactor {
    pub factor_type: String,
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
