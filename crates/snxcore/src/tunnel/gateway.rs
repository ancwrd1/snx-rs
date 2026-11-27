use std::{
    borrow::Cow,
    collections::HashMap,
    path::Path,
    sync::{
        Arc, LazyLock, Mutex,
        atomic::{AtomicU32, Ordering},
    },
    time::Duration,
};

use anyhow::anyhow;
use async_trait::async_trait;
use i18n::tr;
use itertools::Itertools;
use reqwest::{Certificate, Identity};
use rusqlite::OptionalExtension;
use secrecy::ExposeSecret;
use tokio::sync::OnceCell;
use tracing::{debug, trace, warn};

use crate::{
    model::{
        CacheType,
        params::{CertType, TlsVersion, TunnelParams},
        proto::*,
        wrappers::SessionId,
    },
    sexpr::SExpression,
    tunnel::GatewayConnector,
    util,
};

static REQUEST_ID: AtomicU32 = AtomicU32::new(1);
static GATEWAY_INFO_CACHE: LazyLock<Mutex<HashMap<String, Arc<OnceCell<GatewayInformation>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
const LONG_TIMEOUT: Duration = Duration::from_secs(600);
const SHORT_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

fn new_request_id() -> u32 {
    REQUEST_ID.fetch_add(1, Ordering::SeqCst)
}

fn connection(path: &Path) -> anyhow::Result<rusqlite::Connection> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }

    let conn = rusqlite::Connection::open(path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS gateway_information(
                    cache_key text not null primary key,
                    info text not null,
                    timestamp text not null)",
        rusqlite::params![],
    )?;

    Ok(conn)
}

#[derive(Clone)]
pub struct CccGatewayConnector {
    params: Arc<TunnelParams>,
    cache_type: CacheType,
}

impl CccGatewayConnector {
    pub fn new(params: Arc<TunnelParams>, cache_type: CacheType) -> Self {
        Self { params, cache_type }
    }

    fn cache_key(&self) -> String {
        format!(
            "{}/{}/{}",
            self.params.server_name,
            self.params.ignore_server_cert,
            self.params.ca_cert.iter().map(|s| s.to_string_lossy()).join(",")
        )
    }

    fn gateway_information_cell(&self) -> Arc<OnceCell<GatewayInformation>> {
        GATEWAY_INFO_CACHE
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entry(self.cache_key())
            .or_insert_with(|| Arc::new(OnceCell::new()))
            .clone()
    }

    fn store_gateway_information_in_memory(&self, info: GatewayInformation) {
        GATEWAY_INFO_CACHE
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(self.cache_key(), Arc::new(OnceCell::new_with(Some(info))));
    }

    fn new_auth_request(&self, username: &str) -> CccClientRequestData {
        let (request_type, username, password) = if self.params.cert_type == CertType::None {
            ("UserPass", Some(username.into()), Some(Default::default()))
        } else {
            ("CertAuth", None, None)
        };

        let mut client_logging_data = self
            .params
            .client_logging_data
            .as_ref()
            .and_then(|path| ClientLoggingData::load(path).ok())
            .unwrap_or_default();

        client_logging_data.os_name.get_or_insert_with(|| "Windows".to_owned());
        client_logging_data.device_id.get_or_insert_with(util::get_device_id);

        debug!("Client logging data: {:?}", client_logging_data);

        CccClientRequestData {
            header: RequestHeader {
                id: new_request_id(),
                request_type: request_type.to_owned(),
                session_id: None,
                protocol_version: None,
            },
            data: RequestData::Auth(AuthRequest {
                client_type: self.params.tunnel_type.as_client_type().to_owned(),
                username,
                password,
                client_logging_data: Some(client_logging_data),
                selected_login_option: Some(self.params.login_type.clone()),
                endpoint_os: None,
            }),
        }
    }

    fn new_challenge_code_request(&self, session_id: &SessionId, user_input: &str) -> CccClientRequestData {
        CccClientRequestData {
            header: RequestHeader {
                id: new_request_id(),
                request_type: "MultiChallange".to_string(),
                session_id: Some(session_id.clone()),
                protocol_version: None,
            },
            data: RequestData::MultiChallenge(MultiChallengeRequest {
                client_type: self.params.tunnel_type.as_client_type().to_owned(),
                auth_session_id: session_id.clone(),
                user_input: user_input.into(),
            }),
        }
    }

