use std::net::Ipv4Addr;

use ipnet::Ipv4Subnets;
use tracing::debug;
use tun::Device;

use crate::model::HelloReply;

pub struct TunDevice {
    pub(crate) inner: tun::AsyncDevice,
    reply: HelloReply,
    ipaddr: Ipv4Addr,
    dev_name: String,
}

impl TunDevice {
    pub fn new(reply: &HelloReply) -> anyhow::Result<Self> {
        let mut config = tun::Configuration::default();
        let ipaddr = reply.office_mode.ipaddr.parse::<Ipv4Addr>()?;

        config.address(reply.office_mode.ipaddr.as_str()).up();
        if let Some(ref netmask) = reply.optional {
            config.netmask(netmask.subnet.as_str());
        }

        #[cfg(target_os = "linux")]
        config.platform(|config| {
            config.packet_information(true);
        });

        let dev = tun::create_as_async(&config)?;

        let dev_name = dev.get_ref().name().to_owned();

        debug!("Created tun device: {}", dev_name);

        Ok(Self {
            inner: dev,
            reply: reply.clone(),
            dev_name,
            ipaddr,
        })
    }

    pub async fn setup_dns_and_routing<I, S>(&self, search_domains: I) -> anyhow::Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for range in &self.reply.range {
            let subnets = Ipv4Subnets::new(range.from.parse()?, range.to.parse()?, 0);
            for subnet in subnets {
                if subnet.contains(&self.ipaddr) {
                    let snet = subnet.to_string();
                    debug!("Adding route for {}", snet);
                    let _ = crate::net::add_route(&snet, &self.dev_name, &self.ipaddr).await;
                }
            }
        }

        if let Some(ref suffixes) = self.reply.office_mode.dns_suffix {
            debug!("Adding DNS suffixes: {}", suffixes);
            let provided = search_domains
                .into_iter()
                .map(|s| s.as_ref().to_owned())
                .collect::<Vec<_>>();
            let suffixes = suffixes
                .trim_matches('"')
                .split(',')
                .chain(provided.iter().map(|s| s.as_ref()));
            let _ = crate::net::add_dns_suffixes(suffixes, &self.dev_name).await;
        }

        if let Some(ref servers) = self.reply.office_mode.dns_servers {
            debug!("Adding DNS servers: {:?}", servers);
            let _ = crate::net::add_dns_servers(servers, &self.dev_name).await;
        }

        Ok(())
    }
}
