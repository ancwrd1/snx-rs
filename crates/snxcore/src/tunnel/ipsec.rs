use crate::{
    model::{IpsecSession, params::TunnelParams},
    platform::{Platform, PlatformAccess, ResolverConfig, SearchDomain},
};

pub mod connector;
pub mod imp;
pub mod keepalive;
pub mod natt;
pub mod scv;

pub async fn make_resolver_config(session: &IpsecSession, params: &TunnelParams) -> ResolverConfig {
    let features = Platform::get().get_features().await;

    let search_domains = session
        .domains
        .iter()
        .map(|d| SearchDomain::new(d, params.set_routing_domains && features.split_dns))
        .chain(params.search_domains.iter().map(|d| {
            if let Some(s) = d.strip_prefix("~") {
                SearchDomain::new(s, features.split_dns)
            } else {
                SearchDomain::new(d, params.set_routing_domains && features.split_dns)
            }
        }))
        .filter(|s| {
            !s.name.is_empty()
                && !params
                    .ignore_search_domains
                    .iter()
                    .any(|d| d.eq_ignore_ascii_case(&s.name))
        })
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