    fn new_client_settings_request(&self, session_id: &SessionId) -> CccClientRequestData {
        CccClientRequestData {
            header: RequestHeader {
                id: new_request_id(),
                request_type: "ClientSettings".to_string(),
                session_id: Some(session_id.clone()),
                protocol_version: Some(100),
            },
            data: RequestData::ClientSettings(ClientSettingsRequest {
                data: ClientSettingsData {
                    requested_policies_and_current_versions: PoliciesAndVersions {
                        range: Vec::new(),
                        mep: self.params.use_internal_gateway_ip.then(String::new),
                    },
                },
            }),
        }
    }

    fn new_signout_request(&self, session_id: &SessionId) -> CccClientRequestData {
        CccClientRequestData {
            header: RequestHeader {
                id: new_request_id(),
                request_type: "Signout".to_string(),
                session_id: Some(session_id.clone()),
                protocol_version: Some(100),
            },
            data: RequestData::SignOut(SignOutRequest::default()),
        }
    }

    fn new_client_hello_request(&self) -> CccClientRequestData {
        CccClientRequestData {
            header: RequestHeader {
                id: new_request_id(),
                request_type: "ClientHello".to_string(),
                session_id: None,
                protocol_version: None,
            },
            data: RequestData::ClientHello {
                client_info: ClientInfo {
                    client_type: self.params.tunnel_type.as_client_type().to_owned(),
                    client_version: 1,
                    client_support_saml: true,
                },
            },
        }
    }

    fn new_enroll_certificate_request(&self, registration_key: &str, password: &str) -> CccClientRequestData {
        CccClientRequestData {
            header: RequestHeader {
                id: new_request_id(),
                request_type: "CertEnrollmentRequest".to_string(),
                session_id: None,
                protocol_version: Some(100),
            },
            data: RequestData::CertEnrollment(CertEnrollmentRequest {
                regkey: registration_key.into(),
                password: password.into(),
                device_type: String::new(),
                device_id: String::new(),
                device_name: String::new(),
            }),
        }
    }

    fn new_renew_certificate_request<T: AsRef<[u8]>>(&self, pkcs12: T, password: &str) -> CccClientRequestData {
        CccClientRequestData {
            header: RequestHeader {
                id: new_request_id(),
                request_type: "CertRenewalRequest".to_string(),
                session_id: None,
                protocol_version: Some(100),
            },
            data: RequestData::CertRenewal(CertRenewalRequest {
                binary: hex::encode(pkcs12.as_ref().iter().rev().cloned().collect::<Vec<_>>()),
                password: password.into(),
            }),
        }
    }

    async fn send_request(&self, request: CccClientRequestData, timeout: Duration) -> anyhow::Result<SExpression> {
        let with_cert = matches!(request.data, RequestData::Auth(_));
        let expr = SExpression::from(CccClientRequest { data: request });

        let mut builder = reqwest::Client::builder().connect_timeout(CONNECT_TIMEOUT);

        match self.params.tls_version_max {
            TlsVersion::Tls12 => builder = builder.tls_version_max(reqwest::tls::Version::TLS_1_2),
            TlsVersion::Tls13 => builder = builder.tls_version_max(reqwest::tls::Version::TLS_1_3),
            TlsVersion::Default => {}
        }

        for ca_cert in &self.params.ca_cert {
            let data = tokio::fs::read(ca_cert).await?;
            let cert = Certificate::from_pem(&data).or_else(|_| Certificate::from_der(&data))?;
            builder = builder.tls_certs_merge(Some(cert));
        }

        if self.params.ignore_server_cert {
            warn!("Disabling all certificate checks!!!");
            builder = builder.danger_accept_invalid_certs(true);
        }

        let mut path = Cow::Borrowed("/clients/");

        if let (true, Some(cert_data)) = (with_cert, &self.params.cert_data) {
            let data = cert_data.expose_secret();
            let identity = match self.params.cert_type {
                CertType::Pkcs8 => Some(Identity::from_pkcs8_pem(data, data)?),
                CertType::Pkcs12 => Some(Identity::from_pkcs12_der(
                    data,
                    self.params
                        .cert_password
                        .as_ref()
                        .map(|s| s.expose_secret())
                        .unwrap_or_default(),
                )?),
                _ => None,
            };

            if let Some(identity) = identity {
                builder = builder.identity(identity);
                let cell = self.gateway_information_cell();
                path = match cell.get() {
                    Some(info) => Cow::Owned(info.connectivity_info.connect_with_certificate_url.clone()),
                    None => Cow::Borrowed("/clients/cert/"),
                };
            }
        }

        let client = builder.build()?;

        trace!("Request to server: {}", expr);

        let req = client
            .post(format!("https://{}{}", self.params.server_name, path))
            .body(expr.to_string())
            .build()?;

        let reply = tokio::time::timeout(timeout, client.execute(req))
            .await??
            .error_for_status()?
            .text()
            .await?;

        trace!("Reply from server: {}", reply);

        reply.parse::<SExpression>()
    }

