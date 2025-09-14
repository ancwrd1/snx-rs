use std::{
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
    time::Duration,
};

use anyhow::anyhow;
use i18n::tr;
use reqwest::{Certificate, Identity};
use tracing::{trace, warn};

use crate::{
    model::{
        VpnSession,
        params::{CertType, TunnelParams},
        proto::*,
    },
    sexpr::SExpression,
};

static REQUEST_ID: AtomicU32 = AtomicU32::new(2);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(600);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const INFO_TIMEOUT: Duration = Duration::from_secs(10);

fn new_request_id() -> u32 {
    REQUEST_ID.fetch_add(1, Ordering::SeqCst)
}

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

    fn new_auth_request(&self) -> CccClientRequestData {
        let (request_type, username, password) = if self.params.cert_type == CertType::None {
            (
                "UserPass",
                Some(self.params.user_name.as_str().into()),
                Some(Default::default()),
            )
        } else {
            ("CertAuth", None, None)
        };

        CccClientRequestData {
            header: RequestHeader {
                id: new_request_id(),
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
                    device_id: Some(crate::util::get_device_id()),
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
                id: new_request_id(),
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

    fn new_client_settings_request(&self) -> CccClientRequestData {
        CccClientRequestData {
            header: RequestHeader {
                id: new_request_id(),
                request_type: "ClientSettings".to_string(),
                session_id: self.session_id(),
                protocol_version: Some(100),
            },
            data: RequestData::ClientSettings(ClientSettingsRequest::default()),
        }
    }

    fn new_signout_request(&self) -> CccClientRequestData {
        CccClientRequestData {
            header: RequestHeader {
                id: new_request_id(),
                request_type: "Signout".to_string(),
                session_id: self.session_id(),
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

    async fn send_request(&self, request: CccClientRequestData, timeout: Duration) -> anyhow::Result<SExpression> {
        let with_cert = matches!(request.data, RequestData::Auth(_));
        let expr = SExpression::from(CccClientRequest { data: request });

        let mut builder = reqwest::Client::builder().connect_timeout(CONNECT_TIMEOUT);

        for ca_cert in &self.params.ca_cert {
            let data = tokio::fs::read(ca_cert).await?;
            let cert = Certificate::from_pem(&data).or_else(|_| Certificate::from_der(&data))?;
            builder = builder.add_root_certificate(cert);
        }

        if self.params.ignore_server_cert {
            warn!("Disabling all certificate checks!!!");
            builder = builder.danger_accept_invalid_certs(true);
        }

        let mut path = "/clients/";

        if let (true, Some(client_cert)) = (with_cert, &self.params.cert_path) {
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
                path = "/clients/cert/";
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

    async fn send_ccc_request(&self, req: CccClientRequestData) -> anyhow::Result<ResponseData> {
        self.send_request(req, REQUEST_TIMEOUT)
            .await?
            .try_into::<CccServerResponse>()?
            .data
            .into_data()
    }

    pub async fn authenticate(&self) -> anyhow::Result<AuthResponse> {
        let req = self.new_auth_request();

        match self.send_ccc_request(req).await? {
            ResponseData::Auth(data) => Ok(data),
            _ => Err(anyhow!(tr!("error-invalid-auth-response"))),
        }
    }

    pub async fn challenge_code(&self, user_input: &str) -> anyhow::Result<AuthResponse> {
        let req = self.new_challenge_code_request(user_input);

        match self.send_ccc_request(req).await? {
            ResponseData::Auth(data) => Ok(data),
            _ => Err(anyhow!(tr!("error-invalid-auth-response"))),
        }
    }

    pub async fn get_client_settings(&self) -> anyhow::Result<ClientSettingsResponse> {
        let req = self.new_client_settings_request();

        match self.send_ccc_request(req).await? {
            ResponseData::ClientSettings(data) => Ok(data),
            _ => Err(anyhow!(tr!("error-invalid-client-settings"))),
        }
    }

    pub async fn get_server_info(&self) -> anyhow::Result<SExpression> {
        self.send_request(self.new_client_hello_request(), INFO_TIMEOUT).await
    }

    pub async fn signout(&self) -> anyhow::Result<()> {
        let req = self.new_signout_request();

        self.send_ccc_request(req).await?;

        Ok(())
    }
}
