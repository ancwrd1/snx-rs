use std::net::Ipv4Addr;

use crate::platform::ResolverConfig;
use crate::{
    model::{params::TunnelParams, proto::HelloReplyData},
    platform::{self, new_resolver_configurator},
    util,
};
use tracing::debug;
use tun::AbstractDevice;

pub struct TunDevice {
    inner: tun::AsyncDevice,
    reply: HelloReplyData,
    ipaddr: Ipv4Addr,
    dev_name: String,
}

impl TunDevice {
    pub fn new(name: &str, reply: &HelloReplyData) -> anyhow::Result<Self> {
        let mut config = platform::new_tun_config();
        let ipaddr = reply.office_mode.ipaddr.parse::<Ipv4Addr>()?;

        config.address(reply.office_mode.ipaddr.as_str()).up();
        config.tun_name(name);

        if let Some(ref netmask) = reply.optional {
            config.netmask(netmask.subnet.as_str());
        }

        let dev = tun::create_as_async(&config)?;

        let dev_name = dev.tun_name()?;

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

    pub async fn setup_routing(&self, params: &TunnelParams) -> anyhow::Result<()> {
        let dest_ip = util::resolve_ipv4_host(&format!("{}:443", params.server_name))?;
        let mut subnets = params.add_routes.clone();

        if !params.no_routing {
            if params.default_route {
                platform::setup_default_route(&self.dev_name, dest_ip).await?;
            } else {
                subnets.extend(util::ranges_to_subnets(&self.reply.range));
            }
        }

        subnets.retain(|s| !s.contains(&dest_ip));

        if !subnets.is_empty() {
            let _ = platform::add_routes(&subnets, &self.dev_name, self.ipaddr, &params.ignore_routes).await;
        }

        Ok(())
    }

    pub async fn setup_dns(&self, params: &TunnelParams, cleanup: bool) -> anyhow::Result<()> {
        let search_domains = if let Some(ref suffixes) = self.reply.office_mode.dns_suffix {
            suffixes
                .0
                .iter()
                .chain(params.search_domains.iter())
                .filter(|s| {
                    !s.is_empty()
                        && !params
                            .ignore_search_domains
                            .iter()
                            .any(|d| d.to_lowercase() == s.to_lowercase())
                })
                .cloned()
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        let dns_servers = if let Some(ref servers) = self.reply.office_mode.dns_servers {
            servers.clone()
        } else {
            Vec::new()
        };

        let config = ResolverConfig {
            search_domains,
            dns_servers,
        };

        let resolver = new_resolver_configurator(&self.dev_name)?;

        if cleanup {
            resolver.cleanup(&config).await?;
        } else {
            resolver.configure(&config).await?;
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
