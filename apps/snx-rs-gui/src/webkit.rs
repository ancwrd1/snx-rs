use std::{sync::Arc, time::Duration};

use gtk4::{ApplicationWindow, glib, glib::clone, prelude::*};
use i18n::tr;
use snxcore::{
    browser::{BrowserController, SystemBrowser},
    model::params::TunnelParams,
};
use webkit6::{LoadEvent, NetworkSession, TLSErrorsPolicy, WebView, prelude::*};

const COOKIE_TIMEOUT: Duration = Duration::from_secs(120);

const JS_COOKIE_SCRIPT: &str = r#"
(function() {
  try {
    SNXParams.prototype.FetchFromServer();
    const cookie = SNXParams.prototype.getPassword();
    if (cookie != undefined && cookie != "") return cookie;
  } catch (e) {}

  const regex = /Extender\.password\s*=\s*"([^"]+)"/;

  const scripts = document.querySelectorAll("script:not([src])");
  for (const s of scripts) {
    match = s.textContent.match(regex);
    if (match) return match[1];
  }

  return "";
})();
"#;

pub struct WebKitBrowser {
    params: Arc<TunnelParams>,
}

impl WebKitBrowser {
    pub fn new(params: Arc<TunnelParams>) -> Self {
        Self { params }
    }
}

#[async_trait::async_trait]
impl BrowserController for WebKitBrowser {
    fn open(&self, url: &str) -> anyhow::Result<()> {
        SystemBrowser.open(url)
    }

    fn close(&self) {}

    async fn acquire_access_cookie(&self, url: &str) -> anyhow::Result<String> {
        let url = url.to_owned();
        let params = self.params.clone();

        let (tx, mut rx) = tokio::sync::mpsc::channel(1);

        glib::idle_add(move || {
            let window = ApplicationWindow::builder()
                .title("Mobile Access")
                .width_request(700)
                .height_request(500)
                .build();

            let session = NetworkSession::new_ephemeral();
            if params.ignore_server_cert {
                session.set_tls_errors_policy(TLSErrorsPolicy::Ignore);
            }
            let webview = WebView::builder().network_session(&session).build();
            webview.load_uri(&url);
            window.set_child(Some(&webview));

            window.present();

            let tx = tx.clone();
            webview.connect_load_changed(clone!(
                #[weak]
                window,
                move |webview, event| {
                    if event == LoadEvent::Finished {
                        let tx = tx.clone();
                        webview.evaluate_javascript(
                            JS_COOKIE_SCRIPT,
                            None,
                            None,
                            gtk4::gio::Cancellable::NONE,
                            move |result| {
                                if let Ok(value) = result
                                    && value.is_string()
                                {
                                    let cookie = value.to_str();
                                    if !cookie.is_empty() {
                                        let tx = tx.clone();
                                        tokio::spawn(async move { tx.send(cookie.to_string()).await });
                                        window.close();
                                    }
                                }
                            },
                        );
                    }
                }
            ));

            glib::ControlFlow::Break
        });

        match tokio::time::timeout(COOKIE_TIMEOUT, rx.recv()).await {
            Ok(Some(cookie)) => Ok(cookie),
            _ => anyhow::bail!(tr!("error-cannot-acquire-access-cookie")),
        }
    }
}
