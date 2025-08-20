use std::net::Ipv4Addr;

use tracing::debug;
use tun::AbstractDevice;

pub struct TunDevice {
    inner: Option<tun::AsyncDevice>,
    dev_name: String,
}

impl TunDevice {
    pub fn new(name: &str, ip_address: Ipv4Addr, netmask: Option<Ipv4Addr>, mtu: u16) -> anyhow::Result<Self> {
        let mut config = tun::Configuration::default();

        config.address(ip_address).mtu(mtu).up();

        if cfg!(target_os = "macos") {
            debug!("Ignoring tun device name on macOS: {name}");
        } else {
            config.tun_name(name);
        }

        if let Some(netmask) = netmask {
            config.netmask(netmask);
        }

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
