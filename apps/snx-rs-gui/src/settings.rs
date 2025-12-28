use std::{net::Ipv4Addr, path::Path, rc::Rc, sync::Arc, time::Duration};

use gtk4::{
    Align, ButtonsType, Dialog, DialogFlags, MessageType, Orientation, ResponseType, Widget, Window,
    glib::{self, clone},
    prelude::*,
};
use itertools::Itertools;
use snxcore::{
    model::{
        params::{CertType, DEFAULT_PROFILE_UUID, IconTheme, TransportType, TunnelParams, TunnelType},
        proto::LoginOption,
    },
    server_info,
    util::parse_ipv4_or_subnet,
};
use tokio::sync::mpsc::Sender;
use tracing::warn;
use uuid::Uuid;

fn set_container_visible(widget: &Widget, flag: bool) {
    if let Some(parent) = widget.parent()
        && let Some(parent) = parent.parent()
    {
        if flag {
            parent.show();
        } else {
            parent.hide();
        }
    }
}

use crate::{get_window, profiles::ConnectionProfilesStore, set_window, tr, tray::TrayCommand};

struct SettingsDialog {
    dialog: Dialog,
    widgets: Rc<MyWidgets>,
    revealers: Vec<gtk4::Revealer>,
}

struct MyWidgets {
    server_name: gtk4::Entry,
    fetch_info: gtk4::Button,
    auth_type: gtk4::ComboBoxText,
    tunnel_type: gtk4::ComboBoxText,
    username: gtk4::Entry,
    password: gtk4::PasswordEntry,
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
    cert_password: gtk4::PasswordEntry,
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
    auto_connect: gtk4::Switch,
    ip_lease_time: gtk4::Entry,
    disable_ipv6: gtk4::Switch,
    mtu: gtk4::Entry,
    transport_type: gtk4::ComboBoxText,
    profile_select: gtk4::ComboBoxText,
    profile_new: gtk4::Button,
    profile_rename: gtk4::Button,
    profile_delete: gtk4::Button,
}

impl MyWidgets {
    fn validate(&self) -> anyhow::Result<()> {
        if self.server_name.text().is_empty() {
            anyhow::bail!(tr!("error-no-server-name"));
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
                parse_ipv4_or_subnet(r)?;
            }
        }

        let ignored_routes = self.ignored_routes.text();
        if !ignored_routes.is_empty() {
            for r in ignored_routes.split(',') {
                parse_ipv4_or_subnet(r)?;
            }
        }

        let ip_lease_time = self.ip_lease_time.text();
        if !ip_lease_time.trim().is_empty() {
            ip_lease_time.parse::<u32>()?;
        }

        self.mtu.text().parse::<u16>()?;

