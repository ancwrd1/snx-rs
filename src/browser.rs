#[derive(Debug, Clone, Copy)]
enum BrowserType {
    System,
    Webkit,
}

pub struct BrowserController {
    browser_type: BrowserType,
}

impl BrowserController {
    pub fn system() -> Self {
        Self {
            browser_type: BrowserType::System,
        }
    }

    pub fn webkit() -> Self {
        Self {
            browser_type: BrowserType::Webkit,
        }
    }

    pub fn open<S: AsRef<str>>(&self, url: S) -> anyhow::Result<()> {
        match self.browser_type {
            BrowserType::System => Ok(opener::open_browser(url.as_ref())?),
            #[cfg(feature = "gui")]
            BrowserType::Webkit => crate::gui::webkit::open_browser(url.as_ref().to_owned()),
            #[cfg(not(feature = "gui"))]
            BrowserType::Webkit => Err(anyhow!("Webkit feature is not compiled in")),
        }
    }

    pub fn close(&self) -> anyhow::Result<()> {
        match self.browser_type {
            BrowserType::System => {}
            #[cfg(feature = "gui")]
            BrowserType::Webkit => crate::gui::webkit::close_browser(),
            #[cfg(not(feature = "gui"))]
            BrowserType::Webkit => {}
        }
        Ok(())
    }
}
