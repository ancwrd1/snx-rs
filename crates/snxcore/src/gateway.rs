use async_trait::async_trait;

use crate::model::{
    proto::{AuthResponse, CertificateResponse, ClientSettingsResponse, GatewayInformation},
    wrappers::SessionId,
};

pub mod ccc;

#[async_trait]
pub trait GatewayConnector {
    async fn authenticate(&self, username: &str) -> anyhow::Result<AuthResponse>;
    async fn challenge_code(&self, session_id: &SessionId, user_input: &str) -> anyhow::Result<AuthResponse>;
    async fn get_client_settings(&self, session_id: &SessionId) -> anyhow::Result<ClientSettingsResponse>;
    async fn get_gateway_information(&self) -> anyhow::Result<GatewayInformation>;
    async fn enroll_certificate(&self, registration_key: &str, password: &str) -> anyhow::Result<CertificateResponse>;
    async fn renew_certificate(&self, pkcs12: &[u8], password: &str) -> anyhow::Result<CertificateResponse>;
    async fn signout(&self, session_id: &SessionId) -> anyhow::Result<()>;
}
