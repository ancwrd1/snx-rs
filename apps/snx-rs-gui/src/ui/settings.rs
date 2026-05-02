use std::{cell::RefCell, net::Ipv4Addr, path::Path, rc::Rc, sync::Arc, time::Duration};

use i18n::fluent_templates::LanguageIdentifier;
use itertools::Itertools;
use secrecy::ExposeSecret;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use snxcore::{
    model::{
        params::{CertType, DEFAULT_PROFILE_UUID, TunnelParams, TunnelType},
        proto::LoginOption,
    },
    platform::{Keychain, Platform, PlatformAccess},
    profiles::ConnectionProfilesStore,
    server_info,
    util::parse_ipv4_or_subnet,
};
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

use crate::{
    tr,
    tray::TrayCommand,
    ui::{SettingsWindow, WindowController, WindowScope, close_window},
};

thread_local! {
    static PENDING_CONFIRM: RefCell<Option<async_channel::Sender<bool>>> = const { RefCell::new(None) };
    static PENDING_ENTRY: RefCell<Option<async_channel::Sender<Option<String>>>> = const { RefCell::new(None) };
}

#[derive(Default)]
struct SettingsState {
    auth_ids: Vec<String>,
    auth_factors: Vec<Vec<String>>,
    profile_ids: Vec<Uuid>,
    locales: Vec<LanguageIdentifier>,
}

pub struct SettingsWindowController {
    scope: Rc<WindowScope<SettingsWindow>>,
    sender: Sender<TrayCommand>,
    state: Rc<RefCell<SettingsState>>,
}

impl SettingsWindowController {
    pub const NAME: &str = "settings";

    pub fn new(sender: Sender<TrayCommand>) -> anyhow::Result<Rc<Self>> {
        Ok(Rc::new(Self {
            scope: WindowScope::new(SettingsWindow::new()?),
            sender,
            state: Rc::new(RefCell::new(SettingsState::default())),
        }))
    }

    fn set_static_models(&self) {
        let using_bundled_icons = std::fs::read_dir("/usr/share/icons/hicolor/symbolic/apps")
            .map(|r| {
                r.flatten()
                    .any(|e| e.file_name().to_string_lossy().starts_with("snx-rs"))
            })
            .unwrap_or(false);

        self.scope.window.set_icon_theme_visible(!using_bundled_icons);

        let tunnel_types: Vec<SharedString> = vec![tr!("tunnel-type-ipsec").into(), tr!("tunnel-type-ssl").into()];
        self.scope
            .window
            .set_tunnel_types(ModelRc::new(VecModel::from(tunnel_types)));

        let cert_types: Vec<SharedString> = vec![
            tr!("cert-type-none").into(),
            tr!("cert-type-pfx").into(),
            tr!("cert-type-pem").into(),
            tr!("cert-type-hw").into(),
        ];
        self.scope
            .window
            .set_cert_types(ModelRc::new(VecModel::from(cert_types)));

        let transport_types: Vec<SharedString> = vec![
            tr!("transport-type-autodetect").into(),
            tr!("transport-type-kernel").into(),
            tr!("transport-type-udp").into(),
            tr!("transport-type-tcpt").into(),
        ];
        self.scope
            .window
            .set_transport_types(ModelRc::new(VecModel::from(transport_types)));

        let tls_version_max_options: Vec<SharedString> =
            vec!["TLS 1.2".into(), "TLS 1.3".into(), tr!("label-system-default").into()];

        self.scope
            .window
            .set_tls_version_max_options(ModelRc::new(VecModel::from(tls_version_max_options)));

        let theme_names: Vec<SharedString> = vec![
            tr!("theme-autodetect").into(),
            tr!("theme-dark").into(),
            tr!("theme-light").into(),
        ];
        self.scope
            .window
            .set_icon_themes(ModelRc::new(VecModel::from(theme_names.clone())));

        self.scope
            .window
            .set_color_themes(ModelRc::new(VecModel::from(theme_names)));

        let mut locale_labels: Vec<SharedString> = vec![tr!("label-system-default").into()];
        let locales = i18n::get_locales();
        for locale in &locales {
            let message = format!("language-{locale}");
            locale_labels.push(
                format!(
                    "{} ({})",
                    i18n::translate_for_locale(locale, &message),
                    i18n::translate(&message)
                )
                .into(),
            );
        }
        self.scope
            .window
            .set_locales(ModelRc::new(VecModel::from(locale_labels)));

        self.state.borrow_mut().locales = locales;
    }

