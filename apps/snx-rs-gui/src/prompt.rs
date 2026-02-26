use anyhow::{Context, anyhow};
use gtk4::{
    Align, Orientation,
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
                let window = gtk4::Window::builder()
                    .title(tr!("auth-dialog-title"))
                    .transient_for(&main_window())
                    .modal(true)
                    .build();

                let ok = gtk4::Button::builder().label(tr!("button-ok")).build();
                let cancel = gtk4::Button::builder().label(tr!("button-cancel")).build();

                let button_box = gtk4::Box::builder()
                    .orientation(Orientation::Horizontal)
                    .spacing(6)
                    .margin_top(6)
                    .margin_start(6)
                    .margin_end(6)
                    .margin_bottom(6)
                    .homogeneous(true)
                    .halign(Align::End)
                    .valign(Align::End)
                    .build();

                button_box.append(&ok);
                button_box.append(&cancel);

                let inner = gtk4::Box::builder()
                    .orientation(Orientation::Vertical)
                    .margin_top(6)
                    .margin_start(6)
                    .margin_end(6)
                    .margin_bottom(6)
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

                inner.append(&entry);

                let outer_box = gtk4::Box::builder().orientation(Orientation::Vertical).build();
                outer_box.append(&inner);
                outer_box.append(&button_box);

                window.set_child(Some(&outer_box));
                window.set_default_widget(Some(&ok));

                let tx_ok = tx.clone();
                ok.connect_clicked(clone!(
                    #[weak]
                    window,
                    #[weak]
                    entry,
                    move |_| {
                        let _ = tx_ok.try_send(Ok(entry.text().to_string()));
                        window.close();
                    }
                ));

                let tx_cancel = tx.clone();
                cancel.connect_clicked(clone!(
                    #[weak]
                    window,
                    move |_| {
                        let _ = tx_cancel.try_send(Err(anyhow!(tr!("error-user-input-canceled"))));
                        window.close();
                    }
                ));

                let tx_entry = tx.clone();
                entry.connect_activate(clone!(
                    #[weak]
                    window,
                    #[weak]
                    entry,
                    move |_| {
                        let _ = tx_entry.try_send(Ok(entry.text().to_string()));
                        window.close();
                    }
                ));

                window.connect_close_request(move |_| {
                    let _ = tx.try_send(Err(anyhow!(tr!("error-user-input-canceled"))));
                    glib::Propagation::Proceed
                });

                {
                    let key_controller = gtk4::EventControllerKey::new();
                    key_controller.connect_key_pressed(clone!(
                        #[weak]
                        window,
                        #[upgrade_or]
                        glib::Propagation::Proceed,
                        move |_, key, _, _| {
                            if key == gtk4::gdk::Key::Escape {
                                window.close();
                                glib::Propagation::Stop
                            } else {
                                glib::Propagation::Proceed
                            }
                        }
                    ));
                    window.add_controller(key_controller);
                }

                window.present();
                let current_size = window.default_size();
                let new_width = current_size.0.max(400);
                window.set_default_size(new_width, current_size.1);
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
