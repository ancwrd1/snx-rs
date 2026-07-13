use std::sync::Arc;

use tracing::debug;
use tun::AbstractDevice;

pub struct TunDevice {
    inner: Arc<tun::AsyncDevice>,
    dev_name: String,
}

impl TunDevice {
    pub fn new(name: &str) -> anyhow::Result<Self> {
        let mut config = tun::Configuration::default();

        // macOS utun devices must be named `utunN`; the custom hint is rejected, so let the
        // kernel assign a name (read back below). Linux/Windows keep honoring the hint.
        #[cfg(not(target_os = "macos"))]
        config.tun_name(name).up();
        #[cfg(target_os = "macos")]
        {
            let _ = name;
            config.up();
        }

        let dev = tun::create_as_async(&config)?;

        let dev_name = dev.tun_name()?;

        debug!("Created tun device: {dev_name}");

        Ok(Self {
            inner: Arc::new(dev),
            dev_name,
        })
    }

    pub fn name(&self) -> &str {
        &self.dev_name
    }

    pub fn inner(&self) -> Arc<tun::AsyncDevice> {
        self.inner.clone()
    }
}