    fn populate_profiles(&self) {
        let profiles = ConnectionProfilesStore::instance().all();
        let names: Vec<SharedString> = profiles.iter().map(|p| p.profile_name.as_str().into()).collect();
        let ids: Vec<Uuid> = profiles.iter().map(|p| p.profile_id).collect();

        let default_index = ids.iter().position(|id| *id == DEFAULT_PROFILE_UUID).unwrap_or(0) as i32;

        self.scope.window.set_profiles(ModelRc::new(VecModel::from(names)));
        self.scope.window.set_default_profile_index(default_index);

        let connected = ConnectionProfilesStore::instance().get_connected();
        let index = profiles
            .iter()
            .position(|p| p.profile_id == connected.profile_id)
            .unwrap_or(0) as i32;
        self.scope.window.set_profile_index(index);

        self.state.borrow_mut().profile_ids = ids;
    }

    fn bind_callbacks(&self) {
        {
            let weak = self.scope.weak();
            let state = self.state.clone();
            self.scope.window.on_profile_changed(move || {
                if let Some(w) = weak.upgrade() {
                    load_profile_into_window(&w.window, &state);
                }
            });
        }

        {
            let weak = self.scope.weak();
            self.scope.window.on_auth_type_changed(move || {
                if let Some(w) = weak.upgrade() {
                    refresh_auth_visibility(&w.window);
                }
            });
        }

        {
            let weak = self.scope.weak();
            self.scope.window.on_machine_cert_changed(move || {
                if let Some(w) = weak.upgrade() {
                    refresh_auth_visibility(&w.window);
                }
            });
        }

        {
            let weak = self.scope.weak();
            let state = self.state.clone();
            self.scope.window.on_fetch_info_clicked(move || {
                let Some(w) = weak.upgrade() else { return };
                if let Some(params) = current_profile_params(&w.window, &state) {
                    w.window.set_fetch_enabled(false);
                    fetch_server_info(&w.window, &state, &params);
                }
            });
        }

        {
            let weak = self.scope.weak();
            let state = self.state.clone();
            let sender = self.sender.clone();
            self.scope.window.on_profile_new_clicked(move || {
                let weak = weak.clone();
                let state = state.clone();
                let sender = sender.clone();
                let _ = slint::spawn_local(async move {
                    if let Some(w) = weak.upgrade() {
                        let name =
                            show_entry_dialog(&w.window, &tr!("profile-new-title"), &tr!("label-profile-name"), "")
                                .await;
                        if let Some(name) = name {
                            on_profile_new(&w.window, &state, name, sender);
                        }
                    }
                });
            });
        }

        {
            let weak = self.scope.weak();
            let state = self.state.clone();
            let sender = self.sender.clone();
            self.scope.window.on_profile_rename_clicked(move || {
                let weak = weak.clone();
                let state = state.clone();
                let sender = sender.clone();
                let _ = slint::spawn_local(async move {
                    let Some(w) = weak.upgrade() else { return };
                    let active = w.window.get_profile_index() as usize;
                    let current_name = w
                        .window
                        .get_profiles()
                        .iter()
                        .nth(active)
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    if let Some(w) = weak.upgrade() {
                        let name = show_entry_dialog(
                            &w.window,
                            &tr!("profile-rename-title"),
                            &tr!("label-profile-name"),
                            &current_name,
                        )
                        .await;
                        if let Some(name) = name {
                            on_profile_rename(&w.window, &state, name, sender);
                        }
                    }
                });
            });
        }

        {
            let weak = self.scope.weak();
            let state = self.state.clone();
            let sender = self.sender.clone();
            self.scope.window.on_profile_reorder(move |from, to| {
                if let Some(w) = weak.upgrade() {
                    on_profile_reorder(&w.window, &state, from as usize, to as usize, sender.clone());
                }
            });
        }

        {
            let weak = self.scope.weak();
            let state = self.state.clone();
            let sender = self.sender.clone();
            self.scope.window.on_profile_delete_clicked(move || {
                let weak = weak.clone();
                let state = state.clone();
                let sender = sender.clone();
                let _ = slint::spawn_local(async move {
                    if let Some(w) = weak.upgrade()
                        && confirm_dialog(&w.window, &tr!("profile-delete-prompt")).await
                    {
                        on_profile_delete(&w.window, &state, sender);
                    }
                });
            });
        }

        {
            let weak = self.scope.weak();
            self.scope.window.on_browse_ca_cert_clicked(move || {
                let weak = weak.clone();
                let _ = slint::spawn_local(async move {
                    let picked = pick_files(
                        true,
                        &[
                            (tr!("label-ca-cert-files"), vec!["*.pem", "*.der", "*.cer", "*.crt"]),
                            (tr!("label-all-files"), vec!["*"]),
                        ],
                    )
                    .await;
                    if let Some(paths) = picked
                        && let Some(w) = weak.upgrade()
                    {
                        w.window.set_ca_cert(paths.join(",").into());
                    }
                });
            });
        }

        {
            let weak = self.scope.weak();
            self.scope.window.on_browse_cert_path_clicked(move || {
                let weak = weak.clone();
                let _ = slint::spawn_local(async move {
                    let picked = pick_files(
                        false,
                        &[
                            (tr!("label-keychain-files"), vec!["*.pfx", "*.p12", "*.pem", "*.so"]),
                            (tr!("label-all-files"), vec!["*"]),
                        ],
                    )
                    .await;
                    if let Some(paths) = picked
                        && let Some(first) = paths.into_iter().next()
                        && let Some(w) = weak.upgrade()
                    {
                        w.window.set_cert_path(first.into());
                    }
                });
            });
        }

        {
            let weak = self.scope.weak();
            let state = self.state.clone();

            self.scope.window.on_ok_clicked(move || {
                let Some(w) = weak.upgrade() else { return };
                match save_settings(&w.window, &state) {
                    Ok(()) => close_window(Self::NAME),
                    Err(e) => w.window.set_error_text(e.to_string().into()),
                }
            });
        }

        {
            let weak = self.scope.weak();
            let state = self.state.clone();

            self.scope.window.on_apply_clicked(move || {
                let Some(w) = weak.upgrade() else { return };
                match save_settings(&w.window, &state) {
                    Ok(()) => w.window.set_error_text("".into()),
                    Err(e) => w.window.set_error_text(e.to_string().into()),
                }
            });
        }

        self.scope.window.on_cancel_clicked(|| close_window(Self::NAME));

        self.scope.window.on_confirm_ok_clicked(|| {
            if let Some(tx) = PENDING_CONFIRM.with(|cell| cell.borrow_mut().take()) {
                tokio::spawn(async move {
                    let _ = tx.send(true).await;
                });
            }
        });

        self.scope.window.on_confirm_cancel_clicked(|| {
            if let Some(tx) = PENDING_CONFIRM.with(|cell| cell.borrow_mut().take()) {
                tokio::spawn(async move {
                    let _ = tx.send(false).await;
                });
            }
        });

        self.scope.window.on_entry_ok_clicked(|text| {
            if let Some(tx) = PENDING_ENTRY.with(|cell| cell.borrow_mut().take()) {
                let text = text.to_string();
                tokio::spawn(async move {
                    let _ = tx.send(Some(text)).await;
                });
            }
        });

        self.scope.window.on_entry_cancel_clicked(|| {
            if let Some(tx) = PENDING_ENTRY.with(|cell| cell.borrow_mut().take()) {
                tokio::spawn(async move {
                    let _ = tx.send(None).await;
                });
            }
        });

        self.scope.window.window().on_close_requested(|| {
            close_window(Self::NAME);
            slint::CloseRequestResponse::HideWindow
        });

        refresh_auth_visibility(&self.scope.window);
    }
}

