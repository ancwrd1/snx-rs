use std::{net::Ipv4Addr, path::Path, rc::Rc, sync::Arc, time::Duration};

use gtk4::{
    Align, ButtonsType, DialogFlags, MessageType, Orientation, ResponseType, Widget, Window,
    glib::{self, clone},
    prelude::*,
};
use ipnet::Ipv4Net;
use snxcore::{
    model::{
        params::{TunnelParams, TunnelType},
        proto::LoginOption,
    },
    server_info,
};
use tokio::sync::mpsc::Sender;
use tracing::warn;

use crate::{tr, tray::TrayCommand};

fn set_container_visible(widget: &Widget, flag: bool) {
    if let Some(parent) = widget.parent() {
        if let Some(parent) = parent.parent() {
            if flag {
                parent.show();
            } else {
                parent.hide();
            }
        }
    }
}

struct SettingsDialog {
    params: Arc<TunnelParams>,
    dialog: gtk4::Dialog,
    widgets: Rc<MyWidgets>,
    revealers: Vec<gtk4::Revealer>,
}

struct MyWidgets {
    server_name: gtk4::Entry,
    fetch_info: gtk4::Button,
    auth_type: gtk4::ComboBoxText,
    tunnel_type: gtk4::ComboBoxText,
    user_name: gtk4::Entry,
    password: gtk4::Entry,
    password_factor: gtk4::Entry,
    no_dns: gtk4::Switch,
    search_domains: gtk4::Entry,
    ignored_domains: gtk4::Entry,
    dns_servers: gtk4::Entry,
    ignored_dns_servers: gtk4::Entry,
    set_routing_domains: gtk4::Switch,
    no_routing: gtk4::Switch,
    default_routing: gtk4::Switch,
    add_routes: gtk4::Entry,
    ignored_routes: gtk4::Entry,
    no_keychain: gtk4::Switch,
    no_cert_check: gtk4::Switch,
    cert_type: gtk4::ComboBoxText,
    cert_path: gtk4::Entry,
    cert_password: gtk4::Entry,
    cert_id: gtk4::Entry,
    ca_cert: gtk4::Entry,
    ike_lifetime: gtk4::Entry,
    ike_persist: gtk4::Switch,
    no_keepalive: gtk4::Switch,
    port_knock: gtk4::Switch,
    icon_theme: gtk4::ComboBoxText,
    error: gtk4::Label,
    button_box: gtk4::Box,
    locale: gtk4::ComboBoxText,
}

