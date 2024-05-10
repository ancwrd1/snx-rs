use std::{path::Path, rc::Rc, sync::Arc, time::Duration};

use anyhow::anyhow;
use gtk::{
    glib::{self, clone},
    prelude::*,
    Align, ButtonsType, DialogFlags, MessageType, Orientation, ResponseType, WindowPosition,
};
use ipnet::Ipv4Net;
use tracing::warn;

use snxcore::{
    model::params::{TunnelParams, TunnelType},
    server_info,
};

const CSS_ERROR: &str = r#"label {
    padding: 6px;
    border: 1px solid #f44336;
    color: #ffffff;
    background-color: #a02a2a;
}
"#;

struct SettingsDialog {
    params: Arc<TunnelParams>,
    dialog: gtk::Dialog,
    widgets: Rc<MyWidgets>,
}

struct MyWidgets {
    server_name: gtk::Entry,
    fetch_info: gtk::Button,
    auth_type: gtk::ComboBoxText,
    tunnel_type: gtk::ComboBoxText,
    user_name: gtk::Entry,
    password: gtk::Entry,
    no_dns: gtk::CheckButton,
    search_domains: gtk::Entry,
    ignored_domains: gtk::Entry,
    no_routing: gtk::CheckButton,
    default_routing: gtk::CheckButton,
    add_routes: gtk::Entry,
    ignored_routes: gtk::Entry,
    mfa_prompts: gtk::CheckButton,
    no_keychain: gtk::CheckButton,
    no_cert_name_check: gtk::CheckButton,
    no_cert_check: gtk::CheckButton,
    cert_type: gtk::ComboBoxText,
    cert_path: gtk::Entry,
    cert_password: gtk::Entry,
    cert_id: gtk::Entry,
    ca_cert: gtk::Entry,
    ike_lifetime: gtk::Entry,
    esp_lifetime: gtk::Entry,
    ike_port: gtk::Entry,
    error: gtk::Label,
}

impl MyWidgets {
    fn validate(&self) -> anyhow::Result<()> {
        if self.server_name.text().is_empty() {
            return Err(anyhow!("No server address specified"));
        }

        if self.auth_type.active().is_none() {
            return Err(anyhow!("No authentication method selected"));
        }

        if self.user_name.is_sensitive() && self.user_name.text().is_empty() {
            return Err(anyhow!("No user name specified"));
        }

        let cert_path = self.cert_path.text();

        if !cert_path.is_empty() && !Path::new(&cert_path).exists() {
            return Err(anyhow!("File does not exist: {}", cert_path));
        }

        let cert_id = self.cert_id.text().replace(':', "");
        if !cert_id.is_empty() && hex::decode(&cert_id).is_err() {
            return Err(anyhow!("Certificate ID not in hex format: {}", cert_id));
        }

        let ca_cert = self.ca_cert.text();

        if !ca_cert.is_empty() && !Path::new(&ca_cert).exists() {
            return Err(anyhow!("CA root certificate does not exist: {}", ca_cert));
        }

        self.ike_lifetime.text().parse::<u32>()?;
        self.esp_lifetime.text().parse::<u32>()?;
        self.ike_port.text().parse::<u16>()?;

        let add_routes = self.add_routes.text();
        if !add_routes.is_empty() {
            for r in add_routes.split(',') {
                r.parse::<Ipv4Net>()?;
            }
        }

        let ignored_routes = self.ignored_routes.text();
        if !ignored_routes.is_empty() {
            for r in ignored_routes.split(',') {
                r.parse::<Ipv4Net>()?;
            }
        }

        Ok(())
    }
}

