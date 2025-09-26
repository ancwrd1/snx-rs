use std::{collections::HashSet, net::Ipv4Addr};

use async_trait::async_trait;
use ipnet::Ipv4Net;
use tracing::debug;

use crate::{model::params::TunnelParams, platform::RoutingConfigurator};

pub struct LinuxRoutingConfigurator {
    device: String,
}

impl LinuxRoutingConfigurator {
    pub fn new<S: AsRef<str>>(device: S, _address: Ipv4Addr) -> Self {
        Self {
            device: device.as_ref().to_string(),
        }
    }

    async fn add_route(&self, route: Ipv4Net) -> anyhow::Result<()> {
        debug!("Adding route: {} via {}", route, self.device);
        crate::util::run_command("ip", ["route", "add", &route.to_string(), "dev", &self.device]).await?;
        Ok(())
    }
}

#[async_trait]
impl RoutingConfigurator for LinuxRoutingConfigurator {
    async fn add_routes(&self, routes: &[Ipv4Net], ignore_routes: &[Ipv4Net]) -> anyhow::Result<()> {
        let routes = routes.iter().collect::<HashSet<_>>();
        debug!("Routes to add: {:?}", routes);

        for route in routes {
            if ignore_routes.iter().any(|ignore| ignore == route) {
                debug!("Ignoring route: {}", route);
                continue;
            }
            let _ = self.add_route(*route).await;
        }

        Ok(())
    }

    async fn setup_default_route(&self, destination: Ipv4Addr, disable_ipv6: bool) -> anyhow::Result<()> {
        debug!(
            "Setting up default route through {}, disable IPv6: {disable_ipv6}",
            self.device
        );

        let port = TunnelParams::IPSEC_KEEPALIVE_PORT.to_string();
        let dst = destination.to_string();

        crate::util::run_command("ip", ["route", "add", "table", &port, "default", "dev", &self.device]).await?;
        crate::util::run_command("ip", ["rule", "add", "not", "to", &dst, "table", &port]).await?;

        if disable_ipv6 {
            super::sysctl("net.ipv6.conf.all.disable_ipv6", "1")?;
            super::sysctl("net.ipv6.conf.default.disable_ipv6", "1")?;
        }

        Ok(())
    }

    async fn setup_keepalive_route(&self, destination: Ipv4Addr, with_table: bool) -> anyhow::Result<()> {
        debug!("Setting up keepalive route through {}", self.device);

        let port = TunnelParams::IPSEC_KEEPALIVE_PORT.to_string();
        let dst = destination.to_string();

        if with_table {
            crate::util::run_command("ip", &["route", "add", "table", &port, &dst, "dev", &self.device]).await?;
        }

        crate::util::run_command(
            "ip",
            &[
                "rule", "add", "to", &dst, "ipproto", "udp", "dport", &port, "table", &port,
            ],
        )
        .await?;

        Ok(())
    }

    async fn remove_default_route(&self, destination: Ipv4Addr, enable_ipv6: bool) -> anyhow::Result<()> {
        let port = TunnelParams::IPSEC_KEEPALIVE_PORT.to_string();
        let dst = destination.to_string();

        crate::util::run_command("ip", ["rule", "del", "not", "to", &dst, "table", &port]).await?;

        if enable_ipv6 {
            super::sysctl("net.ipv6.conf.all.disable_ipv6", "0")?;
            super::sysctl("net.ipv6.conf.default.disable_ipv6", "0")?;
        }

        Ok(())
    }

    async fn remove_keepalive_route(&self, destination: Ipv4Addr) -> anyhow::Result<()> {
        let port = TunnelParams::IPSEC_KEEPALIVE_PORT.to_string();
        let dst = destination.to_string();

        crate::util::run_command(
            "ip",
            &[
                "rule", "del", "to", &dst, "ipproto", "udp", "dport", &port, "table", &port,
            ],
        )
        .await?;

        Ok(())
    }
}