impl MyWidgets {
    fn validate(&self) -> anyhow::Result<()> {
        if self.server_name.text().is_empty() {
            anyhow::bail!(tr!("error-no-server"));
        }

        if self.auth_type.active().is_none() {
            anyhow::bail!(tr!("error-no-auth"));
        }

        let cert_path = self.cert_path.text();

        if !cert_path.is_empty() && !Path::new(&cert_path).exists() {
            anyhow::bail!(tr!("error-file-not-exist", path = cert_path.to_string()));
        }

        let cert_id = self.cert_id.text().replace(':', "");
        if !cert_id.is_empty() && hex::decode(&cert_id).is_err() {
            anyhow::bail!(tr!("error-invalid-cert-id", id = cert_id));
        }

        let ca_cert = self.ca_cert.text();

        if !ca_cert.is_empty() {
            for c in ca_cert.split(',') {
                if !Path::new(c.trim()).exists() {
                    anyhow::bail!(tr!("error-ca-root-not-exist", path = c));
                }
            }
        }

        self.ike_lifetime.text().parse::<u32>()?;
        self.password_factor.text().parse::<usize>()?;

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
    pub fn new<W: IsA<Window>>(parent: W, params: Arc<TunnelParams>) -> Self {
        let dialog = gtk4::Dialog::builder()
            .title(tr!("dialog-title"))
            .transient_for(&parent)
            .build();

        let button_box = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(6)
            .margin_top(6)
            .homogeneous(true)
            .halign(Align::End)
            .build();

        let ok_button = gtk4::Button::with_label(&tr!("button-ok"));
        ok_button.connect_clicked(clone!(
            #[weak]
            dialog,
            move |_| {
                dialog.response(ResponseType::Ok);
            }
        ));

        let apply_button = gtk4::Button::with_label(&tr!("button-apply"));
        apply_button.connect_clicked(clone!(
            #[weak]
            dialog,
            move |_| {
                dialog.response(ResponseType::Apply);
            }
        ));

        let cancel_button = gtk4::Button::with_label(&tr!("button-cancel"));
        cancel_button.connect_clicked(clone!(
            #[weak]
            dialog,
            move |_| {
                dialog.response(ResponseType::Cancel);
            }
        ));

        button_box.append(&ok_button);
        button_box.append(&apply_button);
        button_box.append(&cancel_button);

        let server_name = gtk4::Entry::builder().text(&params.server_name).hexpand(true).build();

        let fetch_info = gtk4::Button::builder()
            .label(tr!("button-fetch-info"))
            .halign(Align::End)
            .build();
        let auth_type = gtk4::ComboBoxText::builder().build();
        let tunnel_type = gtk4::ComboBoxText::builder().build();
        let user_name = gtk4::Entry::builder().text(&params.user_name).build();
        let password = gtk4::Entry::builder().text(&params.password).visibility(false).build();
        let password_factor = gtk4::Entry::builder().text(params.password_factor.to_string()).build();

        let no_dns = gtk4::Switch::builder()
            .active(params.no_dns)
            .halign(Align::Start)
            .build();
        let set_routing_domains = gtk4::Switch::builder()
            .active(params.set_routing_domains)
            .halign(Align::Start)
            .build();

        let search_domains = gtk4::Entry::builder()
            .placeholder_text(tr!("placeholder-domains"))
            .text(params.search_domains.join(","))
            .build();

        let ignored_domains = gtk4::Entry::builder()
            .placeholder_text(tr!("placeholder-domains"))
            .text(params.ignore_search_domains.join(","))
            .build();

        let dns_servers = gtk4::Entry::builder()
            .placeholder_text(tr!("placeholder-ip-addresses"))
            .text(
                params
                    .dns_servers
                    .iter()
                    .map(|r| r.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            )
            .build();

        let ignored_dns_servers = gtk4::Entry::builder()
            .placeholder_text(tr!("placeholder-ip-addresses"))
            .text(
                params
                    .ignore_dns_servers
                    .iter()
                    .map(|r| r.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            )
            .build();

        let no_routing = gtk4::Switch::builder()
            .active(params.no_routing)
            .halign(Align::Start)
            .build();
        let default_routing = gtk4::Switch::builder()
            .active(params.default_route)
            .halign(Align::Start)
            .build();

        let add_routes = gtk4::Entry::builder()
            .placeholder_text(tr!("placeholder-routes"))
            .text(
                params
                    .add_routes
                    .iter()
                    .map(|r| r.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            )
            .build();

        let ignored_routes = gtk4::Entry::builder()
            .placeholder_text(tr!("placeholder-routes"))
            .text(
                params
                    .ignore_routes
                    .iter()
                    .map(|r| r.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            )
            .build();

        let no_keychain = gtk4::Switch::builder()
            .active(params.no_keychain)
            .halign(Align::Start)
            .build();
        let no_cert_check = gtk4::Switch::builder()
            .active(params.ignore_server_cert)
            .halign(Align::Start)
            .build();
        let cert_type = gtk4::ComboBoxText::builder().build();
        let cert_path = gtk4::Entry::builder()
            .text(
                params
                    .cert_path
                    .as_deref()
                    .map(|p| format!("{}", p.display()))
                    .unwrap_or_default(),
            )
            .build();
        let cert_password = gtk4::Entry::builder()
            .text(params.cert_password.as_deref().unwrap_or_default())
            .visibility(false)
            .build();
        let cert_id = gtk4::Entry::builder()
            .text(params.cert_id.as_deref().unwrap_or_default())
            .build();
        let ca_cert = gtk4::Entry::builder()
            .placeholder_text(tr!("placeholder-certs"))
            .text(
                params
                    .ca_cert
                    .iter()
                    .map(|p| format!("{}", p.display()))
                    .collect::<Vec<_>>()
                    .join(","),
            )
            .build();
        let ike_lifetime = gtk4::Entry::builder()
            .text(params.ike_lifetime.as_secs().to_string())
            .build();
        let ike_persist = gtk4::Switch::builder()
            .active(params.ike_persist)
            .halign(Align::Start)
            .build();
        let no_keepalive = gtk4::Switch::builder()
            .active(params.no_keepalive)
            .halign(Align::Start)
            .build();
        let port_knock = gtk4::Switch::builder()
            .active(params.port_knock)
            .halign(Align::Start)
            .build();
        let icon_theme = gtk4::ComboBoxText::builder().build();
        let locale = gtk4::ComboBoxText::builder().build();

        let error = gtk4::Label::new(None);
        error.set_visible(false);
        error.style_context().add_class("error");

        auth_type.connect_active_notify(clone!(
            #[weak]
            auth_type,
            #[weak]
            user_name,
            #[weak]
            tunnel_type,
            #[weak]
            cert_path,
            #[weak]
            cert_type,
            move |widget| {
                if let Some(id) = widget.active_id() {
                    let factors = unsafe { auth_type.data::<Vec<String>>(&id).map(|p| p.as_ref()) };
                    if let Some(factors) = factors {
                        let is_saml = factors.iter().any(|f| f == "identity_provider");
                        let is_cert = factors.iter().any(|f| f == "certificate");
                        set_container_visible(user_name.as_ref(), !is_saml && !is_cert);
                        set_container_visible(cert_path.as_ref(), is_cert);
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
            }
        ));

        let params2 = params.clone();

        fetch_info.connect_clicked(clone!(
            #[weak]
            dialog,
            #[weak]
            auth_type,
            #[weak]
            server_name,
            #[weak]
            no_cert_check,
            #[weak]
            error,
            move |_| {
                if server_name.text().is_empty() {
                    auth_type.set_sensitive(false);
                } else {
                    dialog.set_sensitive(false);
                    let params = TunnelParams {
                        server_name: server_name.text().into(),
                        ignore_server_cert: no_cert_check.is_active(),
                        ..(*params2).clone()
                    };
                    let params2 = params2.clone();

                    let (tx, rx) = async_channel::bounded(1);
                    tokio::spawn(async move {
                        let response = server_info::get_uncached(&params).await;
                        let _ = tx.send(response).await;
                    });

                    glib::spawn_future_local(async move {
                        let response = rx.recv().await.unwrap();

                        auth_type.remove_all();

                        match response {
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
                                for (i, option) in
                                    options_list.into_values().filter(|opt| opt.show_realm != 0).enumerate()
                                {
                                    let factors = option
                                        .factors
                                        .values()
                                        .map(|factor| factor.factor_type.clone())
                                        .collect::<Vec<_>>();
                                    unsafe {
                                        auth_type.set_data(&option.id, factors);
                                    }
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
                    });
                }
            }
        ));

        // Workaround for GTK4 quirks. Without this hack, the cursor for text entries is not rendered.
        dialog.connect_show(clone!(
            #[weak]
            fetch_info,
            move |dialog| {
                dialog.add_tick_callback(move |dialog, _| {
                    dialog.add_tick_callback(clone!(
                        #[weak]
                        fetch_info,
                        #[upgrade_or]
                        glib::ControlFlow::Break,
                        move |_, _| {
                            fetch_info.emit_clicked();
                            glib::ControlFlow::Break
                        }
                    ));
                    glib::ControlFlow::Break
                });
            }
        ));

        let widgets = Rc::new(MyWidgets {
            server_name,
            fetch_info,
            auth_type,
            tunnel_type,
            user_name,
            password,
            password_factor,
            no_dns,
            search_domains,
            ignored_domains,
            dns_servers,
            ignored_dns_servers,
            set_routing_domains,
            no_routing,
            default_routing,
            add_routes,
            ignored_routes,
            no_keychain,
            no_cert_check,
            cert_type,
            cert_path,
            cert_password,
            cert_id,
            ca_cert,
            ike_lifetime,
            ike_persist,
            no_keepalive,
            port_knock,
            icon_theme,
            error,
            button_box,
            locale,
        });

        let widgets2 = widgets.clone();

        dialog.connect_response(move |dlg, response| {
            if response == ResponseType::Ok || response == ResponseType::Apply {
                if let Err(e) = widgets2.validate() {
                    glib::spawn_future_local(clone!(
                        #[weak]
                        dlg,
                        async move {
                            let msg = gtk4::MessageDialog::new(
                                Some(&dlg),
                                DialogFlags::MODAL,
                                MessageType::Error,
                                ButtonsType::Ok,
                                e.to_string(),
                            );
                            msg.set_title(Some("Validation error"));
                            msg.run_future().await;
                            msg.close();
                        },
                    ));
                    dlg.stop_signal_emission_by_name("response");
                }
            }
        });

        let mut result = Self {
            params,
            dialog,
            widgets,
            revealers: vec![],
        };

        result.create_layout();

        result
    }

    pub async fn run(&self) -> ResponseType {
        self.dialog.present();
        self.dialog.run_future().await
    }

    pub fn save(&self) -> anyhow::Result<TunnelParams> {
        let mut params = (*self.params).clone();
        params.server_name = self.widgets.server_name.text().into();
        params.login_type = self.widgets.auth_type.active_id().unwrap_or_default().into();
        params.tunnel_type = match self.widgets.tunnel_type.active().unwrap_or_default() {
            0 => TunnelType::Ipsec,
            _ => TunnelType::Ssl,
        };
        params.user_name = self.widgets.user_name.text().into();
        params.password = self.widgets.password.text().into();
        params.password_factor = self.widgets.password_factor.text().parse()?;
        params.no_dns = self.widgets.no_dns.is_active();
        params.set_routing_domains = self.widgets.set_routing_domains.is_active();
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
        params.no_keychain = self.widgets.no_keychain.is_active();
        params.ignore_server_cert = self.widgets.no_cert_check.is_active();
        params.cert_type = self.widgets.cert_type.active().unwrap_or_default().into();
        params.cert_path = {
            let text = self.widgets.cert_path.text();
            if text.is_empty() { None } else { Some(text.into()) }
        };
        params.cert_password = {
            let text = self.widgets.cert_password.text();
            if text.is_empty() { None } else { Some(text.into()) }
        };
        params.cert_id = {
            let text = self.widgets.cert_id.text();
            if text.is_empty() { None } else { Some(text.into()) }
        };
        params.ca_cert = self
            .widgets
            .ca_cert
            .text()
            .split(',')
            .map(|s| s.trim().into())
            .collect();
        params.ike_lifetime = Duration::from_secs(self.widgets.ike_lifetime.text().parse()?);
        params.ike_persist = self.widgets.ike_persist.is_active();
        params.no_keepalive = self.widgets.no_keepalive.is_active();
        params.port_knock = self.widgets.port_knock.is_active();
        params.icon_theme = self.widgets.icon_theme.active().unwrap_or_default().into();

        let active = self.widgets.locale.active();
        let new_locale = match active {
            None | Some(0) => None,
            Some(index) => i18n::get_locales().get(index as usize - 1).map(|l| l.to_string()),
        };
        params.locale = new_locale.clone();

        i18n::set_locale(new_locale.and_then(|l| l.parse().ok()));

        params.save()?;

        Ok(params)
    }

    fn form_box(&self, label: &str) -> gtk4::Box {
        let form = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .homogeneous(true)
            .spacing(6)
            .build();

        form.append(&gtk4::Label::builder().label(label).halign(Align::Start).build());
        form
    }

    fn server_box(&self) -> gtk4::Box {
        let entry_box = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(2)
            .homogeneous(false)
            .build();
        entry_box.append(&self.widgets.server_name);
        entry_box.append(&self.widgets.fetch_info);

        let server_box = self.form_box(&tr!("label-server-address"));
        server_box.append(&entry_box);
        server_box
    }

    fn auth_box(&self) -> gtk4::Box {
        let auth_box = self.form_box(&tr!("label-auth-method"));
        auth_box.append(&self.widgets.auth_type);
        auth_box
    }

    fn tunnel_box(&self) -> gtk4::Box {
        let tunnel_box = self.form_box(&tr!("label-tunnel-type"));
        self.widgets.tunnel_type.insert_text(0, &tr!("tunnel-type-ipsec"));
        self.widgets.tunnel_type.insert_text(1, &tr!("tunnel-type-ssl"));
        self.widgets
            .tunnel_type
            .set_active(if self.params.tunnel_type == TunnelType::Ipsec {
                Some(0)
            } else {
                Some(1)
            });
        tunnel_box.append(&self.widgets.tunnel_type);
        tunnel_box
    }

    fn cert_type_box(&self) -> gtk4::Box {
        let cert_type_box = self.form_box(&tr!("label-cert-auth-type"));
        self.widgets.cert_type.insert_text(0, &tr!("cert-type-none"));
        self.widgets.cert_type.insert_text(1, &tr!("cert-type-pfx"));
        self.widgets.cert_type.insert_text(2, &tr!("cert-type-pem"));
        self.widgets.cert_type.insert_text(3, &tr!("cert-type-hw"));
        self.widgets.cert_type.set_active(Some(self.params.cert_type.as_u32()));
        cert_type_box.append(&self.widgets.cert_type);
        cert_type_box
    }

    fn icon_theme_box(&self) -> gtk4::Box {
        let icon_theme_box = self.form_box(&tr!("label-icon-theme"));
        self.widgets.icon_theme.insert_text(0, &tr!("icon-theme-auto"));
        self.widgets.icon_theme.insert_text(1, &tr!("icon-theme-dark"));
        self.widgets.icon_theme.insert_text(2, &tr!("icon-theme-light"));
        self.widgets
            .icon_theme
            .set_active(Some(self.params.icon_theme.as_u32()));
        icon_theme_box.append(&self.widgets.icon_theme);
        icon_theme_box
    }

    fn locale_box(&self) -> gtk4::Box {
        let locale_box = self.form_box(&tr!("label-language"));

        self.widgets.locale.append_text(&tr!("label-system-language"));
        for locale in i18n::get_locales() {
            self.widgets
                .locale
                .append_text(&i18n::translate(&format!("language-{}", locale)));
        }

        if let Some(ref locale) = self.params.locale {
            let translated = i18n::translate(&format!("language-{}", locale));
            self.select_combo_box_item(&self.widgets.locale, &translated);
        } else {
            self.widgets.locale.set_active(Some(0));
        }
        locale_box.append(&self.widgets.locale);
        locale_box
    }

    fn user_box(&self) -> gtk4::Box {
        let user_box = self.form_box(&tr!("label-username"));
        user_box.append(&self.widgets.user_name);
        user_box
    }

    fn password_box(&self) -> gtk4::Box {
        let password_box = self.form_box(&tr!("label-password"));
        password_box.append(&self.widgets.password);
        password_box
    }

    fn dns_box(&self) -> gtk4::Box {
        let dns_box = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(12)
            .margin_end(12)
            .spacing(12)
            .build();

        let no_dns = self.form_box(&tr!("label-no-dns"));
        no_dns.append(&self.widgets.no_dns);
        dns_box.append(&no_dns);

        let dns_servers = self.form_box(&tr!("label-dns-servers"));
        dns_servers.append(&self.widgets.dns_servers);
        dns_box.append(&dns_servers);

        let ignored_dns_servers = self.form_box(&tr!("label-ignored-dns-servers"));
        ignored_dns_servers.append(&self.widgets.ignored_dns_servers);
        dns_box.append(&ignored_dns_servers);

        let search_domains = self.form_box(&tr!("label-search-domains"));
        search_domains.append(&self.widgets.search_domains);
        dns_box.append(&search_domains);

        let ignored_domains = self.form_box(&tr!("label-ignored-domains"));
        ignored_domains.append(&self.widgets.ignored_domains);
        dns_box.append(&ignored_domains);

        let set_routing_domains = self.form_box(&tr!("label-routing-domains"));
        set_routing_domains.append(&self.widgets.set_routing_domains);
        dns_box.append(&set_routing_domains);

        dns_box
    }

    fn certs_box(&self) -> gtk4::Box {
        let certs_box = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(12)
            .margin_end(12)
            .spacing(12)
            .build();

        let ca_cert = self.form_box(&tr!("label-ca-cert"));
        ca_cert.append(&self.widgets.ca_cert);
        certs_box.append(&ca_cert);

        let no_cert_check = self.form_box(&tr!("label-no-cert-check"));
        no_cert_check.append(&self.widgets.no_cert_check);
        certs_box.append(&no_cert_check);

        certs_box
    }

    fn misc_box(&self) -> gtk4::Box {
        let misc_box = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(12)
            .margin_end(12)
            .spacing(12)
            .build();

        let password_factor = self.form_box(&tr!("label-password-factor"));
        password_factor.append(&self.widgets.password_factor);
        misc_box.append(&password_factor);

        let no_keychain = self.form_box(&tr!("label-no-keychain"));
        no_keychain.append(&self.widgets.no_keychain);
        misc_box.append(&no_keychain);

        let ike_lifetime = self.form_box(&tr!("label-ike-lifetime"));
        ike_lifetime.append(&self.widgets.ike_lifetime);
        misc_box.append(&ike_lifetime);

        let ike_persist = self.form_box(&tr!("label-ike-persist"));
        ike_persist.append(&self.widgets.ike_persist);
        misc_box.append(&ike_persist);

        let no_keepalive = self.form_box(&tr!("label-no-keepalive"));
        no_keepalive.append(&self.widgets.no_keepalive);
        misc_box.append(&no_keepalive);

        let port_knock = self.form_box(&tr!("label-port-knock"));
        port_knock.append(&self.widgets.port_knock);
        misc_box.append(&port_knock);

        let icon_theme_box = self.icon_theme_box();
        misc_box.append(&icon_theme_box);

        let locale_box = self.locale_box();
        misc_box.append(&locale_box);

        misc_box
    }

    fn routing_box(&self) -> gtk4::Box {
        let routing_box = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .margin_top(6)
            .margin_bottom(6)
            .margin_bottom(6)
            .margin_start(12)
            .margin_end(12)
            .spacing(6)
            .build();

        let no_routing = self.form_box(&tr!("label-no-routing"));
        no_routing.append(&self.widgets.no_routing);
        routing_box.append(&no_routing);

        let default_routing = self.form_box(&tr!("label-default-routing"));
        default_routing.append(&self.widgets.default_routing);
        routing_box.append(&default_routing);

        let add_routes = self.form_box(&tr!("label-add-routes"));
        add_routes.append(&self.widgets.add_routes);
        routing_box.append(&add_routes);

        let ignored_routes = self.form_box(&tr!("label-ignored-routes"));
        ignored_routes.append(&self.widgets.ignored_routes);
        routing_box.append(&ignored_routes);

        routing_box
    }

    fn user_auth_box(&self) -> gtk4::Box {
        let user_auth_box = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .margin_top(0)
            .margin_bottom(0)
            .margin_start(0)
            .margin_end(0)
            .spacing(6)
            .visible(false)
            .build();
        user_auth_box.append(&self.user_box());
        user_auth_box.append(&self.password_box());

        user_auth_box
    }

    fn cert_auth_box(&self) -> gtk4::Box {
        let certs_box = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .margin_top(0)
            .margin_bottom(0)
            .margin_start(0)
            .margin_end(0)
            .spacing(6)
            .visible(false)
            .build();

        let cert_type_box = self.cert_type_box();
        certs_box.append(&cert_type_box);

        let cert_path = self.form_box(&tr!("label-client-cert"));
        cert_path.append(&self.widgets.cert_path);
        certs_box.append(&cert_path);

        let cert_password = self.form_box(&tr!("label-cert-password"));
        cert_password.append(&self.widgets.cert_password);
        certs_box.append(&cert_password);

        let cert_id = self.form_box(&tr!("label-cert-id"));
        cert_id.append(&self.widgets.cert_id);
        certs_box.append(&cert_id);

        certs_box
    }

    fn general_tab(&self) -> gtk4::Box {
        let tab = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(6)
            .margin_end(6)
            .spacing(12)
            .build();
        tab.append(&self.server_box());
        tab.append(&self.auth_box());
        tab.append(&self.tunnel_box());
        tab.append(&self.user_auth_box());
        tab.append(&self.cert_auth_box());
        tab
    }

    fn add_expander(&mut self, label: &str, parent: &gtk4::Box, child: &impl IsA<Widget>) {
        let arrow = gtk4::Image::from_icon_name("go-next-symbolic");
        arrow.add_css_class("arrow-icon");

        let b = gtk4::Box::builder().build();
        b.append(&arrow);
        b.append(
            &gtk4::Label::builder()
                .label(label)
                .hexpand(true)
                .halign(Align::Center)
                .build(),
        );

        let toggle_button = gtk4::ToggleButton::new();
        toggle_button.set_child(Some(&b));

        let revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::SlideDown)
            .reveal_child(true)
            .build();

        self.revealers.push(revealer.clone());

        // Toggle handler
        toggle_button.connect_toggled(clone!(
            #[weak]
            revealer,
            #[weak]
            arrow,
            move |btn| {
                let active = btn.is_active();
                revealer.set_reveal_child(active);

                if active {
                    arrow.add_css_class("rotate-90");
                } else {
                    arrow.remove_css_class("rotate-90");
                }
            }
        ));

        parent.append(&toggle_button);
        parent.append(&revealer);

        revealer.set_child(Some(child));
    }

    fn advanced_tab(&mut self) -> gtk4::ScrolledWindow {
        let inner = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(6)
            .margin_end(6)
            .spacing(3)
            .build();

        self.add_expander(&tr!("expand-dns"), &inner, &self.dns_box());
        self.add_expander(&tr!("expand-routing"), &inner, &self.routing_box());
        self.add_expander(&tr!("expand-certificates"), &inner, &self.certs_box());
        self.add_expander(&tr!("expand-misc"), &inner, &self.misc_box());

        let viewport = gtk4::Viewport::builder().build();
        viewport.set_child(Some(&inner));

        let scrolled_win = gtk4::ScrolledWindow::builder().build();
        scrolled_win.set_child(Some(&viewport));
        scrolled_win
    }

    fn create_layout(&mut self) {
        let content_area = self.dialog.content_area();
        content_area.set_margin_top(6);
        content_area.set_margin_start(6);
        content_area.set_margin_end(6);

        let notebook = gtk4::Notebook::new();
        notebook.set_vexpand(true);
        content_area.append(&notebook);
        content_area.append(&self.widgets.error);
        content_area.append(&self.widgets.button_box);

        notebook.append_page(&self.general_tab(), Some(&gtk4::Label::new(Some(&tr!("tab-general")))));
        notebook.append_page(
            &self.advanced_tab(),
            Some(&gtk4::Label::new(Some(&tr!("tab-advanced")))),
        );

        // self.dialog
        //     .set_default_size(SettingsDialog::DEFAULT_WIDTH, SettingsDialog::DEFAULT_HEIGHT);
        self.resize_dialog_to_fit_revealers();
    }

    fn resize_dialog_to_fit_revealers(&self) {
        let mut max_width = 0;

        // Iterate through all revealers
        for revealer in &self.revealers {
            if let Some(child) = revealer.child() {
                child.queue_resize();
                let requisition = child.preferred_size();
                let width = requisition.1.width(); // Natural width
                max_width = max_width.max(width);
            }
            revealer.set_reveal_child(false);
        }

        max_width += 50;

        let current_size = self.dialog.default_size();
        let height = current_size.1.max(400);

        // Set the dialog's default size
        self.dialog.set_default_size(max_width, height);
    }

    fn select_combo_box_item(&self, combo_box: &gtk4::ComboBoxText, target_text: &str) {
        if let Some(model) = combo_box.model() {
            let mut index = 0;
            let mut found = false;
            model.foreach(|model, _path, iter| {
                if let Ok(text) = model.get_value(iter, 0).get::<String>() {
                    if text == target_text {
                        combo_box.set_active(Some(index));
                        found = true;
                        return true;
                    }
                }
                index += 1;
                false
            });

            if !found {
                combo_box.set_active(Some(0));
            }
        }
    }
}

impl Drop for SettingsDialog {
    fn drop(&mut self) {
        self.dialog.close();
    }
}

pub fn start_settings_dialog<W: IsA<Window>>(parent: W, sender: Sender<TrayCommand>, params: Arc<TunnelParams>) {
    let dialog = SettingsDialog::new(parent, params.clone());
    let sender = sender.clone();
    glib::spawn_future_local(async move {
        loop {
            let response = dialog.run().await;

            match response {
                ResponseType::Ok | ResponseType::Apply => {
                    if let Err(e) = dialog.save() {
                        warn!("{}", e);
                    } else {
                        let _ = sender.send(TrayCommand::Update(None)).await;
                    }
                }
                _ => {}
            }
            if response != ResponseType::Apply {
                break;
            }
        }
    });
}
