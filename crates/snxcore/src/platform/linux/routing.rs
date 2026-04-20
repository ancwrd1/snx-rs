use std::{collections::HashSet, net::Ipv4Addr};

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
    handle: Handle,
    device_index: u32,
    tunnel_type: TunnelType,
}

impl LinuxRoutingConfigurator {
    pub async fn new<S: AsRef<str>>(device: S, tunnel_type: TunnelType) -> anyhow::Result<Self> {
        let handle = super::new_netlink_connection()?;
        let device_index = super::resolve_device_index(&handle, device.as_ref()).await?;

        Ok(Self {
            device: device.as_ref().to_string(),
            handle,
            device_index,
            tunnel_type,
        })
    }

    async fn add_route(&self, route: Ipv4Net) -> anyhow::Result<()> {
        debug!("Adding route: {} via {}", route, self.device);

        let message = RouteMessageBuilder::<Ipv4Addr>::new()
            .table_id(IP_RULE_TABLE)
            .destination_prefix(route.network(), route.prefix_len())
            .output_interface(self.device_index)
            .build();

        super::run_netlink_op(self.handle.route().add(message).execute(), libc::EEXIST).await?;

        Ok(())
    }

    async fn add_routes(&self, routes: &[Ipv4Net]) -> anyhow::Result<()> {
        let routes = routes.iter().collect::<HashSet<_>>();

        for route in routes {
            self.add_route(*route).await?;
        }

        Ok(())
    }

    async fn add_default_route(&self, disable_ipv6: bool) -> anyhow::Result<()> {
        // ip route add table 18000 default dev $device
        let message = RouteMessageBuilder::<Ipv4Addr>::new()
            .table_id(IP_RULE_TABLE)
            .output_interface(self.device_index)
            .build();

        super::run_netlink_op(self.handle.route().add(message).execute(), libc::EEXIST).await?;

        if disable_ipv6 {
            Ctl::new("net.ipv6.conf.all.disable_ipv6")?.set_value_string("1")?;
            Ctl::new("net.ipv6.conf.default.disable_ipv6")?.set_value_string("1")?;
        }

        Ok(())
    }

    async fn add_exclusion_rule(&self, destination: Ipv4Addr) -> anyhow::Result<()> {
        // ip rule add not to $dst table 18000
        let mut rule = self
            .handle
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

        super::run_netlink_op(rule.execute(), libc::EEXIST).await?;

        Ok(())
    }

    async fn remove_exclusion_rule(&self, destination: Ipv4Addr, enable_ipv6: bool) -> anyhow::Result<()> {
        // ip rule del not to $dst table 18000
        let mut rule = self
            .handle
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

        super::run_netlink_op(
            self.handle.rule().del(rule.message_mut().clone()).execute(),
            libc::ENOENT,
        )
        .await?;

        if enable_ipv6 {
            Ctl::new("net.ipv6.conf.all.disable_ipv6")?.set_value_string("0")?;
            Ctl::new("net.ipv6.conf.default.disable_ipv6")?.set_value_string("0")?;
        }

        Ok(())
    }

    async fn add_keepalive_rule(&self, destination: Ipv4Addr) -> anyhow::Result<()> {
        for dest_port in [TunnelParams::IPSEC_SCV_PORT, TunnelParams::IPSEC_KEEPALIVE_PORT] {
            // ip rule add to $dst ipproto udp dport $port table 18000
            let mut rule = self
                .handle
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

            super::run_netlink_op(rule.execute(), libc::EEXIST).await?;
        }

        Ok(())
    }

    async fn remove_keepalive_rule(&self, destination: Ipv4Addr, port: u16) -> anyhow::Result<()> {
        // ip rule del to $dst ipproto udp dport $port table 18000
        let mut rule = self
            .handle
            .rule()
            .add()
            .v4()
            .table_id(IP_RULE_TABLE)
            .priority(port as u32)
            .action(RuleAction::ToTable);

        let msg = rule.message_mut();
        msg.header.dst_len = 32;
        msg.attributes.push(RuleAttribute::Destination(destination.into()));
        msg.attributes.push(RuleAttribute::IpProtocol(IpProtocol::Udp));
        msg.attributes.push(RuleAttribute::DestinationPortRange(RulePortRange {
            start: port,
            end: port,
        }));

        super::run_netlink_op(
            self.handle.rule().del(rule.message_mut().clone()).execute(),
            libc::ENOENT,
        )
        .await?;

        Ok(())
    }
}

impl RoutingConfigurator for LinuxRoutingConfigurator {
    async fn configure(&self, config: &RoutingConfig) -> anyhow::Result<()> {
        match config {
            RoutingConfig::Full {
                destination,
                disable_ipv6,
            } => {
                debug!("Configuring full routing via {}", self.device);

                self.add_exclusion_rule(*destination).await?;
                self.add_default_route(*disable_ipv6).await?;

                if self.tunnel_type == TunnelType::IPsec {
                    self.add_route(Ipv4Net::new(*destination, 32)?).await?;
                    self.add_keepalive_rule(*destination).await?;
                }
            }
            RoutingConfig::Split { destination, routes } => {
                debug!("Configuring split routing via {}", self.device);

                self.add_exclusion_rule(*destination).await?;
                self.add_routes(routes).await?;

                if self.tunnel_type == TunnelType::IPsec {
                    self.add_keepalive_rule(*destination).await?;
                }
            }
            RoutingConfig::Cleanup {
                destination,
                enable_ipv6,
            } => {
                debug!("Cleaning up routing for {}", self.device);

                self.remove_exclusion_rule(*destination, *enable_ipv6).await?;

                if self.tunnel_type == TunnelType::IPsec {
                    for dest_port in [TunnelParams::IPSEC_SCV_PORT, TunnelParams::IPSEC_KEEPALIVE_PORT] {
                        self.remove_keepalive_rule(*destination, dest_port).await?;
                    }
                }
            }
        }

        Ok(())
    }
}
