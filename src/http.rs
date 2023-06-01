use std::sync::atomic::{AtomicU32, Ordering};

use anyhow::anyhow;

use crate::{model::*, params::TunnelParams, sexpr, util};

static REQUEST_ID: AtomicU32 = AtomicU32::new(2);

pub struct SnxHttpClient(TunnelParams);

impl SnxHttpClient {
    pub fn new(params: &TunnelParams) -> Self {
        Self(params.clone())
    }

    fn new_auth_request(&self, session_id: Option<&str>) -> CccClientRequest {
        CccClientRequest {
            header: RequestHeader {
                id: REQUEST_ID.fetch_add(1, Ordering::SeqCst),
                request_type: "UserPass".to_string(),
                session_id: session_id.unwrap_or_default().to_string(),
                protocol_version: None,
            },
            data: RequestData::Password(PasswordData {
                client_type: self.0.tunnel_type.as_client_type().to_owned(),
                username: util::encode_to_hex(&self.0.user_name),
                password: util::encode_to_hex(&self.0.password),
                client_logging_data: Some(ClientLoggingData {
                    // Checkpoint gateway checks this and if it's missing or not "Adnroid" IPSec traffic is blocked
                    os_name: Some("\"Android\"".to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        }
    }

    fn new_key_management_request(&self, session_id: &str) -> CccClientRequest {
        CccClientRequest {
            header: RequestHeader {
                id: REQUEST_ID.fetch_add(1, Ordering::SeqCst),
                request_type: "KeyManagement".to_string(),
                session_id: session_id.to_string(),
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
                session_id: session_id.to_string(),
                protocol_version: Some(100),
            },
            data: RequestData::Wrapped(wrapped),
        }
    }

    fn new_location_awareness_request(&self, source_ip: &str) -> CccClientRequest {
        CccClientRequest {
            header: RequestHeader {
                id: REQUEST_ID.fetch_add(1, Ordering::SeqCst),
                request_type: "LocationAwareness".to_string(),
                session_id: String::new(),
                protocol_version: Some(100),
            },
            data: RequestData::LocationAwareness(LocationAwarenessData {
                source_ip: source_ip.to_owned(),
            }),
        }
    }

    async fn send_request(&self, req: CccClientRequest) -> anyhow::Result<CccServerResponse> {
        let expr = sexpr::encode(CccClientRequest::NAME, req)?;
        println!("req: {}", expr);
        let client = reqwest::Client::new();

        let req = client
            .post(format!("https://{}/clients/", self.0.server_name))
            .body(expr)
            .build()?;

        let bytes = client.execute(req).await?.error_for_status()?.bytes().await?;

        let s_bytes = String::from_utf8_lossy(&bytes);
        println!("resp: {}", s_bytes);

        let (_, server_response) = sexpr::decode::<_, CccServerResponse>(&s_bytes)?;

        Ok(server_response)
    }

    pub async fn authenticate(&self, session_id: Option<&str>) -> anyhow::Result<AuthResponseData> {
        let server_response = self.send_request(self.new_auth_request(session_id)).await?;

        match server_response.data {
            ResponseData::Auth(data) => Ok(data),
            _ => Err(anyhow!("Invalid auth response!")),
        }
    }

    pub async fn get_ipsec_tunnel_params(&self, session_id: &str) -> anyhow::Result<IpsecResponseData> {
        let server_response = self.send_request(self.new_key_management_request(session_id)).await?;

        match server_response.data {
            ResponseData::Ipsec(data) => Ok(data),
            _ => Err(anyhow!("Invalid ipsec response!")),
        }
    }

    pub async fn get_client_settings(&self, session_id: &str) -> anyhow::Result<ClientSettingsResponseData> {
        let server_response = self.send_request(self.new_client_settings_request(session_id)).await?;

        match server_response.data {
            ResponseData::ClientSettings(data) => Ok(data),
            _ => Err(anyhow!("Invalid client settings response!")),
        }
    }

    pub async fn get_external_ip(&self, source_ip: &str) -> anyhow::Result<LocationAwarenessResponseData> {
        let server_response = self
            .send_request(self.new_location_awareness_request(source_ip))
            .await?;

        match server_response.data {
            ResponseData::LocationAwareness(data) => Ok(data),
            _ => Err(anyhow!("Invalid location awareness response!")),
        }
    }
}
