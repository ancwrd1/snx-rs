use std::{
    net::Ipv4Addr,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::anyhow;
use reqwest::{Certificate, Identity};
use tracing::{trace, warn};

use crate::{
    model::{
        params::{CertType, TunnelParams},
        proto::*,
        VpnSession,
    },
    sexpr::SExpression,
};

static REQUEST_ID: AtomicU32 = AtomicU32::new(2);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(600);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct CccHttpClient {
    params: Arc<TunnelParams>,
    session: Option<Arc<VpnSession>>,
}

impl CccHttpClient {
    pub fn new(params: Arc<TunnelParams>, session: Option<Arc<VpnSession>>) -> Self {
        Self { params, session }
    }

    fn session_id(&self) -> Option<String> {
        self.session.as_ref().map(|s| s.ccc_session_id.clone())
    }

    fn new_request_id(&self) -> u32 {
        REQUEST_ID.fetch_add(1, Ordering::SeqCst)
    }

    fn new_auth_request(&self) -> CccClientRequestData {
        let (request_type, username, password) = if self.params.cert_type == CertType::None {
            (
                "UserPass",
                Some(self.params.user_name.as_str().into()),
                Some(self.params.password.as_str().into()),
            )
        } else {
            ("CertAuth", None, None)
        };

        CccClientRequestData {
            header: RequestHeader {
                id: self.new_request_id(),
                request_type: request_type.to_owned(),
                session_id: self.session_id(),
                protocol_version: None,
            },
            data: RequestData::Auth(AuthRequest {
                client_type: self.params.tunnel_type.as_client_type().to_owned(),
                username,
                password,
                client_logging_data: Some(ClientLoggingData {
                    os_name: Some("Windows".into()),
                    device_id: Some(crate::util::get_device_id().into()),
                    ..Default::default()
                }),
                selected_login_option: Some(self.params.login_type.clone()),
                endpoint_os: None,
            }),
        }
    }

    fn new_challenge_code_request(&self, user_input: &str) -> CccClientRequestData {
        CccClientRequestData {
            header: RequestHeader {
                id: self.new_request_id(),
                request_type: "MultiChallange".to_string(),
                session_id: self.session_id(),
                protocol_version: None,
            },
            data: RequestData::MultiChallenge(MultiChallengeRequest {
                client_type: self.params.tunnel_type.as_client_type().to_owned(),
                auth_session_id: self.session_id().unwrap_or_default(),
                user_input: user_input.into(),
            }),
        }
    }

    fn new_key_management_request(&self, spi: u32) -> CccClientRequestData {
        CccClientRequestData {
            header: RequestHeader {
                id: self.new_request_id(),
                request_type: "KeyManagement".to_string(),
                session_id: self.session_id(),
                protocol_version: Some(100),
            },
            data: RequestData::KeyManagement(KeyManagementRequest {
                spi,
                rekey: false,
                req_om_addr: 0x00000000,
            }),
        }
    }

    fn new_client_settings_request(&self) -> CccClientRequestData {
        CccClientRequestData {
            header: RequestHeader {
                id: self.new_request_id(),
                request_type: "ClientSettings".to_string(),
                session_id: self.session_id(),
                protocol_version: Some(100),
            },
            data: RequestData::ClientSettings(ClientSettingsRequest::default()),
        }
    }

    fn new_location_awareness_request(&self, source_ip: Ipv4Addr) -> CccClientRequestData {
        CccClientRequestData {
            header: RequestHeader {
                id: self.new_request_id(),
                request_type: "LocationAwareness".to_string(),
                session_id: None,
                protocol_version: Some(100),
            },
            data: RequestData::LocationAwareness(LocationAwarenessRequest { source_ip }),
        }
    }

    fn new_client_hello_request(&self) -> CccClientRequestData {
        CccClientRequestData {
            header: RequestHeader {
                id: self.new_request_id(),
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

    async fn send_raw_request(&self, request: CccClientRequestData) -> anyhow::Result<SExpression> {
        let expr = SExpression::from(CccClientRequest { data: request });

        let mut builder = reqwest::Client::builder().connect_timeout(CONNECT_TIMEOUT);

        for ca_cert in &self.params.ca_cert {
            let data = tokio::fs::read(ca_cert).await?;
            let cert = Certificate::from_pem(&data).or_else(|_| Certificate::from_der(&data))?;
            builder = builder.add_root_certificate(cert);
        }

        if self.params.no_cert_check {
            builder = builder.danger_accept_invalid_hostnames(true);
        }

        if self.params.ignore_server_cert {
            warn!("Disabling all certificate checks!!!");
            builder = builder.danger_accept_invalid_certs(true);
        }

        let path = if let Some(ref client_cert) = self.params.cert_path {
            let data = std::fs::read(client_cert)?;
            let identity = match self.params.cert_type {
                CertType::Pkcs8 => Some(Identity::from_pkcs8_pem(&data, &data)?),
                CertType::Pkcs12 => Some(Identity::from_pkcs12_der(
                    &data,
                    self.params.cert_password.as_deref().unwrap_or_default(),
                )?),
                _ => None,
            };
            if let Some(identity) = identity {
                builder = builder.identity(identity);
                "/clients/cert/"
            } else {
                "/clients/"
            }
        } else {
            "/clients/"
        };

        let client = builder.build()?;

        trace!("Request to server: {}", expr);

        let req = client
            .post(format!("https://{}{}", self.params.server_name, path))
            .body(expr.to_string())
            .build()?;

        let reply = tokio::time::timeout(REQUEST_TIMEOUT, client.execute(req))
            .await??
            .error_for_status()?
            .text()
            .await?;

        trace!("Reply from server: {}", reply);

        reply.parse::<SExpression>()
    }

    async fn send_request(&self, request: CccClientRequestData) -> anyhow::Result<CccServerResponseData> {
        Ok(self
            .send_raw_request(request)
            .await?
            .try_into::<CccServerResponse>()?
            .data)
    }

    async fn send_ccc_request(&self, req: CccClientRequestData) -> anyhow::Result<ResponseData> {
        self.send_request(req).await?.into_data()
    }

    pub async fn authenticate(&self) -> anyhow::Result<AuthResponse> {
        let req = self.new_auth_request();

        match self.send_ccc_request(req).await? {
            ResponseData::Auth(data) => Ok(data),
            _ => Err(anyhow!("Invalid authentication response!")),
        }
    }

    pub async fn challenge_code(&self, user_input: &str) -> anyhow::Result<AuthResponse> {
        let req = self.new_challenge_code_request(user_input);

        match self.send_ccc_request(req).await? {
            ResponseData::Auth(data) => Ok(data),
            _ => Err(anyhow!("Invalid authentication response!")),
        }
    }

    pub async fn get_ipsec_tunnel_params(&self, spi: u32) -> anyhow::Result<KeyManagementResponse> {
        let req = self.new_key_management_request(spi);

        match self.send_ccc_request(req).await? {
            ResponseData::KeyManagement(data) => Ok(data),
            _ => Err(anyhow!("Invalid key management response!")),
        }
    }

    pub async fn get_client_settings(&self) -> anyhow::Result<ClientSettingsResponse> {
        let req = self.new_client_settings_request();

        match self.send_ccc_request(req).await? {
            ResponseData::ClientSettings(data) => Ok(data),
            _ => Err(anyhow!("Invalid client settings response!")),
        }
    }

    pub async fn get_external_ip(&self, source_ip: Ipv4Addr) -> anyhow::Result<LocationAwarenessResponse> {
        let req = self.new_location_awareness_request(source_ip);

        match self.send_ccc_request(req).await? {
            ResponseData::LocationAwareness(data) => Ok(data),
            _ => Err(anyhow!("Invalid location awareness response!")),
        }
    }

    pub async fn get_server_info(&self) -> anyhow::Result<SExpression> {
        self.send_raw_request(self.new_client_hello_request()).await
    }
}
