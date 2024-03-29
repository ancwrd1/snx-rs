use gtk::{
    prelude::{ApplicationExt, ApplicationExtManual},
    Application,
};
use tracing::level_filters::LevelFilter;

use snx_rs::{browser::BrowserController, controller::ServiceController, prompt::SecurePrompt};

fn main() -> anyhow::Result<()> {
    let _ = snx_rs::util::block_on(snx_rs::platform::init_theme_monitoring());

    let app = Application::builder().application_id("com.github.snx-rs").build();

    app.connect_activate(move |_| {
        let browser_controller = BrowserController::webkit();
        let service_controller = ServiceController::new(SecurePrompt::tty(), &browser_controller).unwrap();

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
            let _ = snx_rs::gui::tray_icon::show_tray_icon(&browser_controller);
        });

        gtk::main();
    });

    app.run();

    Ok(())
}
