use std::net::Ipv4Addr;

use tracing::debug;
use tun::Device;

use crate::model::{params::TunnelParams, snx::HelloReply};

pub struct TunDevice {
    inner: tun::AsyncDevice,
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

    pub fn name(&self) -> &str {
        self.inner.get_ref().name()
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
                    let _ = crate::platform::add_route(range, &self.dev_name, self.ipaddr).await;
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
                    .chain(params.search_domains.iter().map(|s| s.as_ref()));
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
