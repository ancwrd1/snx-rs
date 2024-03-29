use anyhow::anyhow;
use tracing::level_filters::LevelFilter;

use snxcore::{
    browser::BrowserController,
    controller::{ServiceCommand, ServiceController},
    prompt::TtyPrompt,
};

struct SystemBrowser;

impl BrowserController for SystemBrowser {
    fn open(&self, url: &str) -> anyhow::Result<()> {
        Ok(opener::open(url)?)
    }

    fn close(&self) {}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();

    let mut service_controller = ServiceController::new(TtyPrompt, SystemBrowser)?;

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(
            service_controller
                .params
                .log_level
                .parse::<LevelFilter>()
                .unwrap_or(LevelFilter::OFF),
        )
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    if args.len() == 1 {
        return Err(anyhow!(
            "usage: {} {{status|connect|disconnect|reconnect|info}}",
            args[0]
        ));
    }

    let command: ServiceCommand = args
        .get(1)
        .map(|v| v.as_str())
        .ok_or_else(|| anyhow!("No command"))?
        .parse()?;

    match service_controller.command(command).await {
        Ok(status) if command != ServiceCommand::Info => {
            if let Some(since) = status.connected_since {
                println!(
                    "{} since: {}",
                    if status.mfa.is_some() {
                        "MFA pending"
                    } else {
                        "Connected"
                    },
                    since
                );
            } else {
                println!("Disconnected");
            }
        }
        Err(e) => println!("Error: {}", e),
        _ => {}
    }

    Ok(())
}
