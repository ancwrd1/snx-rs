use i18n::tr;

use crate::{
    model::PromptInfo,
    prompt::{SecurePrompt, TtyPrompt},
};

#[async_trait::async_trait]
pub trait BrowserController {
    fn open(&self, url: &str) -> anyhow::Result<()>;

    fn close(&self);

    async fn acquire_access_cookie(&self, url: &str) -> anyhow::Result<String>;
}

pub struct SystemBrowser;

#[async_trait::async_trait]
impl BrowserController for SystemBrowser {
    fn open(&self, url: &str) -> anyhow::Result<()> {
        Ok(opener::open(url)?)
    }

    fn close(&self) {}

    async fn acquire_access_cookie(&self, url: &str) -> anyhow::Result<String> {
        println!("{}", tr!("cli-mobile-access-auth"));
        let prompt = PromptInfo::new(url, tr!("label-password"));
        TtyPrompt.get_secure_input(prompt).await
    }
}
