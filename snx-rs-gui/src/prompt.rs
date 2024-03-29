use std::sync::mpsc;

use anyhow::anyhow;
use gtk::{
    glib,
    prelude::{BoxExt, DialogExt, EntryExt, GtkWindowExt, WidgetExt},
    Align, DialogFlags, Orientation, ResponseType,
};
use webkit2gtk::glib::ControlFlow;

use snxcore::{platform, prompt::SecurePrompt};

pub struct GtkPrompt;

impl SecurePrompt for GtkPrompt {
    fn get_secure_input(&self, prompt: &str) -> anyhow::Result<String> {
        let (tx, rx) = mpsc::channel();

        let prompt = prompt.to_owned();

        glib::idle_add(move || {
            let dialog = gtk::Dialog::with_buttons(
                Some("Challenge code"),
                None::<&gtk::Window>,
                DialogFlags::MODAL,
                &[("OK", ResponseType::Ok), ("Cancel", ResponseType::Cancel)],
            );

            dialog.set_default_width(300);
            dialog.set_default_height(120);

            let content = dialog.content_area();
            let inner = gtk::Box::builder().orientation(Orientation::Vertical).margin(6).build();

            inner.pack_start(
                &gtk::Label::builder().label(&prompt).halign(Align::Start).build(),
                false,
                true,
                6,
            );

            let entry = gtk::Entry::builder().visibility(false).build();
            inner.pack_start(&entry, false, true, 6);

            content.pack_start(&inner, false, true, 6);

            dialog.show_all();

            let result = dialog.run();
            dialog.close();

            if result == ResponseType::Ok {
                let _ = tx.send(Ok(entry.text().into()));
            } else {
                let _ = tx.send(Err(anyhow!("User input canceled")));
            }

            ControlFlow::Break
        });

        rx.recv()?
    }

    fn show_notification(&self, summary: &str, message: &str) -> anyhow::Result<()> {
        std::thread::scope(|s| {
            s.spawn(|| snxcore::util::block_on(platform::send_notification(summary, message)))
                .join()
                .unwrap()
        })
    }
}
