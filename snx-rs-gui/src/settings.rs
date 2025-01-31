use std::{net::Ipv4Addr, path::Path, rc::Rc, sync::Arc, time::Duration};

use async_channel::Sender;
use gtk::{
    glib::{self, clone},
    prelude::*,
    Align, ButtonsType, DialogFlags, MessageType, Orientation, ResponseType, Widget, WindowPosition,
};
use ipnet::Ipv4Net;
use tracing::warn;

use snxcore::{
    model::{
        params::{TunnelParams, TunnelType},
        proto::LoginOption,
    },
    server_info,
};

use crate::tray::TrayCommand;

const CSS_ERROR: &str = r"label {
    padding: 6px;
    border: 1px solid #f44336;
    color: #ffffff;
    background-color: #a02a2a;
}
";

fn set_container_visible(widget: &Widget, flag: bool) {
    if let Some(parent) = widget.parent() {
        if let Some(parent) = parent.parent() {
            if flag {
                parent.show_all();
            } else {
                parent.hide();
            }
        }
    }
}

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
    dns_servers: gtk::Entry,
    ignored_dns_servers: gtk::Entry,
    no_routing: gtk::CheckButton,
    default_routing: gtk::CheckButton,
    add_routes: gtk::Entry,
    ignored_routes: gtk::Entry,
    mfa_prompts: gtk::CheckButton,
    no_keychain: gtk::CheckButton,
    no_cert_name_check: gtk::CheckButton,
    no_cert_check: gtk::CheckButton,
    ipsec_cert_check: gtk::CheckButton,
    cert_type: gtk::ComboBoxText,
    cert_path: gtk::Entry,
    cert_password: gtk::Entry,
    cert_id: gtk::Entry,
    ca_cert: gtk::Entry,
    ike_lifetime: gtk::Entry,
    esp_lifetime: gtk::Entry,
    ike_port: gtk::Entry,
    ike_persist: gtk::CheckButton,
    ike_transport: gtk::ComboBoxText,
    esp_transport: gtk::ComboBoxText,
    no_keepalive: gtk::CheckButton,
    icon_theme: gtk::ComboBoxText,
    error: gtk::Label,
}

