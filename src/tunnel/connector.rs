use std::time::Duration;
use std::{
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
};

use anyhow::anyhow;
use async_trait::async_trait;
use byteorder::{BigEndian, ReadBytesExt};
use bytes::{Buf, Bytes};
use isakmp::{
    ikev1::Ikev1,
    model::ConfigAttributeType,
    payload::AttributesPayload,
    session::{EspCryptMaterial, Ikev1Session},
    transport::UdpTransport,
};
use parking_lot::RwLock;
use rand::random;
use tokio::net::UdpSocket;
use tracing::{debug, warn};

use crate::{
    ccc::CccHttpClient,
    model::{params::TunnelParams, proto::AuthResponse, CccSession, IpsecSession, MfaChallenge, MfaType, SessionState},
    platform,
    sexpr2::SExpression,
    tunnel::{ipsec::IpsecTunnel, ssl::SslTunnel, CheckpointTunnel, TunnelConnector},
};

pub struct CccTunnelConnector(Arc<TunnelParams>);

impl CccTunnelConnector {
    pub async fn new(params: Arc<TunnelParams>) -> anyhow::Result<Self> {
        Ok(Self(params))
    }

    async fn process_auth_response(&self, data: AuthResponse) -> anyhow::Result<Arc<CccSession>> {
        let session_id = data.session_id.unwrap_or_default();

        match data.authn_status.as_str() {
            "continue" => {
                return Ok(Arc::new(CccSession {
                    session_id,
                    state: SessionState::PendingChallenge(MfaChallenge {
                        mfa_type: MfaType::UserInput,
                        prompt: data.prompt.map(|p| p.0).unwrap_or_default(),
                    }),
                    ipsec_session: None,
                }))
            }
            "done" => {}
            other => {
                warn!("Authn status: {}", other);
                return Err(anyhow!("Authentication failed!"));
            }
        }

        let active_key = match (data.is_authenticated, data.active_key) {
            (Some(true), Some(ref key)) => key.clone(),
            _ => {
                let msg = match (data.error_message, data.error_id, data.error_code) {
                    (Some(message), Some(id), Some(code)) => format!("[{} {}] {}", code, id.0, message.0),
                    _ => "Authentication failed!".to_owned(),
                };
                warn!("{}", msg);
                return Err(anyhow!(msg));
            }
        };

        debug!("Authentication OK, session id: {session_id}");

        let session = Arc::new(CccSession {
            session_id,
            state: SessionState::Authenticated(active_key.0),
            ipsec_session: None,
        });

        let client = CccHttpClient::new(self.0.clone(), Some(session.clone()));
        if let Ok(ipsec_params) = client.get_ipsec_tunnel_params(random()).await {
            let esp_in = EspCryptMaterial {
                spi: ipsec_params.client_decsa.spi,
                sk_e: hex::decode(ipsec_params.client_decsa.enckey.0.as_bytes())?.into(),
                sk_a: hex::decode(ipsec_params.client_decsa.authkey.0.as_bytes())?.into(),
            };
            let esp_out = EspCryptMaterial {
                spi: ipsec_params.client_encsa.spi,
                sk_e: hex::decode(ipsec_params.client_encsa.enckey.0.as_bytes())?.into(),
                sk_a: hex::decode(ipsec_params.client_encsa.authkey.0.as_bytes())?.into(),
            };

            let ipsec_session = IpsecSession {
                address: ipsec_params.om_addr.into(),
                netmask: ipsec_params.om_subnet_mask.into(),
                dns: [ipsec_params.om_dns0, ipsec_params.om_dns1, ipsec_params.om_dns2]
                    .into_iter()
                    .flatten()
                    .filter(|d| *d != 0)
                    .map(Into::into)
                    .collect(),
                domains: ipsec_params
                    .om_domain_name
                    .as_ref()
                    .map(|s| s.0.split(',').map(|s| s.trim().to_owned()).collect())
                    .unwrap_or_default(),
                esp_in,
                esp_out,
            };

            Ok(Arc::new(CccSession {
                ipsec_session: Some(ipsec_session),
                ..(*session).clone()
            }))
        } else {
            Ok(session)
        }
    }
}

#[async_trait]
impl TunnelConnector for CccTunnelConnector {
    async fn authenticate(&mut self) -> anyhow::Result<Arc<CccSession>> {
        debug!("Authenticating to endpoint: {}", self.0.server_name);
        let client = CccHttpClient::new(self.0.clone(), None);

        let data = client.authenticate().await?;

        self.process_auth_response(data).await
    }

    async fn challenge_code(&mut self, session: Arc<CccSession>, user_input: &str) -> anyhow::Result<Arc<CccSession>> {
        debug!("Authenticating with challenge code to endpoint: {}", self.0.server_name);
        let client = CccHttpClient::new(self.0.clone(), Some(session));

        let data = client.challenge_code(user_input).await?;

        self.process_auth_response(data).await
    }

