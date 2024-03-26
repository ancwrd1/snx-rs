use std::{thread, time::Duration};

use gtk::{glib, prelude::*, ApplicationWindow, Window, WindowPosition, WindowType};
use tokio::{io::AsyncWriteExt, net::TcpStream};
use tracing::debug;
use webkit2gtk::{
    CookieManagerExt, CookiePersistentStorage, SettingsExt, URIRequestExt, UserContentManager, WebContext, WebView,
    WebViewExt, WebViewExtManual, WebsiteDataManager, WebsiteDataManagerExt,
};

use crate::{model::params::TunnelParams, util};

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
        let mut socket = tokio::time::timeout(Duration::from_secs(1), TcpStream::connect("127.0.0.1:7779")).await??;

        socket
            .write_all(b"GET / HTTP/1.1\r\nHost:localhost\r\nConnection:Close\r\n\r\n")
            .await?;

        socket.shutdown().await?;

        Ok::<_, anyhow::Error>(())
    });
}

pub fn open_browser(url: String) -> anyhow::Result<()> {
    glib::idle_add(move || {
        let window = ApplicationWindow::builder()
            .title("Identity Provider Authentication")
            .type_(WindowType::Toplevel)
            .width_request(700)
            .height_request(500)
            .build();

        let data_manager = WebsiteDataManager::default();
        data_manager.set_persistent_credential_storage_enabled(true);

        let dir = TunnelParams::default_config_path()
            .unwrap()
            .parent()
            .unwrap()
            .to_owned();

        let _ = std::fs::create_dir_all(&dir);
        let cookies_file = dir.join("cookies.db");

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

        webview.load_uri(&url);

        window.connect_destroy(|_| notify_listener());
        window.add(&webview);
        window.set_position(WindowPosition::Mouse);
        window.show_all();

        glib::ControlFlow::Break
    });

    Ok(())
}
