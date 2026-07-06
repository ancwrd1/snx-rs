use std::net::Ipv4Addr;

use async_trait::async_trait;

use crate::{
    model::IPsecSession,
    platform::{DeviceConfig, IPsecConfigurator},
};

pub struct MacosIPsecStub {
    _device_config: DeviceConfig,
    _ipsec_session: IPsecSession,
    _src_ip: Ipv4Addr,
    _src_port: u16,
    _dest_ip: Ipv4Addr,
    _dest_port: u16,
}

impl MacosIPsecStub {
    pub fn new(
        device_config: DeviceConfig,
        ipsec_session: IPsecSession,
        src_ip: Ipv4Addr,
        src_port: u16,
        dest_ip: Ipv4Addr,
        dest_port: u16,
    ) -> Self {
        Self {
            _device_config: device_config,
            _ipsec_session: ipsec_session,
            _src_ip: src_ip,
            _src_port: src_port,
            _dest_ip: dest_ip,
            _dest_port: dest_port,
        }
    }
}

// Userspace ESP (ipsec_native = false) handles the data path, so there is no kernel SA to configure.
#[async_trait]
impl IPsecConfigurator for MacosIPsecStub {
    async fn configure(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn rekey(&mut self, _session: &IPsecSession) -> anyhow::Result<()> {
        Ok(())
    }

    async fn cleanup(&self) {}
}