impl MyWidgets {
    fn validate(&self) -> anyhow::Result<()> {
        if self.server_name.text().is_empty() {
            anyhow::bail!("No server address specified");
        }

        if self.auth_type.active().is_none() {
            anyhow::bail!("No authentication method selected");
        }

        let cert_path = self.cert_path.text();

        if !cert_path.is_empty() && !Path::new(&cert_path).exists() {
            anyhow::bail!("File does not exist: {}", cert_path);
        }

        let cert_id = self.cert_id.text().replace(':', "");
        if !cert_id.is_empty() && hex::decode(&cert_id).is_err() {
            anyhow::bail!("Certificate ID not in hex format: {}", cert_id);
        }

        let ca_cert = self.ca_cert.text();

        if !ca_cert.is_empty() {
            for c in ca_cert.split(',') {
                if !Path::new(c.trim()).exists() {
                    anyhow::bail!("CA root path does not exist: {}", c);
                }
            }
        }

        self.ike_lifetime.text().parse::<u32>()?;
        self.esp_lifetime.text().parse::<u32>()?;
        self.ike_port.text().parse::<u16>()?;

        let dns_servers = self.dns_servers.text();
        if !dns_servers.is_empty() {
            for r in dns_servers.split(',') {
                r.parse::<Ipv4Addr>()?;
            }
        }

        let ignored_dns_servers = self.ignored_dns_servers.text();
        if !ignored_dns_servers.is_empty() {
            for r in ignored_dns_servers.split(',') {
                r.parse::<Ipv4Addr>()?;
            }
        }

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
    const DEFAULT_WIDTH: i32 = 700;
    const DEFAULT_HEIGHT: i32 = 370;

    pub fn new(params: Arc<TunnelParams>) -> Self {
        let dialog = gtk::Dialog::with_buttons(
            Some("VPN settings"),
            None::<&gtk::Window>,
            DialogFlags::MODAL,
            &[
                ("OK", ResponseType::Ok),
                ("Apply", ResponseType::Apply),
                ("Cancel", ResponseType::Cancel),
            ],
        );

        dialog.set_default_width(Self::DEFAULT_WIDTH);
        dialog.set_default_height(Self::DEFAULT_HEIGHT);
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

        let dns_servers = gtk::Entry::builder()
            .placeholder_text("Comma-separated IP addresses")
            .text(
                params
                    .dns_servers
                    .iter()
                    .map(|r| r.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            )
            .build();

        let ignored_dns_servers = gtk::Entry::builder()
            .placeholder_text("Comma-separated IP addresses")
            .text(
                params
                    .ignore_dns_servers
                    .iter()
                    .map(|r| r.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            )
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
        let ipsec_cert_check = gtk::CheckButton::builder().active(params.ipsec_cert_check).build();
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
            .placeholder_text("Comma-separated PEM or DER files")
            .text(
                params
                    .ca_cert
                    .iter()
                    .map(|p| format!("{}", p.display()))
                    .collect::<Vec<_>>()
                    .join(","),
            )
            .build();
        let ike_lifetime = gtk::Entry::builder()
            .text(params.ike_lifetime.as_secs().to_string())
            .build();
        let esp_lifetime = gtk::Entry::builder()
            .text(params.esp_lifetime.as_secs().to_string())
            .build();
        let esp_transport = gtk::ComboBoxText::builder().build();
        let ike_port = gtk::Entry::builder().text(params.ike_port.to_string()).build();
        let ike_persist = gtk::CheckButton::builder().active(params.ike_persist).build();
        let ike_transport = gtk::ComboBoxText::builder().build();
        let no_keepalive = gtk::CheckButton::builder().active(params.no_keepalive).build();
        let icon_theme = gtk::ComboBoxText::builder().build();

        let provider = gtk::CssProvider::new();
        provider.load_from_data(CSS_ERROR.as_bytes()).unwrap();

        let error = gtk::Label::new(None);
        error.style_context().add_provider(&provider, 100);

        auth_type.connect_active_notify(clone!(@weak dialog,
            @weak auth_type,
            @weak user_name,
            @weak tunnel_type,
            @weak cert_path,
            @weak cert_type => move |widget| {
            if let Some(id) = widget.active_id() {
                let factors = unsafe { auth_type.data::<Vec<String>>(&id).map(|p| p.as_ref()) };
                if let Some(factors) = factors {
                    let is_saml = factors.iter().any(|f| f == "identity_provider");
                    let is_cert = factors.iter().any(|f| f == "certificate");
                    set_container_visible(user_name.as_ref(), !is_saml && !is_cert);
                    set_container_visible(cert_path.as_ref(), is_cert);
                    dialog.resize(SettingsDialog::DEFAULT_WIDTH, SettingsDialog::DEFAULT_HEIGHT);
                    if !is_cert {
                        cert_type.set_active(Some(0));
                    }
                    if is_saml {
                        tunnel_type.set_active(Some(0));
                        tunnel_type.set_sensitive(false);
                    } else {
                        tunnel_type.set_sensitive(true);
                    }
                }
            }
        }));

        let (sender, receiver) = async_channel::bounded(1);
        let params2 = params.clone();

        fetch_info.connect_clicked(clone!(@weak dialog,
            @weak auth_type,
            @weak server_name,
            @weak no_cert_name_check,
            @weak no_cert_check => move |_| {
            if server_name.text().is_empty() {
                auth_type.set_sensitive(false);
            } else {
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
                        let mut options_list = server_info
                            .login_options_data
                            .map(|d| d.login_options_list)
                            .unwrap_or_default();
                        if options_list.is_empty() {
                            options_list.insert(String::new(), LoginOption::unspecified());
                        }
                        for (i, (_, option)) in options_list.into_iter().enumerate() {
                            let factors = option
                                .factors
                                .values()
                                .map(|factor| factor.factor_type.clone())
                                .collect::<Vec<_>>();
                            unsafe { auth_type.set_data(&option.id, factors); }
                            auth_type.append(Some(&option.id), &option.display_name);
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
            dns_servers,
            ignored_dns_servers,
            no_routing,
            default_routing,
            add_routes,
            ignored_routes,
            mfa_prompts,
            no_keychain,
            no_cert_name_check,
            no_cert_check,
            ipsec_cert_check,
            cert_type,
            cert_path,
            cert_password,
            cert_id,
            ca_cert,
            ike_lifetime,
            esp_lifetime,
            esp_transport,
            ike_port,
            ike_persist,
            ike_transport,
            no_keepalive,
            icon_theme,
            error,
        });

        let widgets2 = widgets.clone();

        dialog.connect_response(move |dlg, response| {
            if response == ResponseType::Ok || response == ResponseType::Apply {
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
        params.dns_servers = self
            .widgets
            .dns_servers
            .text()
            .split(',')
            .flat_map(|s| s.trim().parse().ok())
            .collect();
        params.ignore_dns_servers = self
            .widgets
            .ignored_dns_servers
            .text()
            .split(',')
            .flat_map(|s| s.trim().parse().ok())
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
        params.ipsec_cert_check = self.widgets.ipsec_cert_check.is_active();
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
        params.ca_cert = self
            .widgets
            .ca_cert
            .text()
            .split(',')
            .map(|s| s.trim().into())
            .collect();
        params.ike_lifetime = Duration::from_secs(self.widgets.ike_lifetime.text().parse()?);
        params.esp_lifetime = Duration::from_secs(self.widgets.esp_lifetime.text().parse()?);
        params.esp_transport = self.widgets.esp_transport.active().unwrap_or_default().into();
        params.ike_port = self.widgets.ike_port.text().parse()?;
        params.ike_persist = self.widgets.ike_persist.is_active();
        params.no_keepalive = self.widgets.no_keepalive.is_active();
        params.icon_theme = self.widgets.icon_theme.active().unwrap_or_default().into();
        params.ike_transport = self.widgets.ike_transport.active().unwrap_or_default().into();

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

        let server_box = self.form_box("Check Point VPN server");
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
        self.widgets.cert_type.insert_text(3, "Hardware token");
        self.widgets.cert_type.set_active(Some(self.params.cert_type.as_u32()));
        cert_type_box.pack_start(&self.widgets.cert_type, false, true, 0);
        cert_type_box
    }

    fn icon_theme_box(&self) -> gtk::Box {
        let icon_theme_box = self.form_box("Icon theme");
        self.widgets.icon_theme.insert_text(0, "Auto");
        self.widgets.icon_theme.insert_text(1, "Dark");
        self.widgets.icon_theme.insert_text(2, "Light");
        self.widgets
            .icon_theme
            .set_active(Some(self.params.icon_theme.as_u32()));
        icon_theme_box.pack_start(&self.widgets.icon_theme, false, true, 0);
        icon_theme_box
    }

    fn ike_transport_box(&self) -> gtk::Box {
        let ike_transport_box = self.form_box("IKE transport");
        self.widgets.ike_transport.insert_text(0, "UDP");
        self.widgets.ike_transport.insert_text(1, "TCPT");
        self.widgets
            .ike_transport
            .set_active(Some(self.params.ike_transport.as_u32()));
        ike_transport_box.pack_start(&self.widgets.ike_transport, false, true, 0);
        ike_transport_box
    }

    fn esp_transport_box(&self) -> gtk::Box {
        let esp_transport_box = self.form_box("ESP transport");
        self.widgets.esp_transport.insert_text(0, "UDP");
        self.widgets.esp_transport.insert_text(1, "TCPT");
        self.widgets
            .esp_transport
            .set_active(Some(self.params.esp_transport.as_u32()));
        esp_transport_box.pack_start(&self.widgets.esp_transport, false, true, 0);
        esp_transport_box
    }

    fn user_box(&self) -> gtk::Box {
        let user_box = self.form_box("User name");
        user_box.pack_start(&self.widgets.user_name, false, true, 0);
        user_box
    }

    fn password_box(&self) -> gtk::Box {
        let password_box = self.form_box("Password");
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

        let dns_servers = self.form_box("Additional DNS servers");
        dns_servers.pack_start(&self.widgets.dns_servers, false, true, 0);
        dns_box.pack_start(&dns_servers, false, true, 6);

        let ignored_dns_servers = self.form_box("Ignored DNS servers");
        ignored_dns_servers.pack_start(&self.widgets.ignored_dns_servers, false, true, 0);
        dns_box.pack_start(&ignored_dns_servers, false, true, 6);

        let search_domains = self.form_box("Additional search domains");
        search_domains.pack_start(&self.widgets.search_domains, false, true, 0);
        dns_box.pack_start(&search_domains, false, true, 6);

        let ignored_domains = self.form_box("Ignored search domains");
        ignored_domains.pack_start(&self.widgets.ignored_domains, false, true, 0);
        dns_box.pack_start(&ignored_domains, false, true, 6);

        dns_box
    }

    fn certs_box(&self) -> gtk::Box {
        let certs_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .margin(6)
            .margin_start(16)
            .margin_end(16)
            .build();

        let ca_cert = self.form_box("Server CA root certificates");
        ca_cert.pack_start(&self.widgets.ca_cert, false, true, 0);
        certs_box.pack_start(&ca_cert, false, true, 6);

        let no_cert_name_check = self.form_box("Disable TLS server hostname check");
        no_cert_name_check.pack_start(&self.widgets.no_cert_name_check, false, true, 0);
        certs_box.pack_start(&no_cert_name_check, false, true, 6);

        let no_cert_check = self.form_box("Disable all TLS certificate checks (INSECURE!)");
        no_cert_check.pack_start(&self.widgets.no_cert_check, false, true, 0);
        certs_box.pack_start(&no_cert_check, false, true, 6);

        let ipsec_cert_check = self.form_box("Enable IPSec certificate validation");
        ipsec_cert_check.pack_start(&self.widgets.ipsec_cert_check, false, true, 0);
        certs_box.pack_start(&ipsec_cert_check, false, true, 6);

        certs_box
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

        let ike_lifetime = self.form_box("IKE lifetime, seconds");
        ike_lifetime.pack_start(&self.widgets.ike_lifetime, false, true, 0);
        misc_box.pack_start(&ike_lifetime, false, true, 6);

        let esp_lifetime = self.form_box("ESP lifetime, seconds");
        esp_lifetime.pack_start(&self.widgets.esp_lifetime, false, true, 0);
        misc_box.pack_start(&esp_lifetime, false, true, 6);

        let esp_transport_box = self.esp_transport_box();
        misc_box.pack_start(&esp_transport_box, false, true, 6);

        let ike_port = self.form_box("IKE port");
        ike_port.pack_start(&self.widgets.ike_port, false, true, 0);
        misc_box.pack_start(&ike_port, false, true, 6);

        let ike_persist = self.form_box("Save IKE session and reconnect automatically");
        ike_persist.pack_start(&self.widgets.ike_persist, false, true, 0);
        misc_box.pack_start(&ike_persist, false, true, 6);

        let ike_transport_box = self.ike_transport_box();
        misc_box.pack_start(&ike_transport_box, false, true, 6);

        let no_keepalive = self.form_box("Disable keepalive packets");
        no_keepalive.pack_start(&self.widgets.no_keepalive, false, true, 0);
        misc_box.pack_start(&no_keepalive, false, true, 6);

        let icon_theme_box = self.icon_theme_box();
        misc_box.pack_start(&icon_theme_box, false, true, 6);

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

    fn user_auth_box(&self) -> gtk::Box {
        let user_auth_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .margin(0)
            .margin_start(0)
            .margin_end(0)
            .build();
        user_auth_box.pack_start(&self.user_box(), false, true, 6);
        user_auth_box.pack_start(&self.password_box(), false, true, 6);

        user_auth_box
    }

    fn cert_auth_box(&self) -> gtk::Box {
        let certs_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .margin(0)
            .margin_start(0)
            .margin_end(0)
            .build();

        let cert_type_box = self.cert_type_box();
        certs_box.pack_start(&cert_type_box, false, true, 6);

        let cert_path = self.form_box("Client certificate or driver path (.pem, .pfx/.p12, .so)");
        cert_path.pack_start(&self.widgets.cert_path, false, true, 0);
        certs_box.pack_start(&cert_path, false, true, 6);

        let cert_password = self.form_box("PFX password or PKCS11 pin");
        cert_password.pack_start(&self.widgets.cert_password, false, true, 0);
        certs_box.pack_start(&cert_password, false, true, 6);

        let cert_id = self.form_box("Hex ID of PKCS11 certificate");
        cert_id.pack_start(&self.widgets.cert_id, false, true, 0);
        certs_box.pack_start(&cert_id, false, true, 6);

        certs_box
    }

    fn general_tab(&self) -> gtk::Box {
        let tab = gtk::Box::builder().orientation(Orientation::Vertical).margin(6).build();
        tab.pack_start(&self.server_box(), false, true, 6);
        tab.pack_start(&self.auth_box(), false, true, 6);
        tab.pack_start(&self.tunnel_box(), false, true, 6);
        tab.show_all();
        tab.pack_start(&self.user_auth_box(), false, true, 6);
        tab.pack_start(&self.cert_auth_box(), false, true, 6);
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

        let certs = gtk::Expander::new(Some("Certificates"));
        certs.add(&self.certs_box());
        inner.pack_start(&certs, false, true, 6);

        let misc = gtk::Expander::new(Some("Misc settings"));
        misc.add(&self.misc_box());
        inner.pack_start(&misc, false, true, 6);

        let viewport = gtk::Viewport::builder().build();
        viewport.add(&inner);

        let scrolled_win = gtk::ScrolledWindow::builder().build();
        scrolled_win.add(&viewport);
        scrolled_win.show_all();
        scrolled_win
    }

    fn create_layout(&self) {
        let content_area = self.dialog.content_area();
        let notebook = gtk::Notebook::new();
        content_area.pack_start(&notebook, true, true, 6);
        content_area.pack_end(&self.widgets.error, true, true, 6);

        notebook.append_page(&self.general_tab(), Some(&gtk::Label::new(Some("General"))));
        notebook.append_page(&self.advanced_tab(), Some(&gtk::Label::new(Some("Advanced"))));

        notebook.show();
    }
}

impl Drop for SettingsDialog {
    fn drop(&mut self) {
        self.dialog.close();
    }
}

pub fn start_settings_dialog(sender: Sender<TrayCommand>, params: Arc<TunnelParams>) {
    glib::idle_add(move || {
        let dialog = SettingsDialog::new(params.clone());
        loop {
            let response = dialog.run();

            match response {
                ResponseType::Ok | ResponseType::Apply => {
                    if let Err(e) = dialog.save() {
                        warn!("{}", e);
                    } else {
                        let _ = sender.send_blocking(TrayCommand::Update);
                    }
                }
                _ => {}
            }
            if response != ResponseType::Apply {
                break;
            }
        }
        glib::ControlFlow::Break
    });
}
