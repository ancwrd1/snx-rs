use std::{cell::RefCell, rc::Rc, sync::Arc};

use slint::{ComponentHandle, LogicalSize, ModelRc, SharedString, VecModel};
use snxcore::{
    browser::SystemBrowser,
    controller::{ServiceCommand, ServiceController},
    model::{ConnectionInfo, ConnectionStatus, params::DEFAULT_PROFILE_UUID},
    profiles::ConnectionProfilesStore,
};
use tokio::sync::mpsc::Sender;

use crate::{
    POLL_INTERVAL, tr,
    tray::TrayEvent,
    ui::{StatusEntry, StatusWindow, WindowController, WindowScope, close_window, prompt::SlintPrompt},
};

pub fn same_status(lhs: &anyhow::Result<ConnectionStatus>, rhs: &anyhow::Result<ConnectionStatus>) -> bool {
    match (lhs, rhs) {
        (Ok(lhs), Ok(rhs)) => lhs == rhs,
        (Err(e1), Err(e2)) => e1.to_string() == e2.to_string(),
        _ => false,
    }
}

fn get_info(status: &anyhow::Result<ConnectionStatus>) -> ConnectionInfo {
    if let Ok(ConnectionStatus::Connected(info)) = status {
        (**info).clone()
    } else {
        ConnectionInfo::default()
    }
}

fn to_status_entries(info: &ConnectionInfo, with_stats: bool) -> Vec<StatusEntry> {
    info.to_values(with_stats)
        .into_iter()
        .map(|(key, value)| StatusEntry {
            label: i18n::translate(key).into(),
            value: value.into(),
        })
        .collect()
}

fn close_fn(exit_on_close: bool, stop_tx: Sender<()>, sender: Sender<TrayEvent>) {
    let stop_tx = stop_tx.clone();
    tokio::spawn(async move { stop_tx.send(()).await });

    if exit_on_close {
        let sender = sender.clone();
        tokio::spawn(async move { sender.send(TrayEvent::Exit).await });
    }

    close_window(StatusWindowController::NAME);
}

pub struct StatusWindowController {
    scope: Rc<WindowScope<StatusWindow>>,
    exit_on_close: bool,
    tray_event_sender: Sender<TrayEvent>,
}

impl StatusWindowController {
    pub const NAME: &str = "status";

    pub fn new(exit_on_close: bool, tray_event_sender: Sender<TrayEvent>) -> anyhow::Result<Rc<Self>> {
        Ok(Rc::new(Self {
            scope: WindowScope::new(StatusWindow::new()?),
            exit_on_close,
            tray_event_sender,
        }))
    }
}

