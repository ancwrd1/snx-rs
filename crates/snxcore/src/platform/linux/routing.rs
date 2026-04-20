use std::{collections::HashSet, net::Ipv4Addr};

use async_trait::async_trait;
use ipnet::Ipv4Net;
use rtnetlink::{
    Handle, RouteMessageBuilder,
    packet_route::{
        IpProtocol,
        rule::{RuleAction, RuleAttribute, RuleFlags, RulePortRange},
    },
};
use sysctl::{Ctl, Sysctl};
use tracing::debug;

use crate::{
    model::params::{TunnelParams, TunnelType},
    platform::{RoutingConfig, RoutingConfigurator},
};

const IP_RULE_TABLE: u32 = 18000;

pub struct LinuxRoutingConfigurator {
    device: String,
    tunnel_type: TunnelType,
}

impl LinuxRoutingConfigurator {
    pub fn new<S: AsRef<str>>(device: S, tunnel_type: TunnelType) -> Self {
        Self {
            device: device.as_ref().to_string(),
            tunnel_type,
        }
    }

    async fn add_route(&self, handle: &Handle, device_index: u32, route: Ipv4Net) -> anyhow::Result<()> {
        debug!("Adding route: {} via {}", route, self.device);

        let message = RouteMessageBuilder::<Ipv4Addr>::new()
            .table_id(IP_RULE_TABLE)
            .destination_prefix(route.network(), route.prefix_len())
            .output_interface(device_index)
            .build();

        handle.route().add(message).execute().await?;
        Ok(())
    }

    async fn add_routes(&self, destination: Ipv4Addr, routes: &[Ipv4Net]) -> anyhow::Result<()> {
        let routes = routes.iter().collect::<HashSet<_>>();

        let handle = super::new_netlink_connection()?;
        let index = super::resolve_device_index(&handle, &self.device).await?;

        for route in routes {
            let _ = self.add_route(&handle, index, *route).await;
        }

        self.add_exclusion_rule(&handle, destination).await?;

        Ok(())
    }

    async fn setup_default_route(&self, destination: Ipv4Addr, disable_ipv6: bool) -> anyhow::Result<()> {
        let handle = super::new_netlink_connection()?;
        let index = super::resolve_device_index(&handle, &self.device).await?;

        // ip route add table 18000 default dev $device
        let message = RouteMessageBuilder::<Ipv4Addr>::new()
            .table_id(IP_RULE_TABLE)
            .output_interface(index)
            .build();

        handle.route().add(message).execute().await?;

        self.add_exclusion_rule(&handle, destination).await?;

        if disable_ipv6 {
            Ctl::new("net.ipv6.conf.all.disable_ipv6")?.set_value_string("1")?;
            Ctl::new("net.ipv6.conf.default.disable_ipv6")?.set_value_string("1")?;
        }

        Ok(())
    }

    async fn add_exclusion_rule(&self, handle: &Handle, destination: Ipv4Addr) -> anyhow::Result<()> {
        // ip rule add not to $dst table 18000
        let mut rule = handle
            .rule()
            .add()
            .v4()
            .table_id(IP_RULE_TABLE)
            .priority(IP_RULE_TABLE)
            .action(RuleAction::ToTable);

        let msg = rule.message_mut();
        msg.header.dst_len = 32;
        msg.header.flags.insert(RuleFlags::Invert);
        msg.attributes.push(RuleAttribute::Destination(destination.into()));

        rule.execute().await?;

        Ok(())
    }

