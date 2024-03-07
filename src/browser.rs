use std::sync::mpsc;
use std::thread::JoinHandle;

enum BrowserCommand {
    Open(String),
    Exit,
}

pub struct BrowserController {
    sender: mpsc::Sender<BrowserCommand>,
    handle: Option<JoinHandle<()>>,
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
        while let Ok(command) = receiver.recv() {
            match command {
                BrowserCommand::Open(url) => Self::open_browser(url),
                BrowserCommand::Exit => break,
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
        let handle = std::thread::spawn(move || Self::run(rx));
        Self {
            sender: tx,
            handle: Some(handle),
        }
    }
}

impl Drop for BrowserController {
    fn drop(&mut self) {
        let _ = self.sender.send(BrowserCommand::Exit);
        let _ = self.handle.take().map(|h| h.join());
    }
}

#[cfg(feature = "webkit2gtk")]
mod webkit {
    use std::thread;
    use std::time::Duration;

    use anyhow::anyhow;
    use directories_next::ProjectDirs;
    use gtk::{glib, prelude::*, Application, ApplicationWindow, Window, WindowPosition, WindowType};
    use tokio::{io::AsyncWriteExt, net::TcpStream};
    use tracing::debug;
    use webkit2gtk::{
        CookieManagerExt, CookiePersistentStorage, SettingsExt, URIRequestExt, UserContentManager, WebContext, WebView,
        WebViewExt, WebViewExtManual, WebsiteDataManager, WebsiteDataManagerExt,
    };

    use crate::util;

    pub fn close_browser() {
        thread::spawn(|| {
            glib::idle_add(|| {
                for win in Window::list_toplevels() {
                    if let Some(w) = win.downcast_ref::<ApplicationWindow>() {
                        w.close();
                    }
                }
                glib::ControlFlow::Break
            });
        });
    }

    fn notify_listener() {
        let _ = util::block_on(async {
            let mut socket =
                tokio::time::timeout(Duration::from_secs(1), TcpStream::connect("127.0.0.1:7779")).await??;

            socket
                .write_all(b"GET / HTTP/1.1\r\nHost:localhost\r\nConnection:Close\r\n\r\n")
                .await?;

            socket.shutdown().await?;

            Ok::<_, anyhow::Error>(())
        });
    }

    pub fn open_browser(url: String) -> anyhow::Result<()> {
        let app = Application::builder().application_id("com.github.snx-rs").build();

        app.connect_activate(move |app| {
            let window = ApplicationWindow::builder()
                .application(app)
                .title("Identity Provider Authentication")
                .type_(WindowType::Toplevel)
                .width_request(700)
                .height_request(500)
                .build();

            let data_manager = WebsiteDataManager::default();
            data_manager.set_persistent_credential_storage_enabled(true);

            let dir = ProjectDirs::from("", "", "snx-rs")
                .ok_or(anyhow!("No project directory!"))
                .unwrap();
            let _ = std::fs::create_dir_all(dir.config_dir());
            let cookies_file = dir.config_dir().join("cookies.db");

            data_manager
                .cookie_manager()
                .unwrap()
                .set_persistent_storage(&format!("{}", cookies_file.display()), CookiePersistentStorage::Sqlite);

            let context = WebContext::builder().website_data_manager(&data_manager).build();
            let webview = WebView::new_with_context_and_user_content_manager(&context, &UserContentManager::new());

            let settings = WebViewExt::settings(&webview).unwrap();
            settings.set_javascript_can_open_windows_automatically(true);

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

            webview.load_uri(&format!("{}", url));

            window.add(&webview);
            window.set_position(WindowPosition::Mouse);
            window.show_all();
        });

        app.run();
        notify_listener();

        Ok(())
    }
}