impl WindowController for StatusWindowController {
    fn present(&self) -> anyhow::Result<()> {
        self.scope.set_globals();

        let profiles = ConnectionProfilesStore::instance().all();
        let profile_names: Vec<SharedString> = profiles.iter().map(|p| p.profile_name.as_str().into()).collect();
        self.scope
            .window
            .set_profile_names(ModelRc::new(VecModel::from(profile_names)));
        self.scope.window.set_can_connect(false);
        self.scope.window.set_can_disconnect(false);
        self.scope
            .window
            .set_entries(ModelRc::new(VecModel::from(Vec::<StatusEntry>::new())));

        let profile_ids: Rc<Vec<_>> = Rc::new(profiles.iter().map(|p| p.profile_id).collect());

        let sender = self.tray_event_sender.clone();
        let profile_ids = profile_ids.clone();

        self.scope.window.on_connect_clicked(move |index| {
            let uuid = if index < 0 {
                profile_ids.first().copied().unwrap_or(DEFAULT_PROFILE_UUID)
            } else {
                profile_ids.get(index as usize).copied().unwrap_or(DEFAULT_PROFILE_UUID)
            };
            let sender = sender.clone();
            tokio::spawn(async move { sender.send(TrayEvent::Connect(uuid)).await });
        });

        let sender = self.tray_event_sender.clone();
        self.scope.window.on_disconnect_clicked(move || {
            let sender = sender.clone();
            tokio::spawn(async move { sender.send(TrayEvent::Disconnect).await });
        });

        let sender = self.tray_event_sender.clone();
        self.scope.window.on_settings_clicked(move || {
            let sender = sender.clone();
            tokio::spawn(async move { sender.send(TrayEvent::Settings).await });
        });

        let exit_on_close = self.exit_on_close;
        let (stop_tx, mut stop_rx) = tokio::sync::mpsc::channel(1);

        let sender = self.tray_event_sender.clone();
        let stop_sender = stop_tx.clone();
        self.scope.window.on_ok_clicked(move || {
            let stop_sender = stop_sender.clone();
            let sender = sender.clone();
            close_fn(exit_on_close, stop_sender, sender);
        });

        let sender = self.tray_event_sender.clone();
        let stop_sender = stop_tx.clone();
        self.scope.window.window().on_close_requested(move || {
            let stop_sender = stop_sender.clone();
            let sender = sender.clone();
            close_fn(exit_on_close, stop_sender, sender);
            slint::CloseRequestResponse::HideWindow
        });

        let last_info: Rc<RefCell<ConnectionInfo>> = Rc::new(RefCell::new(ConnectionInfo::default()));

        let weak_window = self.scope.window.as_weak();
        let last_info_for_toggle = last_info.clone();
        self.scope.window.on_show_stats_toggled(move || {
            if let Some(window) = weak_window.upgrade() {
                let show = window.get_show_stats();
                let entries = to_status_entries(&last_info_for_toggle.borrow(), show);
                window.set_entries(ModelRc::new(VecModel::from(entries)));
                if !show {
                    let preferred_height = window.get_preferred_content_height();
                    let win = window.window();
                    let current = win.size().to_logical(win.scale_factor());
                    win.set_size(LogicalSize::new(current.width, preferred_height));
                }
            }
        });

        self.scope.window.show()?;

        let (status_tx, status_rx) = async_channel::bounded::<Arc<anyhow::Result<ConnectionStatus>>>(1);

        tokio::spawn(async move {
            let mut controller = ServiceController::new(SlintPrompt, SystemBrowser::new(SlintPrompt));
            let mut old_status = Arc::new(Err(anyhow::anyhow!(tr!("app-connection-error"))));
            loop {
                let params = ConnectionProfilesStore::instance().get_connected();
                let new_status = controller.command(ServiceCommand::Status, params).await;
                if !same_status(&new_status, &old_status) {
                    old_status = Arc::new(new_status);
                    if status_tx.send(old_status.clone()).await.is_err() {
                        break;
                    }
                }

                tokio::select! {
                    _ = tokio::time::sleep(POLL_INTERVAL) => {}
                    _ = stop_rx.recv() => break,
                }
            }
        });

        let weak_scope = self.scope.weak();

        let _ = slint::spawn_local(async move {
            while let Ok(status) = status_rx.recv().await {
                if let Some(scope) = weak_scope.upgrade() {
                    let info = get_info(&status);
                    *last_info.borrow_mut() = info.clone();
                    let show_stats = scope.window.get_show_stats();
                    scope
                        .window
                        .set_entries(ModelRc::new(VecModel::from(to_status_entries(&info, show_stats))));
                    scope
                        .window
                        .set_can_connect(matches!(*status, Ok(ConnectionStatus::Disconnected)));
                    scope.window.set_can_disconnect(matches!(
                        *status,
                        Ok(ConnectionStatus::Connected(_) | ConnectionStatus::Connecting | ConnectionStatus::Mfa(_))
                    ));
                    scope
                        .window
                        .set_is_connected(matches!(*status, Ok(ConnectionStatus::Connected(_))));

                    let preferred_height = scope.window.get_preferred_content_height();
                    let window = scope.window.window();
                    let current = window.size().to_logical(window.scale_factor());
                    if preferred_height > current.height {
                        window.set_size(LogicalSize::new(current.width, preferred_height));
                    }
                }
            }
        });

        Ok(())
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&self) {
        self.scope.set_globals();
    }
}