    async fn create_tunnel(&self, session: Arc<CccSession>) -> anyhow::Result<Box<dyn CheckpointTunnel + Send>> {
        Ok(Box::new(SslTunnel::create(self.0.clone(), session).await?))
    }
}

pub struct IpsecTunnelConnector {
    params: Arc<TunnelParams>,
    ikev1: Ikev1<UdpTransport>,
    session: Arc<RwLock<Ikev1Session>>,
    gateway_address: Ipv4Addr,
    last_message_id: u32,
    last_identifier: u16,
    last_challenge_type: ConfigAttributeType,
    ccc_session: String,
}

impl IpsecTunnelConnector {
    pub async fn new(params: Arc<TunnelParams>) -> anyhow::Result<Self> {
        let session = Arc::new(RwLock::new(Ikev1Session::new()?));

        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect(format!("{}:4500", params.server_name)).await?;

        let gateway_address = match socket.peer_addr()?.ip() {
            IpAddr::V4(v4) => v4,
            _ => return Err(anyhow!("No IPv4 address for {}", params.server_name)),
        };

        let transport = UdpTransport::new(socket, session.clone());

        let ikev1 = Ikev1::new(transport, session.clone())?;

        Ok(Self {
            params,
            ikev1,
            session,
            gateway_address,
            last_message_id: 0,
            last_identifier: 0,
            last_challenge_type: ConfigAttributeType::Other(0),
            ccc_session: String::new(),
        })
    }

    fn get_challenge_attribute_type(&self, payload: &AttributesPayload) -> ConfigAttributeType {
        payload
            .attributes
            .iter()
            .find_map(|a| {
                let attr: ConfigAttributeType = a.attribute_type.into();
                if attr != ConfigAttributeType::AuthType
                    && attr != ConfigAttributeType::Challenge
                    && attr != ConfigAttributeType::Status
                {
                    Some(attr)
                } else {
                    None
                }
            })
            .unwrap_or(ConfigAttributeType::Other(0))
    }

    fn get_long_attributes(&self, payload: &AttributesPayload, attr: ConfigAttributeType) -> Vec<Bytes> {
        let attr: u16 = attr.into();
        payload
            .attributes
            .iter()
            .filter_map(|a| {
                if a.attribute_type == attr {
                    a.as_long().cloned()
                } else {
                    None
                }
            })
            .collect()
    }

    fn get_long_attribute(&self, payload: &AttributesPayload, attr: ConfigAttributeType) -> Option<Bytes> {
        self.get_long_attributes(payload, attr).first().cloned()
    }

    fn get_short_attribute(&self, payload: &AttributesPayload, attr: ConfigAttributeType) -> Option<u16> {
        let attr: u16 = attr.into();
        payload
            .attributes
            .iter()
            .find_map(|a| if a.attribute_type == attr { a.as_short() } else { None })
    }

    async fn do_challenge_attr(&mut self, attr: Bytes) -> anyhow::Result<Arc<CccSession>> {
        let parts = attr
            .split(|c| *c == b'\0')
            .map(|p| String::from_utf8_lossy(p).into_owned())
            .collect::<Vec<_>>();

        debug!("Challenge msg: {}", parts[0]);

        let msg_obj = parts[1].parse::<SExpression>()?;

        let state = msg_obj
            .get_value::<String>("msg_obj:authentication_state")
            .ok_or_else(|| anyhow!("No state"))?;

        if state != "challenge" {
            return Err(anyhow!("Not a challenge state!"));
        }

        let inner = msg_obj
            .get("msg_obj:arguments:0:val")
            .ok_or_else(|| anyhow!("Invalid challenge reply!"))?;

        let id = inner
            .get_value::<String>("msg_obj:id")
            .ok_or_else(|| anyhow!("No challenge id!"))?;

        debug!("Challenge ID: {}", id);

        let prompt = inner
            .get_value::<String>("msg_obj:def_msg")
            .ok_or_else(|| anyhow!("No challenge prompt!"))?;

        debug!("Challenge prompt: {}", prompt);

        Ok(Arc::new(CccSession {
            session_id: self.ccc_session.clone(),
            ipsec_session: None,
            state: SessionState::PendingChallenge(MfaChallenge {
                mfa_type: MfaType::from_id(&id),
                prompt,
            }),
        }))
    }

