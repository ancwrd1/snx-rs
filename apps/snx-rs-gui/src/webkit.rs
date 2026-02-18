use std::{
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
    time::Duration,
};

use gtk4::{ApplicationWindow, glib, glib::clone, prelude::*};
use i18n::tr;
use snxcore::{
    browser::{BrowserController, SystemBrowser},
    model::params::TunnelParams,
};
use webkit6::{LoadEvent, NetworkSession, TLSErrorsPolicy, WebView, prelude::*};

const PASSWORD_TIMEOUT: Duration = Duration::from_secs(120);

const JS_PASSWORD_SCRIPT: &str = r#"
(function() {
  const regexes = [
    /sPropertyName = "password";\n\s*SNXParams\.addProperty\(sPropertyName, Function\.READ_WRITE, "([^"]*)"\);/,
    /Extender\.password\s*=\s*"([^"]*)"/,
  ];

  const scripts = document.querySelectorAll("script:not([src])");
  for (const s of scripts) {
    for (const regex of regexes) {
      const match = s.textContent.match(regex);
      if (match) return match[1];
    }
  }

  return null;
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

    async fn acquire_tunnel_password(&self, url: &str) -> anyhow::Result<String> {
        let url = url.to_owned();
        let params = self.params.clone();

        let (tx, mut rx) = tokio::sync::mpsc::channel(1);

        glib::idle_add(move || {
            let window = ApplicationWindow::builder()
                .title(tr!("label-mobile-access"))
                .width_request(720)
                .height_request(500)
                .build();

            let session = NetworkSession::new_ephemeral();
            if params.ignore_server_cert {
                session.set_tls_errors_policy(TLSErrorsPolicy::Ignore);
            }
            let webview = WebView::builder().network_session(&session).build();
            if let Some(settings) = WebViewExt::settings(&webview) {
                settings.set_enable_developer_extras(true);
                settings.set_user_agent(Some("Mozilla/5.0 (X11; Linux x86_64; rv:147.0) Gecko/20100101 Firefox/147.0"));
            }

            let tx = tx.clone();
            let reload_counter = Arc::new(AtomicU32::new(0));

            webview.connect_load_changed(clone!(
                #[weak]
                window,
                move |webview, event| {
                    if event == LoadEvent::Finished {
                        let tx = tx.clone();
                        let reload_counter = reload_counter.clone();
                        webview.evaluate_javascript(
                            JS_PASSWORD_SCRIPT,
                            None,
                            None,
                            gtk4::gio::Cancellable::NONE,
                            clone!(
                                #[weak]
                                webview,
                                move |result| {
                                    if let Ok(value) = result
                                        && value.is_string()
                                    {
                                        let password = value.to_str();
                                        if !password.is_empty() {
                                            let tx = tx.clone();
                                            tokio::spawn(async move { tx.send(password.to_string()).await });
                                            window.close();
                                        } else if reload_counter.fetch_add(1, Ordering::SeqCst) < 3 {
                                            webview.reload();
                                        } else {
                                            window.close();
                                        }
                                    }
                                },
                            ),
                        );
                    }
                }
            ));

            window.set_child(Some(&webview));
            window.present();
            webview.load_uri(&url);

            glib::ControlFlow::Break
        });

        match tokio::time::timeout(PASSWORD_TIMEOUT, rx.recv()).await {
            Ok(Some(password)) => Ok(password),
            _ => anyhow::bail!(tr!("error-cannot-acquire-access-cookie")),
        }
    }
}
