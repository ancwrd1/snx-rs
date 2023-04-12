use std::net::Ipv4Addr;

use ipnet::Ipv4Subnets;
use tracing::debug;
use tun::Device;

use crate::model::HelloReply;
use crate::params::TunnelParams;

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

        debug!("Created tun device: {dev_name}");

        Ok(Self {
            inner: dev,
            reply: reply.clone(),
            dev_name,
            ipaddr,
        })
    }

    pub async fn setup_dns_and_routing(&self, params: &TunnelParams) -> anyhow::Result<()> {
        if !params.no_routing {
            if params.default_route {
                debug!("Setting default route for {}", self.dev_name);
                let _ = crate::net::add_route("default", &self.dev_name, &self.ipaddr).await;
            } else {
                for range in &self.reply.range {
                    let subnets = Ipv4Subnets::new(range.from.parse()?, range.to.parse()?, 0);
                    for subnet in subnets {
                        if subnet.contains(&self.ipaddr) {
                            let snet = subnet.to_string();
                            debug!("Adding route for {snet}");
                            let _ =
                                crate::net::add_route(&snet, &self.dev_name, &self.ipaddr).await;
                        }
                    }
                }
            }
        }

        if !params.no_dns {
            if let Some(ref suffixes) = self.reply.office_mode.dns_suffix {
                debug!("Adding acquired DNS suffixes: {suffixes}");
                debug!("Adding provided DNS suffixes: {:?}", params.search_domains);
                let suffixes = suffixes
                    .trim_matches('"')
                    .split(',')
                    .chain(params.search_domains.iter().map(|s| s.as_ref()));
                let _ = crate::net::add_dns_suffixes(suffixes, &self.dev_name).await;
            }

            if let Some(ref servers) = self.reply.office_mode.dns_servers {
                debug!("Adding DNS servers: {servers:?}");
                let _ = crate::net::add_dns_servers(servers, &self.dev_name).await;
            }
        }

        Ok(())
    }
}
