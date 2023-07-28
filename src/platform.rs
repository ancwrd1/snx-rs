use std::sync::Arc;

#[cfg(target_os = "linux")]
pub use linux::net::{
    add_default_route, add_dns_servers, add_dns_suffixes, add_route, get_default_ip, is_online,
    start_network_state_monitoring,
};
#[cfg(target_os = "macos")]
pub use macos::net::{
    add_default_route, add_dns_servers, add_dns_suffixes, add_route, get_default_ip, is_online,
    start_network_state_monitoring,
};

use crate::model::{
    params::TunnelParams,
    snx::{ClientSettingsResponseData, IpsecResponseData},
};

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

#[async_trait::async_trait]
pub trait IpsecConfigurator {
    async fn configure(&mut self) -> anyhow::Result<()>;
    async fn cleanup(&mut self);
    async fn run_keepalive(&self) -> anyhow::Result<()>;
}

#[cfg(target_os = "linux")]
pub fn new_ipsec_configurator(
    tunnel_params: Arc<TunnelParams>,
    ipsec_params: IpsecResponseData,
    client_settings: ClientSettingsResponseData,
) -> impl IpsecConfigurator {
    linux::xfrm::XfrmConfigurator::new(tunnel_params, ipsec_params, client_settings)
}

#[cfg(target_os = "macos")]
pub fn new_ipsec_configurator(
    tunnel_params: Arc<TunnelParams>,
    ipsec_params: IpsecResponseData,
    client_settings: ClientSettingsResponseData,
) -> impl IpsecConfigurator {
    macos::ipsec::BsdIpsecConfigurator::new(tunnel_params, ipsec_params, client_settings)
}
