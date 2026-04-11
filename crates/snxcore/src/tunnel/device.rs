use tracing::debug;
use tun::AbstractDevice;

pub struct TunDevice {
    inner: Option<tun::AsyncDevice>,
    dev_name: String,
}

impl TunDevice {
    pub fn new(name: &str) -> anyhow::Result<Self> {
        let mut config = tun::Configuration::default();

        config.tun_name(name).up();

        let dev = tun::create_as_async(&config)?;

        let dev_name = dev.tun_name()?;

        debug!("Created tun device: {dev_name}");

        Ok(Self {
            inner: Some(dev),
            dev_name,
        })
    }

    pub fn name(&self) -> &str {
        &self.dev_name
    }

    pub fn take_inner(&mut self) -> Option<tun::AsyncDevice> {
        self.inner.take()
    }
}
