use std::{
    net::{IpAddr, Ipv4Addr, ToSocketAddrs},
    sync::Arc,
    time::{Duration, SystemTime},
};

use anyhow::{Context, anyhow};
use async_trait::async_trait;
use byteorder::{BigEndian, ReadBytesExt};
use bytes::{Buf, Bytes};
use chrono::{Local, TimeZone};
use i18n::tr;
use ipnet::Ipv4Net;
use isakmp::{
    ikev1::{
        model::{ConfigAttributeType, EspAttributeType, IdentityRequest, PayloadType},
        payload::AttributesPayload,
        service::Ikev1Service,
        session::Ikev1Session,
    },
    session::{IsakmpSession, OfficeMode, SessionType},
    transport::{TcptDataType, TcptTransport},
};
use tokio::{net::UdpSocket, sync::mpsc::Sender};
use tracing::{debug, info, trace, warn};

use crate::{
    model::{
        AuthenticatedSession, IPsecSession, MfaChallenge, MfaType, SessionState, TunnelSession,
        params::{TransportType, TunnelParams},
        proto::GatewayInformation,
        wrappers::SessionId,
    },
    platform::{NetworkInterface, Platform, PlatformAccess},
    sexpr::SExpression,
    tunnel::{
        GatewayConnector, TunnelCommand, TunnelConnector, TunnelEvent, VpnTunnel,
        ipsec::{
            DEFAULT_ESP_LIFETIME, ESP_LIFETIME_LEEWAY, SessionStore, auth,
            imp::{native::NativeIPsecTunnel, tcpt::TcptIPsecTunnel, udp::UdpIPsecTunnel},
            natt::NattProber,
        },
    },
    util,
};

const ADDRESS_LIFETIME_LEEWAY: Duration = Duration::from_secs(300);