impl WindowController for SettingsWindowController {
    fn present(&self) -> anyhow::Result<()> {
        self.scope.set_globals();

        self.set_static_models();
        self.populate_profiles();

        load_profile_into_window(&self.scope.window, &self.state);

        self.bind_callbacks();

        self.scope.window.show()?;

        Ok(())
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&self) {
        self.scope.set_globals();
    }
}

fn load_profile_into_window(window: &SettingsWindow, state: &Rc<RefCell<SettingsState>>) {
    let selected_id = {
        let s = state.borrow();
        s.profile_ids
            .get(window.get_profile_index() as usize)
            .copied()
            .unwrap_or(DEFAULT_PROFILE_UUID)
    };
    let Some(params) = ConnectionProfilesStore::instance().get(selected_id) else {
        return;
    };

    window.set_server_name(params.server_name.as_str().into());
    window.set_tunnel_type_index(params.tunnel_type.as_u32() as i32);
    window.set_username(params.user_name.as_str().into());
    window.set_password(params.password.expose_secret().into());

    let is_mfa = is_multi_factor_login_type(&params);
    window.set_machine_cert(params.cert_type != CertType::None && is_mfa);

    window.set_password_factor(params.password_factor.to_string().into());
    window.set_no_dns(params.no_dns);
    window.set_search_domains(params.search_domains.join(",").into());
    window.set_ignored_domains(params.ignore_search_domains.join(",").into());
    window.set_dns_servers(params.dns_servers.iter().map(|ip| ip.to_string()).join(",").into());
    window.set_ignored_dns_servers(
        params
            .ignore_dns_servers
            .iter()
            .map(|ip| ip.to_string())
            .join(",")
            .into(),
    );
    window.set_set_routing_domains(params.set_routing_domains);
    window.set_no_routing(params.no_routing);
    window.set_default_routing(params.default_route);
    window.set_add_routes(params.add_routes.iter().map(|ip| ip.to_string()).join(",").into());
    window.set_ignored_routes(params.ignore_routes.iter().map(|ip| ip.to_string()).join(",").into());
    window.set_keychain(params.keychain);
    window.set_no_cert_check(params.ignore_server_cert);
    window.set_cert_type_index(params.cert_type.as_u32() as i32);
    window.set_cert_path(
        params
            .cert_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default()
            .into(),
    );
    window.set_cert_password(
        params
            .cert_password
            .as_ref()
            .map(|s| s.expose_secret().to_string())
            .unwrap_or_default()
            .into(),
    );
    window.set_cert_id(params.cert_id.clone().unwrap_or_default().into());
    window.set_ca_cert(params.ca_cert.iter().map(|p| p.display().to_string()).join(",").into());
    window.set_ike_lifetime(params.ike_lifetime.as_secs().to_string().into());
    window.set_ike_persist(params.ike_persist);
    window.set_no_keepalive(params.no_keepalive);
    window.set_port_knock(params.port_knock);
    window.set_ip_lease_time(
        params
            .ip_lease_time
            .map(|v| v.as_secs().to_string())
            .unwrap_or_default()
            .into(),
    );
    window.set_disable_ipv6(params.disable_ipv6);
    window.set_mtu(params.mtu.to_string().into());
    window.set_transport_type_index(params.transport_type.as_u32() as i32);
    window.set_tls_version_max_index(params.tls_version_max.as_u32() as i32);
    window.set_allow_forwarding(params.allow_forwarding);

    // Global (default-profile) settings
    let defaults = ConnectionProfilesStore::instance().get_default();
    window.set_icon_theme_index(defaults.icon_theme.as_u32() as i32);
    window.set_color_theme_index(defaults.color_theme.as_u32() as i32);
    window.set_auto_connect(defaults.auto_connect);
    let locale_index = defaults
        .locale
        .as_ref()
        .and_then(|l| l.parse::<LanguageIdentifier>().ok())
        .and_then(|lang| {
            state
                .borrow()
                .locales
                .iter()
                .position(|l| *l == lang)
                .map(|i| i as i32 + 1)
        })
        .unwrap_or(0);
    window.set_locale_index(locale_index);

    fetch_server_info(window, state, &params);
}

