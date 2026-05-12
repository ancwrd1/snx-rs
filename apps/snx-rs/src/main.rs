use std::{future::Future, path::Path, sync::Arc};

use clap::{CommandFactory, Parser};
use futures::pin_mut;
use i18n::tr;
use nix::unistd::Uid;
use secrecy::ExposeSecret;
use snxcore::{
    model::{
        MfaType, PromptInfo, SessionState,
        params::{OperationMode, TunnelParams, TunnelType},
        proto::CertificateResponse,
    },
    otp::OtpListener,
    platform::{NetworkInterface, Platform, PlatformAccess, SingleInstance},
    prompt::{SecurePrompt, TtyPrompt},
    server::CommandServer,
    tunnel::{TunnelConnectorFactory, TunnelEvent, connector::CheckPointConnectorFactory},
    util,
};
use tokio::{signal::unix, sync::mpsc};
use tracing::{debug, metadata::LevelFilter, warn};

use crate::cmdline::CmdlineParams;

mod cmdline;

async fn await_termination<F, R>(f: F) -> anyhow::Result<()>
where
    F: Future<Output = anyhow::Result<R>>,
{
    let ctrl_c = tokio::signal::ctrl_c();
    pin_mut!(ctrl_c);

    let mut sig = unix::signal(unix::SignalKind::terminate())?;
    let term = sig.recv();
    pin_mut!(term);

    let select = futures::future::select(ctrl_c, term);

    tokio::select! {
        result = f => {
            result?;
            Ok(())
        }

        _ = select => {
            debug!("Application terminated due to a signal");
            Ok(())
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cmdline_params = CmdlineParams::parse();

    // Handle completions immediately and exit
    if let Some(shell) = cmdline_params.completions {
        clap_complete::generate(shell, &mut CmdlineParams::command(), "snx-rs", &mut std::io::stdout());
        return Ok(());
    }

    if cmdline_params.mode.requires_root() && !Uid::effective().is_root() {
        anyhow::bail!(tr!("error-no-root-privileges"));
    }

    Platform::get().init();

    let mode = cmdline_params.mode;

    let mut params = if let Some(ref config_file) = cmdline_params.config_file {
        TunnelParams::load(config_file)?
    } else {
        TunnelParams::default()
    };
    cmdline_params.merge_into_tunnel_params(&mut params);

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(params.log_level.parse::<LevelFilter>().unwrap_or(LevelFilter::OFF))
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    debug!(">>> Starting snx-rs client version {}", env!("CARGO_PKG_VERSION"));

    let factory = CheckPointConnectorFactory::default();

    match mode {
        OperationMode::Standalone => {
            debug!("Running in standalone mode");
            main_standalone(factory, params).await
        }
        OperationMode::Command => {
            debug!("Running in command mode");
            main_command(factory).await
        }
        OperationMode::Info => main_info(factory, params).await,
        OperationMode::Enroll => main_enroll(factory, params).await,
        OperationMode::Renew => main_renew(factory, params).await,
    }
}

async fn main_info<F>(factory: F, params: TunnelParams) -> anyhow::Result<()>
where
    F: TunnelConnectorFactory + Send + Sync + 'static,
{
    if params.server_name.is_empty() {
        anyhow::bail!(tr!("error-missing-server-name"));
    }
    let params = Arc::new(params);
    let connector = factory.new_gateway_connector(params.clone());
    let info = connector.get_gateway_information().await?;
    info.print_login_options(&params.server_name);

    Ok(())
}

async fn main_enroll<F>(factory: F, params: TunnelParams) -> anyhow::Result<()>
where
    F: TunnelConnectorFactory + Send + Sync + 'static,
{
    let params = Arc::new(params);

    if params.server_name.is_empty() {
        anyhow::bail!(tr!("error-missing-server-name"));
    }

    let Some(ref cert_path) = params.cert_path else {
        anyhow::bail!(tr!("error-missing-cert-path"));
    };

    let Some(ref cert_password) = params.cert_password else {
        anyhow::bail!(tr!("error-missing-cert-password"));
    };

    let Some(ref reg_key) = params.reg_key else {
        anyhow::bail!(tr!("error-missing-reg-key"));
    };

    let connector = factory.new_gateway_connector(params.clone());

    let resp = connector
        .enroll_certificate(reg_key, cert_password.expose_secret())
        .await?;

    process_cert_response(cert_path, resp)
}

async fn main_renew<F>(factory: F, params: TunnelParams) -> anyhow::Result<()>
where
    F: TunnelConnectorFactory + Send + Sync + 'static,
{
    let params = Arc::new(params);

    if params.server_name.is_empty() {
        anyhow::bail!(tr!("error-missing-server-name"));
    }

    let Some(ref cert_path) = params.cert_path else {
        anyhow::bail!(tr!("error-missing-cert-path"));
    };

    let Some(ref cert_password) = params.cert_password else {
        anyhow::bail!(tr!("error-missing-cert-password"));
    };

    let pkcs12 = std::fs::read(cert_path)?;

    let connector = factory.new_gateway_connector(params.clone());

    let resp = connector
        .renew_certificate(&pkcs12, cert_password.expose_secret())
        .await?;

    process_cert_response(cert_path, resp)
}

fn process_cert_response(path: &Path, resp: CertificateResponse) -> anyhow::Result<()> {
    if resp.error_code == 0
        && let Some(binary) = resp.binary
    {
        std::fs::write(path, util::snx_deobfuscate(binary)?)?;
        println!("{}", tr!("cli-certificate-enrolled"));
        Ok(())
    } else {
        anyhow::bail!(tr!("error-certificate-enrollment-failed", code = resp.error_code));
    }
}

async fn main_command<F>(factory: F) -> anyhow::Result<()>
where
    F: TunnelConnectorFactory + Send + Sync + 'static,
{
    let instance = Platform::get().new_single_instance("/var/run/snx-rs.lock")?;
    if !instance.is_single() {
        eprintln!("{}", tr!("cli-another-instance-running"));
        return Ok(());
    }

    if let Err(e) = Platform::get()
        .new_network_interface()
        .start_network_state_monitoring()
        .await
    {
        warn!("Unable to start network monitoring: {}", e);
    }
    let server = CommandServer::new(factory);

    await_termination(server.run()).await
}

async fn main_standalone<F>(factory: F, params: TunnelParams) -> anyhow::Result<()>
where
    F: TunnelConnectorFactory + Send + Sync + 'static,
{
    let (command_sender, command_receiver) = mpsc::channel(16);

    let tty_prompt = TtyPrompt;

    if params.server_name.is_empty() || params.login_type.is_empty() {
        anyhow::bail!(tr!("error-missing-required-parameters"));
    }

    let params = Arc::new(params);

    let connector = factory.new_gateway_connector(params.clone());
    let info = connector.get_gateway_information().await?;

    let mfa_prompts = info.get_login_prompts(&params.login_type);

    let mut tunnel_connector = factory.new_tunnel_connector(params.clone()).await?;

    let mut session = if params.ike_persist {
        debug!("Attempting to load IKE session");
        match tunnel_connector.restore_session().await {
            Ok(session) => session,
            Err(_) => {
                tunnel_connector = factory.new_tunnel_connector(params.clone()).await?;
                tunnel_connector.authenticate().await?
            }
        }
    } else {
        tunnel_connector.authenticate().await?
    };

    let mut mfa_index = 0;

    while let SessionState::PendingChallenge(challenge) = session.state.clone() {
        match challenge.mfa_type {
            MfaType::PasswordInput => {
                let prompt_info = mfa_prompts
                    .get(mfa_index)
                    .cloned()
                    .unwrap_or_else(|| PromptInfo::new("", &challenge.prompt));

                mfa_index += 1;

                let input = if !params.password.expose_secret().is_empty() && mfa_index == params.password_factor {
                    Ok(params.password.expose_secret().to_owned())
                } else if let Some(ref mfa_code) = params.mfa_code
                    && mfa_index != params.password_factor
                {
                    Ok(mfa_code.clone())
                } else {
                    tty_prompt.get_secure_input(prompt_info).await
                };

                match input {
                    Ok(input) => {
                        session = tunnel_connector.challenge_code(session, &input).await?;
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
            MfaType::IdentityProvider => {
                println!("{}", tr!("cli-identity-provider-auth"));
                println!("{}", challenge.prompt);
                let otp = OtpListener::new().await?.acquire_otp().await?;
                session = tunnel_connector.challenge_code(session, &otp).await?;
            }
            MfaType::MobileAccess => {
                let prompt_info = PromptInfo::new(
                    tr!("cli-mobile-access-auth", url = &challenge.prompt),
                    tr!("label-password"),
                );
                let input = tty_prompt.get_secure_input(prompt_info).await?;
                session = tunnel_connector.challenge_code(session, &input).await?;
            }
            MfaType::UserNameInput => {
                let prompt_info = PromptInfo::new(tr!("label-username-required"), &challenge.prompt);
                let input = tty_prompt.get_plain_input(prompt_info).await?;
                session = tunnel_connector.challenge_code(session, &input).await?;
            }
        }
    }

    let mut tunnel = tunnel_connector.create_tunnel(session.clone(), command_sender).await?;

    if let Err(e) = Platform::get()
        .new_network_interface()
        .start_network_state_monitoring()
        .await
    {
        warn!("Unable to start network monitoring: {}", e);
    }

    let (event_sender, mut event_receiver) = mpsc::channel(16);
    let tunnel_fut = await_termination(tunnel.run(command_receiver, event_sender));

    pin_mut!(tunnel_fut);

    loop {
        tokio::select! {
            event = event_receiver.recv() => {
                if let Some(event) = event {
                    let _ = tunnel_connector.handle_tunnel_event(event.clone()).await;

                    if let TunnelEvent::Connected(info) = event {
                        println!("{}", info.print(false));
                        println!("{}", tr!("cli-tunnel-connected"));
                    }
                }
            }
            result = &mut tunnel_fut => {
                if params.tunnel_type == TunnelType::SSL || !params.ike_persist {
                    debug!("Signing out");
                    let _ = connector.signout(&session.session_id).await;
                }
                println!("\n{}", tr!("cli-tunnel-disconnected"));
                break result;
            }
        }
    }
}