fn get_challenge_attribute_type(payload: &AttributesPayload) -> ConfigAttributeType {
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

fn get_long_attributes(payload: &AttributesPayload, attr: ConfigAttributeType) -> Vec<Bytes> {
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

fn get_long_attribute(payload: &AttributesPayload, attr: ConfigAttributeType) -> Option<Bytes> {
    get_long_attributes(payload, attr).first().cloned()
}

fn get_short_attribute(payload: &AttributesPayload, attr: ConfigAttributeType) -> Option<u16> {
    let attr: u16 = attr.into();
    payload
        .attributes
        .iter()
        .find_map(|a| if a.attribute_type == attr { a.as_short() } else { None })
}

pub struct Ikev1TunnelConnector {
    params: Arc<TunnelParams>,
    service: Ikev1Service,
    gateway_address: Ipv4Addr,
    last_message_id: u32,
    last_identifier: u16,
    last_challenge_type: ConfigAttributeType,
    ccc_session: SessionId,
    username: String,
    ipsec_session: IPsecSession,
    last_rekey: Option<SystemTime>,
    last_ip_lease: Option<SystemTime>,
    command_sender: Option<Sender<TunnelCommand>>,
    esp_transport: TransportType,
    gateway_connector: Arc<dyn GatewayConnector + Send + Sync>,
}

impl Ikev1TunnelConnector {
    pub async fn new(
        params: Arc<TunnelParams>,
        gateway_connector: Arc<dyn GatewayConnector + Send + Sync>,
    ) -> anyhow::Result<Self> {
        let info = gateway_connector.get_gateway_information().await?;

        let socket = UdpSocket::bind("0.0.0.0:0").await?;

        let host = if let Some((host, _)) = params.server_name.split_once(':') {
            host
        } else {
            &params.server_name
        };

        socket
            .connect(&format!("{host}:{}", info.connectivity_info.natt_port))
            .await?;

        let IpAddr::V4(gateway_address) = socket.peer_addr()?.ip() else {
            anyhow::bail!(tr!("error-no-ipv4", server = params.server_name));
        };

        let esp_transport = if params.transport_type == TransportType::AutoDetect {
            let prober = NattProber::new(socket.peer_addr()?, params.port_knock);

            if prober.probe().await.is_ok() {
                if Platform::get().get_features().await.ipsec_native {
                    TransportType::Kernel
                } else {
                    TransportType::Udp
                }
            } else {
                TransportType::Tcpt
            }
        } else {
            params.transport_type
        };

        debug!("ESP transport: {}", esp_transport);

        Ok(Self {
            params: params.clone(),
            service: Self::new_service(&params, &info)?,
            gateway_address,
            last_message_id: 0,
            last_identifier: 0,
            last_challenge_type: ConfigAttributeType::Other(0),
            ccc_session: SessionId::default(),
            username: String::new(),
            ipsec_session: IPsecSession::default(),
            last_rekey: None,
            last_ip_lease: None,
            command_sender: None,
            esp_transport,
            gateway_connector,
        })
    }

    fn do_challenge_attr(&mut self, attr: &Bytes) -> anyhow::Result<Arc<TunnelSession>> {
        let parts = attr
            .split(|c| *c == b'\0')
            .map(|p| String::from_utf8_lossy(p).into_owned())
            .collect::<Vec<_>>();

        // Bail instead of indexing so a malformed challenge without a NUL separator cannot panic us.
        if parts.len() < 2 {
            anyhow::bail!("Invalid challenge reply!");
        }

        debug!("Challenge msg: {}", parts[0]);
        trace!("msg_obj: {}", parts[1]);

        let challenge = auth::challenge_from_msg_obj(&parts[1].parse::<SExpression>()?)?;

        Ok(Arc::new(TunnelSession {
            session_id: self.ccc_session.clone(),
            state: SessionState::PendingChallenge(challenge),
            username: None,
        }))
    }

    async fn do_session_exchange(&mut self, username: String) -> anyhow::Result<Arc<TunnelSession>> {
        let mac = Bytes::copy_from_slice(&util::get_device_id().as_bytes()[0..6]);
        debug!("Using dummy MAC address: {}", hex::encode(&mac));

        let om_reply = self
            .service
            .send_om_request(
                Ipv4Net::with_netmask(self.ipsec_session.address, self.ipsec_session.netmask).ok(),
                Some(mac),
            )
            .await?;

        self.ccc_session = get_long_attribute(&om_reply, ConfigAttributeType::CccSessionId)
            .map(|v| String::from_utf8_lossy(&v).trim_matches('\0').to_string())
            .context(tr!("error-no-om-session"))?
            .into();

        self.ipsec_session.address = get_long_attribute(&om_reply, ConfigAttributeType::Ipv4Address)
            .context("No IPv4 in reply!")?
            .reader()
            .read_u32::<BigEndian>()?
            .into();

        self.ipsec_session.netmask = get_long_attribute(&om_reply, ConfigAttributeType::Ipv4Netmask)
            .context("No netmask in reply!")?
            .reader()
            .read_u32::<BigEndian>()?
            .into();

        self.ipsec_session.address_lifetime = if let Some(lease) = self.params.ip_lease_time {
            lease
        } else {
            Duration::from_secs(
                get_long_attribute(&om_reply, ConfigAttributeType::AddressExpiry)
                    .context("No address expiry in reply!")?
                    .reader()
                    .read_u32::<BigEndian>()?
                    .into(),
            )
        };
        self.last_ip_lease = Some(SystemTime::now());

        self.ipsec_session.dns = get_long_attributes(&om_reply, ConfigAttributeType::Ipv4Dns)
            .into_iter()
            .flat_map(|b| b.reader().read_u32::<BigEndian>().ok())
            .map(Into::into)
            .collect();

        self.ipsec_session.domains = get_long_attribute(&om_reply, ConfigAttributeType::InternalDomainName)
            .map(|v| String::from_utf8_lossy(&v).into_owned())
            .unwrap_or_default()
            .split([',', ';'])
            .map(ToOwned::to_owned)
            .collect();

        self.ipsec_session.transport_type = self.esp_transport;
        self.username = username;

        debug!("OM IP address: {}", self.ipsec_session.address);
        debug!("OM IP netmask: {}", self.ipsec_session.netmask);
        debug!(
            "OM IP lifetime: {} seconds",
            self.ipsec_session.address_lifetime.as_secs()
        );
        debug!("OM DNS servers: {:?}", self.ipsec_session.dns);
        debug!("OM search domains: {:?}", self.ipsec_session.domains);

        self.do_esp_proposal().await?;

        if self.params.ike_persist
            && let Err(e) = self.save_ike_session()
        {
            warn!("Cannot save IKE session: {}", e);
        }

        Ok(self.new_session())
    }

    async fn process_auth_attributes(&mut self, id_reply: AttributesPayload) -> anyhow::Result<Arc<TunnelSession>> {
        self.last_identifier = id_reply.identifier;
        let status = get_short_attribute(&id_reply, ConfigAttributeType::Status);
        match status {
            Some(1) => {
                debug!("IPsec authentication succeeded");
                self.service
                    .send_ack_response(id_reply.identifier, self.last_message_id)
                    .await?;

                let message = get_long_attribute(&id_reply, ConfigAttributeType::Message)
                    .and_then(|m| String::from_utf8_lossy(&m).split_once('\0').map(|(_, m)| m.to_owned()))
                    .and_then(|m| m.parse::<SExpression>().ok());

                let username = if let Some(message) = message {
                    message
                        .get_value::<String>("msg_obj:arguments:0:val:msg_obj:arguments:0:val")
                        .unwrap_or_else(|| self.params.user_name.clone())
                } else {
                    self.params.user_name.clone()
                };
                debug!("Authenticated username: {}", username);

                self.do_session_exchange(username).await
            }
            Some(status) => {
                warn!("IPsec authentication failed, status: {}", status);
                Err(anyhow!(tr!("error-auth-failed")))
            }
            None => {
                let attr = get_challenge_attribute_type(&id_reply);
                debug!("No status in reply, requested challenge for: {:?}", attr);

                let session = if let Some(challenge) = get_long_attribute(&id_reply, ConfigAttributeType::Challenge) {
                    self.do_challenge_attr(&challenge)?
                } else if attr == ConfigAttributeType::UserName {
                    Arc::new(TunnelSession {
                        session_id: self.ccc_session.clone(),
                        state: SessionState::PendingChallenge(MfaChallenge {
                            mfa_type: MfaType::UserNameInput,
                            prompt: tr!("label-username"),
                        }),
                        username: None,
                    })
                } else {
                    anyhow::bail!(tr!("error-no-challenge"));
                };

                match attr {
                    ConfigAttributeType::UserName => {
                        if self.last_challenge_type == ConfigAttributeType::UserName {
                            anyhow::bail!(tr!("error-endless-challenges"));
                        }

                        self.last_challenge_type = attr;

                        if self.params.user_name.is_empty() {
                            Ok(session)
                        } else {
                            let user_name = self.params.user_name.clone();
                            self.challenge_code(Arc::new(TunnelSession::empty()), &user_name).await
                        }
                    }
                    other => {
                        self.last_challenge_type = other;
                        Ok(session)
                    }
                }
            }
        }
    }

    async fn do_esp_proposal(&mut self) -> anyhow::Result<()> {
        let attributes = self
            .service
            .do_esp_proposal(self.ipsec_session.address, DEFAULT_ESP_LIFETIME)
            .await?;

        let lifetime = attributes
            .iter()
            .find_map(|a| match EspAttributeType::from(a.attribute_type) {
                EspAttributeType::LifeDuration => a.as_long().and_then(|v| {
                    let data: Option<[u8; 4]> = v.as_ref().try_into().ok();
                    data.map(u32::from_be_bytes)
                }),
                _ => None,
            })
            .context("No lifetime in reply!")?;

        debug!("ESP lifetime: {} seconds", lifetime);

        let session = self.service.session();
        self.ipsec_session.lifetime = Duration::from_secs(lifetime as u64);
        self.ipsec_session.esp_in = session.esp_in();
        self.ipsec_session.esp_out = session.esp_out();

        self.last_rekey = Some(SystemTime::now());

        Ok(())
    }

    async fn parse_isakmp(&mut self, data: Bytes) -> anyhow::Result<()> {
        let mut codec = self.service.session().new_codec();

        if let Some(msg) = codec.decode(&data)? {
            let payload_types = msg.payloads.iter().map(|p| p.as_payload_type()).collect::<Vec<_>>();
            debug!(
                "Received unsolicited ISAKMP message, exchange type: {:?}, message id: {:08x}, payloads: {:?}",
                msg.exchange_type, msg.message_id, payload_types,
            );

            if payload_types.contains(&PayloadType::SecurityAssociation) {
                self.rekey_tunnel(false).await?;
            } else if payload_types.contains(&PayloadType::Delete) {
                self.terminate_tunnel(true).await?;
                self.delete_session().await?;
            }
        }
        Ok(())
    }

    async fn rekey_tunnel(&mut self, force: bool) -> anyhow::Result<()> {
        if !Platform::get().new_network_interface().is_online() {
            return Ok(());
        }

        // check for IKE SA expiration and disconnect if needed
        if self.ipsec_session.ike_timestamp + self.ipsec_session.ike_lifetime <= Local::now() {
            info!("IKE SA expired, disconnecting tunnel");
            let _ = self.delete_session().await;
            let _ = self.terminate_tunnel(true).await;
            return Ok(());
        }

        let lifetime = if self.ipsec_session.lifetime < ESP_LIFETIME_LEEWAY {
            self.ipsec_session.lifetime
        } else {
            self.ipsec_session.lifetime - ESP_LIFETIME_LEEWAY
        };

        let address_lifetime = if self.ipsec_session.address_lifetime < ADDRESS_LIFETIME_LEEWAY {
            self.ipsec_session.address_lifetime
        } else {
            self.ipsec_session.address_lifetime - ADDRESS_LIFETIME_LEEWAY
        };

        let now = SystemTime::now();
        let mut rekeyed = false;

        if self.last_ip_lease.is_some_and(|last_ip_lease| {
            now.duration_since(last_ip_lease).unwrap_or(address_lifetime) >= address_lifetime
        }) {
            debug!("Start refreshing IPsec session");
            self.do_session_exchange(self.username.clone()).await?;
            rekeyed = true;
        } else if force
            || self
                .last_rekey
                .is_some_and(|last_rekey| now.duration_since(last_rekey).unwrap_or(lifetime) >= lifetime)
        {
            debug!("Start rekeying IPsec tunnel");
            self.do_esp_proposal().await?;
            rekeyed = true;
        }

        if rekeyed {
            debug!(
                "New ESP SPI: {:04x}, {:04x}",
                self.ipsec_session.esp_in.spi, self.ipsec_session.esp_out.spi
            );

            if let Some(ref mut sender) = self.command_sender {
                Ok(sender.send(TunnelCommand::ReKey(self.ipsec_session.clone())).await?)
            } else {
                Err(anyhow!(tr!("error-no-sender")))
            }
        } else {
            Ok(())
        }
    }

    async fn delete_sa(&mut self) -> anyhow::Result<()> {
        self.service.delete_sa().await
    }

    fn save_ike_session(&mut self) -> anyhow::Result<()> {
        let office_mode = OfficeMode {
            ccc_session: self.ccc_session.clone().into(),
            username: self.username.clone(),
            ip_address: self.ipsec_session.address,
            netmask: self.ipsec_session.netmask,
            dns: self.ipsec_session.dns.clone(),
            domains: self.ipsec_session.domains.clone(),
        };

        let data = self.service.session().save(&office_mode)?;

        self.store().save(&data)
    }

    fn store(&self) -> SessionStore {
        SessionStore::new("ike_session", self.params.profile_id, self.params.server_name.clone())
    }

    fn load_ike_session(&mut self) -> anyhow::Result<OfficeMode> {
        let data = self.store().load()?;
        let office_mode = self.service.session().load(&data)?;

        debug!("Loaded IKE session: {:?}", office_mode);

        if !office_mode.ccc_session.is_empty() {
            Ok(office_mode)
        } else {
            Err(anyhow::anyhow!("Empty CCC session!"))
        }
    }

    async fn do_restore_session(&mut self) -> anyhow::Result<Arc<TunnelSession>> {
        let office_mode = self.load_ike_session()?;

        self.ipsec_session.ike_lifetime = self.service.session().lifetime();
        self.ipsec_session.ike_timestamp = Local
            .timestamp_opt(self.service.session().timestamp() as i64, 0)
            .single()
            .unwrap_or_else(Local::now);
        self.ipsec_session.initiator_spi = self.service.session().initiator_spi();
        self.ipsec_session.responder_spi = self.service.session().responder_spi();

        match self.do_session_exchange(office_mode.username.clone()).await {
            Ok(session) => Ok(session),
            Err(e) => {
                warn!("OM session exchange failed: {}, reusing previous settings", e);

                self.ccc_session = office_mode.ccc_session.into();
                self.username = office_mode.username;
                self.ipsec_session.address = office_mode.ip_address;
                self.ipsec_session.netmask = office_mode.netmask;
                self.ipsec_session.dns = office_mode.dns;
                self.ipsec_session.domains = office_mode.domains;

                self.do_esp_proposal().await?;

                Ok(self.new_session())
            }
        }
    }

    fn new_session(&self) -> Arc<TunnelSession> {
        Arc::new(TunnelSession {
            session_id: self.ccc_session.clone(),
            state: SessionState::Authenticated(AuthenticatedSession::IPsecSession(self.ipsec_session.clone())),
            username: Some(self.username.clone()),
        })
    }

    fn new_service(params: &TunnelParams, info: &GatewayInformation) -> anyhow::Result<Ikev1Service> {
        let identity = auth::new_identity(params, info)?;

        let ikev1_session = Ikev1Session::new(identity, SessionType::Initiator)?;

        let address = util::server_name_with_port(&params.server_name, info.connectivity_info.tcpt_port);

        let tcpt_address = address.to_socket_addrs()?.next().context("No address!")?;

        let transport = Box::new(TcptTransport::new(
            TcptDataType::Ike,
            tcpt_address,
            ikev1_session.new_codec(),
        ));

        Ikev1Service::new(transport, ikev1_session)
    }
}

#[async_trait]
impl TunnelConnector for Ikev1TunnelConnector {
    async fn authenticate(&mut self) -> anyhow::Result<Arc<TunnelSession>> {
        let my_address = Platform::get().new_network_interface().get_default_ipv4().await?;
        self.service.do_sa_proposal(self.params.ike_lifetime).await?;
        self.service.do_key_exchange(my_address, self.gateway_address).await?;

        self.ipsec_session.ike_lifetime = self.service.session().lifetime();
        self.ipsec_session.ike_timestamp = Local
            .timestamp_opt(self.service.session().timestamp() as i64, 0)
            .single()
            .unwrap_or_else(Local::now);
        self.ipsec_session.initiator_spi = self.service.session().initiator_spi();
        self.ipsec_session.responder_spi = self.service.session().responder_spi();

        let info = self.gateway_connector.get_gateway_information().await?;

        let machine_name = if self.service.session().hybrid_auth()
            && let Some(cert) = self.service.session().client_certificate()
        {
            auth::machine_name(&*cert)
        } else {
            None
        };

        debug!("Machine name: {:?}", machine_name);

        let realm_expr = auth::auth_blob(&self.params, &info, machine_name);

        trace!("Authentication blob: {}", realm_expr);

        let internal_ca_fingerprints = info
            .connectivity_info
            .internal_ca_fingerprint
            .values()
            .flat_map(|fp| util::snx_deobfuscate(fp).ok())
            .map(|v| String::from_utf8_lossy(&v).into_owned())
            .collect();

        let identity_request = IdentityRequest {
            auth_blob: realm_expr.to_string(),
            with_mfa: info.is_multi_factor_login_type(&self.params.login_type),
            internal_ca_fingerprints,
        };

        let reply = self
            .service
            .do_identity_protection(identity_request)
            .await
            .map_err(|e| {
                if e.downcast_ref::<tokio::time::error::Elapsed>().is_some() {
                    anyhow::anyhow!(tr!("error-identity-timeout"))
                } else {
                    e
                }
            })?;

        if let (Some(attrs_reply), message_id) = reply {
            self.last_message_id = message_id;

            self.process_auth_attributes(attrs_reply).await
        } else {
            let result = self.do_session_exchange(self.params.user_name.clone()).await?;

            Ok(result)
        }
    }

    async fn delete_session(&mut self) -> anyhow::Result<()> {
        self.store().delete()
    }

    async fn restore_session(&mut self) -> anyhow::Result<Arc<TunnelSession>> {
        match self.do_restore_session().await {
            Ok(result) => Ok(result),
            Err(e) => {
                let _ = self.delete_session().await;
                let info = self.gateway_connector.get_gateway_information().await?;
                self.service = Self::new_service(&self.params, &info)?;
                Err(e)
            }
        }
    }

    async fn challenge_code(
        &mut self,
        _session: Arc<TunnelSession>,
        user_input: &str,
    ) -> anyhow::Result<Arc<TunnelSession>> {
        let id_reply = self
            .service
            .send_attribute(
                self.last_identifier,
                self.last_message_id,
                self.last_challenge_type,
                Bytes::copy_from_slice(user_input.trim().as_bytes()),
                Some(Duration::from_secs(120)),
            )
            .await?
            .0;
        self.process_auth_attributes(id_reply).await
    }

    async fn create_tunnel(
        &mut self,
        session: Arc<TunnelSession>,
        command_sender: Sender<TunnelCommand>,
    ) -> anyhow::Result<Box<dyn VpnTunnel + Send>> {
        self.command_sender = Some(command_sender);
        let result: anyhow::Result<Box<dyn VpnTunnel + Send>> = match self.esp_transport {
            TransportType::Kernel => Ok(Box::new(
                NativeIPsecTunnel::create(self.params.clone(), session, self.gateway_connector.clone()).await?,
            )),
            TransportType::Tcpt => Ok(Box::new(
                TcptIPsecTunnel::create(self.params.clone(), session, self.gateway_connector.clone()).await?,
            )),
            TransportType::Udp => Ok(Box::new(
                UdpIPsecTunnel::create(self.params.clone(), session, self.gateway_connector.clone()).await?,
            )),
            _ => Err(anyhow!(tr!("error-invalid-transport-type"))),
        };

        if let Err(ref e) = result {
            warn!("Create tunnel failed: {}", e);
            let _ = self.delete_session().await;
        }

        result
    }

    async fn terminate_tunnel(&mut self, signout: bool) -> anyhow::Result<()> {
        if let Some(sender) = self.command_sender.take() {
            let _ = sender.send(TunnelCommand::Terminate(signout)).await;
        }
        Ok(())
    }

    async fn handle_tunnel_event(&mut self, event: TunnelEvent) -> anyhow::Result<()> {
        match event {
            TunnelEvent::Connected(_) => {
                debug!("Tunnel connected");
            }
            TunnelEvent::Disconnected => {
                debug!("Tunnel disconnected");
                let _ = self.delete_sa().await;
            }
            TunnelEvent::RekeyCheck => {
                self.rekey_tunnel(false).await?;
            }
            TunnelEvent::RemoteControlData(data) => {
                self.parse_isakmp(data).await?;
            }
            TunnelEvent::Rekeyed(_) => {
                debug!("Tunnel rekeyed");
            }
            TunnelEvent::Rtt(_) => {}
        }
        Ok(())
    }

    async fn rekey(&mut self) -> anyhow::Result<()> {
        self.rekey_tunnel(true).await
    }
}

impl Drop for Ikev1TunnelConnector {
    fn drop(&mut self) {
        std::thread::scope(|s| {
            s.spawn(|| {
                util::block_on(async {
                    self.delete_sa().await?;
                    self.terminate_tunnel(false).await
                })
            });
        });
    }
}
