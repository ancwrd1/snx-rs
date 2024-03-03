use anyhow::anyhow;
use tracing::level_filters::LevelFilter;

use snx_rs::browser::BrowserController;
use snx_rs::{
    controller::{ServiceCommand, ServiceController},
    prompt::SecurePrompt,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();

    let browser_controller = BrowserController::new();

    let mut controller = ServiceController::new(SecurePrompt::tty(), &browser_controller)?;

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(
            controller
                .params
                .log_level
                .parse::<LevelFilter>()
                .unwrap_or(LevelFilter::OFF),
        )
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    if args.len() == 1 {
        #[cfg(feature = "tray-icon")]
        return snx_rs::tray_icon::show_tray_icon(&browser_controller);

        #[cfg(not(feature = "tray-icon"))]
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

    match controller.command(command).await {
        Ok(status) => {
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
    }

    Ok(())
}
