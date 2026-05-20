use std::{cell::RefCell, rc::Rc};

use gtk4::{Application, ApplicationWindow, glib, glib::clone, prelude::*};
use i18n::tr;
use webkit6::{LoadEvent, NetworkSession, TLSErrorsPolicy, WebView, prelude::*};

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
                            crate::webkit::JS_PASSWORD_SCRIPT,
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
