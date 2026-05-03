use std::{collections::VecDeque, rc::Rc};

use anyhow::{Context, anyhow};
use async_channel::Sender;
use i18n::tr;
use slint::ComponentHandle;
use snxcore::{
    model::{PromptInfo, params::TunnelParams},
    prompt::SecurePrompt,
    server_info,
};

use crate::{
    dbus::send_notification,
    ui::{PromptWindow, WindowController, WindowScope, close_window, open_window},
};

struct PromptWindowController {
    scope: Rc<WindowScope<PromptWindow>>,
    prompt: PromptInfo,
    secure: bool,
    sender: Sender<anyhow::Result<String>>,
}

impl PromptWindowController {
    const NAME: &'static str = "prompt";

    fn new(prompt: PromptInfo, secure: bool, sender: Sender<anyhow::Result<String>>) -> anyhow::Result<Rc<Self>> {
        Ok(Rc::new(Self {
            scope: WindowScope::new(PromptWindow::new()?),
            prompt,
            secure,
            sender,
        }))
    }
}

impl WindowController for PromptWindowController {
    fn present(&self) -> anyhow::Result<()> {
        self.scope.set_globals();

        self.scope.window.set_header(self.prompt.header.as_str().into());
        self.scope.window.set_prompt(
            self.prompt
                .prompt
                .trim_end_matches(|c: char| c.is_whitespace() || c == ':')
                .into(),
        );
        self.scope
            .window
            .set_default_entry(self.prompt.default_entry.as_deref().unwrap_or_default().into());
        self.scope.window.set_secure(self.secure);

        let sender = self.sender.clone();
        self.scope.window.on_ok_clicked(move |text| {
            let sender = sender.clone();
            tokio::spawn(async move { sender.send(Ok(text.to_string())).await });
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

pub struct SlintPrompt;

impl SlintPrompt {
    async fn get_input(&self, prompt: PromptInfo, secure: bool) -> anyhow::Result<String> {
        let (tx, rx) = async_channel::bounded(1);

        open_window(PromptWindowController::NAME, move || {
            Ok(PromptWindowController::new(prompt, secure, tx)?)
        });

        rx.recv().await.with_context(|| tr!("error-user-input-canceled"))?
    }
}

#[async_trait::async_trait]
impl SecurePrompt for SlintPrompt {
    async fn get_secure_input(&self, prompt: PromptInfo) -> anyhow::Result<String> {
        self.get_input(prompt, true).await
    }

    async fn get_plain_input(&self, prompt: PromptInfo) -> anyhow::Result<String> {
        self.get_input(prompt, false).await
    }

    async fn show_notification(&self, summary: &str, message: &str) -> anyhow::Result<()> {
        send_notification(summary, message).await
    }

    async fn get_server_prompts(&self, params: &TunnelParams) -> anyhow::Result<VecDeque<PromptInfo>> {
        server_info::get_login_prompts(params).await
    }
}
