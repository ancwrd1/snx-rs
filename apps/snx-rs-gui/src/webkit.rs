use std::{cell::RefCell, process::Stdio, rc::Rc, sync::Arc, time::Duration};

use gtk4::{Application, ApplicationWindow, glib, glib::clone, prelude::*};
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
    /sPropertyName = "password";\n\s*SNXParams\.addProperty\(sPropertyName, Function\.READ_WRITE, "([^"]+)"\);/,
    /Extender\.password\s*=\s*"([^"]+)"/,
  ];

  const scripts = document.querySelectorAll("script:not([src])");
  for (const s of scripts) {
    for (const regex of regexes) {
      const match = s.textContent.match(regex);
      if (match) return match[1];
    }
  }

  return "";
})();
"#;

pub fn webkit_main(url: &str, ignore_cert: bool) -> i32 {
    let app = Application::builder()
        .application_id("com.github.snx-rs.webkit")
        .build();

    let password: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    let url = url.to_string();
    app.connect_activate(clone!(
        #[strong]
        password,
        move |app| {
            let window = ApplicationWindow::builder()
                .application(app)
                .title(tr!("label-mobile-access"))
                .width_request(720)
                .height_request(500)
                .build();

            let session = NetworkSession::new_ephemeral();
            if ignore_cert {
                session.set_tls_errors_policy(TLSErrorsPolicy::Ignore);
            }
            let webview = WebView::builder().network_session(&session).build();
            if let Some(settings) = WebViewExt::settings(&webview) {
                settings.set_enable_developer_extras(true);
            }

            webview.connect_load_changed(clone!(
                #[weak]
                window,
                #[strong]
                password,
                #[weak]
                app,
                move |webview, event| {
                    if event == LoadEvent::Finished {
                        webview.evaluate_javascript(
                            JS_PASSWORD_SCRIPT,
                            None,
                            None,
                            gtk4::gio::Cancellable::NONE,
                            clone!(
                                #[strong]
                                password,
                                move |result| {
                                    if let Ok(value) = result
                                        && value.is_string()
                                    {
                                        let found = value.to_str();
                                        if !found.is_empty() {
                                            *password.borrow_mut() = Some(found.to_string());
                                            window.close();
                                            app.quit();
                                        }
                                    }
                                }
                            ),
                        );
                    }
                }
            ));

            window.set_child(Some(&webview));
            window.present();
            webview.load_uri(&url);
        }
    ));

    let empty: [&str; 0] = [];
    app.run_with_args(&empty);

    match password.borrow().as_ref() {
        Some(p) => {
            println!("{p}");
            0
        }
        None => 1,
    }
}

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
        SystemBrowser::default().open(url)
    }

    fn close(&self) {}

    async fn acquire_tunnel_password(&self, url: &str) -> anyhow::Result<String> {
        let exe = std::env::current_exe()?;

        let mut cmd = tokio::process::Command::new(exe);
        cmd.arg("--webkit").arg(url);
        if self.params.ignore_server_cert {
            cmd.arg("--webkit-ignore-cert");
        }
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);

        let output = tokio::time::timeout(PASSWORD_TIMEOUT, cmd.output()).await;

        if let Ok(Ok(output)) = output
            && output.status.success()
        {
            let password = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !password.is_empty() {
                return Ok(password);
            }
        }

        anyhow::bail!(tr!("error-cannot-acquire-access-cookie"))
    }
}
