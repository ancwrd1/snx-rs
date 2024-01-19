use std::net::Ipv4Addr;

use ipnet::Ipv4Subnets;
use tracing::debug;
use tun::Device;

use crate::model::{params::TunnelParams, proto::HelloReply};

pub struct TunDevice {
    inner: tun::AsyncDevice,
    reply: HelloReply,
    ipaddr: Ipv4Addr,
    dev_name: String,
}

impl TunDevice {
    pub fn new(name: &str, reply: &HelloReply) -> anyhow::Result<Self> {
        let mut config = tun::Configuration::default();
        let ipaddr = reply.office_mode.ipaddr.parse::<Ipv4Addr>()?;

        config.address(reply.office_mode.ipaddr.as_str()).up();
        config.name(name);

        if let Some(ref netmask) = reply.optional {
            config.netmask(netmask.subnet.as_str());
        }

        #[cfg(target_os = "linux")]
        config.platform(|config| {
            config.packet_information(true);
        });

        let dev = tun::create_as_async(&config)?;

        let dev_name = dev.get_ref().name()?;

        debug!("Created tun device: {dev_name}");

        Ok(Self {
            inner: dev,
            reply: reply.clone(),
            dev_name,
            ipaddr,
        })
    }

    pub fn name(&self) -> &str {
        &self.dev_name
    }

    pub fn into_inner(self) -> tun::AsyncDevice {
        self.inner
    }

    pub async fn setup_dns_and_routing(&self, params: &TunnelParams) -> anyhow::Result<()> {
        if !params.no_routing {
            if params.default_route {
                let _ = crate::platform::add_default_route(&self.dev_name, self.ipaddr).await;
            } else {
                for range in &self.reply.range {
                    let subnets = Ipv4Subnets::new(range.from, range.to, 0);
                    for subnet in subnets.into_iter().filter(|s| s.contains(&self.ipaddr)) {
                        crate::platform::add_route(&subnet.to_string(), &self.dev_name, self.ipaddr).await?;
                    }
                }
                for route in &params.add_routes {
                    crate::platform::add_route(route, &self.dev_name, self.ipaddr).await?;
                }
            }
        }

        if !params.no_dns {
            if let Some(ref suffixes) = self.reply.office_mode.dns_suffix {
                debug!("Adding acquired DNS suffixes: {}", suffixes.0);
                debug!("Adding provided DNS suffixes: {:?}", params.search_domains);
                let suffixes = suffixes
                    .0
                    .split(',')
                    .chain(params.search_domains.iter().map(|s| s.as_ref()))
                    .filter(|&s| {
                        !params
                            .ignore_search_domains
                            .iter()
                            .any(|d| d.to_lowercase() == s.to_lowercase())
                    });
                let _ = crate::platform::add_dns_suffixes(suffixes, &self.dev_name).await;
            }

            if let Some(ref servers) = self.reply.office_mode.dns_servers {
                debug!("Adding DNS servers: {servers:?}");
                let _ = crate::platform::add_dns_servers(servers, &self.dev_name).await;
            }
        }

        Ok(())
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
