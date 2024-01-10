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
use serde::Deserialize;
use tracing::trace;

use crate::{
    model::{params::TunnelParams, snx::*},
    sexpr,
};

static REQUEST_ID: AtomicU32 = AtomicU32::new(2);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(600);

pub struct SnxHttpClient(Arc<TunnelParams>);

impl SnxHttpClient {
    pub fn new(params: Arc<TunnelParams>) -> Self {
        Self(params)
    }

    fn new_auth_request(&self, session_id: Option<&str>) -> CccClientRequest {
        let (request_type, username, password) = if self.0.client_cert.is_none() {
            (
                "UserPass",
                Some(self.0.user_name.as_str().into()),
                Some(self.0.password.as_str().into()),
            )
        } else {
            ("CertAuth", None, None)
        };
        CccClientRequest {
            header: RequestHeader {
                id: REQUEST_ID.fetch_add(1, Ordering::SeqCst),
                request_type: request_type.to_owned(),
                session_id: session_id.map(ToOwned::to_owned),
                protocol_version: None,
            },
            data: RequestData::Auth(AuthData {
                client_type: self.0.tunnel_type.as_client_type().to_owned(),
                username,
                password,
                client_logging_data: Some(ClientLoggingData {
                    // Checkpoint gateway checks this and if it's missing or not "Android" the IPSec traffic is blocked
                    os_name: Some("Android".into()),
                    ..Default::default()
                }),
                selected_login_option: Some(self.0.login_type.clone()),
                endpoint_os: None,
            }),
        }
    }

    fn new_challenge_code_request(&self, session_id: &str, user_input: &str) -> CccClientRequest {
        CccClientRequest {
            header: RequestHeader {
                id: REQUEST_ID.fetch_add(1, Ordering::SeqCst),
                request_type: "MultiChallange".to_string(),
                session_id: Some(session_id.to_owned()),
                protocol_version: None,
            },
            data: RequestData::MultiChallenge(MultiChallengeData {
                client_type: self.0.tunnel_type.as_client_type().to_owned(),
                auth_session_id: session_id.to_owned(),
                user_input: user_input.into(),
            }),
        }
    }

    fn new_key_management_request(&self, session_id: &str) -> CccClientRequest {
        CccClientRequest {
            header: RequestHeader {
                id: REQUEST_ID.fetch_add(1, Ordering::SeqCst),
                request_type: "KeyManagement".to_string(),
                session_id: Some(session_id.to_string()),
                protocol_version: Some(100),
            },
            data: RequestData::Ipsec(IpsecData {
                spi: rand::random::<u32>(),
                rekey: false,
                req_om_addr: 0x00000000,
            }),
        }
    }

    fn new_client_settings_request(&self, session_id: &str) -> CccClientRequest {
        let data = sexpr::encode_value(ClientSettingsData::default()).unwrap_or_default();
        let wrapped = format!("ClientSettings {}", data);

        CccClientRequest {
            header: RequestHeader {
                id: REQUEST_ID.fetch_add(1, Ordering::SeqCst),
                request_type: "ClientSettings".to_string(),
                session_id: Some(session_id.to_string()),
                protocol_version: Some(100),
            },
            data: RequestData::Wrapped(wrapped),
        }
    }

    fn new_location_awareness_request(&self, source_ip: Ipv4Addr) -> CccClientRequest {
        CccClientRequest {
            header: RequestHeader {
                id: REQUEST_ID.fetch_add(1, Ordering::SeqCst),
                request_type: "LocationAwareness".to_string(),
                session_id: None,
                protocol_version: Some(100),
            },
            data: RequestData::LocationAwareness(LocationAwarenessData { source_ip }),
        }
    }

