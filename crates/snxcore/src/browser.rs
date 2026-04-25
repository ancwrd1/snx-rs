use i18n::tr;

use crate::{
    model::PromptInfo,
    prompt::{SecurePrompt, TtyPrompt},
};

#[async_trait::async_trait]
pub trait BrowserController {
    fn open(&self, url: &str) -> anyhow::Result<()>;

    fn close(&self);

    async fn acquire_tunnel_password(&self, url: &str) -> anyhow::Result<String>;
}

pub struct SystemBrowser<P> {
    fallback_prompt: P,
}

impl<P> SystemBrowser<P> {
    pub fn new(fallback_prompt: P) -> Self {
        Self { fallback_prompt }
    }
}

impl Default for SystemBrowser<TtyPrompt> {
    fn default() -> Self {
        SystemBrowser::new(TtyPrompt)
    }
}

#[async_trait::async_trait]
impl<P> BrowserController for SystemBrowser<P>
where
    P: SecurePrompt + Send + Sync + 'static,
{
    fn open(&self, url: &str) -> anyhow::Result<()> {
        Ok(opener::open(url)?)
    }

    fn close(&self) {}

    async fn acquire_tunnel_password(&self, url: &str) -> anyhow::Result<String> {
        let prompt = PromptInfo::new(tr!("cli-mobile-access-auth", url = url), tr!("label-password"));
        self.fallback_prompt.get_secure_input(prompt).await
    }
}