fn refresh_default_profile_index(window: &SettingsWindow, state: &Rc<RefCell<SettingsState>>) {
    let idx = state
        .borrow()
        .profile_ids
        .iter()
        .position(|id| *id == DEFAULT_PROFILE_UUID)
        .unwrap_or(0) as i32;
    window.set_default_profile_index(idx);
}

fn current_profile_params(window: &SettingsWindow, state: &Rc<RefCell<SettingsState>>) -> Option<Arc<TunnelParams>> {
    let idx = window.get_profile_index() as usize;
    let id = state.borrow().profile_ids.get(idx).copied()?;
    ConnectionProfilesStore::instance().get(id)
}

fn refresh_auth_visibility(window: &SettingsWindow) {
    // These derive from auth-types/auth-factors; we recompute using factors from state.
    // We re-read factors on every change, so look up through the weak-held state.
    let machine_cert = window.get_machine_cert();
    let factors = SETTINGS_STATE.with(|cell| {
        cell.borrow()
            .as_ref()
            .and_then(|state| {
                let idx = window.get_auth_type_index() as usize;
                state.borrow().auth_factors.get(idx).cloned()
            })
            .unwrap_or_default()
    });
    let is_saml = factors.iter().any(|f| f == "identity_provider");
    let is_cert = factors.iter().any(|f| f == "certificate");
    let is_mobile = factors.iter().any(|f| f == "mobile_access");
    let has_factors = !factors.is_empty();

    window.set_show_user_auth(!is_saml && !is_cert && !is_mobile && has_factors);
    window.set_show_cert_auth(is_cert);
    window.set_tunnel_type_enabled(!is_mobile);
    if is_mobile {
        window.set_tunnel_type_index(TunnelType::SSL.as_u32() as i32);
    }
    window.set_machine_cert_enabled(!is_cert && !is_mobile);
    if (is_cert || is_mobile) && machine_cert {
        window.set_machine_cert(false);
    }
    if !is_cert && !machine_cert {
        window.set_cert_type_index(CertType::None.as_u32() as i32);
    }
}

