use std::{
    borrow::Cow,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
    time::Duration,
};

use anyhow::anyhow;
use async_trait::async_trait;
use i18n::tr;
use reqwest::{Certificate, Identity};
use secrecy::ExposeSecret;
use tokio::sync::OnceCell;
use tracing::{debug, trace, warn};

use crate::{
    model::{
        params::{CertType, TlsVersion, TunnelParams},
        proto::*,
        wrappers::SessionId,
    },
    sexpr::SExpression,
    tunnel::GatewayConnector,
    util,
};

static REQUEST_ID: AtomicU32 = AtomicU32::new(1);
const LONG_TIMEOUT: Duration = Duration::from_secs(600);
const SHORT_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

fn new_request_id() -> u32 {
    REQUEST_ID.fetch_add(1, Ordering::SeqCst)
}

#[derive(Clone)]
pub struct CccGatewayConnector {
    params: Arc<TunnelParams>,
    gateway_information: Arc<OnceCell<GatewayInformation>>,
}

impl CccGatewayConnector {
    pub fn new(params: Arc<TunnelParams>) -> Self {
        Self {
            params,
            gateway_information: Arc::new(OnceCell::new()),
        }
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
            data: RequestData::ClientSettings(ClientSettingsRequest::default()),
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

        if let (true, Some(client_cert)) = (with_cert, &self.params.cert_path) {
            let data = std::fs::read(client_cert)?;
            let identity = match self.params.cert_type {
                CertType::Pkcs8 => Some(Identity::from_pkcs8_pem(&data, &data)?),
                CertType::Pkcs12 => Some(Identity::from_pkcs12_der(
                    &data,
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
                path = match self.gateway_information.get() {
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
        self.gateway_information
            .get_or_try_init(|| self.get_gateway_information_uncached())
            .await
            .cloned()
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
