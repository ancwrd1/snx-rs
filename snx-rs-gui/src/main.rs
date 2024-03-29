use clap::Parser;
use gtk::{
    prelude::{ApplicationExt, ApplicationExtManual},
    Application,
};
use tracing::level_filters::LevelFilter;

use snxcore::{controller::ServiceController, platform::SingleInstance};

pub mod assets;
pub mod params;
pub mod prompt;
pub mod settings;
pub mod tray_icon;
pub mod webkit;

fn main() -> anyhow::Result<()> {
    let instance = SingleInstance::new("/tmp/snx-rs-gui.s")?;
    if !instance.is_single() {
        return Ok(());
    }

    let params = params::CmdlineParams::parse();

    let _ = snxcore::platform::init_theme_monitoring();

    let app = Application::builder().application_id("com.github.snx-rs").build();

    app.connect_activate(move |_| {
        let service_controller =
            ServiceController::new(prompt::GtkPrompt, webkit::WebkitBrowser, params.config_file()).unwrap();

        let subscriber = tracing_subscriber::fmt()
            .with_max_level(
                service_controller
                    .params
                    .log_level
                    .parse::<LevelFilter>()
                    .unwrap_or(LevelFilter::OFF),
            )
            .finish();
        tracing::subscriber::set_global_default(subscriber).unwrap();

        let params = params.clone();

        std::thread::spawn(move || {
            let _ = tray_icon::show_tray_icon(params);
        });

        gtk::main();
    });

    app.run_with_args::<&str>(&[]);

    Ok(())
}
