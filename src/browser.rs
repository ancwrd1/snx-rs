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

    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || Self::run(rx));
        Self { sender: tx }
    }

    pub fn open<S: AsRef<str>>(&self, url: S) -> anyhow::Result<()> {
        Ok(self.sender.send(BrowserCommand::Open(url.as_ref().to_owned()))?)
    }

    pub fn close(&self) -> anyhow::Result<()> {
        Self::close_browser();
        Ok(())
    }
}

#[cfg(feature = "webkit2gtk")]
mod webkit {
    use anyhow::anyhow;
    use directories_next::ProjectDirs;
    use gtk::{glib, prelude::*, Window, WindowPosition, WindowType};
    use std::thread;
    use std::time::Duration;
    use webkit2gtk::{
        CookieManagerExt, CookiePersistentStorage, UserContentManager, WebContext, WebView, WebViewExt,
        WebViewExtManual, WebsiteDataManager, WebsiteDataManagerExt,
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
                glib::ControlFlow::Continue
            });
        });
    }

    pub fn open_browser(url: String) -> anyhow::Result<()> {
        let window = Window::new(WindowType::Toplevel);
        window.resize(700, 500);

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

        webview.load_uri(&format!("{}&notab=1", url));
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
