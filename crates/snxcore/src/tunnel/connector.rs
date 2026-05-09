use std::sync::Arc;

use crate::{
    model::{
        params::{TunnelParams, TunnelType},
        proto::LoginOption,
    },
    tunnel::{
        GatewayConnector, TunnelConnector, TunnelConnectorFactory, gateway::CccGatewayConnector,
        ipsec::connector::IPsecTunnelConnector, ssl::connector::SslTunnelConnector,
    },
};

#[derive(Clone, Default)]
pub struct CheckPointConnectorFactory {}

impl TunnelConnectorFactory for CheckPointConnectorFactory {
    async fn new_tunnel_connector(
        &self,
        params: Arc<TunnelParams>,
    ) -> anyhow::Result<Box<dyn TunnelConnector + Send + Sync>> {
        let result: anyhow::Result<Box<dyn TunnelConnector + Send + Sync>> = match params.tunnel_type {
            TunnelType::IPsec if params.login_type != LoginOption::MOBILE_ACCESS_ID => Ok(Box::new(
                IPsecTunnelConnector::new(params.clone(), self.new_gateway_connector(params)).await?,
            )),
            _ => Ok(Box::new(
                SslTunnelConnector::new(params.clone(), self.new_gateway_connector(params)).await?,
            )),
        };
        result
    }

    fn new_gateway_connector(&self, params: Arc<TunnelParams>) -> Arc<dyn GatewayConnector + Send + Sync> {
        Arc::new(CccGatewayConnector::new(params.clone()))
    }
}