thread_local! {
    static SETTINGS_STATE: RefCell<Option<Rc<RefCell<SettingsState>>>> = const { RefCell::new(None) };
}

fn is_multi_factor_login_type(params: &TunnelParams) -> bool {
    let (tx, rx) = async_channel::bounded(1);
    let params = params.clone();
    tokio::spawn(async move {
        let result = server_info::is_multi_factor_login_type(&params).await.unwrap_or(true);
        let _ = tx.send(result).await;
    });
    rx.recv_blocking().unwrap_or(true)
}

fn fetch_server_info(window: &SettingsWindow, state: &Rc<RefCell<SettingsState>>, params: &TunnelParams) {
    if window.get_server_name().is_empty() {
        window.set_auth_types(ModelRc::new(VecModel::from(Vec::<SharedString>::new())));
        state.borrow_mut().auth_ids.clear();
        state.borrow_mut().auth_factors.clear();
        window.set_show_user_auth(false);
        window.set_show_cert_auth(false);
        window.set_fetch_enabled(true);
        return;
    }

    let login_type = params.login_type.clone();
    let new_params = TunnelParams {
        server_name: window.get_server_name().to_string(),
        ignore_server_cert: window.get_no_cert_check(),
        ..params.clone()
    };

    let (tx, rx) = async_channel::bounded(1);
    tokio::spawn(async move {
        let response = server_info::get(&new_params).await;
        let _ = tx.send(response).await;
    });

    SETTINGS_STATE.with(|cell| *cell.borrow_mut() = Some(state.clone()));

    let weak = window.as_weak();
    let state = state.clone();
    let _ = slint::spawn_local(async move {
        let recv = rx.recv().await;

        let Some(window) = weak.upgrade() else { return };
        window.set_fetch_enabled(true);

        let Ok(response) = recv else { return };

        match response {
            Ok(server_info) => {
                window.set_error_text("".into());
                let mut options_list = server_info
                    .login_options_data
                    .map(|d| d.login_options_list.into_values().collect::<Vec<_>>())
                    .unwrap_or_default();
                if options_list.is_empty() {
                    options_list.push(LoginOption::unspecified());
                }
                #[cfg(feature = "mobile-access")]
                options_list.push(LoginOption::mobile_access());

                let mut names: Vec<SharedString> = Vec::new();
                let mut ids: Vec<String> = Vec::new();
                let mut factors_list: Vec<Vec<String>> = Vec::new();
                let mut selected: i32 = 0;
                for option in options_list.into_iter().filter(|opt| opt.show_realm != 0) {
                    let factors = option
                        .factors
                        .values()
                        .map(|f| f.factor_type.clone())
                        .collect::<Vec<_>>();
                    names.push(option.display_name.as_str().into());
                    if login_type == option.id {
                        selected = ids.len() as i32;
                    }
                    ids.push(option.id.clone());
                    factors_list.push(factors);
                }
                window.set_auth_types(ModelRc::new(VecModel::from(names)));
                state.borrow_mut().auth_ids = ids;
                state.borrow_mut().auth_factors = factors_list;
                window.set_auth_type_index(selected);
                refresh_auth_visibility(&window);
            }
            Err(e) => {
                let label = e
                    .chain()
                    .find_map(|error| {
                        if error.to_string().contains("certificate verify failed") {
                            Some(tr!("error-certificate-verify-failed"))
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| e.to_string());
                window.set_error_text(label.into());
            }
        }
    });
}

fn on_profile_new(
    window: &SettingsWindow,
    state: &Rc<RefCell<SettingsState>>,
    name: String,
    sender: Sender<TrayCommand>,
) {
    let profile_id = Uuid::new_v4();
    let params = Arc::new(TunnelParams {
        profile_name: name.clone(),
        profile_id,
        config_file: TunnelParams::default_config_dir().join(format!("{}.conf", profile_id)),
        ..Default::default()
    });
    ConnectionProfilesStore::instance().save(params);

    let mut names: Vec<SharedString> = window.get_profiles().iter().collect();
    names.push(name.into());
    window.set_profiles(ModelRc::new(VecModel::from(names)));

    let mut s = state.borrow_mut();
    s.profile_ids.push(profile_id);
    let new_index = (s.profile_ids.len() - 1) as i32;
    drop(s);

    refresh_default_profile_index(window, state);
    window.set_profile_index(new_index);
    load_profile_into_window(window, state);

    tokio::spawn(async move { sender.send(TrayCommand::Update(None)).await });
    super::update_windows();
}

fn on_profile_rename(
    window: &SettingsWindow,
    state: &Rc<RefCell<SettingsState>>,
    name: String,
    sender: Sender<TrayCommand>,
) {
    let active = window.get_profile_index() as usize;
    let id = match state.borrow().profile_ids.get(active).copied() {
        Some(id) => id,
        None => return,
    };
    if let Some(profile) = ConnectionProfilesStore::instance().get(id) {
        let new_profile = Arc::new(TunnelParams {
            profile_name: name.clone(),
            ..(*profile).clone()
        });
        ConnectionProfilesStore::instance().save(new_profile);
    }

    let mut names: Vec<SharedString> = window.get_profiles().iter().collect();
    if let Some(slot) = names.get_mut(active) {
        *slot = name.into();
    }
    window.set_profiles(ModelRc::new(VecModel::from(names)));
    window.set_profile_index(active as i32);

    tokio::spawn(async move { sender.send(TrayCommand::Update(None)).await });
    super::update_windows();
}

fn on_profile_reorder(
    window: &SettingsWindow,
    state: &Rc<RefCell<SettingsState>>,
    from: usize,
    to: usize,
    sender: Sender<TrayCommand>,
) {
    if from == to {
        return;
    }
    let active_id = state
        .borrow()
        .profile_ids
        .get(window.get_profile_index() as usize)
        .copied();

    ConnectionProfilesStore::instance().reorder(from, to);

    let profiles = ConnectionProfilesStore::instance().all();
    let names: Vec<SharedString> = profiles.iter().map(|p| p.profile_name.as_str().into()).collect();
    let ids: Vec<Uuid> = profiles.iter().map(|p| p.profile_id).collect();

    window.set_profiles(ModelRc::new(VecModel::from(names)));
    state.borrow_mut().profile_ids = ids.clone();
    refresh_default_profile_index(window, state);

    if let Some(id) = active_id {
        let new_idx = ids.iter().position(|x| *x == id).unwrap_or(0) as i32;
        window.set_profile_index(new_idx);
    }

    tokio::spawn(async move { sender.send(TrayCommand::Update(None)).await });
    super::update_windows();
}

fn on_profile_delete(window: &SettingsWindow, state: &Rc<RefCell<SettingsState>>, sender: Sender<TrayCommand>) {
    let active = window.get_profile_index() as usize;
    let id = match state.borrow().profile_ids.get(active).copied() {
        Some(id) => id,
        None => return,
    };
    if id == DEFAULT_PROFILE_UUID {
        return;
    }
    ConnectionProfilesStore::instance().remove(id);

    tokio::spawn(async move { Platform::get().new_keychain().delete_password(id).await });

    let mut names: Vec<SharedString> = window.get_profiles().iter().collect();
    if active < names.len() {
        names.remove(active);
    }
    window.set_profiles(ModelRc::new(VecModel::from(names)));
    state.borrow_mut().profile_ids.remove(active);
    refresh_default_profile_index(window, state);
    window.set_profile_index(0);
    load_profile_into_window(window, state);

    tokio::spawn(async move { sender.send(TrayCommand::Update(None)).await });
    super::update_windows();
}

fn validate(window: &SettingsWindow) -> anyhow::Result<()> {
    if window.get_server_name().is_empty() {
        anyhow::bail!(tr!("error-no-server-name"));
    }

    let cert_path = window.get_cert_path();
    if !cert_path.is_empty() && !Path::new(cert_path.as_str()).exists() {
        anyhow::bail!(tr!("error-file-not-exist", path = cert_path.to_string()));
    }

    let cert_id = window.get_cert_id().to_string().replace(':', "");
    if !cert_id.is_empty() && hex::decode(&cert_id).is_err() {
        anyhow::bail!(tr!("error-invalid-cert-id", id = cert_id));
    }

    let ca_cert = window.get_ca_cert();
    if !ca_cert.is_empty() {
        for c in ca_cert.split(',') {
            if !Path::new(c.trim()).exists() {
                anyhow::bail!(tr!("error-ca-root-not-exist", path = c));
            }
        }
    }

    window.get_ike_lifetime().parse::<u32>()?;
    window.get_password_factor().parse::<usize>()?;

    for field in [window.get_dns_servers(), window.get_ignored_dns_servers()] {
        if !field.is_empty() {
            for r in field.split(',') {
                r.parse::<Ipv4Addr>()?;
            }
        }
    }

    for field in [window.get_add_routes(), window.get_ignored_routes()] {
        if !field.is_empty() {
            for r in field.split(',') {
                parse_ipv4_or_subnet(r)?;
            }
        }
    }

    let ip_lease_time = window.get_ip_lease_time();
    if !ip_lease_time.trim().is_empty() {
        ip_lease_time.parse::<u32>()?;
    }

    window.get_mtu().parse::<u16>()?;

    Ok(())
}

fn save_settings(window: &SettingsWindow, state: &Rc<RefCell<SettingsState>>) -> anyhow::Result<()> {
    validate(window)?;

    let Some(current) = current_profile_params(window, state) else {
        anyhow::bail!("No profile selected");
    };
    let mut params = (*current).clone();

    params.server_name = window.get_server_name().to_string();
    params.login_type = state
        .borrow()
        .auth_ids
        .get(window.get_auth_type_index() as usize)
        .cloned()
        .unwrap_or_default();
    params.tunnel_type = (window.get_tunnel_type_index() as u32).into();
    params.user_name = window.get_username().to_string();
    params.password = window.get_password().to_string().into();
    params.password_factor = window.get_password_factor().parse()?;
    params.no_dns = window.get_no_dns();
    params.set_routing_domains = window.get_set_routing_domains();
    params.search_domains = split_non_empty(&window.get_search_domains());
    params.ignore_search_domains = split_non_empty(&window.get_ignored_domains());
    params.dns_servers = window
        .get_dns_servers()
        .split(',')
        .flat_map(|s| s.trim().parse().ok())
        .collect();
    params.ignore_dns_servers = window
        .get_ignored_dns_servers()
        .split(',')
        .flat_map(|s| s.trim().parse().ok())
        .collect();
    params.no_routing = window.get_no_routing();
    params.default_route = window.get_default_routing();
    params.add_routes = window
        .get_add_routes()
        .split(',')
        .flat_map(|s| parse_ipv4_or_subnet(s).ok())
        .collect();
    params.ignore_routes = window
        .get_ignored_routes()
        .split(',')
        .flat_map(|s| parse_ipv4_or_subnet(s).ok())
        .collect();
    params.keychain = window.get_keychain();
    params.ignore_server_cert = window.get_no_cert_check();
    params.cert_type = (window.get_cert_type_index() as u32).into();
    params.cert_path = {
        let text = window.get_cert_path().to_string();
        if text.is_empty() { None } else { Some(text.into()) }
    };
    params.cert_password = {
        let text = window.get_cert_password().to_string();
        if text.is_empty() { None } else { Some(text.into()) }
    };
    params.cert_id = {
        let text = window.get_cert_id().to_string();
        if text.is_empty() { None } else { Some(text) }
    };
    params.ca_cert = window
        .get_ca_cert()
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.into())
        .collect();
    params.ike_lifetime = Duration::from_secs(window.get_ike_lifetime().parse()?);
    params.ike_persist = window.get_ike_persist();
    params.no_keepalive = window.get_no_keepalive();
    params.port_knock = window.get_port_knock();
    params.transport_type = (window.get_transport_type_index() as u32).into();
    params.tls_version_max = (window.get_tls_version_max_index() as u32).into();
    params.disable_ipv6 = window.get_disable_ipv6();
    params.allow_forwarding = window.get_allow_forwarding();

    let ip_lease_time = window.get_ip_lease_time();
    params.ip_lease_time = if ip_lease_time.trim().is_empty() {
        None
    } else {
        Some(Duration::from_secs(ip_lease_time.parse()?))
    };

    params.mtu = window.get_mtu().parse()?;
    params.icon_theme = (window.get_icon_theme_index() as u32).into();
    params.color_theme = (window.get_color_theme_index() as u32).into();

    let selected_locale = window.get_locale_index();
    let new_locale: Option<String> = if selected_locale <= 0 {
        None
    } else {
        state
            .borrow()
            .locales
            .get(selected_locale as usize - 1)
            .map(|l| l.to_string())
    };
    params.locale = new_locale.clone();
    params.auto_connect = window.get_auto_connect();

    if params.profile_id != DEFAULT_PROFILE_UUID {
        let mut default_params = (*ConnectionProfilesStore::instance().get_default()).clone();
        default_params.icon_theme = params.icon_theme;
        default_params.color_theme = params.color_theme;
        default_params.locale = params.locale.clone();
        default_params.auto_connect = params.auto_connect;
        ConnectionProfilesStore::instance().save(Arc::new(default_params));
    }

    if !params.keychain && current.keychain {
        let uuid = params.profile_id;
        tokio::spawn(async move { Platform::get().new_keychain().delete_password(uuid).await });
    }

    ConnectionProfilesStore::instance().save(Arc::new(params));

    i18n::set_locale(new_locale.and_then(|l| l.parse().ok()));

    super::update_windows();

    Ok(())
}

fn split_non_empty(text: &SharedString) -> Vec<String> {
    text.split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

async fn pick_files(multiple: bool, patterns: &[(String, Vec<&str>)]) -> Option<Vec<String>> {
    let mut dialog = rfd::AsyncFileDialog::new().set_title(tr!("label-select-file"));

    for (name, pats) in patterns {
        let extensions: Vec<&str> = pats.iter().map(|p| p.strip_prefix("*.").unwrap_or(p)).collect();
        dialog = dialog.add_filter(name, &extensions);
    }

    if multiple {
        let files = dialog.pick_files().await?;
        let paths: Vec<String> = files
            .into_iter()
            .map(|f| f.path().to_string_lossy().into_owned())
            .collect();
        (!paths.is_empty()).then_some(paths)
    } else {
        let file = dialog.pick_file().await?;
        Some(vec![file.path().to_string_lossy().into_owned()])
    }
}

async fn confirm_dialog(parent: &SettingsWindow, message: &str) -> bool {
    let (tx, rx) = async_channel::bounded::<bool>(1);
    PENDING_CONFIRM.with(|cell| *cell.borrow_mut() = Some(tx));
    parent.invoke_show_confirm(message.into());

    rx.recv().await.unwrap_or(false)
}

async fn show_entry_dialog(parent: &SettingsWindow, title: &str, label: &str, value: &str) -> Option<String> {
    let (tx, rx) = async_channel::bounded::<Option<String>>(1);
    PENDING_ENTRY.with(|cell| *cell.borrow_mut() = Some(tx));
    parent.invoke_show_entry(title.into(), label.into(), value.into());

    rx.recv().await.ok().flatten()
}
