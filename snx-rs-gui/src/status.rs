use gtk4::{
    Align, Orientation, ResponseType,
    glib::{self, clone},
    prelude::{BoxExt, ButtonExt, DialogExt, DialogExtManual, DisplayExt, GtkWindowExt, WidgetExt},
};
use snxcore::model::ConnectionInfo;

use crate::main_window;

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

pub async fn show_status_dialog(info: ConnectionInfo) {
    let dialog = gtk4::Dialog::builder()
        .title("Connection information")
        .transient_for(&main_window())
        .modal(true)
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
        gtk4::gdk::Display::default()
            .unwrap()
            .clipboard()
            .set_text(
                &info_copy
                    .to_values()
                    .into_iter()
                    .fold(String::new(), |mut acc, (k, v)| {
                        acc.push_str(&format!("{}: {}\n", k, v));
                        acc
                    }),
            );
    }));

    let button_box = gtk4::Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(6)
        .margin_top(6)
        .margin_start(6)
        .margin_end(6)
        .homogeneous(true)
        .halign(Align::End)
        .build();

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

    for (key, value) in info.to_values() {
        inner.append(&status_entry(&format!("{}:", key), &value));
    }
    content.append(&inner);
    content.append(&button_box);

    GtkWindowExt::set_focus(&dialog, Some(&ok));
    dialog.run_future().await;
    dialog.close();
}