    async fn send_ccc_request(&self, req: CccClientRequestData, timeout: Duration) -> anyhow::Result<ResponseData> {
        self.send_request(req, timeout)
            .await?
            .try_into::<CccServerResponse>()?
            .data
            .into_data()
    }

    async fn get_gateway_information_uncached(&self) -> anyhow::Result<GatewayInformation> {
        let data = self
            .send_ccc_request(self.new_client_hello_request(), SHORT_TIMEOUT)
            .await?;

        match data {
            ResponseData::ServerInfo(data) => Ok(data),
            _ => Err(anyhow!(tr!("error-invalid-gateway-info"))),
        }
    }

    async fn get_gateway_information_from_db(&self, path: &Path) -> anyhow::Result<Option<GatewayInformation>> {
        let cache_key = self.cache_key();
        let path = path.to_path_buf();

        tokio::task::spawn_blocking(move || {
            let conn = connection(&path)?;
            let info = conn
                .query_row(
                    "SELECT info FROM gateway_information WHERE cache_key = ?1",
                    rusqlite::params![&cache_key],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            Ok(info.map(|info| serde_json::from_str(&info)).transpose()?)
        })
        .await?
    }

    async fn store_gateway_information_in_db(&self, path: &Path, info: &GatewayInformation) -> anyhow::Result<()> {
        let cache_key = self.cache_key();
        let path = path.to_path_buf();
        let info = serde_json::to_string(&info)?;

        tokio::task::spawn_blocking(move || {
            connection(&path)?.execute(
                "INSERT OR REPLACE INTO gateway_information (cache_key, info, timestamp) VALUES (?1, ?2, current_timestamp)",
                rusqlite::params![&cache_key, &info],
            )?;

            Ok(())
        })
        .await?
    }

    async fn fetch_and_store_gateway_information(&self, path: &Path) -> anyhow::Result<GatewayInformation> {
        let info = self.get_gateway_information_uncached().await?;
        if let Err(e) = self.store_gateway_information_in_db(path, &info).await {
            warn!("Unable to cache gateway information in {}: {}", path.display(), e);
        }
        self.store_gateway_information_in_memory(info.clone());
        Ok(info)
    }
}

#[async_trait]
impl GatewayConnector for CccGatewayConnector {
    async fn authenticate(&self, username: &str) -> anyhow::Result<AuthResponse> {
        let req = self.new_auth_request(username);

        match self.send_ccc_request(req, LONG_TIMEOUT).await? {
            ResponseData::Auth(data) => Ok(data),
            _ => Err(anyhow!(tr!("error-invalid-auth-response"))),
        }
    }

    async fn challenge_code(&self, session_id: &SessionId, user_input: &str) -> anyhow::Result<AuthResponse> {
        let req = self.new_challenge_code_request(session_id, user_input);

        match self.send_ccc_request(req, LONG_TIMEOUT).await? {
            ResponseData::Auth(data) => Ok(data),
            _ => Err(anyhow!(tr!("error-invalid-auth-response"))),
        }
    }

    async fn get_client_settings(&self, session_id: &SessionId) -> anyhow::Result<ClientSettingsResponse> {
        let req = self.new_client_settings_request(session_id);

        match self.send_ccc_request(req, SHORT_TIMEOUT).await? {
            ResponseData::ClientSettings(data) => Ok(data),
            _ => Err(anyhow!(tr!("error-invalid-client-settings"))),
        }
    }

