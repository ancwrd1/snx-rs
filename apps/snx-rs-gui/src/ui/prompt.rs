use std::rc::Rc;

use anyhow::{Context, anyhow};
use async_channel::Sender;
use i18n::tr;
use secrecy::ExposeSecret;
use slint::{ComponentHandle, LogicalSize, Window};
use snxcore::{
    model::{PromptInfo, params::NotificationLevel},
    profiles::ConnectionProfilesStore,
    prompt::{CombinedInput, NotificationCategory, SaveOffer, SaveTarget, SecurePrompt},
};

use crate::{
    platform::send_notification,
    ui::{PromptWindow, WindowController, WindowScope, close_window, open_window},
};

type PromptResult = anyhow::Result<CombinedInput>;

struct PromptWindowController {
    scope: Rc<WindowScope<PromptWindow>>,
    prompt: PromptInfo,
    user_name_prompt: Option<PromptInfo>,
    save: Option<SaveOffer>,
    secure: bool,
    sender: Sender<PromptResult>,
}

impl PromptWindowController {
    const NAME: &'static str = "prompt";

    fn new(
        prompt: PromptInfo,
        user_name_prompt: Option<PromptInfo>,
        save: Option<SaveOffer>,
        secure: bool,
        sender: Sender<PromptResult>,
    ) -> anyhow::Result<Rc<Self>> {
        Ok(Rc::new(Self {
            scope: WindowScope::new(PromptWindow::new()?),
            prompt,
            user_name_prompt,
            save,
            secure,
            sender,
        }))
    }
}

impl WindowController for PromptWindowController {
    fn present(&self) -> anyhow::Result<()> {
        self.scope.set_globals();

        self.scope.window.set_header(self.prompt.header.as_str().into());
        self.scope.window.set_prompt(trim_prompt(&self.prompt.prompt).into());

        if let Some(ref user_name_prompt) = self.user_name_prompt {
            self.scope.window.set_show_user_name(true);
            self.scope
                .window
                .set_user_name_prompt(trim_prompt(&user_name_prompt.prompt).into());
            self.scope
                .window
                .set_user_name(user_name_prompt.default_entry.as_deref().unwrap_or_default().into());
        }
        self.scope
            .window
            .set_default_entry(self.prompt.default_entry.as_deref().unwrap_or_default().into());
        self.scope.window.set_secure(self.secure);

        if let Some(offer) = self.save {
            self.scope.window.set_show_save(true);
            self.scope.window.set_save_checked(offer.checked);
            self.scope
                .window
                .set_save_in_plain_text(offer.target == SaveTarget::Profile);
        }

        let weak_window = self.scope.window.as_weak();
        self.scope.window.on_fit_height(move || {
            let weak_window = weak_window.clone();
            // deferred until the layout has picked up the toggled content
            slint::Timer::single_shot(std::time::Duration::ZERO, move || {
                if let Some(window) = weak_window.upgrade() {
                    let win = window.window();
                    let width = win.size().to_logical(win.scale_factor()).width;
                    win.set_size(LogicalSize::new(width, window.get_preferred_content_height()));
                }
            });
        });

        let sender = self.sender.clone();
        let with_user_name = self.user_name_prompt.is_some();
        let with_save = self.save.is_some();
        self.scope.window.on_ok_clicked(move |user_name, text, save| {
            let sender = sender.clone();
            let input = CombinedInput {
                user_name: with_user_name.then(|| user_name.to_string()),
                secret: text.as_str().into(),
                save: with_save.then_some(save),
            };
            tokio::spawn(async move { sender.send(Ok(input)).await });
            close_window(Self::NAME);
        });

        let sender = self.sender.clone();
        self.scope.window.on_cancel_clicked(move || {
            let sender = sender.clone();
            tokio::spawn(async move { sender.send(Err(anyhow!(tr!("error-user-input-canceled")))).await });
            close_window(Self::NAME);
        });

        let sender = self.sender.clone();
        self.scope.window.window().on_close_requested(move || {
            let sender = sender.clone();
            tokio::spawn(async move { sender.send(Err(anyhow!(tr!("error-user-input-canceled")))).await });
            close_window(Self::NAME);
            slint::CloseRequestResponse::HideWindow
        });

        self.scope.show()?;

        Ok(())
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&self) {
        self.scope.set_globals();
    }

    fn window(&self) -> &Window {
        self.scope.window.window()
    }
}

#[derive(Clone, Default)]
pub struct SlintPrompt;

impl SlintPrompt {
    pub fn new() -> Self {
        Self
    }

    async fn get_input(
        &self,
        prompt: PromptInfo,
        user_name_prompt: Option<PromptInfo>,
        save: Option<SaveOffer>,
        secure: bool,
    ) -> PromptResult {
        let (tx, rx) = async_channel::bounded(1);

        open_window(PromptWindowController::NAME, move || {
            Ok(PromptWindowController::new(prompt, user_name_prompt, save, secure, tx)?)
        });

        rx.recv().await.with_context(|| tr!("error-user-input-canceled"))?
    }
}

fn trim_prompt(prompt: &str) -> &str {
    prompt.trim_end_matches(|c: char| c.is_whitespace() || c == ':')
}

impl SecurePrompt for SlintPrompt {
    async fn get_secure_input(&self, prompt: PromptInfo) -> anyhow::Result<String> {
        Ok(self
            .get_input(prompt, None, None, true)
            .await?
            .secret
            .expose_secret()
            .to_owned())
    }

    async fn get_plain_input(&self, prompt: PromptInfo) -> anyhow::Result<String> {
        Ok(self
            .get_input(prompt, None, None, false)
            .await?
            .secret
            .expose_secret()
            .to_owned())
    }

    async fn get_combined_input(
        &self,
        prompt: PromptInfo,
        user_name_prompt: Option<PromptInfo>,
        save: Option<SaveOffer>,
    ) -> anyhow::Result<CombinedInput> {
        self.get_input(prompt, user_name_prompt, save, true).await
    }

    async fn show_notification(
        &self,
        summary: &str,
        message: &str,
        category: NotificationCategory,
        level: NotificationLevel,
    ) -> anyhow::Result<()> {
        if level <= ConnectionProfilesStore::instance().get_default().notification_level {
            send_notification(summary, message, category).await
        } else {
            Ok(())
        }
    }
}
