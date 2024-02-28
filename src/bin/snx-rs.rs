use std::{future::Future, pin::Pin, sync::Arc, sync::Mutex};
use std::collections::VecDeque;

use anyhow::anyhow;
use base64::Engine;
use clap::Parser;
use futures::pin_mut;
use serde_json::Value;
use tokio::{signal::unix, sync::oneshot};
use tracing::{debug, metadata::LevelFilter, warn};

use snx_rs::{
    ccc::CccHttpClient,
    model::{
        params::{CmdlineParams, OperationMode, TunnelParams},
        ConnectionStatus, SessionState,
    },
    prompt::SecurePrompt,
    server::CommandServer,
    tunnel::TunnelConnector,
};
use snx_rs::model::proto::{LoginDisplayLabelSelect, ServerInfoResponse};

fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cmdline_params = CmdlineParams::parse();

    if cmdline_params.mode != OperationMode::Info && !is_root() {
        return Err(anyhow!("Please run me as a root user!"));
    }

    let mode = cmdline_params.mode;

    let mut params = if let Some(ref config_file) = cmdline_params.config_file {
        TunnelParams::load(config_file)?
    } else {
        TunnelParams::default()
    };
    params.merge(cmdline_params);

    if !params.password.is_empty() {
        // decode password
        params.password =
            String::from_utf8_lossy(&base64::engine::general_purpose::STANDARD.decode(&params.password)?).into_owned();
    }

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(params.log_level.parse::<LevelFilter>().unwrap_or(LevelFilter::OFF))
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    debug!(">>> Starting snx-rs client version {}", env!("CARGO_PKG_VERSION"));

    let (_tx, rx) = oneshot::channel();

    let fut: Pin<Box<dyn Future<Output = anyhow::Result<()>>>> = match mode {
        OperationMode::Standalone => {
            debug!("Running in standalone mode");

            if params.server_name.is_empty() || params.login_type.is_empty() {
                return Err(anyhow!("Missing required parameters: server name and/or login type"));
            }
            
            let mut pwd_prompts = get_server_pwd_prompts(&params).await.unwrap_or_default();

            if params.password.is_empty() && params.client_cert.is_none() {
                let prompt = pwd_prompts.pop_front().unwrap_or(format!("Enter password for {}: ", params.user_name));
                params.password = SecurePrompt::tty()
                    .get_secure_input(&prompt)?
                    .trim()
                    .to_owned();
            }

            let connector = TunnelConnector::new(Arc::new(params));
            let mut session = Arc::new(connector.authenticate().await?);

            while let SessionState::Pending { prompt } = session.state.clone() {
                let prompt =  pwd_prompts.pop_front().unwrap_or(prompt.unwrap_or("Multi-factor code: ".to_string()));
                match SecurePrompt::tty().get_secure_input(&prompt) {
                    Ok(input) => {
                        session = Arc::new(connector.challenge_code(session, &input).await?);
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }

            let status = Arc::new(Mutex::new(ConnectionStatus::default()));
            let tunnel = connector.create_tunnel(session).await?;

            if let Err(e) = snx_rs::platform::start_network_state_monitoring().await {
                warn!("Unable to start network monitoring: {}", e);
            }

            let (status_sender, _) = oneshot::channel();
            let result = Box::pin(tunnel.run(rx, status, status_sender));
            result
        }
        OperationMode::Command => {
            debug!("Running in command mode");

            if let Err(e) = snx_rs::platform::start_network_state_monitoring().await {
                warn!("Unable to start network monitoring: {}", e);
            }
            let server = CommandServer::new(snx_rs::server::LISTEN_PORT);

            Box::pin(server.run())
        }
        OperationMode::Info => {
            if params.server_name.is_empty() {
                return Err(anyhow!("Missing required parameters: server name!"));
            }
            let client = CccHttpClient::new(Arc::new(params), None);
            let info = client.get_server_info().await?;
            println!("{}", serde_json::to_string_pretty(&info)?);
            Box::pin(futures::future::ok(()))
        }
    };

    let ctrl_c = tokio::signal::ctrl_c();
    pin_mut!(ctrl_c);

    let mut sig = unix::signal(unix::SignalKind::terminate())?;
    let term = sig.recv();
    pin_mut!(term);

    let select = futures::future::select(ctrl_c, term);

    let result = tokio::select! {
        result = fut => {
            result
        }

        _ = select => {
            debug!("Application terminated due to a signal");
            Ok(())
        }
    };

    debug!("<<< Stopping snx-rs client");

    result
}

async fn get_server_info(params: &TunnelParams) -> anyhow::Result<ServerInfoResponse> {
    let client = CccHttpClient::new(Arc::new(params.clone()), None);
    let info = client.get_server_info().await?;
    let response_data = info.get("ResponseData").unwrap_or(&Value::Null);
    Ok(serde_json::from_value::<ServerInfoResponse>(response_data.clone())?)
}

async fn get_server_pwd_prompts(params: &TunnelParams) -> anyhow::Result<VecDeque<String>> {
    let mut pwd_prompts = VecDeque::new();
    if !params.server_prompt {
        return Ok(pwd_prompts);
    }
    let server_info = get_server_info(params).await?;
    let login_type = &params.login_type;
    let login_factors = server_info
        .login_options_data
        .login_options_list
        .iter()
        .find(|login_option| login_option.id == *login_type)
        .map(|login_option| login_option.to_owned())
        .unwrap()
        .factors;
    login_factors
        .iter()
        .filter_map(|factor| match &factor.custom_display_labels {
            LoginDisplayLabelSelect::LoginDisplayLabel(label) => Some(&label.password),
            LoginDisplayLabelSelect::Empty(_) => None,
        })
        .for_each(|prompt| { pwd_prompts.push_back(format!("{}: ", prompt.0.clone())) });
    Ok(pwd_prompts)
}
