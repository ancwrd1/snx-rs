use std::{
    net::{IpAddr, Ipv4Addr, ToSocketAddrs},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime},
};

use anyhow::{anyhow, Context};
use async_trait::async_trait;
use byteorder::{BigEndian, ReadBytesExt};
use bytes::{Buf, Bytes};
use isakmp::{
    ikev1::{service::Ikev1Service, session::Ikev1Session},
    model::{ConfigAttributeType, EspAttributeType, Identity, IdentityRequest, PayloadType},
    payload::AttributesPayload,
    session::{IsakmpSession, OfficeMode},
    transport::{IsakmpTransport, TcptDataType, TcptTransport, UdpTransport},
};
use tokio::{net::UdpSocket, sync::mpsc::Sender};
use tracing::{debug, trace, warn};

use crate::{
    model::{
        params::{CertType, TransportType, TunnelParams},
        proto::{AuthenticationRealm, ClientLoggingData},
        IpsecSession, MfaChallenge, MfaType, SessionState, VpnSession,
    },
    platform, server_info,
    sexpr::SExpression,
    tunnel::{
        ipsec::{native::NativeIpsecTunnel, natt::NattProber, tcpt::TcptIpsecTunnel},
        TunnelCommand, TunnelConnector, TunnelEvent, VpnTunnel,
    },
};

const MIN_ESP_LIFETIME: Duration = Duration::from_secs(60);

const SESSIONS_PATH: &str = "/var/cache/snx-rs/sessions";

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

pub struct IpsecTunnelConnector {
    params: Arc<TunnelParams>,
    service: Ikev1Service,
    gateway_address: Ipv4Addr,
    last_message_id: u32,
    last_identifier: u16,
    last_challenge_type: ConfigAttributeType,
    ccc_session: String,
    ipsec_session: IpsecSession,
    last_rekey: Option<SystemTime>,
    command_sender: Option<Sender<TunnelCommand>>,
}

impl IpsecTunnelConnector {
    pub async fn new(params: Arc<TunnelParams>) -> anyhow::Result<Self> {
        let identity = match params.cert_type {
            CertType::Pkcs12 => match (&params.cert_path, &params.cert_password) {
                (Some(path), Some(password)) => Identity::Pkcs12 {
                    path: path.clone(),
                    password: password.clone(),
                },
                _ => anyhow::bail!("No PKCS12 path and password provided!"),
            },
            CertType::Pkcs8 => match params.cert_path {
                Some(ref path) => Identity::Pkcs8 { path: path.clone() },
                None => anyhow::bail!("No PKCS8 PEM path provided!"),
            },
            CertType::Pkcs11 => match params.cert_password {
                Some(ref pin) => Identity::Pkcs11 {
                    driver_path: params.cert_path.clone().unwrap_or_else(|| "opensc-pkcs11.so".into()),
                    pin: pin.clone(),
                    key_id: params
                        .cert_id
                        .as_ref()
                        .map(|s| hex::decode(s.replace(':', "")).unwrap_or_default().into()),
                },
                None => anyhow::bail!("No PKCS11 pin provided!"),
            },

            CertType::None => Identity::None,
        };

        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket
            .connect(format!("{}:{}", params.server_name, params.ike_port))
            .await?;

        let IpAddr::V4(gateway_address) = socket.peer_addr()?.ip() else {
            anyhow::bail!("No IPv4 address for {}", params.server_name);
        };

        if params.esp_transport == TransportType::Udp {
            let prober = NattProber::new(gateway_address);
            prober.probe().await?;
        }

        debug!("Using ESP transport: {}", params.esp_transport);

        let ikev1_session = Box::new(Ikev1Session::new(identity)?);

        debug!("Using IKE transport: {}", params.ike_transport);

        let transport: Box<dyn IsakmpTransport + Send + Sync> = if params.ike_transport == TransportType::Udp {
            Box::new(UdpTransport::new(socket, ikev1_session.new_codec()))
        } else {
            let socket_address = format!("{}:443", params.server_name)
                .to_socket_addrs()?
                .next()
                .context("No address!")?;
            Box::new(TcptTransport::new(
                TcptDataType::Ike,
                socket_address,
                ikev1_session.new_codec(),
            ))
        };

        let service = Ikev1Service::new(transport, ikev1_session)?;

        Ok(Self {
            params,
            service,
            gateway_address,
            last_message_id: 0,
            last_identifier: 0,
            last_challenge_type: ConfigAttributeType::Other(0),
            ccc_session: String::new(),
            ipsec_session: IpsecSession::default(),
            last_rekey: None,
            command_sender: None,
        })
    }

