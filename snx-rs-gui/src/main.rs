use gtk::{
    prelude::{ApplicationExt, ApplicationExtManual},
    Application,
};
use tracing::level_filters::LevelFilter;

use snxcore::controller::ServiceController;

pub mod assets;
pub mod prompt;
pub mod settings;
pub mod tray_icon;
pub mod webkit;

fn main() -> anyhow::Result<()> {
    let _ = snxcore::util::block_on(snxcore::platform::init_theme_monitoring());

    let app = Application::builder().application_id("com.github.snx-rs").build();

    app.connect_activate(move |_| {
        let service_controller = ServiceController::new(prompt::GtkPrompt, webkit::WebkitBrowser).unwrap();

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

        std::thread::spawn(move || {
            let _ = tray_icon::show_tray_icon();
        });

        gtk::main();
    });

    app.run();

    Ok(())
}
