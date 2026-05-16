use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use anyhow::anyhow;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SystemColorTheme {
    #[default]
    NoPreference,
    Light,
    Dark,
}

impl SystemColorTheme {
    pub fn is_dark(self) -> bool {
        matches!(self, Self::NoPreference | Self::Dark)
    }
}

impl TryFrom<u32> for SystemColorTheme {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(SystemColorTheme::NoPreference),
            1 => Ok(SystemColorTheme::Dark),
            2 => Ok(SystemColorTheme::Light),
            _ => Err(anyhow!(i18n::tr!("error-unknown-color-scheme"))),
        }
    }
}

pub struct ThemeMonitor {
    theme: Arc<AtomicU32>,
}

impl ThemeMonitor {
    pub fn new() -> Self {
        let theme = Arc::new(AtomicU32::new(0));

        crate::platform::spawn_theme_monitor(theme.clone());

        Self { theme }
    }

    pub fn current_theme(&self) -> SystemColorTheme {
        self.theme.load(Ordering::SeqCst).try_into().unwrap_or_default()
    }
}
