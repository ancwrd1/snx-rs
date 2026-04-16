use std::rc::Rc;

use i18n::tr;
use slint::ComponentHandle;
use snxcore::browser::{BrowserController, SystemBrowser};

use crate::ui::{AboutWindow, WindowController, WindowScope, close_window};

const WEBSITE_URL: &str = "https://github.com/ancwrd1/snx-rs";

pub struct AboutWindowController {
    scope: Rc<WindowScope<AboutWindow>>,
}

impl AboutWindowController {
    pub const NAME: &str = "about";

    pub fn new() -> anyhow::Result<Rc<Self>> {
        Ok(Rc::new(Self {
            scope: WindowScope::new(AboutWindow::new()?),
        }))
    }
}

impl WindowController for AboutWindowController {
    fn present(&self) -> anyhow::Result<()> {
        self.scope.set_globals();
        self.scope.window.set_app_name(tr!("app-title").into());
        self.scope.window.set_version(env!("CARGO_PKG_VERSION").into());
        self.scope.window.set_authors(env!("CARGO_PKG_AUTHORS").into());
        self.scope.window.set_website(WEBSITE_URL.into());
        self.scope.window.set_license(env!("CARGO_PKG_LICENSE").into());

        self.scope.window.on_website_clicked(|| {
            let _ = SystemBrowser.open(WEBSITE_URL);
        });

        self.scope.window.on_ok_clicked(|| close_window(Self::NAME));

        self.scope.window.window().on_close_requested(move || {
            close_window(Self::NAME);
            slint::CloseRequestResponse::HideWindow
        });

        self.scope.window.show()?;

        Ok(())
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&self) {
        self.scope.set_globals();
    }
}
