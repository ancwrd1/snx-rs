use anyhow::{Context, anyhow};
use gtk4::{
    Align, Orientation, ResponseType,
    glib::{self, clone},
    prelude::*,
};
use i18n::tr;
use snxcore::{model::PromptInfo, prompt::SecurePrompt};

use crate::{dbus::send_notification, main_window};

pub struct GtkPrompt;

impl GtkPrompt {
    async fn get_input(&self, prompt: PromptInfo, secure: bool) -> anyhow::Result<String> {
        let (tx, rx) = async_channel::bounded(1);

        glib::idle_add_once(move || {
            glib::spawn_future_local(async move {
                let dialog = gtk4::Dialog::builder()
                    .title(tr!("auth-dialog-title"))
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
                    .text(prompt.default_entry.unwrap_or_default())
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

                dialog.show();
                let current_size = dialog.default_size();
                let new_width = current_size.0.max(400);
                dialog.set_default_size(new_width, current_size.1);

                let response = dialog.run_future().await;
                dialog.close();

                if response == ResponseType::Ok {
                    let _ = tx.send(Ok(entry.text().to_string())).await;
                } else {
                    let _ = tx.send(Err(anyhow!(tr!("error-user-input-canceled")))).await;
                }
            });
        });

        rx.recv().await.context(tr!("error-user-input-canceled"))?
    }
}

#[async_trait::async_trait]
impl SecurePrompt for GtkPrompt {
    async fn get_secure_input(&self, prompt: PromptInfo) -> anyhow::Result<String> {
        self.get_input(prompt, true).await
    }

    async fn get_plain_input(&self, prompt: PromptInfo) -> anyhow::Result<String> {
        self.get_input(prompt, false).await
    }

    async fn show_notification(&self, summary: &str, message: &str) -> anyhow::Result<()> {
        send_notification(summary, message).await
    }
}
