use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use i18n::tr;
use tokio::sync::mpsc::Sender;
use tracing::{debug, warn};

use crate::{
    ccc::CccHttpClient,
    model::{
        MfaChallenge, MfaType, SessionState, VpnSession,
        params::{CertType, TunnelParams},
        proto::AuthResponse,
    },
    tunnel::{TunnelCommand, TunnelConnector, TunnelEvent, VpnTunnel, ssl::SslTunnel},
};

pub struct CccTunnelConnector {
    params: Arc<TunnelParams>,
    command_sender: Option<Sender<TunnelCommand>>,
}

impl CccTunnelConnector {
    pub async fn new(params: Arc<TunnelParams>) -> anyhow::Result<Self> {
        Ok(Self {
            params,
            command_sender: None,
        })
    }

    async fn process_auth_response(&self, data: AuthResponse) -> anyhow::Result<Arc<VpnSession>> {
        let session_id = data.session_id.unwrap_or_default();

        match data.authn_status.as_str() {
            "continue" => {
                return Ok(Arc::new(VpnSession {
                    ccc_session_id: session_id,
                    state: SessionState::PendingChallenge(MfaChallenge {
                        mfa_type: MfaType::PasswordInput,
                        prompt: data.prompt.map(|p| p.0).unwrap_or_default(),
                    }),
                    ipsec_session: None,
                    username: None,
                }));
            }
            "done" => {}
            other => {
                warn!("Authn status: {}", other);
                anyhow::bail!(tr!("error-auth-failed"));
            }
        }

        let active_key = match (data.is_authenticated, data.active_key) {
            (Some(true), Some(ref key)) => key.clone(),
            _ => {
                let msg = match (data.error_message, data.error_id, data.error_code) {
                    (Some(message), Some(id), Some(code)) => format!("[{} {}] {}", code, id.0, message.0),
                    _ => tr!("error-auth-failed"),
                };
                warn!("{}", msg);
                anyhow::bail!(msg);
            }
        };

        debug!("Authentication OK, session id: {session_id}");

        let session = Arc::new(VpnSession {
            ccc_session_id: session_id,
            state: SessionState::Authenticated(active_key.0),
            ipsec_session: None,
            username: data.username,
        });
        Ok(session)
    }
}

#[async_trait]
impl TunnelConnector for CccTunnelConnector {
    async fn authenticate(&mut self) -> anyhow::Result<Arc<VpnSession>> {
        debug!("Authenticating to endpoint: {}", self.params.server_name);

        if self.params.cert_type == CertType::None && self.params.user_name.is_empty() {
            Ok(Arc::new(VpnSession {
                ccc_session_id: String::new(),
                state: SessionState::PendingChallenge(MfaChallenge {
                    mfa_type: MfaType::UserNameInput,
                    prompt: "User name: ".to_owned(),
                }),
                ipsec_session: None,
                username: None,
            }))
        } else {
            let client = CccHttpClient::new(self.params.clone(), None);

            let data = client.authenticate().await?;

            self.process_auth_response(data).await
        }
    }

    async fn delete_session(&mut self) {}

    async fn restore_session(&mut self) -> anyhow::Result<Arc<VpnSession>> {
        Err(anyhow!(tr!("error-not-implemented")))
    }

    async fn challenge_code(&mut self, session: Arc<VpnSession>, user_input: &str) -> anyhow::Result<Arc<VpnSession>> {
        debug!(
            "Authenticating with challenge code to endpoint: {}",
            self.params.server_name
        );

        let data = if session.ccc_session_id.is_empty() {
            let params = Arc::new(TunnelParams {
                user_name: user_input.to_owned(),
                ..(*self.params).clone()
            });
            let client = CccHttpClient::new(params, Some(session.clone()));
            client.authenticate().await?
        } else {
            let client = CccHttpClient::new(self.params.clone(), Some(session.clone()));
            client.challenge_code(user_input).await?
        };

        self.process_auth_response(data).await
    }

    async fn create_tunnel(
        &mut self,
        session: Arc<VpnSession>,
        command_sender: Sender<TunnelCommand>,
    ) -> anyhow::Result<Box<dyn VpnTunnel + Send>> {
        self.command_sender = Some(command_sender);
        Ok(Box::new(SslTunnel::create(self.params.clone(), session).await?))
    }

    async fn terminate_tunnel(&mut self, signout: bool) -> anyhow::Result<()> {
        if let Some(sender) = self.command_sender.take() {
            let _ = sender.send(TunnelCommand::Terminate(signout)).await;
        }
        Ok(())
    }

    async fn handle_tunnel_event(&mut self, event: TunnelEvent) -> anyhow::Result<()> {
        match event {
            TunnelEvent::Connected(_) => {
                debug!("Tunnel connected");
            }
            TunnelEvent::Disconnected => {
                debug!("Tunnel disconnected");
            }
            TunnelEvent::RekeyCheck => {}
            TunnelEvent::RemoteControlData(_) => {
                warn!("Tunnel data received: shouldn't happen for SSL tunnel!");
            }
            TunnelEvent::Rekeyed(_) => {
                debug!("Tunnel rekeyed");

            }
        }
        Ok(())
    }
}
