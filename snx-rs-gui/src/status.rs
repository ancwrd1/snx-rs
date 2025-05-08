use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use gtk4::{
    glib::{self, clone}, prelude::{BoxExt, ButtonExt, DialogExt, DialogExtManual, DisplayExt, GtkWindowExt, WidgetExt}, Align,
    Orientation,
    ResponseType,
};
use snxcore::{
    browser::SystemBrowser,
    controller::{ServiceCommand, ServiceController},
    model::{params::TunnelParams, ConnectionInfo, ConnectionStatus},
};
use tokio::sync::mpsc::Sender;

use crate::{main_window, prompt::GtkPrompt, tr, tray::TrayEvent};

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

fn get_info(status: &anyhow::Result<ConnectionStatus>) -> ConnectionInfo {
    if let Ok(ConnectionStatus::Connected(info)) = status {
        info.clone()
    } else {
        ConnectionInfo::default()
    }
}

pub async fn show_status_dialog(sender: Sender<TrayEvent>, params: Arc<TunnelParams>) {
    let mut controller = ServiceController::new(GtkPrompt, SystemBrowser);

    let status = Arc::new(RwLock::new(
        controller.command(ServiceCommand::Status, params.clone()).await,
    ));

    let dialog = gtk4::Dialog::builder()
        .title(tr!("status-dialog-title"))
        .transient_for(&main_window())
        .build();

    let ok = gtk4::Button::builder().label(tr!("button-ok")).build();

    ok.connect_clicked(clone!(
        #[weak]
        dialog,
        move |_| {
            dialog.response(ResponseType::Ok);
        }
    ));

    let copy = gtk4::Button::builder().label(tr!("status-button-copy")).build();

    let status_copy = status.clone();
    copy.connect_clicked(clone!(move |_| {
        let info = get_info(&status_copy.read().unwrap());
        gtk4::gdk::Display::default()
            .unwrap()
            .clipboard()
            .set_text(&info.to_values().into_iter().fold(String::new(), |mut acc, (k, v)| {
                acc.push_str(&format!("{}: {}\n", k, v));
                acc
            }));
    }));

    let settings = gtk4::Button::builder().label(tr!("status-button-settings")).build();

    let sender2 = sender.clone();
    settings.connect_clicked(move |_| {
        let sender = sender2.clone();
        tokio::spawn(async move { sender.send(TrayEvent::Settings).await });
    });

    let connect = gtk4::Button::builder()
        .label(tr!("status-button-connect"))
        .sensitive(matches!(*status.read().unwrap(), Ok(ConnectionStatus::Disconnected)))
        .build();

    let sender2 = sender.clone();
    connect.connect_clicked(move |btn| {
        let sender = sender2.clone();
        tokio::spawn(async move { sender.send(TrayEvent::Connect).await });
        btn.set_sensitive(false);
    });

    let disconnect = gtk4::Button::builder()
        .label(tr!("status-button-disconnect"))
        .sensitive(matches!(
            *status.read().unwrap(),
            Ok(ConnectionStatus::Connected(_) | ConnectionStatus::Connecting | ConnectionStatus::Mfa(_))
        ))
        .build();

    let sender2 = sender.clone();
    disconnect.connect_clicked(move |btn| {
        let sender = sender2.clone();
        tokio::spawn(async move { sender.send(TrayEvent::Disconnect).await });
        btn.set_sensitive(false);
    });

    let button_box = gtk4::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .margin_top(6)
        .margin_start(6)
        .margin_end(6)
        .homogeneous(true)
        .halign(Align::End)
        .build();

    button_box.append(&connect);
    button_box.append(&disconnect);
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

    let update_ui = clone!(
        #[weak]
        inner,
        #[weak]
        connect,
        #[weak]
        disconnect,
        move |status: &anyhow::Result<ConnectionStatus>| {
            connect.set_sensitive(matches!(*status, Ok(ConnectionStatus::Disconnected)));
            disconnect.set_sensitive(matches!(
                *status,
                Ok(ConnectionStatus::Connected(_) | ConnectionStatus::Connecting | ConnectionStatus::Mfa(_))
            ));

            let mut child = inner.first_child();

            while let Some(widget) = child {
                child = widget.next_sibling();
                inner.remove(&widget);
            }

            let info = get_info(status);
            for (key, value) in info.to_values() {
                inner.append(&status_entry(&format!("{}:", key), &value));
            }
        }
    );

    update_ui(&status.read().unwrap());

    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let status2 = status.clone();

    glib::spawn_future_local(async move {
        while rx.recv().await.is_some() {
            let status = status2.read().unwrap();
            update_ui(&status);
        }
    });

    tokio::spawn(async move {
        let mut old_status = String::new();
        loop {
            let new_status = controller.command(ServiceCommand::Status, params.clone()).await;
            let status_str = format!("{:?}", new_status);
            if old_status != status_str {
                old_status = status_str;
                *status.write().unwrap() = new_status;
                if tx.send(()).await.is_err() {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });

    content.append(&inner);
    content.append(&button_box);

    GtkWindowExt::set_focus(&dialog, Some(&ok));
    dialog.run_future().await;
    dialog.close();
}