        Ok(())
    }

    fn update_from_params(&self, params: &TunnelParams) {
        self.server_name.set_text(&params.server_name);

        self.auth_type.set_active_id(Some(&params.login_type));
        self.tunnel_type.set_active_id(Some(params.tunnel_type.as_str()));
        self.username.set_text(&params.user_name);
        self.password.set_text(&params.password);
        self.password_factor.set_text(&params.password_factor.to_string());
        self.no_dns.set_active(params.no_dns);
        self.search_domains.set_text(&params.search_domains.join(","));
        self.ignored_domains.set_text(&params.ignore_search_domains.join(","));
        self.dns_servers
            .set_text(&params.dns_servers.iter().map(|ip| ip.to_string()).join(","));
        self.ignored_dns_servers
            .set_text(&params.ignore_dns_servers.iter().map(|ip| ip.to_string()).join(","));
        self.set_routing_domains.set_active(params.set_routing_domains);
        self.no_routing.set_active(params.no_routing);
        self.default_routing.set_active(params.default_route);
        self.add_routes
            .set_text(&params.add_routes.iter().map(|ip| ip.to_string()).join(","));
        self.ignored_routes
            .set_text(&params.ignore_routes.iter().map(|ip| ip.to_string()).join(","));
        self.no_keychain.set_active(params.no_keychain);
        self.no_cert_check.set_active(params.ignore_server_cert);
        self.cert_type.set_active_id(Some(&params.cert_type.to_string()));
        self.cert_path.set_text(
            &params
                .cert_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
        );
        self.cert_password
            .set_text(params.cert_password.as_deref().unwrap_or_default());
        self.cert_id.set_text(params.cert_id.as_deref().unwrap_or_default());
        self.ca_cert
            .set_text(&params.ca_cert.iter().map(|path| path.display().to_string()).join(","));
        self.ike_lifetime.set_text(&params.ike_lifetime.as_secs().to_string());
        self.ike_persist.set_active(params.ike_persist);
        self.no_keepalive.set_active(params.no_keepalive);
        self.port_knock.set_active(params.port_knock);
        self.ip_lease_time.set_text(
            &params
                .ip_lease_time
                .map(|v| v.as_secs().to_string())
                .unwrap_or_default(),
        );
        self.disable_ipv6.set_active(params.disable_ipv6);
        self.mtu.set_text(&params.mtu.to_string());
        self.transport_type
            .set_active_id(Some(&params.transport_type.to_string()));
        self.fetch_info.activate();
    }

    fn on_auth_type_changed(&self) {
        if let Some(id) = self.auth_type.active_id() {
            let factors = unsafe { self.auth_type.data::<Vec<String>>(&id).map(|p| p.as_ref()) };
            if let Some(factors) = factors {
                let is_saml = factors.iter().any(|f| f == "identity_provider");
                let is_cert = factors.iter().any(|f| f == "certificate");
                let is_mobile_access = factors.iter().any(|f| f == "mobile_access");
                set_container_visible(self.username.as_ref(), !is_saml && !is_cert && !is_mobile_access);
                set_container_visible(self.cert_path.as_ref(), is_cert);
                if !is_cert {
                    self.cert_type.set_active(Some(0));
                }
                self.tunnel_type.set_sensitive(!is_mobile_access);
                self.tunnel_type.set_active(Some(is_mobile_access as _));
            }
        }
    }

    fn on_profile_changed(&self) {
        if let Some(id) = self.profile_select.active_id()
            && let Ok(uuid) = id.parse::<Uuid>()
            && let Some(params) = ConnectionProfilesStore::instance().get(uuid)
        {
            self.update_from_params(&params);
            self.profile_delete.set_sensitive(uuid != DEFAULT_PROFILE_UUID);
        }
    }

    async fn on_profile_delete(&self, parent: &Dialog) {
        let msg = gtk4::MessageDialog::new(
            Some(parent),
            DialogFlags::MODAL,
            MessageType::Question,
            ButtonsType::YesNo,
            tr!("profile-delete-prompt"),
        );
        msg.set_title(Some(&tr!("label-confirmation")));
        if msg.run_future().await == ResponseType::Yes
            && let Some(id) = self.profile_select.active_id()
            && let Ok(uuid) = id.parse::<Uuid>()
        {
            ConnectionProfilesStore::instance().remove(uuid);
            self.profile_select.remove(self.profile_select.active().unwrap() as _);
            self.profile_select.set_active(Some(0));
        }
        msg.close();
    }

    async fn on_profile_new(&self, parent: &Dialog) {
        let name = show_entry_dialog(parent, &tr!("profile-new-title"), &tr!("label-profile-name"), "").await;
        if let Some(name) = name {
            let profile_id = Uuid::new_v4();
            let params = Arc::new(TunnelParams {
                profile_name: name.clone(),
                profile_id,
                config_file: TunnelParams::default_config_dir().join(format!("{}.conf", profile_id)),
                ..Default::default()
            });
            ConnectionProfilesStore::instance().save(params.clone());
            self.profile_select.append(Some(&profile_id.to_string()), &name);
            self.profile_select.set_active_id(Some(&profile_id.to_string()));
            self.update_from_params(&params);
        }
    }

    async fn on_profile_rename(&self, parent: &Dialog) {
        let name = show_entry_dialog(
            parent,
            &tr!("profile-rename-title"),
            &tr!("label-profile-name"),
            &self.profile_select.active_text().unwrap_or_default(),
        )
        .await;
        if let Some(name) = name
            && let Some(id) = self.profile_select.active_id()
            && let Some(active) = self.profile_select.active()
            && let Ok(uuid) = id.parse::<Uuid>()
        {
            self.profile_select.remove(active as _);
            self.profile_select.insert(active as _, Some(&id), &name);
            self.profile_select.set_active_id(Some(&id));

            if let Some(profile) = ConnectionProfilesStore::instance().get(uuid) {
                let new_profile = Arc::new(TunnelParams {
                    profile_name: name,
                    ..(*profile).clone()
                });
                ConnectionProfilesStore::instance().save(new_profile);
            }
        }
    }

    fn on_fetch_info(self: Rc<MyWidgets>, dialog: Dialog) {
        let params = if let Some(id) = self.profile_select.active_id()
            && let Ok(uuid) = id.parse::<Uuid>()
            && let Some(params) = ConnectionProfilesStore::instance().get(uuid)
        {
            params
        } else {
            return;
        };

        let login_type = params.login_type.clone();

        if self.server_name.text().is_empty() {
            self.auth_type.remove_all();
            self.auth_type.set_sensitive(false);
        } else {
            dialog.set_sensitive(false);
            let new_params = TunnelParams {
                server_name: self.server_name.text().into(),
                ignore_server_cert: self.no_cert_check.is_active(),
                ..(*params).clone()
            };

            let (tx, rx) = async_channel::bounded(1);
            tokio::spawn(async move {
                let response = server_info::get(&new_params).await;
                let _ = tx.send(response).await;
            });

            glib::spawn_future_local(async move {
                let response = rx.recv().await.unwrap();

                self.auth_type.remove_all();

                match response {
                    Ok(server_info) => {
                        self.error.set_label("");
                        self.error.set_visible(false);
                        let mut options_list = server_info
                            .login_options_data
                            .map(|d| d.login_options_list.into_values().collect::<Vec<_>>())
                            .unwrap_or_default();
                        if options_list.is_empty() {
                            options_list.push(LoginOption::unspecified());
                        }
                        #[cfg(feature = "mobile-access")]
                        options_list.push(LoginOption::mobile_access());
                        for (i, option) in options_list.into_iter().filter(|opt| opt.show_realm != 0).enumerate() {
                            let factors = option
                                .factors
                                .values()
                                .map(|factor| factor.factor_type.clone())
                                .collect::<Vec<_>>();
                            unsafe {
                                self.auth_type.set_data(&option.id, factors);
                            }
                            self.auth_type.append(Some(&option.id), &option.display_name);
                            if login_type == option.id {
                                self.auth_type.set_active(Some(i as _));
                            }
                        }
                        self.auth_type.set_sensitive(true);
                        if self.auth_type.active().is_none() {
                            self.auth_type.set_active(Some(0));
                        }
                    }

                    Err(e) => {
                        self.error.set_label(&e.to_string());
                        self.error.set_visible(true);
                    }
                }
                dialog.set_sensitive(true);
            });
        }
    }
}

