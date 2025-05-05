use crate::main_window;
use crate::prompt::GtkPrompt;
use crate::tray::TrayCommand;
use gtk4::{
    Align, Orientation, ResponseType,
    glib::{self, clone},
    prelude::{BoxExt, ButtonExt, DialogExt, DialogExtManual, DisplayExt, GtkWindowExt, WidgetExt},
};
use snxcore::browser::SystemBrowser;
use snxcore::controller::{ServiceCommand, ServiceController};
use snxcore::model::params::TunnelParams;
use snxcore::model::{ConnectionInfo, ConnectionStatus};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;

fn status_entry(label: &str, value: &str) -> gtk4::Box {
    let form = gtk4::Box::builder()
        .orientation(Orientation::Horizontal)
        .homogeneous(true)
        .spacing(6)
        .build();

    form.append(
        &gtk4::Label::builder()
            .label(label)
            .halign(Align::End)
            .css_classes(vec!["darkened"])
            .build(),
    );
    form.append(&gtk4::Label::builder().label(value).halign(Align::Start).build());
    form
}

pub async fn show_status_dialog(sender: Sender<TrayCommand>, params: Arc<TunnelParams>) {
    let info = Arc::new(Mutex::new(ConnectionInfo::default()));

    let dialog = gtk4::Dialog::builder()
        .title("Connection information")
        .transient_for(&main_window())
        .build();

    let ok = gtk4::Button::builder().label("OK").build();

    ok.connect_clicked(clone!(
        #[weak]
        dialog,
        move |_| {
            dialog.response(ResponseType::Ok);
        }
    ));

    let copy = gtk4::Button::builder().label("Copy").build();

    let info_copy = info.clone();
    copy.connect_clicked(clone!(move |_| {
        gtk4::gdk::Display::default().unwrap().clipboard().set_text(
            &info_copy
                .lock()
                .unwrap()
                .to_values()
                .into_iter()
                .fold(String::new(), |mut acc, (k, v)| {
                    acc.push_str(&format!("{}: {}\n", k, v));
                    acc
                }),
        );
    }));

    let settings = gtk4::Button::builder().label("Settings").build();

    let params2 = params.clone();
    settings.connect_clicked(clone!(move |_| crate::settings::start_settings_dialog(
        main_window(),
        sender.clone(),
        params2.clone()
    )));

    let button_box = gtk4::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .margin_top(6)
        .margin_start(6)
        .margin_end(6)
        .homogeneous(true)
        .halign(Align::End)
        .build();

    button_box.append(&settings);
    button_box.append(&copy);
    button_box.append(&ok);

    dialog.set_default_response(ResponseType::Ok);

    let content = dialog.content_area();

    let inner = gtk4::Box::builder()
        .orientation(Orientation::Vertical)
        .margin_bottom(6)
        .margin_top(6)
        .margin_start(6)
        .margin_end(6)
        .spacing(6)
        .vexpand(true)
        .build();
    inner.add_css_class("bordered");

    let params = params.clone();

    glib::spawn_future_local(clone!(
        #[weak]
        inner,
        #[weak]
        dialog,
        async move {
            while dialog.is_visible() {
                let status = ServiceController::new(GtkPrompt, SystemBrowser)
                    .command(ServiceCommand::Status, params.clone())
                    .await;

                let new_info = if let Ok(ConnectionStatus::Connected(info)) = status {
                    info
                } else {
                    ConnectionInfo::default()
                };

                *info.lock().unwrap() = new_info.clone();

                let mut child = inner.first_child();

                while let Some(widget) = child {
                    child = widget.next_sibling();
                    inner.remove(&widget);
                }

                for (key, value) in new_info.to_values() {
                    inner.append(&status_entry(&format!("{}:", key), &value));
                }

                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    ));

    content.append(&inner);
    content.append(&button_box);

    GtkWindowExt::set_focus(&dialog, Some(&ok));
    dialog.run_future().await;
    dialog.close();
}
