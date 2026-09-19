use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, ToSocketAddrs},
    sync::Arc,
    time::{Duration, SystemTime},
};

use anyhow::{Context, anyhow};
use async_trait::async_trait;
use bytes::Bytes;
use chrono::{Local, TimeZone};
use i18n::tr;
use isakmp::{
    ikev2::{
        message::Ikev2Message,
        model::EapType,
        payload::Payload,
        service::{AuthPrompt, AuthResult, Ikev2AuthRequest, Ikev2Service, Ikev2Step, PeerRequest},
        session::Ikev2Session,
    },
    session::{IsakmpSession, OfficeMode, SessionType},
    transport::{IsakmpTransport, TcptDataType, TcptTransport, UdpTransport},
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

// How long a rekey check waits for a request the gateway may have opened.
const PEER_REQUEST_POLL: Duration = Duration::from_millis(200);

fn is_transport_failure(error: &anyhow::Error) -> bool {
    error
        .chain()
        .any(|e| e.is::<tokio::time::error::Elapsed>() || e.is::<std::io::Error>())
}

fn identity_timeout(error: anyhow::Error) -> anyhow::Error {
    match error.downcast_ref::<tokio::time::error::Elapsed>() {
        Some(_) => anyhow!(tr!("error-identity-timeout")),
        None => error,
    }
}

pub struct Ikev2TunnelConnector {
    params: Arc<TunnelParams>,
    service: Ikev2Service,
    session: Ikev2Session,
    gateway_address: SocketAddrV4,
    ccc_session: SessionId,
    username: String,
    ipsec_session: IPsecSession,
    awaiting_username: bool,
    last_rekey: Option<SystemTime>,
    last_address_renewal: Option<SystemTime>,
    command_sender: Option<Sender<TunnelCommand>>,
    esp_transport: TransportType,
    gateway_connector: Arc<dyn GatewayConnector + Send + Sync>,
}

impl Ikev2TunnelConnector {
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

        let SocketAddr::V4(gateway_address) = socket.peer_addr()? else {
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

        let (service, session) = Self::new_service(&params, &info, esp_transport).await?;

        Ok(Self {
            params,
            service,
            session,
            gateway_address,
            ccc_session: SessionId::default(),
            username: String::new(),
            ipsec_session: IPsecSession::default(),
            awaiting_username: false,
            last_rekey: None,
            last_address_renewal: None,
            command_sender: None,
            esp_transport,
            gateway_connector,
        })
    }

    async fn new_service(
        params: &TunnelParams,
        info: &GatewayInformation,
        esp_transport: TransportType,
    ) -> anyhow::Result<(Ikev2Service, Ikev2Session)> {
        let identity = auth::new_identity(params, info)?;

        let session = Ikev2Session::new(identity, SessionType::Initiator)?;

        let transport: Box<dyn IsakmpTransport<Ikev2Message> + Send + Sync> = match esp_transport {
            TransportType::Tcpt => {
                let address = util::server_name_with_port(&params.server_name, info.connectivity_info.tcpt_port);
                let tcpt_address = address.to_socket_addrs()?.next().context("No address!")?;

                Box::new(TcptTransport::new(TcptDataType::Ike, tcpt_address, session.new_codec()))
            }
            _ => {
                let address = util::server_name_with_port(&params.server_name, info.connectivity_info.natt_port);
                let natt_address = address.to_socket_addrs()?.next().context("No address!")?;

                let socket = UdpSocket::bind("0.0.0.0:0").await?;
                socket.connect(natt_address).await?;

                Box::new(UdpTransport::new(Arc::new(socket), session.new_codec()))
            }
        };

        let service = Ikev2Service::new(transport, session.clone())?;

        Ok((service, session))
    }

    async fn do_authenticate(&mut self, username: String) -> anyhow::Result<Arc<TunnelSession>> {
        let result = self.try_authenticate(username.clone()).await;

        let Err(error) = result else {
            return result;
        };

        if self.params.transport_type == TransportType::AutoDetect
            && self.esp_transport != TransportType::Tcpt
            && is_transport_failure(&error)
        {
            warn!("IKE exchange over UDP failed: {}, retrying over TCPT", error);

            let info = self.gateway_connector.get_gateway_information().await?;
            let (service, session) = Self::new_service(&self.params, &info, TransportType::Tcpt).await?;

            self.service = service;
            self.session = session;
            self.esp_transport = TransportType::Tcpt;

            return self.try_authenticate(username).await.map_err(identity_timeout);
        }

        Err(identity_timeout(error))
    }

    async fn try_authenticate(&mut self, username: String) -> anyhow::Result<Arc<TunnelSession>> {
        let my_address = Platform::get().new_network_interface().get_default_ipv4().await?;

        let sa_init = self
            .service
            .do_sa_init(SocketAddrV4::new(my_address, 0), self.gateway_address)
            .await?;

        debug!(
            "IKE SA established, local NAT: {}, remote NAT: {}",
            sa_init.local_nat, sa_init.remote_nat
        );

        self.ipsec_session.initiator_spi = self.session.initiator_spi();
        self.ipsec_session.responder_spi = self.session.responder_spi();

        let info = self.gateway_connector.get_gateway_information().await?;

        let machine_name = if self.session.hybrid_auth()
            && let Some(cert) = self.session.client_certificate()
        {
            auth::machine_name(&*cert)
        } else {
            None
        };

        debug!("Machine name: {:?}", machine_name);

        let auth_blob = auth::auth_blob(&self.params, &info, machine_name.clone());

        trace!("Authentication blob: {}", auth_blob);

        self.username = username.clone();

        let step = self
            .service
            .do_auth(Ikev2AuthRequest {
                username,
                auth_blob: auth_blob.to_string(),
                address: None,
                machine_name,
            })
            .await?;

        self.process_step(step).await
    }

    async fn process_step(&mut self, mut step: Ikev2Step) -> anyhow::Result<Arc<TunnelSession>> {
        loop {
            match step {
                // RFC 3748 §5.1: the identity is ours to state, not something to ask the user for.
                Ikev2Step::NeedsChallenge(ref prompt) if prompt.eap_type() == Some(EapType::Identity) => {
                    debug!("Answering the EAP identity request");
                    step = self
                        .service
                        .step(Bytes::from(self.username.clone().into_bytes()))
                        .await?;
                }
                Ikev2Step::NeedsChallenge(prompt) => break self.new_challenge(&prompt),
                Ikev2Step::Done(result) => break self.finish_auth(*result),
            }
        }
    }

    fn new_challenge(&mut self, prompt: &AuthPrompt) -> anyhow::Result<Arc<TunnelSession>> {
        debug!("Challenge requested, EAP type: {:?}", prompt.eap_type());

        let parts = prompt
            .data()
            .split(|c| *c == b'\0')
            .map(|p| String::from_utf8_lossy(p).trim().to_owned())
            .collect::<Vec<_>>();

        let text = parts.first().cloned().unwrap_or_default();

        debug!("Challenge msg: {}", text);

        let challenge = match parts.get(1).filter(|msg_obj| !msg_obj.is_empty()) {
            Some(msg_obj) => {
                trace!("msg_obj: {}", msg_obj);
                auth::challenge_from_msg_obj(&msg_obj.parse::<SExpression>()?)?
            }
            None if text.starts_with("https://") => MfaChallenge {
                mfa_type: MfaType::IdentityProvider,
                prompt: text,
            },
            None => MfaChallenge {
                mfa_type: MfaType::PasswordInput,
                prompt: if text.is_empty() { tr!("label-password") } else { text },
            },
        };

        Ok(Arc::new(TunnelSession {
            session_id: self.ccc_session.clone(),
            state: SessionState::PendingChallenge(challenge),
            username: None,
        }))
    }

    fn finish_auth(&mut self, result: AuthResult) -> anyhow::Result<Arc<TunnelSession>> {
        let message = result
            .auth_log
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

        let office_mode = result.office_mode;

        if office_mode.ccc_session.is_empty() {
            anyhow::bail!(tr!("error-no-om-session"));
        }

        self.ccc_session = office_mode.ccc_session.into();
        self.username = username;
        self.ipsec_session.address = office_mode.ip_address;
        self.ipsec_session.netmask = if office_mode.netmask.is_unspecified() {
            Ipv4Addr::BROADCAST
        } else {
            office_mode.netmask
        };
        self.ipsec_session.dns = office_mode.dns;
        self.ipsec_session.domains = office_mode.domains;

        // N(AUTH_LIFETIME) is the *IKE* SA's; the child SA has no negotiated
        // lifetime of its own and is rekeyed on our own schedule.
        let ike_lifetime = match self.session.lifetime() {
            lifetime if lifetime.is_zero() => self.params.ike_lifetime,
            lifetime => lifetime,
        };

        self.ipsec_session.lifetime = DEFAULT_ESP_LIFETIME;
        self.last_address_renewal = Some(SystemTime::now());
        self.ipsec_session.ike_lifetime = ike_lifetime;
        self.last_rekey = Some(SystemTime::now());
        self.ipsec_session.ike_timestamp = Local
            .timestamp_opt(self.session.timestamp() as i64, 0)
            .single()
            .unwrap_or_else(Local::now);
        // The gateway's own lease, far shorter than the IKE SA: the captured
        // one is 15 minutes against 8 hours, and it is renewed on its own
        // schedule in `renew_address_lease`.
        self.ipsec_session.address_lifetime = self
            .params
            .ip_lease_time
            .or(result.address_expiry)
            .unwrap_or(ike_lifetime);
        self.ipsec_session.esp_in = self.session.esp_in();
        self.ipsec_session.esp_out = self.session.esp_out();
        self.ipsec_session.transport_type = self.esp_transport;

        debug!("OM IP address: {}", self.ipsec_session.address);
        debug!("OM IP netmask: {}", self.ipsec_session.netmask);
        debug!("OM DNS servers: {:?}", self.ipsec_session.dns);
        debug!("OM search domains: {:?}", self.ipsec_session.domains);
        debug!("IKE SA lifetime: {} seconds", ike_lifetime.as_secs());
        debug!(
            "Office mode address lifetime: {} seconds",
            self.ipsec_session.address_lifetime.as_secs()
        );
        debug!(
            "ESP SPI: {:08x}, {:08x}",
            self.ipsec_session.esp_in.spi, self.ipsec_session.esp_out.spi
        );

        let selector_address = |data: &Bytes| {
            <[u8; 4]>::try_from(data.as_ref())
                .map(|octets| Ipv4Addr::from(octets).to_string())
                .unwrap_or_else(|_| hex::encode(data))
        };

        self.persist_session();

        for selector in &result.ts_r {
            debug!(
                "Remote traffic selector: {} - {}",
                selector_address(&selector.start_address),
                selector_address(&selector.end_address)
            );
        }

        Ok(self.new_session())
    }

    fn new_session(&self) -> Arc<TunnelSession> {
        Arc::new(TunnelSession {
            session_id: self.ccc_session.clone(),
            state: SessionState::Authenticated(AuthenticatedSession::IPsecSession(self.ipsec_session.clone())),
            username: Some(self.username.clone()),
        })
    }

    async fn parse_isakmp(&mut self, data: Bytes) -> anyhow::Result<()> {
        let mut codec = self.session.new_codec();

        if let Some(msg) = codec.decode(&data)? {
            let payload_types = msg.payloads.iter().map(|p| p.as_payload_type()).collect::<Vec<_>>();
            debug!(
                "Received unsolicited IKEv2 message, exchange type: {:?}, message id: {:08x}, payloads: {:?}",
                msg.exchange_type, msg.message_id, payload_types,
            );

            if msg.payloads.iter().any(|p| matches!(p, Payload::Delete(_))) {
                self.terminate_tunnel(true).await?;
            }
        }
        Ok(())
    }

    async fn on_rekey_check(&mut self) -> anyhow::Result<()> {
        if !Platform::get().new_network_interface().is_online() {
            return Ok(());
        }

        if self.ipsec_session.ike_timestamp + self.ipsec_session.ike_lifetime <= Local::now() {
            info!("IKE SA expired, disconnecting tunnel");
            let _ = self.terminate_tunnel(true).await;
            return Ok(());
        }

        self.handle_peer_request().await?;

        self.renew_address_lease().await?;

        let lifetime = self
            .ipsec_session
            .lifetime
            .saturating_sub(ESP_LIFETIME_LEEWAY)
            .max(ESP_LIFETIME_LEEWAY);

        let due = self
            .last_rekey
            .is_some_and(|last| SystemTime::now().duration_since(last).unwrap_or(lifetime) >= lifetime);

        if due {
            self.rekey_tunnel().await?;
        }

        Ok(())
    }

    async fn handle_peer_request(&mut self) -> anyhow::Result<()> {
        let Some(request) = self.service.poll_request(PEER_REQUEST_POLL).await? else {
            return Ok(());
        };

        match self.service.handle_request(&request).await? {
            PeerRequest::ChildSaRekeyed => self.apply_new_child_sa().await,
            PeerRequest::Deleted => {
                info!("Gateway deleted the SA, disconnecting tunnel");
                self.terminate_tunnel(true).await
            }
            PeerRequest::Other => Ok(()),
        }
    }

    async fn renew_address_lease(&mut self) -> anyhow::Result<()> {
        let lifetime = self.ipsec_session.address_lifetime;

        if lifetime.is_zero() {
            return Ok(());
        }

        let due = self
            .last_address_renewal
            .is_some_and(|last| SystemTime::now().duration_since(last).unwrap_or(lifetime) >= lifetime / 2);

        if !due {
            return Ok(());
        }

        let lease = self.service.renew_office_mode(self.ipsec_session.address).await?;

        self.last_address_renewal = Some(SystemTime::now());
        self.persist_session();

        if let Some(expiry) = lease.expiry {
            self.ipsec_session.address_lifetime = self.params.ip_lease_time.unwrap_or(expiry);
        }

        let address_changed = lease.address != self.ipsec_session.address;

        self.ipsec_session.address = lease.address;
        if let Some(netmask) = lease.netmask {
            self.ipsec_session.netmask = netmask;
        }

        // The data path only needs telling when the address moved — it replaces
        // the interface's address on a rekey command, and a renewal that keeps
        // the same one would otherwise stall the tunnel for nothing.
        match address_changed {
            true => {
                info!("Office mode address changed to {}", lease.address);
                self.apply_new_child_sa().await
            }
            false => Ok(()),
        }
    }

    fn store(&self) -> SessionStore {
        // its own table: the blob is IKEv2's format, and a profile that switches
        // version must not be handed the other's bytes
        SessionStore::new("ikev2_session", self.params.profile_id, self.params.server_name.clone())
    }

    fn persist_session(&mut self) {
        if !self.params.ike_persist {
            return;
        }

        if let Err(e) = self.save_ike_session() {
            warn!("Cannot save IKE session: {}", e);
        }
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

        let data = self.service.save_session(&office_mode)?;

        self.store().save(&data)
    }

    async fn do_restore_session(&mut self) -> anyhow::Result<Arc<TunnelSession>> {
        let data = self.store().load()?;
        let office_mode = self.service.load_session(&data)?;

        anyhow::ensure!(!office_mode.ccc_session.is_empty(), "Empty CCC session!");

        debug!("Loaded IKE session: {:?}", office_mode);

        self.ccc_session = office_mode.ccc_session.into();
        self.username = office_mode.username;

        self.ipsec_session.initiator_spi = self.session.initiator_spi();
        self.ipsec_session.responder_spi = self.session.responder_spi();
        self.ipsec_session.ike_lifetime = self.session.lifetime();
        self.ipsec_session.ike_timestamp = Local
            .timestamp_opt(self.session.timestamp() as i64, 0)
            .single()
            .unwrap_or_else(Local::now);
        self.ipsec_session.address = office_mode.ip_address;
        self.ipsec_session.netmask = office_mode.netmask;
        self.ipsec_session.dns = office_mode.dns;
        self.ipsec_session.domains = office_mode.domains;
        self.ipsec_session.lifetime = DEFAULT_ESP_LIFETIME;
        self.ipsec_session.transport_type = self.esp_transport;

        // the first exchange on the restored SA, which fails if the gateway has
        // forgotten it — and refreshes the lease when it has not
        let lease = self.service.renew_office_mode(self.ipsec_session.address).await?;

        self.ipsec_session.address = lease.address;
        if let Some(netmask) = lease.netmask {
            self.ipsec_session.netmask = netmask;
        }
        self.ipsec_session.address_lifetime = self
            .params
            .ip_lease_time
            .or(lease.expiry)
            .unwrap_or(self.ipsec_session.ike_lifetime);

        self.service.create_child_sa().await?;

        self.ipsec_session.esp_in = self.session.esp_in();
        self.ipsec_session.esp_out = self.session.esp_out();

        let now = SystemTime::now();
        self.last_rekey = Some(now);
        self.last_address_renewal = Some(now);

        self.persist_session();

        Ok(self.new_session())
    }

    async fn rekey_tunnel(&mut self) -> anyhow::Result<()> {
        debug!("Rekeying the child SA");

        self.service.rekey_child_sa().await?;
        self.apply_new_child_sa().await?;
        self.persist_session();

        Ok(())
    }

    async fn apply_new_child_sa(&mut self) -> anyhow::Result<()> {
        self.ipsec_session.esp_in = self.session.esp_in();
        self.ipsec_session.esp_out = self.session.esp_out();
        self.last_rekey = Some(SystemTime::now());

        debug!(
            "New ESP SPI: {:08x}, {:08x}",
            self.ipsec_session.esp_in.spi, self.ipsec_session.esp_out.spi
        );

        match self.command_sender {
            Some(ref sender) => Ok(sender.send(TunnelCommand::ReKey(self.ipsec_session.clone())).await?),
            None => Err(anyhow!(tr!("error-no-sender"))),
        }
    }
}

#[async_trait]
impl TunnelConnector for Ikev2TunnelConnector {
    async fn authenticate(&mut self) -> anyhow::Result<Arc<TunnelSession>> {
        // IDi travels in the first IKE_AUTH request, so unlike IKEv1 the gateway never gets a chance to ask for the username later.
        let info = self.gateway_connector.get_gateway_information().await?;

        if self.params.user_name.is_empty()
            && !info.is_identity_provider_login_type(&self.params.login_type)
            && !info.is_certificate_login_type(&self.params.login_type)
        {
            self.awaiting_username = true;

            return Ok(Arc::new(TunnelSession {
                session_id: SessionId::default(),
                state: SessionState::PendingChallenge(MfaChallenge {
                    mfa_type: MfaType::UserNameInput,
                    prompt: tr!("label-username"),
                }),
                username: None,
            }));
        }

        let username = self.params.user_name.clone();

        self.do_authenticate(username).await
    }

    async fn delete_session(&mut self) -> anyhow::Result<()> {
        self.store().delete()
    }

    async fn restore_session(&mut self) -> anyhow::Result<Arc<TunnelSession>> {
        match self.do_restore_session().await {
            Ok(session) => Ok(session),
            Err(e) => {
                // whatever went wrong, the saved SA is no longer usable, and the
                // service is carrying its keys — both have to go before a login
                let _ = self.delete_session().await;

                let info = self.gateway_connector.get_gateway_information().await?;
                let (service, session) = Self::new_service(&self.params, &info, self.esp_transport).await?;

                self.service = service;
                self.session = session;

                Err(e)
            }
        }
    }

    async fn challenge_code(
        &mut self,
        _session: Arc<TunnelSession>,
        user_input: &str,
    ) -> anyhow::Result<Arc<TunnelSession>> {
        if self.awaiting_username {
            self.awaiting_username = false;
            return self.do_authenticate(user_input.trim().to_owned()).await;
        }

        let step = self
            .service
            .step(Bytes::copy_from_slice(user_input.trim().as_bytes()))
            .await?;

        self.process_step(step).await
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

                if !self.params.ike_persist {
                    let _ = self.service.delete_ike_sa().await;
                }
            }
            TunnelEvent::RekeyCheck => {
                self.on_rekey_check().await?;
            }
            TunnelEvent::RemoteControlData(data) => {
                self.parse_isakmp(data).await?;
            }
            TunnelEvent::Rekeyed(_) => {}
            TunnelEvent::Rtt(_) => {}
        }
        Ok(())
    }

    async fn rekey(&mut self) -> anyhow::Result<()> {
        self.rekey_tunnel().await
    }
}

