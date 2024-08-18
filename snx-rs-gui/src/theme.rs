use std::sync::atomic::{AtomicU32, Ordering};

use anyhow::anyhow;
use futures::StreamExt;
use once_cell::sync::Lazy;
use tokio::runtime::Runtime;
use tracing::debug;
use zbus::Connection;

use crate::dbus::DesktopSettingsProxy;

static COLOR_THEME: AtomicU32 = AtomicU32::new(0);

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
            _ => Err(anyhow!("Unknown color-scheme value")),
        }
    }
}

pub fn system_color_theme() -> anyhow::Result<SystemColorTheme> {
    COLOR_THEME.load(Ordering::SeqCst).try_into()
}

pub fn init_theme_monitoring() -> anyhow::Result<()> {
    static RT: Lazy<Runtime> = Lazy::new(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
    });

    RT.block_on(async {
        let connection = Connection::session().await?;
        let proxy = DesktopSettingsProxy::new(&connection).await?;
        let scheme = proxy.read_one("org.freedesktop.appearance", "color-scheme").await?;
        let scheme = u32::try_from(scheme)?;
        COLOR_THEME.store(scheme, Ordering::SeqCst);

        debug!("System color scheme: {}", scheme);

        tokio::spawn(async move {
            let mut stream = proxy.receive_setting_changed().await?;
            while let Some(signal) = stream.next().await {
                let args = signal.args()?;
                if args.namespace == "org.freedesktop.appearance" && args.key == "color-scheme" {
                    let scheme = u32::try_from(args.value)?;
                    debug!("New system color scheme: {}", scheme);
                    COLOR_THEME.store(scheme, Ordering::SeqCst);
                }
            }
            Ok::<_, anyhow::Error>(())
        });

        Ok(())
    })
}