    async fn remove_exclusion_rule(&self, destination: Ipv4Addr, enable_ipv6: bool) -> anyhow::Result<()> {
        let handle = super::new_netlink_connection()?;

        // ip rule del not to $dst table 18000
        let mut rule = handle
            .rule()
            .add()
            .v4()
            .table_id(IP_RULE_TABLE)
            .priority(IP_RULE_TABLE)
            .action(RuleAction::ToTable);

        let msg = rule.message_mut();
        msg.header.dst_len = 32;
        msg.header.flags.insert(RuleFlags::Invert);
        msg.attributes.push(RuleAttribute::Destination(destination.into()));

        handle.rule().del(rule.message_mut().clone()).execute().await?;

        if enable_ipv6 {
            Ctl::new("net.ipv6.conf.all.disable_ipv6")?.set_value_string("0")?;
            Ctl::new("net.ipv6.conf.default.disable_ipv6")?.set_value_string("0")?;
        }

        Ok(())
    }

    async fn add_keepalive_rule(&self, destination: Ipv4Addr, with_dest_route: bool) -> anyhow::Result<()> {
        let handle = super::new_netlink_connection()?;
        let index = super::resolve_device_index(&handle, &self.device).await?;

        if with_dest_route {
            self.add_route(&handle, index, Ipv4Net::new(destination, 32)?).await?;
        }

        for dest_port in [TunnelParams::IPSEC_SCV_PORT, TunnelParams::IPSEC_KEEPALIVE_PORT] {
            // ip rule add to $dst ipproto udp dport $port table 18000
            let mut rule = handle
                .rule()
                .add()
                .v4()
                .table_id(IP_RULE_TABLE)
                .priority(dest_port as u32)
                .action(RuleAction::ToTable);

            let msg = rule.message_mut();
            msg.header.dst_len = 32;
            msg.attributes.push(RuleAttribute::Destination(destination.into()));
            msg.attributes.push(RuleAttribute::IpProtocol(IpProtocol::Udp));
            msg.attributes.push(RuleAttribute::DestinationPortRange(RulePortRange {
                start: dest_port,
                end: dest_port,
            }));
            rule.execute().await?;
        }

        Ok(())
    }

    async fn remove_keepalive_rule(&self, destination: Ipv4Addr) -> anyhow::Result<()> {
        let handle = super::new_netlink_connection()?;

        for dest_port in [TunnelParams::IPSEC_SCV_PORT, TunnelParams::IPSEC_KEEPALIVE_PORT] {
            // ip rule del to $dst ipproto udp dport $port table 18000
            let mut rule = handle
                .rule()
                .add()
                .v4()
                .table_id(IP_RULE_TABLE)
                .priority(dest_port as u32)
                .action(RuleAction::ToTable);
            let msg = rule.message_mut();
            msg.header.dst_len = 32;
            msg.attributes.push(RuleAttribute::Destination(destination.into()));
            msg.attributes.push(RuleAttribute::IpProtocol(IpProtocol::Udp));
            msg.attributes.push(RuleAttribute::DestinationPortRange(RulePortRange {
                start: dest_port,
                end: dest_port,
            }));
            handle.rule().del(rule.message_mut().clone()).execute().await?;
        }

        Ok(())
    }
}

#[async_trait]
impl RoutingConfigurator for LinuxRoutingConfigurator {
    async fn configure(&self, config: &RoutingConfig) -> anyhow::Result<()> {
        debug!("Configuring routing: {:?}", config);

        match config {
            RoutingConfig::Full {
                destination,
                disable_ipv6,
            } => {
                self.setup_default_route(*destination, *disable_ipv6).await?;

                if self.tunnel_type == TunnelType::IPsec {
                    self.add_keepalive_rule(*destination, true).await?;
                }
            }
            RoutingConfig::Split { destination, routes } => {
                self.add_routes(*destination, routes).await?;

                if self.tunnel_type == TunnelType::IPsec {
                    self.add_keepalive_rule(*destination, false).await?;
                }
            }
            RoutingConfig::Cleanup {
                destination,
                enable_ipv6,
            } => {
                self.remove_exclusion_rule(*destination, *enable_ipv6).await?;

                if self.tunnel_type == TunnelType::IPsec {
                    self.remove_keepalive_rule(*destination).await?;
                }
            }
        }

        Ok(())
    }
}