    fn do_challenge_attr(&mut self, attr: &Bytes) -> anyhow::Result<Arc<VpnSession>> {
        let parts = attr
            .split(|c| *c == b'\0')
            .map(|p| String::from_utf8_lossy(p).into_owned())
            .collect::<Vec<_>>();

        debug!("Challenge msg: {}", parts[0]);
        trace!("msg_obj: {}", parts[1]);

        let msg_obj = parts[1].parse::<SExpression>()?;

        let state = msg_obj
            .get_value::<String>("msg_obj:authentication_state")
            .unwrap_or_else(|| "challenge".to_owned());

        if state != "challenge" && state != "new_factor" && state != "failed_attempt" {
            anyhow::bail!("Not a challenge state!");
        }

        let inner = msg_obj
            .get("msg_obj:arguments:0:val")
            .context("Invalid challenge reply!")?;

        let id = inner.get_value::<String>("msg_obj:id").unwrap_or_else(String::new);

        debug!("Challenge ID: {}", id);

        let prompt = inner
            .get_value::<String>("msg_obj:def_msg")
            .context("No challenge prompt!")?;

        debug!("Challenge prompt: {}", prompt);

        Ok(Arc::new(VpnSession {
            ccc_session_id: self.ccc_session.clone(),
            ipsec_session: None,
            state: SessionState::PendingChallenge(MfaChallenge {
                mfa_type: MfaType::from_id(&id),
                prompt,
            }),
        }))
    }

    async fn do_session_exchange(&mut self) -> anyhow::Result<Arc<VpnSession>> {
        let om_reply = self.service.send_om_request().await?;

        self.ccc_session = get_long_attribute(&om_reply, ConfigAttributeType::CccSessionId)
            .map(|v| String::from_utf8_lossy(&v).trim_matches('\0').to_string())
            .context("No CCC session in reply!")?;

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

        self.do_esp_proposal().await?;

        if self.params.ike_persist {
            if let Err(e) = self.save_ike_session() {
                warn!("Cannot save IKE session: {}", e);
            }
        }

        Ok(self.new_vpn_session())
    }

