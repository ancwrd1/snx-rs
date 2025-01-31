use std::net::Ipv4Addr;

use crate::platform;
use tracing::debug;
use tun::AbstractDevice;

pub struct TunDevice {
    inner: Option<tun::AsyncDevice>,
    dev_name: String,
}

impl TunDevice {
    pub fn new(name: &str, ip_address: Ipv4Addr, netmask: Option<Ipv4Addr>) -> anyhow::Result<Self> {
        let mut config = platform::new_tun_config();

        config.address(ip_address).up();
        config.tun_name(name);

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

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use ipnet::Ipv4Subnets;

    use crate::model::proto::NetworkRange;

    #[test]
    fn parse_range() {
        let ipaddr = "10.0.10.10".parse::<Ipv4Addr>().unwrap();
        let range = NetworkRange {
            from: "10.0.0.0".parse().unwrap(),
            to: "10.255.255.255".parse().unwrap(),
        };

        let subnets = Ipv4Subnets::new(range.from, range.to, 0);
        assert!(subnets.clone().any(|s| s.contains(&ipaddr)));

        for subnet in subnets {
            assert_eq!(subnet.to_string(), "10.0.0.0/8");
        }
    }
}
