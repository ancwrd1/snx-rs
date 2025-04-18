use std::sync::mpsc;

use anyhow::anyhow;
use gtk4::{
    glib::{self, clone, ControlFlow},
    prelude::*,
    Align, Orientation, ResponseType,
};
use snxcore::{model::AuthPrompt, prompt::SecurePrompt};

use crate::dbus::send_notification;

pub struct GtkPrompt;

impl GtkPrompt {
    fn get_input(&self, prompt: &AuthPrompt, secure: bool) -> anyhow::Result<String> {
        let (tx, rx) = mpsc::channel();

        let prompt = prompt.to_owned();

        glib::idle_add(move || {
            let dialog = gtk4::Dialog::builder().title("Authentication").modal(true).build();

            let ok = gtk4::Button::builder().label("OK").build();
            ok.connect_clicked(clone!(
                #[weak]
                dialog,
                move |_| {
                    dialog.response(ResponseType::Ok);
                }
            ));

            let cancel = gtk4::Button::builder().label("Cancel").build();
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
                .build();

            button_box.append(&ok);
            button_box.append(&cancel);

            dialog.set_default_response(ResponseType::Ok);
            dialog.set_default_size(320, 120);

            let content = dialog.content_area();
            let inner = gtk4::Box::builder()
                .orientation(Orientation::Vertical)
                .margin_bottom(6)
                .margin_top(6)
                .margin_start(6)
                .margin_end(6)
                .spacing(6)
                .build();

            if !prompt.header.is_empty() {
                inner.append(
                    &gtk4::Label::builder()
                        .label(&prompt.header)
                        .halign(Align::Start)
                        .build(),
                );
            }
            inner.append(
                &gtk4::Label::builder()
                    .label(&prompt.prompt)
                    .halign(Align::Start)
                    .build(),
            );

            let entry = gtk4::Entry::builder()
                .name("entry")
                .visibility(!secure)
                .activates_default(true)
                .build();

            entry.connect_activate(clone!(
                #[weak]
                dialog,
                move |_| {
                    dialog.response(ResponseType::Ok);
                }
            ));

            inner.append(&entry);

            content.append(&inner);
            content.append(&button_box);

            let tx = tx.clone();

            dialog.run_async(move |dlg, response| {
                if response == ResponseType::Ok {
                    let _ = tx.send(Ok(entry.text().to_string()));
                } else {
                    let _ = tx.send(Err(anyhow!("User input canceled")));
                }
                dlg.close();
            });

            dialog.show();

            ControlFlow::Break
        });

        rx.recv()?
    }
}

impl SecurePrompt for GtkPrompt {
    fn get_secure_input(&self, prompt: &AuthPrompt) -> anyhow::Result<String> {
        self.get_input(prompt, true)
    }

    fn get_plain_input(&self, prompt: &AuthPrompt) -> anyhow::Result<String> {
        self.get_input(prompt, false)
    }

    fn show_notification(&self, summary: &str, message: &str) -> anyhow::Result<()> {
        std::thread::scope(|s| {
            s.spawn(|| snxcore::util::block_on(send_notification(summary, message)))
                .join()
                .unwrap()
        })
    }
}
