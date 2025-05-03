use crate::model::IpsecSession;
use crate::model::params::TunnelParams;
use crate::platform::ResolverConfig;

pub mod connector;
pub mod imp;
pub mod keepalive;
pub mod natt;

pub fn make_resolver_config(session: &IpsecSession, params: &TunnelParams) -> ResolverConfig {
    let search_domains = session
        .domains
        .iter()
        .chain(&params.search_domains)
        .filter(|s| {
            !s.is_empty()
                && !params
                    .ignore_search_domains
                    .iter()
                    .any(|d| d.to_lowercase() == s.trim_matches('~').to_lowercase())
        })
        .cloned()
        .collect::<Vec<_>>();

    let dns_servers = session
        .dns
        .iter()
        .chain(&params.dns_servers)
        .filter(|s| !params.ignore_dns_servers.iter().any(|d| *d == **s))
        .cloned()
        .collect::<Vec<_>>();

    ResolverConfig {
        search_domains,
        dns_servers,
    }
}