    fn new_client_hello_request(&self) -> CccClientRequest {
        CccClientRequest {
            header: RequestHeader {
                id: REQUEST_ID.fetch_add(1, Ordering::SeqCst),
                request_type: "ClientHello".to_string(),
                session_id: None,
                protocol_version: None,
            },
            data: RequestData::ClientInfo {
                client_info: ClientInfo {
                    client_type: self.0.tunnel_type.as_client_type().to_owned(),
                    client_version: 1,
                    client_support_saml: true,
                },
            },
        }
    }

    async fn send_request<T>(&self, req: CccClientRequest) -> anyhow::Result<T>
    where
        for<'de> T: Deserialize<'de>,
    {
        let expr = sexpr::encode(CccClientRequest::NAME, req)?;

        let mut builder = reqwest::Client::builder();

        if let Some(ref ca_cert) = self.0.ca_cert {
            let data = tokio::fs::read(ca_cert).await?;
            let cert = Certificate::from_pem(&data).or_else(|_| Certificate::from_der(&data))?;
            builder = builder.add_root_certificate(cert);
        }

        if self.0.no_cert_check {
            builder = builder.danger_accept_invalid_hostnames(true);
        }

        if let Some(ref client_cert) = self.0.client_cert {
            let data = std::fs::read(client_cert)?;
            builder = builder.identity(Identity::from_pkcs8_pem(&data, &data)?);
        }

        let client = builder.build()?;

        trace!("Request to server: {}", expr);

        let req = client
            .post(format!("https://{}/clients/", self.0.server_name))
            .body(expr)
            .build()?;

        let reply = tokio::time::timeout(REQUEST_TIMEOUT, client.execute(req))
            .await??
            .error_for_status()?
            .text()
            .await?;

        trace!("Reply from server: {}", reply);

        let (_, server_response) = sexpr::decode::<_, T>(&reply)?;

        Ok(server_response)
    }

    pub async fn authenticate(&self, session_id: Option<&str>) -> anyhow::Result<AuthResponseData> {
        let server_response = self
            .send_request::<CccServerResponse>(self.new_auth_request(session_id))
            .await?;

        match server_response.to_data()? {
            ResponseData::Auth(data) => Ok(data),
            _ => Err(anyhow!("Invalid auth response!")),
        }
    }

    pub async fn challenge_code(&self, session_id: &str, user_input: &str) -> anyhow::Result<AuthResponseData> {
        let server_response = self
            .send_request::<CccServerResponse>(self.new_challenge_code_request(session_id, user_input))
            .await?;

        match server_response.to_data()? {
            ResponseData::Auth(data) => Ok(data),
            _ => Err(anyhow!("Invalid auth response!")),
        }
    }

    pub async fn get_ipsec_tunnel_params(&self, session_id: &str) -> anyhow::Result<IpsecResponseData> {
        let server_response = self
            .send_request::<CccServerResponse>(self.new_key_management_request(session_id))
            .await?;

        match server_response.to_data()? {
            ResponseData::Ipsec(data) => Ok(data),
            _ => Err(anyhow!("Invalid ipsec response!")),
        }
    }

    pub async fn get_client_settings(&self, session_id: &str) -> anyhow::Result<ClientSettingsResponseData> {
        let server_response = self
            .send_request::<CccServerResponse>(self.new_client_settings_request(session_id))
            .await?;

        match server_response.to_data()? {
            ResponseData::ClientSettings(data) => Ok(data),
            _ => Err(anyhow!("Invalid client settings response!")),
        }
    }

    pub async fn get_external_ip(&self, source_ip: Ipv4Addr) -> anyhow::Result<LocationAwarenessResponseData> {
        let server_response = self
            .send_request::<CccServerResponse>(self.new_location_awareness_request(source_ip))
            .await?;

        match server_response.to_data()? {
            ResponseData::LocationAwareness(data) => Ok(data),
            _ => Err(anyhow!("Invalid location awareness response!")),
        }
    }

    pub async fn get_server_info(&self) -> anyhow::Result<serde_json::Value> {
        let server_response = self
            .send_request::<serde_json::Value>(self.new_client_hello_request())
            .await?;
        Ok(server_response)
    }
}
