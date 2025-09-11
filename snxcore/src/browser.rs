pub trait BrowserController {
    fn open(&self, url: &str) -> anyhow::Result<()>;
    fn close(&self);
}

pub struct SystemBrowser;

impl BrowserController for SystemBrowser {
    fn open(&self, url: &str) -> anyhow::Result<()> {
        Ok(opener::open(url)?)
    }

    fn close(&self) {}
}
