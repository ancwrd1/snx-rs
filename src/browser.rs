use std::sync::mpsc;

enum BrowserCommand {
    Open(String),
}

pub struct BrowserController {
    sender: mpsc::Sender<BrowserCommand>,
}

impl BrowserController {
    fn open_browser(url: String) {
        #[cfg(feature = "webkit2gtk")]
        let _ = webkit::open_browser(url);

        #[cfg(not(feature = "webkit2gtk"))]
        let _ = opener::open_browser(url);
    }

    fn close_browser() {
        #[cfg(feature = "webkit2gtk")]
        webkit::close_browser()
    }

    fn run(receiver: mpsc::Receiver<BrowserCommand>) {
        #[cfg(feature = "webkit2gtk")]
        let _ = gtk::init();

        while let Ok(command) = receiver.recv() {
            match command {
                BrowserCommand::Open(url) => Self::open_browser(url),
            }
        }
    }

    pub fn open<S: AsRef<str>>(&self, url: S) -> anyhow::Result<()> {
        Ok(self.sender.send(BrowserCommand::Open(url.as_ref().to_owned()))?)
    }

    pub fn close(&self) -> anyhow::Result<()> {
        Self::close_browser();
        Ok(())
    }
}

impl Default for BrowserController {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || Self::run(rx));
        Self { sender: tx }
    }
}

#[cfg(feature = "webkit2gtk")]
mod webkit {
    use anyhow::anyhow;
    use directories_next::ProjectDirs;
    use gtk::{glib, prelude::*, Window, WindowPosition, WindowType};
    use std::thread;
    use std::time::Duration;
    use tracing::debug;
    use webkit2gtk::{
        CookieManagerExt, CookiePersistentStorage, SettingsExt, URIRequestExt, UserContentManager, WebContext, WebView,
        WebViewExt, WebViewExtManual, WebsiteDataManager, WebsiteDataManagerExt,
    };

    pub fn close_browser() {
        thread::spawn(|| {
            thread::sleep(Duration::from_secs(3));
            glib::idle_add(|| {
                for win in Window::list_toplevels() {
                    if let Some(w) = win.downcast_ref::<Window>() {
                        w.close();
                    }
                }
                glib::ControlFlow::Break
            });
        });
    }

    pub fn open_browser(url: String) -> anyhow::Result<()> {
        let window = Window::builder()
            .title("Identity provider login")
            .type_(WindowType::Toplevel)
            .width_request(700)
            .height_request(500)
            .build();

        let data_manager = WebsiteDataManager::default();
        data_manager.set_persistent_credential_storage_enabled(true);

        let dir = ProjectDirs::from("", "", "snx-rs").ok_or(anyhow!("No project directory!"))?;
        let cookies_file = dir.config_dir().join("cookies.db");

        data_manager
            .cookie_manager()
            .unwrap()
            .set_persistent_storage(&format!("{}", cookies_file.display()), CookiePersistentStorage::Sqlite);

        let context = WebContext::builder().website_data_manager(&data_manager).build();
        let webview = WebView::new_with_context_and_user_content_manager(&context, &UserContentManager::new());

        let settings = WebViewExt::settings(&webview).unwrap();
        settings.set_javascript_can_open_windows_automatically(true);

        // redirect inside the same window
        webview.connect_create(|w, event| {
            if let Some(req) = event.request() {
                match (req.uri(), w.uri()) {
                    (Some(new_uri), Some(current_uri)) if new_uri != current_uri => {
                        debug!("Redirecting to {}", new_uri);
                        w.load_uri(&new_uri);
                    }
                    _ => {}
                }
            }
            None
        });

        debug!("Opening {}", url);
        webview.load_uri(&format!("{}", url));

        window.add(&webview);
        window.set_position(WindowPosition::Mouse);
        window.show_all();

        window.connect_delete_event(|_, _| {
            gtk::main_quit();
            glib::Propagation::Proceed
        });

        gtk::main();

        Ok(())
    }
}
