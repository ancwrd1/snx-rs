use std::sync::Arc;

use tracing::debug;

use crate::{
    model::{
        params::{IkeVersion, TunnelParams, TunnelType},
        proto::LoginOption,
    },
    tunnel::{
        GatewayConnector, TunnelConnector, TunnelConnectorFactory,
        gateway::CccGatewayConnector,
        ipsec::{ikev1::Ikev1TunnelConnector, ikev2::Ikev2TunnelConnector},
        ssl::connector::SslTunnelConnector,
    },
};

#[derive(Clone, Default)]
pub struct CheckPointConnectorFactory {}

impl CheckPointConnectorFactory {
    async fn ike_version(
        &self,
        params: &TunnelParams,
        gateway_connector: &Arc<dyn GatewayConnector + Send + Sync>,
    ) -> anyhow::Result<IkeVersion> {
        match params.ike_version {
            IkeVersion::AutoDetect => {
                let version = if gateway_connector.get_gateway_information().await?.prefers_ikev2() {
                    IkeVersion::V2
                } else {
                    IkeVersion::V1
                };

                debug!("Autodetected IKE version: {}", version);

                Ok(version)
            }
            version => Ok(version),
        }
    }
}

impl TunnelConnectorFactory for CheckPointConnectorFactory {
    async fn new_tunnel_connector(
        &self,
        params: Arc<TunnelParams>,
    ) -> anyhow::Result<Box<dyn TunnelConnector + Send + Sync>> {
        let gateway_connector = self.new_gateway_connector(params.clone());

        let result: anyhow::Result<Box<dyn TunnelConnector + Send + Sync>> = match params.tunnel_type {
            TunnelType::IPsec if params.login_type != LoginOption::MOBILE_ACCESS_ID => {
                match self.ike_version(&params, &gateway_connector).await? {
                    IkeVersion::V2 => Ok(Box::new(
                        Ikev2TunnelConnector::new(params.clone(), gateway_connector).await?,
                    )),
                    _ => Ok(Box::new(
                        Ikev1TunnelConnector::new(params.clone(), gateway_connector).await?,
                    )),
                }
            }
            _ => Ok(Box::new(
                SslTunnelConnector::new(params.clone(), gateway_connector).await?,
            )),
        };
        result
    }

    fn new_gateway_connector(&self, params: Arc<TunnelParams>) -> Arc<dyn GatewayConnector + Send + Sync> {
        Arc::new(CccGatewayConnector::new(params.clone()))
    }
}