    async fn process_auth_attributes(&mut self, id_reply: AttributesPayload) -> anyhow::Result<Arc<VpnSession>> {
        self.last_identifier = id_reply.identifier;
        let status = get_short_attribute(&id_reply, ConfigAttributeType::Status);
        match status {
            Some(1) => {
                debug!("IPSec authentication succeeded");
                self.service
                    .send_ack_response(id_reply.identifier, self.last_message_id)
                    .await?;

                self.do_session_exchange().await
            }
            Some(status) => {
                warn!("IPSec authentication failed, status: {}", status);
                Err(anyhow!("IPSec authentication failed, status: {}", status))
            }
            None => {
                let attr = get_challenge_attribute_type(&id_reply);
                debug!("No status in reply, requested challenge for: {:?}", attr);

                let session = if let Some(challenge) = get_long_attribute(&id_reply, ConfigAttributeType::Challenge) {
                    self.do_challenge_attr(&challenge)?
                } else if attr == ConfigAttributeType::UserName {
                    Arc::new(VpnSession {
                        ccc_session_id: self.ccc_session.clone(),
                        ipsec_session: None,
                        state: SessionState::PendingChallenge(MfaChallenge {
                            mfa_type: MfaType::UserNameInput,
                            prompt: "User name: ".to_owned(),
                        }),
                    })
                } else {
                    anyhow::bail!("No challenge in payload!");
                };

                match attr {
                    ConfigAttributeType::UserName => {
                        if self.last_challenge_type == ConfigAttributeType::UserName {
                            anyhow::bail!("Endless loop of username challenges!");
                        }

                        self.last_challenge_type = attr;

                        if self.params.user_name.is_empty() {
                            Ok(session)
                        } else {
                            let user_name = self.params.user_name.clone();
                            self.challenge_code(Arc::new(VpnSession::empty()), &user_name).await
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
            .do_esp_proposal(self.ipsec_session.address, self.params.esp_lifetime)
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
                "Received unsolicited ISAKMP message, exchange type: {:?}, message id: {:04x}, payloads: {:?}",
                msg.exchange_type, msg.message_id, payload_types,
            );

            if payload_types.iter().any(|p| *p == PayloadType::SecurityAssociation) {
                self.rekey_tunnel().await?;
            }
        }
        Ok(())
    }

    async fn rekey_tunnel(&mut self) -> anyhow::Result<()> {
        let lifetime = if self.ipsec_session.lifetime < MIN_ESP_LIFETIME {
            self.ipsec_session.lifetime
        } else {
            self.ipsec_session.lifetime - MIN_ESP_LIFETIME
        };

        if platform::is_online()
            && self
                .last_rekey
                .is_some_and(|last_rekey| SystemTime::now().duration_since(last_rekey).unwrap_or(lifetime) >= lifetime)
        {
            debug!("Start rekeying IPSec tunnel");
            self.do_esp_proposal().await?;

            debug!(
                "New ESP SPI: {:04x}, {:04x}",
                self.ipsec_session.esp_in.spi, self.ipsec_session.esp_out.spi
            );

            if let Some(ref mut sender) = self.command_sender {
                Ok(sender.send(TunnelCommand::ReKey(self.ipsec_session.clone())).await?)
            } else {
                Err(anyhow!("No sender!"))
            }
        } else {
            Ok(())
        }
    }

    async fn delete_sa(&mut self) -> anyhow::Result<()> {
        self.service.delete_sa().await
    }

    async fn is_multi_factor_login_type(&self) -> anyhow::Result<bool> {
        Ok(server_info::get_login_factors(&self.params)
            .await?
            .into_iter()
            .any(|factor| factor.factor_type != "certificate"))
    }

    fn session_file_name(&self) -> PathBuf {
        Path::new(SESSIONS_PATH).join(&self.params.server_name)
    }

    fn save_ike_session(&mut self) -> anyhow::Result<()> {
        let office_mode = OfficeMode {
            ccc_session: self.ccc_session.clone(),
            ip_address: self.ipsec_session.address,
            netmask: self.ipsec_session.netmask,
            dns: self.ipsec_session.dns.clone(),
            domains: self.ipsec_session.domains.clone(),
        };

        let data = self.service.session().save(&office_mode)?;
        let dir = Path::new(SESSIONS_PATH);

        std::fs::create_dir_all(dir)?;

        let filename = self.session_file_name();
        std::fs::write(&filename, &data)?;

        debug!("Saved IKE session to: {}", filename.display());

        Ok(())
    }

    fn load_ike_session(&mut self) -> anyhow::Result<OfficeMode> {
        let filename = self.session_file_name();
        let data = std::fs::read(&filename)?;
        let office_mode = self.service.session().load(&data)?;

        debug!("Loaded IKE session from: {}: {:?}", filename.display(), office_mode);

        if !office_mode.ccc_session.is_empty() {
            Ok(office_mode)
        } else {
            Err(anyhow::anyhow!("Empty CCC session!"))
        }
    }

    async fn do_restore_session(&mut self) -> anyhow::Result<Arc<VpnSession>> {
        let office_mode = self.load_ike_session()?;

        match self.do_session_exchange().await {
            Ok(session) => Ok(session),
            Err(e) => {
                warn!("OM session exchange failed: {}, reusing previous settings", e);

                self.ccc_session = office_mode.ccc_session;
                self.ipsec_session.address = office_mode.ip_address;
                self.ipsec_session.netmask = office_mode.netmask;
                self.ipsec_session.dns = office_mode.dns;
                self.ipsec_session.domains = office_mode.domains;

                self.do_esp_proposal().await?;

                Ok(self.new_vpn_session())
            }
        }
    }

    fn new_vpn_session(&self) -> Arc<VpnSession> {
        Arc::new(VpnSession {
            ccc_session_id: self.ccc_session.clone(),
            ipsec_session: Some(self.ipsec_session.clone()),
            state: SessionState::Authenticated(String::new()),
        })
    }
}

#[async_trait]
impl TunnelConnector for IpsecTunnelConnector {
    async fn authenticate(&mut self) -> anyhow::Result<Arc<VpnSession>> {
        let my_address = platform::get_default_ip().await?.parse::<Ipv4Addr>()?;
        self.service.do_sa_proposal(self.params.ike_lifetime).await?;
        self.service.do_key_exchange(my_address, self.gateway_address).await?;

        let realm = AuthenticationRealm {
            client_type: self.params.tunnel_type.as_client_type().to_owned(),
            old_session_id: String::new(),
            protocol_version: 100,
            client_mode: self.params.client_mode.clone(),
            selected_realm_id: self.params.login_type.clone(),
            secondary_realm_hash: None,
            client_logging_data: Some(ClientLoggingData {
                os_name: Some("Windows".to_owned()),
                device_id: Some(crate::util::get_device_id()),
                ..Default::default()
            }),
        };

        let realm_expr = SExpression::from(&realm);

        trace!("Authentication blob: {}", realm_expr);

        let identity_request = IdentityRequest {
            auth_blob: Bytes::copy_from_slice(realm_expr.to_string().as_bytes()),
            verify_certs: self.params.ipsec_cert_check,
            ca_certs: self.params.ca_cert.clone(),
            with_mfa: self.params.cert_type == CertType::None
                || self.is_multi_factor_login_type().await.unwrap_or(false),
        };

        if let Some((attrs_reply, message_id)) = self.service.do_identity_protection(identity_request).await? {
            self.last_message_id = message_id;

            self.process_auth_attributes(attrs_reply).await
        } else {
            let result = self.do_session_exchange().await?;

            Ok(result)
        }
    }

    async fn delete_session(&mut self) {
        let _ = std::fs::remove_file(self.session_file_name());
    }

    async fn restore_session(&mut self) -> anyhow::Result<Arc<VpnSession>> {
        match self.do_restore_session().await {
            Ok(result) => Ok(result),
            Err(e) => {
                self.delete_session().await;
                Err(e)
            }
        }
    }

    async fn challenge_code(&mut self, _session: Arc<VpnSession>, user_input: &str) -> anyhow::Result<Arc<VpnSession>> {
        let id_reply = self
            .service
            .send_auth_attribute(
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
        session: Arc<VpnSession>,
        command_sender: Sender<TunnelCommand>,
    ) -> anyhow::Result<Box<dyn VpnTunnel + Send>> {
        self.command_sender = Some(command_sender);
        let result: anyhow::Result<Box<dyn VpnTunnel + Send>> = match self.params.esp_transport {
            TransportType::Udp => Ok(Box::new(NativeIpsecTunnel::create(self.params.clone(), session).await?)),
            TransportType::Tcpt => Ok(Box::new(TcptIpsecTunnel::create(self.params.clone(), session).await?)),
        };

        if let Err(ref e) = result {
            warn!("Create tunnel failed: {}", e);
            self.delete_session().await;
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
            TunnelEvent::Connected => {
                debug!("Tunnel connected");
            }
            TunnelEvent::Disconnected => {
                debug!("Tunnel disconnected");
                let _ = self.delete_sa().await;
            }
            TunnelEvent::RekeyCheck => {
                self.rekey_tunnel().await?;
            }
            TunnelEvent::RemoteControlData(data) => {
                self.parse_isakmp(data).await?;
            }
        }
        Ok(())
    }
}

impl Drop for IpsecTunnelConnector {
    fn drop(&mut self) {
        std::thread::scope(|s| {
            s.spawn(|| {
                crate::util::block_on(async {
                    self.delete_sa().await?;
                    self.terminate_tunnel(false).await
                })
            });
        });
    }
}