impl SettingsDialog {
    pub fn new<W: IsA<Window>>(parent: W, profile_id: Uuid) -> Self {
        let dialog = Dialog::builder()
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
        let apply_button = gtk4::Button::with_label(&tr!("button-apply"));
        let cancel_button = gtk4::Button::with_label(&tr!("button-cancel"));
        button_box.append(&ok_button);
        button_box.append(&apply_button);
        button_box.append(&cancel_button);

        let server_name = gtk4::Entry::builder().hexpand(true).build();

        let fetch_info = gtk4::Button::builder()
            .label(tr!("button-fetch-info"))
            .halign(Align::End)
            .build();
        let auth_type = gtk4::ComboBoxText::builder().build();
        let tunnel_type = gtk4::ComboBoxText::builder().build();
        let username = gtk4::Entry::builder()
            .placeholder_text(std::env::var("USER").unwrap_or_default())
            .build();
        let password = gtk4::PasswordEntry::builder().show_peek_icon(true).build();
        let password_factor = gtk4::Entry::builder().build();

        let no_dns = gtk4::Switch::builder().halign(Align::Start).build();
        let set_routing_domains = gtk4::Switch::builder().halign(Align::Start).build();

        let search_domains = gtk4::Entry::builder()
            .placeholder_text(tr!("placeholder-domains"))
            .build();

        let ignored_domains = gtk4::Entry::builder()
            .placeholder_text(tr!("placeholder-domains"))
            .build();

        let dns_servers = gtk4::Entry::builder()
            .placeholder_text(tr!("placeholder-ip-addresses"))
            .build();

        let ignored_dns_servers = gtk4::Entry::builder()
            .placeholder_text(tr!("placeholder-ip-addresses"))
            .build();

        let no_routing = gtk4::Switch::builder().halign(Align::Start).build();
        let default_routing = gtk4::Switch::builder().halign(Align::Start).build();

        let add_routes = gtk4::Entry::builder()
            .placeholder_text(tr!("placeholder-routes"))
            .build();

        let ignored_routes = gtk4::Entry::builder()
            .placeholder_text(tr!("placeholder-routes"))
            .build();

        let no_keychain = gtk4::Switch::builder().halign(Align::Start).build();
        let no_cert_check = gtk4::Switch::builder().halign(Align::Start).build();
        let cert_type = gtk4::ComboBoxText::builder().build();
        let cert_path = gtk4::Entry::builder().build();
        let cert_password = gtk4::PasswordEntry::builder().show_peek_icon(true).build();
        let cert_id = gtk4::Entry::builder().build();
        let ca_cert = gtk4::Entry::builder()
            .placeholder_text(tr!("placeholder-certs"))
            .build();
        let ike_lifetime = gtk4::Entry::builder().build();
        let ike_persist = gtk4::Switch::builder().halign(Align::Start).build();
        let no_keepalive = gtk4::Switch::builder().halign(Align::Start).build();
        let port_knock = gtk4::Switch::builder().halign(Align::Start).build();
        let icon_theme = gtk4::ComboBoxText::builder().build();
        let locale = gtk4::ComboBoxText::builder().build();
        let auto_connect = gtk4::Switch::builder().halign(Align::Start).build();
        let ip_lease_time = gtk4::Entry::builder().build();
        let disable_ipv6 = gtk4::Switch::builder().halign(Align::Start).build();

        let mtu = gtk4::Entry::builder().build();
        let transport_type = gtk4::ComboBoxText::builder().build();

        let profile_select = gtk4::ComboBoxText::builder().build();
        let profile_new = gtk4::Button::with_label(&tr!("profile-new"));
        let profile_rename = gtk4::Button::with_label(&tr!("profile-rename"));
        let profile_delete = gtk4::Button::with_label(&tr!("profile-delete"));

        let error = gtk4::Label::new(None);
        error.set_visible(false);
        error.style_context().add_class("error");

        let widgets = Rc::new(MyWidgets {
            server_name,
            fetch_info,
            auth_type,
            tunnel_type,
            username,
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
            auto_connect,
            ip_lease_time,
            disable_ipv6,
            mtu,
            transport_type,
            profile_select,
            profile_new,
            profile_rename,
            profile_delete,
        });

        apply_button.connect_clicked(clone!(
            #[weak]
            dialog,
            move |_| dialog.response(ResponseType::Apply)
        ));
        ok_button.connect_clicked(clone!(
            #[weak]
            dialog,
            move |_| dialog.response(ResponseType::Ok)
        ));

        cancel_button.connect_clicked(clone!(
            #[weak]
            dialog,
            move |_| dialog.response(ResponseType::Cancel)
        ));

        widgets.auth_type.connect_active_notify(clone!(
            #[weak]
            widgets,
            move |_| widgets.on_auth_type_changed()
        ));

        widgets.profile_select.connect_active_notify(clone!(
            #[weak]
            widgets,
            move |_| widgets.on_profile_changed()
        ));

        widgets.profile_delete.connect_clicked(clone!(
            #[weak]
            widgets,
            #[weak]
            dialog,
            move |_| {
                glib::spawn_future_local(clone!(
                    #[weak]
                    widgets,
                    async move { widgets.on_profile_delete(&dialog).await }
                ));
            }
        ));

        widgets.profile_new.connect_clicked(clone!(
            #[weak]
            widgets,
            #[weak]
            dialog,
            move |_| {
                glib::spawn_future_local(clone!(
                    #[weak]
                    widgets,
                    async move { widgets.on_profile_new(&dialog).await }
                ));
            }
        ));

        widgets.profile_rename.connect_clicked(clone!(
            #[weak]
            widgets,
            #[weak]
            dialog,
            move |_| {
                glib::spawn_future_local(clone!(
                    #[weak]
                    widgets,
                    async move { widgets.on_profile_rename(&dialog).await }
                ));
            }
        ));

        widgets.fetch_info.connect_clicked(clone!(
            #[weak]
            dialog,
            #[weak]
            widgets,
            move |_| widgets.on_fetch_info(dialog.clone())
        ));

        dialog.connect_response(clone!(
            #[weak]
            widgets,
            move |dialog, response| {
                if (response == ResponseType::Ok || response == ResponseType::Apply)
                    && let Err(e) = widgets.validate()
                {
                    glib::spawn_future_local(clone!(
                        #[weak]
                        dialog,
                        async move {
                            let msg = gtk4::MessageDialog::new(
                                Some(&dialog),
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
                }
            }
        ));

        dialog.connect_show(clone!(
            #[weak]
            widgets,
            move |_| {
                widgets.fetch_info.activate();
            }
        ));

        let mut result = Self {
            dialog,
            widgets,
            revealers: vec![],
        };

        result.create_layout();

        result
            .widgets
            .profile_select
            .set_active_id(Some(&profile_id.to_string()));

        result
    }

    pub async fn run(&self) -> ResponseType {
        set_window("settings", Some(self.dialog.clone()));
        self.dialog.present();
        let result = self.dialog.run_future().await;
        set_window("settings", None::<Dialog>);
        result
    }

    pub fn save(&mut self) -> anyhow::Result<()> {
        let mut params = if let Some(id) = self.widgets.profile_select.active_id()
            && let Ok(uuid) = id.parse::<Uuid>()
            && let Some(params) = ConnectionProfilesStore::instance().get(uuid)
        {
            (*params).clone()
        } else {
            anyhow::bail!("No profile selected");
        };

        params.server_name = self.widgets.server_name.text().into();
        params.login_type = self.widgets.auth_type.active_id().unwrap_or_default().into();
        params.tunnel_type = match self.widgets.tunnel_type.active().unwrap_or_default() {
            0 => TunnelType::Ipsec,
            _ => TunnelType::Ssl,
        };
        params.user_name = self.widgets.username.text().into();
        params.password = self.widgets.password.text().into();
        params.password_factor = self.widgets.password_factor.text().parse()?;
        params.no_dns = self.widgets.no_dns.is_active();
        params.set_routing_domains = self.widgets.set_routing_domains.is_active();
        params.search_domains = self
            .widgets
            .search_domains
            .text()
            .split(',')
            .map(|s| s.trim())
            .filter_map(|s| if s.is_empty() { None } else { Some(s.to_owned()) })
            .collect();
        params.ignore_search_domains = self
            .widgets
            .ignored_domains
            .text()
            .split(',')
            .map(|s| s.trim().to_owned())
            .filter_map(|s| if s.is_empty() { None } else { Some(s.to_owned()) })
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
            .flat_map(|s| parse_ipv4_or_subnet(s).ok())
            .collect();
        params.ignore_routes = self
            .widgets
            .ignored_routes
            .text()
            .split(',')
            .flat_map(|s| parse_ipv4_or_subnet(s).ok())
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
            .map(|s| s.trim())
            .filter_map(|s| if s.is_empty() { None } else { Some(s.into()) })
            .collect();
        params.ike_lifetime = Duration::from_secs(self.widgets.ike_lifetime.text().parse()?);
        params.ike_persist = self.widgets.ike_persist.is_active();
        params.no_keepalive = self.widgets.no_keepalive.is_active();
        params.port_knock = self.widgets.port_knock.is_active();
        params.transport_type = self.widgets.transport_type.active().unwrap_or_default().into();

        params.disable_ipv6 = self.widgets.disable_ipv6.is_active();

        let ip_lease_time = self.widgets.ip_lease_time.text();
        params.ip_lease_time = if ip_lease_time.trim().is_empty() {
            None
        } else {
            Some(Duration::from_secs(ip_lease_time.parse()?))
        };

        params.mtu = self.widgets.mtu.text().parse()?;

        params.icon_theme = self.widgets.icon_theme.active().unwrap_or_default().into();
        let selected_locale = self.widgets.locale.active();
        let new_locale = match selected_locale {
            None | Some(0) => None,
            Some(index) => i18n::get_locales().get(index as usize - 1).map(|l| l.to_string()),
        };
        params.locale = new_locale.clone();
        params.auto_connect = self.widgets.auto_connect.is_active();

        if params.profile_id != DEFAULT_PROFILE_UUID {
            let mut default_params = (*ConnectionProfilesStore::instance().get_default()).clone();
            default_params.icon_theme = params.icon_theme;
            default_params.locale = params.locale.clone();
            default_params.auto_connect = params.auto_connect;
            ConnectionProfilesStore::instance().save(Arc::new(default_params));
        }

        ConnectionProfilesStore::instance().save(Arc::new(params));

        i18n::set_locale(new_locale.and_then(|l| l.parse().ok()));

        Ok(())
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
        self.widgets
            .tunnel_type
            .append(Some(TunnelType::Ipsec.as_str()), &tr!("tunnel-type-ipsec"));
        self.widgets
            .tunnel_type
            .append(Some(TunnelType::Ssl.as_str()), &tr!("tunnel-type-ssl"));
        tunnel_box.append(&self.widgets.tunnel_type);
        tunnel_box
    }

    fn cert_type_box(&self) -> gtk4::Box {
        let cert_type_box = self.form_box(&tr!("label-cert-auth-type"));
        self.widgets
            .cert_type
            .append(Some(&CertType::None.to_string()), &tr!("cert-type-none"));
        self.widgets
            .cert_type
            .append(Some(&CertType::Pkcs12.to_string()), &tr!("cert-type-pfx"));
        self.widgets
            .cert_type
            .append(Some(&CertType::Pkcs8.to_string()), &tr!("cert-type-pem"));
        self.widgets
            .cert_type
            .append(Some(&CertType::Pkcs11.to_string()), &tr!("cert-type-hw"));
        cert_type_box.append(&self.widgets.cert_type);
        cert_type_box
    }

    fn icon_theme_box(&self) -> gtk4::Box {
        let icon_theme_box = self.form_box(&tr!("label-icon-theme"));
        self.widgets
            .icon_theme
            .append(Some(&IconTheme::AutoDetect.to_string()), &tr!("icon-theme-autodetect"));
        self.widgets
            .icon_theme
            .append(Some(&IconTheme::Dark.to_string()), &tr!("icon-theme-dark"));
        self.widgets
            .icon_theme
            .append(Some(&IconTheme::Light.to_string()), &tr!("icon-theme-light"));
        self.widgets.icon_theme.set_active(Some(
            ConnectionProfilesStore::instance().get_default().icon_theme.as_u32(),
        ));
        icon_theme_box.append(&self.widgets.icon_theme);
        icon_theme_box
    }

    fn transport_type_box(&self) -> gtk4::Box {
        let transport_type_box = self.form_box(&tr!("info-transport-type"));
        self.widgets.transport_type.append(
            Some(&TransportType::AutoDetect.to_string()),
            &tr!("transport-type-autodetect"),
        );
        self.widgets
            .transport_type
            .append(Some(&TransportType::Kernel.to_string()), &tr!("transport-type-kernel"));
        self.widgets
            .transport_type
            .append(Some(&TransportType::Udp.to_string()), &tr!("transport-type-udp"));
        self.widgets
            .transport_type
            .append(Some(&TransportType::Tcpt.to_string()), &tr!("transport-type-tcpt"));
        transport_type_box.append(&self.widgets.transport_type);
        transport_type_box
    }

    fn locale_box(&self) -> gtk4::Box {
        let locale_box = self.form_box(&tr!("label-language"));

        self.widgets.locale.append_text(&tr!("label-system-language"));
        for locale in i18n::get_locales() {
            self.widgets.locale.append(
                Some(&locale.to_string()),
                &i18n::translate(&format!("language-{locale}")),
            );
        }

        if let Some(ref locale) = ConnectionProfilesStore::instance().get_default().locale {
            let translated = i18n::translate(&format!("language-{locale}"));
            self.select_combo_box_item(&self.widgets.locale, &translated);
        } else {
            self.widgets.locale.set_active(Some(0));
        }
        locale_box.append(&self.widgets.locale);
        locale_box
    }

    fn user_box(&self) -> gtk4::Box {
        let user_box = self.form_box(&tr!("label-username"));
        user_box.append(&self.widgets.username);
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

        let ip_lease_time = self.form_box(&tr!("label-ip-lease-time"));
        ip_lease_time.append(&self.widgets.ip_lease_time);
        misc_box.append(&ip_lease_time);

        let mtu = self.form_box(&tr!("label-mtu"));
        mtu.append(&self.widgets.mtu);
        misc_box.append(&mtu);

        let transport_type_box = self.transport_type_box();
        misc_box.append(&transport_type_box);

        misc_box
    }

    fn ui_box(&self) -> gtk4::Box {
        let ui_box = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(12)
            .margin_end(12)
            .spacing(12)
            .build();

        let icon_theme_box = self.icon_theme_box();
        ui_box.append(&icon_theme_box);

        let locale_box = self.locale_box();
        ui_box.append(&locale_box);

        let auto_connect = self.form_box(&tr!("label-auto-connect"));
        auto_connect.append(&self.widgets.auto_connect);
        self.widgets
            .auto_connect
            .set_active(ConnectionProfilesStore::instance().get_default().auto_connect);
        ui_box.append(&auto_connect);

        ui_box
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

        let disable_ipv6 = self.form_box(&tr!("label-disable-ipv6"));
        disable_ipv6.append(&self.widgets.disable_ipv6);
        routing_box.append(&disable_ipv6);

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
        tab.append(&self.profile_box());
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
        self.add_expander(&tr!("expand-ui"), &inner, &self.ui_box());

        let viewport = gtk4::Viewport::builder().build();
        viewport.set_child(Some(&inner));

        let scrolled_win = gtk4::ScrolledWindow::builder().build();
        scrolled_win.set_child(Some(&viewport));
        scrolled_win
    }

    fn profile_box(&self) -> gtk4::Box {
        let profile_box = self.form_box(&tr!("label-connection-profile"));

        for profile in ConnectionProfilesStore::instance().all() {
            self.widgets
                .profile_select
                .append(Some(&profile.profile_id.to_string()), &profile.profile_name);
        }

        self.widgets.profile_select.set_hexpand(true);
        let button_box = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(2)
            .homogeneous(false)
            .build();
        button_box.append(&self.widgets.profile_select);
        button_box.append(&self.widgets.profile_new);
        button_box.append(&self.widgets.profile_rename);
        button_box.append(&self.widgets.profile_delete);

        profile_box.append(&button_box);
        profile_box
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
                if let Ok(text) = model.get_value(iter, 0).get::<String>()
                    && text == target_text
                {
                    combo_box.set_active(Some(index));
                    found = true;
                    return true;
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

pub fn start_settings_dialog<W: IsA<Window>>(parent: W, sender: Sender<TrayCommand>, profile_id: Uuid) {
    if let Some(window) = get_window("settings") {
        window.present();
        return;
    }

    let mut dialog = SettingsDialog::new(parent, profile_id);
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

async fn show_entry_dialog(parent: &Dialog, title: &str, label: &str, value: &str) -> Option<String> {
    let dialog = Dialog::builder().title(title).transient_for(parent).modal(true).build();

    let ok = gtk4::Button::builder().label(tr!("button-ok")).build();
    ok.connect_clicked(clone!(
        #[weak]
        dialog,
        move |_| {
            dialog.response(ResponseType::Ok);
        }
    ));

    ok.set_sensitive(!value.trim().is_empty());

    let cancel = gtk4::Button::builder().label(tr!("button-cancel")).build();
    cancel.connect_clicked(clone!(
        #[weak]
        dialog,
        move |_| {
            dialog.response(ResponseType::Cancel);
        }
    ));

    let button_box = gtk4::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .margin_top(6)
        .margin_start(6)
        .margin_end(6)
        .homogeneous(true)
        .halign(Align::End)
        .valign(Align::End)
        .build();

    button_box.append(&ok);
    button_box.append(&cancel);

    dialog.set_default_response(ResponseType::Ok);

    let content = dialog.content_area();
    let inner = gtk4::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_bottom(6)
        .margin_top(6)
        .margin_start(6)
        .margin_end(6)
        .spacing(6)
        .build();

    inner.append(&gtk4::Label::builder().label(label).halign(Align::Start).build());

    let entry = gtk4::Entry::builder()
        .name("entry")
        .activates_default(true)
        .text(value)
        .build();

    entry.connect_changed(clone!(
        #[weak]
        ok,
        move |entry| {
            ok.set_sensitive(!entry.text().trim().is_empty());
        }
    ));

    entry.connect_activate(clone!(
        #[weak]
        dialog,
        #[weak]
        entry,
        move |_| {
            if !entry.text().trim().is_empty() {
                dialog.response(ResponseType::Ok);
            }
        }
    ));

    inner.append(&entry);

    content.append(&inner);
    content.append(&button_box);

    dialog.show();
    let current_size = dialog.default_size();
    let new_width = current_size.0.max(400);
    dialog.set_default_size(new_width, current_size.1);

    let response = dialog.run_future().await;
    dialog.close();

    if response == ResponseType::Ok {
        Some(entry.text().into())
    } else {
        None
    }
}