    async fn get_gateway_information(&self) -> anyhow::Result<GatewayInformation> {
        match &self.cache_type {
            CacheType::None => self.get_gateway_information_uncached().await,
            CacheType::Transient => {
                let cell = self.gateway_information_cell();
                cell.get_or_try_init(|| self.get_gateway_information_uncached())
                    .await
                    .cloned()
            }
            CacheType::Persistent(path) => {
                match self.get_gateway_information_from_db(path).await {
                    Ok(Some(info)) => return Ok(info),
                    Ok(None) => debug!("No cached gateway information in {}", path.display()),
                    Err(e) => warn!(
                        "Unable to read cached gateway information from {}: {}",
                        path.display(),
                        e
                    ),
                }
                self.fetch_and_store_gateway_information(path).await
            }
            CacheType::Writethrough(path) => self.fetch_and_store_gateway_information(path).await,
        }
    }

    async fn enroll_certificate(&self, registration_key: &str, password: &str) -> anyhow::Result<CertificateResponse> {
        let req = self.new_enroll_certificate_request(registration_key, password);

        match self.send_ccc_request(req, LONG_TIMEOUT).await? {
            ResponseData::Certificate(data) => Ok(data),
            _ => Err(anyhow!(tr!("error-invalid-cert-response"))),
        }
    }

    async fn renew_certificate(&self, pkcs12: &[u8], password: &str) -> anyhow::Result<CertificateResponse> {
        let req = self.new_renew_certificate_request(pkcs12, password);

        match self.send_ccc_request(req, LONG_TIMEOUT).await? {
            ResponseData::Certificate(data) => Ok(data),
            _ => Err(anyhow!(tr!("error-invalid-cert-response"))),
        }
    }

    async fn signout(&self, session_id: &SessionId) -> anyhow::Result<()> {
        let req = self.new_signout_request(session_id);

        self.send_ccc_request(req, SHORT_TIMEOUT).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client_settings_request(use_internal_gateway_ip: bool) -> String {
        let params = TunnelParams {
            use_internal_gateway_ip,
            ..Default::default()
        };
        let request =
            CccGatewayConnector::new(Arc::new(params), CacheType::None).new_client_settings_request(&"session".into());
        SExpression::from(CccClientRequest { data: request }).to_string()
    }

    #[test]
    fn mep_policy_is_requested_only_for_the_internal_gateway_ip() {
        assert!(client_settings_request(true).contains(":mep ()"));
        assert!(!client_settings_request(false).contains(":mep"));
    }

    fn gateway_information(protocol_version: u32) -> GatewayInformation {
        GatewayInformation {
            protocol_version: ProtocolVersion { protocol_version },
            connectivity_info: ConnectivityInfo {
                default_authentication_method: None,
                client_enabled: true,
                supported_data_tunnel_protocols: vec![],
                connectivity_type: String::new(),
                server_ip: std::net::Ipv4Addr::LOCALHOST,
                ipsec_transport: String::new(),
                tcpt_port: 0,
                natt_port: 0,
                connect_with_certificate_url: String::new(),
                internal_ca_fingerprint: Default::default(),
            },
            login_options_data: None,
        }
    }

    fn connector(path: &Path, ignore_server_cert: bool) -> CccGatewayConnector {
        let params = TunnelParams {
            server_name: "vpn.invalid".to_owned(),
            ignore_server_cert,
            ..Default::default()
        };
        CccGatewayConnector::new(Arc::new(params), CacheType::Persistent(path.to_path_buf()))
    }

    #[tokio::test]
    async fn gateway_information_round_trips_through_db() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("cache.db");
        let connector = connector(&path, false);

        assert_eq!(connector.get_gateway_information_from_db(&path).await.unwrap(), None);

        connector
            .store_gateway_information_in_db(&path, &gateway_information(100))
            .await
            .unwrap();
        connector
            .store_gateway_information_in_db(&path, &gateway_information(200))
            .await
            .unwrap();

        assert_eq!(
            connector.get_gateway_information_from_db(&path).await.unwrap(),
            Some(gateway_information(200))
        );
    }

    #[tokio::test]
    async fn cached_gateway_information_is_keyed_by_cert_settings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cache.db");

        connector(&path, true)
            .store_gateway_information_in_db(&path, &gateway_information(100))
            .await
            .unwrap();

        assert_eq!(
            connector(&path, false)
                .get_gateway_information_from_db(&path)
                .await
                .unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn persistent_cache_is_used_without_contacting_the_server() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cache.db");
        let connector = connector(&path, false);

        connector
            .store_gateway_information_in_db(&path, &gateway_information(100))
            .await
            .unwrap();

        assert_eq!(
            connector.get_gateway_information().await.unwrap(),
            gateway_information(100)
        );
    }
}