impl SettingsDialog {
    pub fn new(params: Arc<TunnelParams>) -> Self {
        let dialog = gtk::Dialog::with_buttons(
            Some("VPN settings"),
            None::<&gtk::Window>,
            DialogFlags::MODAL,
            &[("OK", ResponseType::Ok), ("Cancel", ResponseType::Cancel)],
        );

        dialog.set_default_width(650);
        dialog.set_default_height(350);
        dialog.set_position(WindowPosition::CenterAlways);

        let server_name = gtk::Entry::builder().text(&params.server_name).hexpand(true).build();
        let fetch_info = gtk::Button::builder().label("Fetch info").halign(Align::End).build();
        let auth_type = gtk::ComboBoxText::builder().build();
        let tunnel_type = gtk::ComboBoxText::builder().build();
        let user_name = gtk::Entry::builder().text(&params.user_name).build();
        let password = gtk::Entry::builder().text(&params.password).visibility(false).build();

        let no_dns = gtk::CheckButton::builder().active(params.no_dns).build();

        let search_domains = gtk::Entry::builder()
            .placeholder_text("Comma-separated domains")
            .text(params.search_domains.join(","))
            .build();

        let ignored_domains = gtk::Entry::builder()
            .placeholder_text("Comma-separated domains")
            .text(params.ignore_search_domains.join(","))
            .build();

        let no_routing = gtk::CheckButton::builder().active(params.no_routing).build();
        let default_routing = gtk::CheckButton::builder().active(params.default_route).build();

        let add_routes = gtk::Entry::builder()
            .placeholder_text("Comma-separated x.x.x.x/x")
            .text(
                params
                    .add_routes
                    .iter()
                    .map(|r| r.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            )
            .build();

        let ignored_routes = gtk::Entry::builder()
            .placeholder_text("Comma-separated x.x.x.x/x")
            .text(
                params
                    .ignore_routes
                    .iter()
                    .map(|r| r.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            )
            .build();

        let mfa_prompts = gtk::CheckButton::builder().active(params.server_prompt).build();
        let no_keychain = gtk::CheckButton::builder().active(params.no_keychain).build();
        let no_cert_name_check = gtk::CheckButton::builder().active(params.no_cert_check).build();
        let no_cert_check = gtk::CheckButton::builder().active(params.ignore_server_cert).build();
        let cert_type = gtk::ComboBoxText::builder().build();
        let cert_path = gtk::Entry::builder()
            .text(
                params
                    .cert_path
                    .as_deref()
                    .map(|p| format!("{}", p.display()))
                    .unwrap_or_default(),
            )
            .build();
        let cert_password = gtk::Entry::builder()
            .text(params.cert_password.as_deref().unwrap_or_default())
            .visibility(false)
            .build();
        let cert_id = gtk::Entry::builder()
            .text(params.cert_id.as_deref().unwrap_or_default())
            .build();
        let ca_cert = gtk::Entry::builder()
            .text(
                params
                    .ca_cert
                    .as_deref()
                    .map(|p| format!("{}", p.display()))
                    .unwrap_or_default(),
            )
            .build();
        let ike_lifetime = gtk::Entry::builder()
            .text(params.ike_lifetime.as_secs().to_string())
            .build();
        let esp_lifetime = gtk::Entry::builder()
            .text(params.esp_lifetime.as_secs().to_string())
            .build();
        let ike_port = gtk::Entry::builder().text(params.ike_port.to_string()).build();

        let provider = gtk::CssProvider::new();
        provider.load_from_data(CSS_ERROR.as_bytes()).unwrap();

        let error = gtk::Label::new(None);
        error.style_context().add_provider(&provider, 100);

        auth_type.connect_active_notify(
            clone!(@weak auth_type, @weak user_name, @weak password, @weak tunnel_type => move |widget| {
                if let Some(id) = widget.active_id() {
                    let factors = unsafe { auth_type.data::<Vec<String>>(&id).map(|p| p.as_ref()) };
                    if let Some(factors) = factors {
                        let is_saml = factors.iter().any(|f| f == "identity_provider");
                        let is_cert = factors.iter().any(|f| f == "certificate");
                        user_name.set_sensitive(!is_saml && !is_cert);
                        password.set_sensitive(!is_saml && !is_cert);
                        if is_saml {
                            tunnel_type.set_active(Some(0));
                            tunnel_type.set_sensitive(false);
                        } else {
                            tunnel_type.set_sensitive(true);
                        }
                    }
                }
            }),
        );

        let (sender, receiver) = async_channel::bounded(1);
        let params2 = params.clone();

        fetch_info.connect_clicked(clone!(@weak dialog,
                                        @weak auth_type,
                                        @weak server_name,
                                        @weak no_cert_name_check,
                                        @weak no_cert_check => move |_| {
            if !server_name.text().is_empty() {
                dialog.set_sensitive(false);
                let params = TunnelParams {
                    server_name: server_name.text().into(),
                    no_cert_check: no_cert_name_check.is_active(),
                    ignore_server_cert: no_cert_check.is_active(),
                    ..(*params2).clone()
                };
                glib::spawn_future_local(clone!(@strong sender => async move {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .enable_all()
                        .build()
                        .unwrap();
                    let response = rt
                        .spawn(async move { server_info::get(&params).await })
                        .await
                        .unwrap();
                    let _ = sender.send(response).await;
                    Ok::<_, anyhow::Error>(())
                }));
            } else {
                auth_type.set_sensitive(false);
            }
        }));

        let params2 = params.clone();

        glib::spawn_future_local(clone!(@weak dialog, @weak auth_type, @weak error => async move {
            while let Ok(result) = receiver.recv().await {
                auth_type.remove_all();
                match result {
                    Ok(server_info) => {
                        error.set_label("");
                        error.set_visible(false);
                        for (i, (_, option)) in server_info.login_options_data.login_options_list.into_iter().enumerate() {
                            let factors = option
                                .factors
                                .values()
                                .map(|factor| factor.factor_type.clone())
                                .collect::<Vec<_>>();
                            unsafe { auth_type.set_data(&option.id, factors); }
                            auth_type.append(Some(&option.id), &option.display_name.0);
                            if params2.login_type == option.id {
                                auth_type.set_active(Some(i as _));
                            }
                        }
                        auth_type.set_sensitive(true);
                    }
                    Err(e) => {
                        error.set_label(&e.to_string());
                        error.set_visible(true);
                    }
                }
                dialog.set_sensitive(true);
            }
        }));

        dialog.connect_show(clone!(@weak fetch_info => move |_| fetch_info.emit_clicked()));

        let widgets = Rc::new(MyWidgets {
            server_name,
            fetch_info,
            auth_type,
            tunnel_type,
            user_name,
            password,
            no_dns,
            search_domains,
            ignored_domains,
            no_routing,
            default_routing,
            add_routes,
            ignored_routes,
            mfa_prompts,
            no_keychain,
            no_cert_name_check,
            no_cert_check,
            cert_type,
            cert_path,
            cert_password,
            cert_id,
            ca_cert,
            ike_lifetime,
            esp_lifetime,
            ike_port,
            error,
        });

        let widgets2 = widgets.clone();

        dialog.connect_response(move |dlg, response| {
            if response == ResponseType::Ok {
                if let Err(e) = widgets2.validate() {
                    let msg = gtk::MessageDialog::new(
                        Some(dlg),
                        DialogFlags::MODAL,
                        MessageType::Error,
                        ButtonsType::Ok,
                        &e.to_string(),
                    );
                    msg.run();
                    msg.close();
                    dlg.stop_signal_emission_by_name("response");
                }
            }
        });

        let result = Self {
            params,
            dialog,
            widgets,
        };

        result.create_layout();

        result
    }

    pub fn run(&self) -> ResponseType {
        self.dialog.run()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let mut params = (*self.params).clone();
        params.server_name = self.widgets.server_name.text().into();
        params.login_type = self.widgets.auth_type.active_id().unwrap_or_default().into();
        params.tunnel_type = match self.widgets.tunnel_type.active().unwrap_or_default() {
            0 => TunnelType::Ipsec,
            _ => TunnelType::Ssl,
        };
        params.user_name = self.widgets.user_name.text().into();
        params.password = self.widgets.password.text().into();
        params.no_dns = self.widgets.no_dns.is_active();
        params.search_domains = self
            .widgets
            .search_domains
            .text()
            .split(',')
            .map(|s| s.trim().to_owned())
            .collect();
        params.ignore_search_domains = self
            .widgets
            .ignored_domains
            .text()
            .split(',')
            .map(|s| s.trim().to_owned())
            .collect();
        params.no_routing = self.widgets.no_routing.is_active();
        params.default_route = self.widgets.default_routing.is_active();
        params.add_routes = self
            .widgets
            .add_routes
            .text()
            .split(',')
            .flat_map(|s| s.trim().parse().ok())
            .collect();
        params.ignore_routes = self
            .widgets
            .ignored_routes
            .text()
            .split(',')
            .flat_map(|s| s.trim().parse().ok())
            .collect();
        params.server_prompt = self.widgets.mfa_prompts.is_active();
        params.no_keychain = self.widgets.no_keychain.is_active();
        params.no_cert_check = self.widgets.no_cert_name_check.is_active();
        params.ignore_server_cert = self.widgets.no_cert_check.is_active();
        params.cert_type = self.widgets.cert_type.active().unwrap_or_default().into();
        params.cert_path = {
            let text = self.widgets.cert_path.text();
            if text.is_empty() {
                None
            } else {
                Some(text.into())
            }
        };
        params.cert_password = {
            let text = self.widgets.cert_password.text();
            if text.is_empty() {
                None
            } else {
                Some(text.into())
            }
        };
        params.cert_id = {
            let text = self.widgets.cert_id.text();
            if text.is_empty() {
                None
            } else {
                Some(text.into())
            }
        };
        params.ca_cert = {
            let text = self.widgets.ca_cert.text();
            if text.is_empty() {
                None
            } else {
                Some(text.into())
            }
        };
        params.ike_lifetime = Duration::from_secs(self.widgets.ike_lifetime.text().parse()?);
        params.esp_lifetime = Duration::from_secs(self.widgets.esp_lifetime.text().parse()?);
        params.ike_port = self.widgets.ike_port.text().parse()?;

        params.save()?;

        Ok(())
    }

    fn form_box(&self, label: &str) -> gtk::Box {
        let form = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .homogeneous(true)
            .build();

        form.pack_start(
            &gtk::Label::builder().label(label).halign(Align::Start).build(),
            false,
            true,
            0,
        );
        form
    }

    fn server_box(&self) -> gtk::Box {
        let entry_box = gtk::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(2)
            .homogeneous(false)
            .build();
        entry_box.pack_start(&self.widgets.server_name, false, true, 0);
        entry_box.pack_start(&self.widgets.fetch_info, false, false, 0);

        let server_box = self.form_box("Checkpoint VPN server");
        server_box.pack_start(&entry_box, false, true, 0);
        server_box
    }

    fn auth_box(&self) -> gtk::Box {
        let auth_box = self.form_box("Authentication method");
        auth_box.pack_start(&self.widgets.auth_type, false, true, 0);
        auth_box
    }

    fn tunnel_box(&self) -> gtk::Box {
        let tunnel_box = self.form_box("Tunnel type");
        self.widgets.tunnel_type.insert_text(0, "IPSec");
        self.widgets.tunnel_type.insert_text(1, "SSL");
        self.widgets
            .tunnel_type
            .set_active(if self.params.tunnel_type == TunnelType::Ipsec {
                Some(0)
            } else {
                Some(1)
            });
        tunnel_box.pack_start(&self.widgets.tunnel_type, false, true, 0);
        tunnel_box
    }

    fn cert_type_box(&self) -> gtk::Box {
        let cert_type_box = self.form_box("Certificate auth type");
        self.widgets.cert_type.insert_text(0, "None");
        self.widgets.cert_type.insert_text(1, "PFX file");
        self.widgets.cert_type.insert_text(2, "PEM file");
        self.widgets.cert_type.insert_text(3, "HW token");
        self.widgets.cert_type.set_active(Some(self.params.cert_type.as_u32()));
        cert_type_box.pack_start(&self.widgets.cert_type, false, true, 0);
        cert_type_box
    }

    fn user_box(&self) -> gtk::Box {
        let user_box = self.form_box("User name");
        user_box.pack_start(&self.widgets.user_name, false, true, 0);
        user_box
    }

    fn password_box(&self) -> gtk::Box {
        let password_box = self.form_box("Password (optional)");
        password_box.pack_start(&self.widgets.password, false, true, 0);
        password_box
    }

    fn dns_box(&self) -> gtk::Box {
        let dns_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .margin(6)
            .margin_start(16)
            .margin_end(16)
            .build();

        let no_dns = self.form_box("Do not change DNS resolver configuration");
        no_dns.pack_start(&self.widgets.no_dns, false, true, 0);
        dns_box.pack_start(&no_dns, false, true, 6);

        let search_domains = self.form_box("Additional search domains");
        search_domains.pack_start(&self.widgets.search_domains, false, true, 0);
        dns_box.pack_start(&search_domains, false, true, 6);

        let ignored_domains = self.form_box("Ignored search domains");
        ignored_domains.pack_start(&self.widgets.ignored_domains, false, true, 0);
        dns_box.pack_start(&ignored_domains, false, true, 6);

        dns_box
    }

    fn misc_box(&self) -> gtk::Box {
        let misc_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .margin(6)
            .margin_start(16)
            .margin_end(16)
            .build();

        let mfa_prompts = self.form_box("Ask server for MFA prompts");
        mfa_prompts.pack_start(&self.widgets.mfa_prompts, false, true, 0);
        misc_box.pack_start(&mfa_prompts, false, true, 6);

        let no_keychain = self.form_box("Do not store passwords in the keychain");
        no_keychain.pack_start(&self.widgets.no_keychain, false, true, 0);
        misc_box.pack_start(&no_keychain, false, true, 6);

        let no_cert_name_check = self.form_box("Do not check certificate CN");
        no_cert_name_check.pack_start(&self.widgets.no_cert_name_check, false, true, 0);
        misc_box.pack_start(&no_cert_name_check, false, true, 6);

        let no_cert_check = self.form_box("Disable all certificate checks (INSECURE!)");
        no_cert_check.pack_start(&self.widgets.no_cert_check, false, true, 0);
        misc_box.pack_start(&no_cert_check, false, true, 6);

        let cert_type_box = self.cert_type_box();
        misc_box.pack_start(&cert_type_box, false, true, 6);

        let cert_path = self.form_box("Client certificate or driver path (.pem, .pfx, .so)");
        cert_path.pack_start(&self.widgets.cert_path, false, true, 0);
        misc_box.pack_start(&cert_path, false, true, 6);

        let cert_password = self.form_box("PFX password or PKCS11 pin");
        cert_password.pack_start(&self.widgets.cert_password, false, true, 0);
        misc_box.pack_start(&cert_password, false, true, 6);

        let cert_id = self.form_box("Hex ID of PKCS11 certificate");
        cert_id.pack_start(&self.widgets.cert_id, false, true, 0);
        misc_box.pack_start(&cert_id, false, true, 6);

        let ca_cert = self.form_box("CA root certificate path (.pem or .der)");
        ca_cert.pack_start(&self.widgets.ca_cert, false, true, 0);
        misc_box.pack_start(&ca_cert, false, true, 6);

        let ike_lifetime = self.form_box("IKE lifetime, seconds");
        ike_lifetime.pack_start(&self.widgets.ike_lifetime, false, true, 0);
        misc_box.pack_start(&ike_lifetime, false, true, 6);

        let esp_lifetime = self.form_box("ESP lifetime, seconds");
        esp_lifetime.pack_start(&self.widgets.esp_lifetime, false, true, 0);
        misc_box.pack_start(&esp_lifetime, false, true, 6);

        let ike_port = self.form_box("IKE port");
        ike_port.pack_start(&self.widgets.ike_port, false, true, 0);
        misc_box.pack_start(&ike_port, false, true, 6);

        misc_box
    }

    fn routing_box(&self) -> gtk::Box {
        let routing_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .margin(6)
            .margin_start(16)
            .margin_end(16)
            .build();

        let no_routing = self.form_box("Ignore all acquired routes");
        no_routing.pack_start(&self.widgets.no_routing, false, true, 0);
        routing_box.pack_start(&no_routing, false, true, 6);

        let default_routing = self.form_box("Set default route through the tunnel");
        default_routing.pack_start(&self.widgets.default_routing, false, true, 0);
        routing_box.pack_start(&default_routing, false, true, 6);

        let add_routes = self.form_box("Additional static routes");
        add_routes.pack_start(&self.widgets.add_routes, false, true, 0);
        routing_box.pack_start(&add_routes, false, true, 6);

        let ignored_routes = self.form_box("Routes to ignore");
        ignored_routes.pack_start(&self.widgets.ignored_routes, false, true, 0);
        routing_box.pack_start(&ignored_routes, false, true, 6);

        routing_box
    }

    fn general_tab(&self) -> gtk::Box {
        let tab = gtk::Box::builder().orientation(Orientation::Vertical).margin(6).build();
        tab.pack_start(&self.server_box(), false, true, 6);
        tab.pack_start(&self.auth_box(), false, true, 6);
        tab.pack_start(&self.tunnel_box(), false, true, 6);
        tab.pack_start(&self.user_box(), false, true, 6);
        tab.pack_start(&self.password_box(), false, true, 6);
        tab
    }

    fn advanced_tab(&self) -> gtk::ScrolledWindow {
        let inner = gtk::Box::builder().orientation(Orientation::Vertical).build();

        let dns = gtk::Expander::new(Some("DNS"));
        dns.add(&self.dns_box());
        inner.pack_start(&dns, false, true, 6);

        let routing = gtk::Expander::new(Some("Routing"));
        routing.add(&self.routing_box());
        inner.pack_start(&routing, false, true, 6);

        let misc = gtk::Expander::new(Some("Misc settings"));
        misc.add(&self.misc_box());
        inner.pack_start(&misc, false, true, 6);

        let viewport = gtk::Viewport::builder().build();
        viewport.add(&inner);

        let scrolled_win = gtk::ScrolledWindow::builder().build();
        scrolled_win.add(&viewport);
        scrolled_win
    }

    fn create_layout(&self) {
        let content_area = self.dialog.content_area();
        let notebook = gtk::Notebook::new();
        content_area.pack_start(&notebook, true, true, 6);
        content_area.pack_end(&self.widgets.error, true, true, 6);

        notebook.append_page(&self.general_tab(), Some(&gtk::Label::new(Some("General"))));
        notebook.append_page(&self.advanced_tab(), Some(&gtk::Label::new(Some("Advanced"))));

        notebook.show_all();
    }
}

impl Drop for SettingsDialog {
    fn drop(&mut self) {
        self.dialog.close();
    }
}

pub fn start_settings_dialog(params: Arc<TunnelParams>) {
    glib::idle_add(move || {
        let dialog = SettingsDialog::new(params.clone());
        if dialog.run() == ResponseType::Ok {
            if let Err(e) = dialog.save() {
                warn!("{}", e);
            }
        }
        glib::ControlFlow::Break
    });
}