    async fn process_id_reply(&mut self, id_reply: AttributesPayload) -> anyhow::Result<Arc<CccSession>> {
        self.last_identifier = id_reply.identifier;
        let status = self.get_short_attribute(&id_reply, ConfigAttributeType::Status);
        match status {
            Some(1) => {
                debug!("IPSec authentication succeeded");
                self.ikev1
                    .send_ack_response(id_reply.identifier, self.last_message_id)
                    .await?;

                let om_reply = self.ikev1.send_om_request().await?;

                self.ccc_session = self
                    .get_long_attribute(&om_reply, ConfigAttributeType::CccSessionId)
                    .map(|v| String::from_utf8_lossy(&v).trim_matches('\0').to_string())
                    .ok_or_else(|| anyhow!("No CCC session in reply!"))?;

                let ipv4addr: Ipv4Addr = self
                    .get_long_attribute(&om_reply, ConfigAttributeType::Ipv4Address)
                    .ok_or_else(|| anyhow!("No IPv4 in reply!"))?
                    .reader()
                    .read_u32::<BigEndian>()?
                    .into();

                let netmask: Ipv4Addr = self
                    .get_long_attribute(&om_reply, ConfigAttributeType::Ipv4Netmask)
                    .ok_or_else(|| anyhow!("No netmask in reply!"))?
                    .reader()
                    .read_u32::<BigEndian>()?
                    .into();

                let dns: Vec<Ipv4Addr> = self
                    .get_long_attributes(&om_reply, ConfigAttributeType::Ipv4Dns)
                    .into_iter()
                    .flat_map(|b| b.reader().read_u32::<BigEndian>().ok())
                    .map(Into::into)
                    .collect();

                let search_domains = self
                    .get_long_attribute(&om_reply, ConfigAttributeType::InternalDomainName)
                    .map(|v| String::from_utf8_lossy(&v).into_owned())
                    .unwrap_or_default()
                    .split(',')
                    .map(ToOwned::to_owned)
                    .collect();

                self.ikev1.do_esp_proposal(ipv4addr).await?;

                let session = Arc::new(CccSession {
                    session_id: self.ccc_session.clone(),
                    ipsec_session: Some(IpsecSession {
                        address: ipv4addr,
                        netmask,
                        dns,
                        domains: search_domains,
                        esp_in: self.session.read().esp_in.clone(),
                        esp_out: self.session.read().esp_out.clone(),
                    }),
                    state: SessionState::Authenticated(String::new()),
                });
                Ok(session)
            }
            Some(status) => {
                warn!("IPSec authentication failed, status: {}", status);
                Err(anyhow!("IPSec authentication failed, status: {}", status))
            }
            None => {
                let attr = self.get_challenge_attribute_type(&id_reply);
                debug!("No status in reply, requested challenge for: {:?}", attr);
                match attr {
                    ConfigAttributeType::UserName => {
                        if self.last_challenge_type == ConfigAttributeType::UserName {
                            return Err(anyhow!("Endless loop of username challenges!"));
                        }
                        self.last_challenge_type = ConfigAttributeType::UserName;
                        let user_name = self.params.user_name.clone();
                        self.challenge_code(Arc::new(CccSession::empty()), &user_name).await
                    }
                    ConfigAttributeType::UserPassword if !self.params.password.is_empty() => {
                        self.last_challenge_type = ConfigAttributeType::UserPassword;
                        let user_password = self.params.password.clone();
                        self.challenge_code(Arc::new(CccSession::empty()), &user_password).await
                    }
                    other => {
                        if let Some(attr) = self.get_long_attribute(&id_reply, ConfigAttributeType::Challenge) {
                            self.last_challenge_type = other;
                            self.do_challenge_attr(attr).await
                        } else {
                            Err(anyhow!("No challenge in payload!"))
                        }
                    }
                }
            }
        }
    }
}

#[async_trait]
impl TunnelConnector for IpsecTunnelConnector {
    async fn authenticate(&mut self) -> anyhow::Result<Arc<CccSession>> {
        let my_address = platform::get_default_ip().await?.parse::<Ipv4Addr>()?;
        self.ikev1.do_sa_proposal().await?;
        self.ikev1.do_key_exchange(my_address, self.gateway_address).await?;

        let realm = format!(
            "(\n\
               :clientType (TRAC)\n\
               :clientOS (Windows_7)\n\
               :oldSessionId ()\n\
               :protocolVersion (100)\n\
               :client_mode (SYMBIAN)\n\
               :selected_realm_id ({})\n\
             )\n",
            self.params.login_type
        );

        let (id_reply, message_id) = self
            .ikev1
            .do_identity_protection(Bytes::copy_from_slice(realm.as_bytes()))
            .await?;

        self.last_message_id = message_id;
        self.process_id_reply(id_reply).await
    }

    async fn challenge_code(&mut self, _session: Arc<CccSession>, user_input: &str) -> anyhow::Result<Arc<CccSession>> {
        let id_reply = self
            .ikev1
            .send_auth_attribute(
                self.last_identifier,
                self.last_message_id,
                self.last_challenge_type,
                Bytes::copy_from_slice(user_input.trim().as_bytes()),
                Some(Duration::from_secs(120)),
            )
            .await?
            .0;
        self.process_id_reply(id_reply).await
    }

    async fn create_tunnel(&self, session: Arc<CccSession>) -> anyhow::Result<Box<dyn CheckpointTunnel + Send>> {
        Ok(Box::new(IpsecTunnel::create(self.params.clone(), session).await?))
    }
}
