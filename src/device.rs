use std::net::Ipv4Addr;

use ipnet::Ipv4Subnets;
use tracing::debug;

use crate::model::HelloReply;

pub struct TunDevice {
    pub(crate) inner: tun::AsyncDevice,
    reply: HelloReply,
    dev_name: String,
    ipaddr: Ipv4Addr,
}

impl TunDevice {
    pub fn new<S: AsRef<str>>(name: S, reply: &HelloReply) -> anyhow::Result<Self> {
        let mut config = tun::Configuration::default();
        let ipaddr = reply.office_mode.ipaddr.parse::<Ipv4Addr>()?;

        config
            .name(name.as_ref())
            .address(reply.office_mode.ipaddr.as_str())
            .up();
        if let Some(ref netmask) = reply.optional {
            config.netmask(netmask.subnet.as_str());
        }

        #[cfg(target_os = "linux")]
        config.platform(|config| {
            config.packet_information(true);
        });

        debug!(
            "Configuring tun device with name {}: {:?}",
            name.as_ref(),
            config
        );

        let dev = tun::create_as_async(&config)?;

        Ok(Self {
            inner: dev,
            reply: reply.clone(),
            dev_name: name.as_ref().to_owned(),
            ipaddr,
        })
    }

    pub async fn setup_dns_and_routing(&self) -> anyhow::Result<()> {
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
            let _ =
                crate::net::add_dns_suffixes(suffixes.trim_matches('"').split(','), &self.dev_name)
                    .await;
        }

        if let Some(ref servers) = self.reply.office_mode.dns_servers {
            debug!("Adding DNS servers: {:?}", servers);
            let _ = crate::net::add_dns_servers(servers, &self.dev_name).await;
        }

        Ok(())
    }
}