impl Drop for Ikev2TunnelConnector {
    fn drop(&mut self) {
        std::thread::scope(|s| {
            s.spawn(|| util::block_on(self.terminate_tunnel(false)));
        });
    }
}

#[cfg(test)]
mod tests {
    use std::{future, io, time::Duration};

    use super::*;

    async fn elapsed() -> anyhow::Error {
        let error = tokio::time::timeout(Duration::ZERO, future::pending::<()>())
            .await
            .unwrap_err();
        anyhow::Error::new(error)
    }

    #[tokio::test]
    async fn transport_failures_are_told_apart_from_refusals() {
        assert!(is_transport_failure(&elapsed().await));
        assert!(is_transport_failure(&elapsed().await.context("IKE_SA_INIT")));
        assert!(is_transport_failure(&anyhow::Error::new(io::Error::from(
            io::ErrorKind::ConnectionRefused
        ))));

        // a gateway that answered and refused is not a transport problem:
        // retrying over TCPT would only be refused again
        assert!(!is_transport_failure(&anyhow!(
            "IKE_AUTH rejected: AuthenticationFailed"
        )));
    }

    #[tokio::test]
    async fn only_a_timeout_becomes_the_login_type_hint() {
        assert_eq!(
            identity_timeout(elapsed().await).to_string(),
            tr!("error-identity-timeout")
        );
        assert_eq!(
            identity_timeout(anyhow!("INVALID_SYNTAX")).to_string(),
            "INVALID_SYNTAX"
        );
    }
}
