use std::{collections::VecDeque, sync::Arc, time::Duration};

use async_trait::async_trait;
use chrono::Local;
use snxcore::{
    browser::BrowserController,
    controller::{ServiceCommand, ServiceController},
    model::{
        AuthenticatedSession, ConnectionInfo, ConnectionStatus, MfaChallenge, MfaType, PromptInfo, SessionState,
        VpnSession, params::TunnelParams,
    },
    prompt::SecurePrompt,
    server::CommandServer,
    tunnel::{TunnelCommand, TunnelConnector, TunnelConnectorFactory, TunnelEvent, VpnTunnel},
};
use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;

const USERNAME: &str = "username";
const PASSWORD: &str = "challenge";

#[derive(Clone, Default)]
struct MockTunnelConnectorFactory;

#[async_trait]
impl TunnelConnectorFactory for MockTunnelConnectorFactory {
    async fn create(&self, params: Arc<TunnelParams>) -> anyhow::Result<Box<dyn TunnelConnector + Send + Sync>> {
        Ok(Box::new(MockTunnelConnector {
            params,
            command_sender: None,
        }))
    }
}

struct MockTunnelConnector {
    params: Arc<TunnelParams>,
    command_sender: Option<Sender<TunnelCommand>>,
}

#[async_trait]
impl TunnelConnector for MockTunnelConnector {
    async fn authenticate(&mut self) -> anyhow::Result<Arc<VpnSession>> {
        Ok(Arc::new(VpnSession {
            ccc_session_id: "1234".to_string(),
            state: SessionState::PendingChallenge(MfaChallenge {
                mfa_type: MfaType::UserNameInput,
                prompt: "username".to_string(),
            }),
            username: None,
        }))
    }

    async fn delete_session(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn restore_session(&mut self) -> anyhow::Result<Arc<VpnSession>> {
        anyhow::bail!("mock restore_session not implemented")
    }

    async fn challenge_code(&mut self, session: Arc<VpnSession>, user_input: &str) -> anyhow::Result<Arc<VpnSession>> {
        match user_input {
            USERNAME => Ok(Arc::new(VpnSession {
                state: SessionState::PendingChallenge(MfaChallenge {
                    mfa_type: MfaType::PasswordInput,
                    prompt: "password".to_string(),
                }),
                username: None,
                ..(*session).clone()
            })),
            PASSWORD => Ok(Arc::new(VpnSession {
                state: SessionState::Authenticated(AuthenticatedSession::SslSessionKey("key".to_string())),
                username: None,
                ..(*session).clone()
            })),
            _ => {
                anyhow::bail!("invalid user input");
            }
        }
    }

    async fn create_tunnel(
        &mut self,
        _session: Arc<VpnSession>,
        command_sender: Sender<TunnelCommand>,
    ) -> anyhow::Result<Box<dyn VpnTunnel + Send>> {
        self.command_sender = Some(command_sender);
        Ok(Box::new(MockTunnel {
            params: self.params.clone(),
        }))
    }

    async fn terminate_tunnel(&mut self, signout: bool) -> anyhow::Result<()> {
        if let Some(sender) = self.command_sender.take() {
            let _ = sender.send(TunnelCommand::Terminate(signout)).await;
        }
        Ok(())
    }

    async fn handle_tunnel_event(&mut self, _event: TunnelEvent) -> anyhow::Result<()> {
        Ok(())
    }
}

struct MockTunnel {
    params: Arc<TunnelParams>,
}

#[async_trait]
impl VpnTunnel for MockTunnel {
    async fn run(
        self: Box<Self>,
        mut command_receiver: Receiver<TunnelCommand>,
        event_sender: Sender<TunnelEvent>,
    ) -> anyhow::Result<()> {
        let info = ConnectionInfo {
            since: Some(Local::now()),
            server_name: self.params.server_name.clone(),
            username: USERNAME.to_string(),
            login_type: self.params.login_type.clone(),
            ..Default::default()
        };

        event_sender.send(TunnelEvent::Connected(Box::new(info))).await?;

        while let Some(command) = command_receiver.recv().await {
            if matches!(command, TunnelCommand::Terminate(_)) {
                break;
            }
        }
        Ok(())
    }
}

struct MockPrompt;

#[async_trait]
impl SecurePrompt for MockPrompt {
    async fn get_secure_input(&self, _prompt: PromptInfo) -> anyhow::Result<String> {
        Ok(PASSWORD.to_string())
    }

    async fn get_plain_input(&self, _prompt: PromptInfo) -> anyhow::Result<String> {
        Ok(USERNAME.to_string())
    }

    async fn show_notification(&self, _summary: &str, _message: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn get_server_prompts(&self, _params: &TunnelParams) -> anyhow::Result<VecDeque<PromptInfo>> {
        Ok(VecDeque::new())
    }
}

struct MockBrowser;

#[async_trait]
impl BrowserController for MockBrowser {
    fn open(&self, _url: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn close(&self) {}

    async fn acquire_tunnel_password(&self, _url: &str) -> anyhow::Result<String> {
        Ok(String::new())
    }
}

struct ServerFixture {
    socket_name: String,
    server_handle: tokio::task::JoinHandle<anyhow::Result<()>>,
}

impl ServerFixture {
    async fn new() -> Self {
        let socket_name = format!("snxcore-test-{}.sock", Uuid::new_v4());

        let server = CommandServer::with_name(&socket_name, MockTunnelConnectorFactory);
        let server_handle = tokio::spawn(server.run());

        tokio::time::sleep(Duration::from_millis(200)).await;

        Self {
            socket_name,
            server_handle,
        }
    }
}

#[tokio::test]
async fn command_server_reports_disconnected_status() {
    let fixture = ServerFixture::new().await;

    let params = Arc::new(TunnelParams::default());
    let mut controller = ServiceController::new_with_server_name(&fixture.socket_name, MockPrompt, MockBrowser);
    let status = controller.command(ServiceCommand::Status, params).await.unwrap();
    assert_eq!(status, ConnectionStatus::Disconnected);

    fixture.server_handle.abort();
}

#[tokio::test]
async fn connect_with_mfa() {
    let fixture = ServerFixture::new().await;

    let params = Arc::new(TunnelParams {
        server_name: "127.0.0.1".to_owned(),
        login_type: "vpn_Test".to_string(),
        ..Default::default()
    });

    let mut controller = ServiceController::new_with_server_name(&fixture.socket_name, MockPrompt, MockBrowser);
    let status = controller
        .command(ServiceCommand::Connect, params.clone())
        .await
        .unwrap();
    match status {
        ConnectionStatus::Connected(info) => {
            assert_eq!(info.server_name, params.server_name);
            assert_eq!(info.username, USERNAME);
            assert_eq!(info.login_type, params.login_type);
        }
        _ => panic!("invalid status"),
    }

    let status = controller.command(ServiceCommand::Disconnect, params).await.unwrap();
    assert_eq!(status, ConnectionStatus::Disconnected);

    fixture.server_handle.abort();
}
