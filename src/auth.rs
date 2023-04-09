use std::sync::atomic::{AtomicU32, Ordering};

use crate::{
    model::{CccClientRequest, CccServerResponse, RequestData, RequestHeader},
    sexpr, util,
};

static REQUEST_ID: AtomicU32 = AtomicU32::new(2);

pub struct SnxHttpAuthenticator {
    server_name: String,
    auth: (String, String),
}

impl SnxHttpAuthenticator {
    pub fn new(server_name: String, auth: (String, String)) -> Self {
        Self { server_name, auth }
    }

    fn new_request(&self, session_id: Option<&str>) -> CccClientRequest {
        CccClientRequest {
            header: RequestHeader {
                id: REQUEST_ID.fetch_add(1, Ordering::SeqCst).to_string(),
                request_type: "UserPass".to_string(),
                session_id: session_id.unwrap_or_default().to_string(),
            },
            data: RequestData {
                client_type: "TRAC".to_string(),
                endpoint_os: "unix".to_string(),
                username: util::encode_to_hex(&self.auth.0),
                password: util::encode_to_hex(&self.auth.1),
            },
        }
    }

    pub async fn authenticate(
        &self,
        session_id: Option<&str>,
    ) -> anyhow::Result<CccServerResponse> {
        let expr = sexpr::encode(CccClientRequest::NAME, self.new_request(session_id))?;

        let client = reqwest::Client::new();

        let req = client
            .post(format!("https://{}/clients/", self.server_name))
            .body(expr)
            .build()?;

        let bytes = client
            .execute(req)
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        let s_bytes = String::from_utf8_lossy(&bytes);

        let (_, server_response) = sexpr::decode::<_, CccServerResponse>(&s_bytes)?;

        Ok(server_response)
    }
}
