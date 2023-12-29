use std::sync::Arc;

use anyhow::anyhow;

use crate::{
    model::{
        params::TunnelParams,
        snx::{ClientSettingsResponseData, IpsecResponseData},
    },
    platform::IpsecConfigurator,
};

pub struct BsdIpsecConfigurator;

impl BsdIpsecConfigurator {
    pub fn new(
        _tunnel_params: Arc<TunnelParams>,
        _ipsec_params: IpsecResponseData,
        _client_settings: ClientSettingsResponseData,
    ) -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl IpsecConfigurator for BsdIpsecConfigurator {
    async fn configure(&mut self) -> anyhow::Result<()> {
        Err(anyhow!("Not implemented"))
    }
    async fn cleanup(&mut self) {}
}
