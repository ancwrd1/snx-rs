//! Pieces of the Check Point login that both IKE versions build identically:
//! the client identity, and the CCC authentication blob which travels in an
//! IKEv1 attribute payload or an IKEv2 notify without changing shape.

use anyhow::Context;
use i18n::tr;
use isakmp::{certs::ClientCertificate, model::Identity};
use openssl::{nid::Nid, x509::X509};
use tracing::debug;

use crate::{
    model::{
        MfaChallenge, MfaType,
        params::{CertType, TunnelParams},
        proto::{AuthenticationRealm, ClientLoggingData, GatewayInformation},
    },
    sexpr::SExpression,
    util,
};

pub(crate) fn new_identity(params: &TunnelParams, info: &GatewayInformation) -> anyhow::Result<Identity> {
    let hybrid_auth = !info.is_certificate_login_type(&params.login_type);

    let identity = match params.cert_type {
        CertType::Pkcs12 => match (&params.cert_path, &params.cert_password) {
            (Some(path), Some(password)) => Identity::Pkcs12 {
                data: std::fs::read(path)?,
                password: password.clone(),
                hybrid_auth,
            },
            _ => anyhow::bail!(tr!("error-no-pkcs12")),
        },
        CertType::Pkcs8 => match params.cert_path {
            Some(ref path) => Identity::Pkcs8 {
                path: path.clone(),
                hybrid_auth,
            },
            None => anyhow::bail!(tr!("error-no-pkcs8")),
        },
        CertType::Pkcs11 => match params.cert_password {
            Some(ref pin) => Identity::Pkcs11 {
                driver_path: params.cert_path.clone().unwrap_or_else(|| "opensc-pkcs11.so".into()),
                pin: pin.clone(),
                key_id: params
                    .cert_id
                    .as_ref()
                    .map(|s| hex::decode(s.replace(':', "")).unwrap_or_default().into()),
                hybrid_auth,
            },
            None => anyhow::bail!(tr!("error-no-pkcs11")),
        },
        #[cfg(windows)]
        CertType::System => {
            let common_name = match params.cert_id {
                Some(ref id) => id.clone(),
                None => hostname::get()?.to_string_lossy().into_owned(),
            };
            Identity::System { common_name }
        }
        #[cfg(not(windows))]
        CertType::System => anyhow::bail!(tr!("error-not-implemented")),
        CertType::None => Identity::None,
    };
    Ok(identity)
}

/// The host part of the machine certificate's common name, which the gateway
/// logs as the machine name.
pub(crate) fn machine_name(certificate: &dyn ClientCertificate) -> Option<String> {
    certificate
        .certs()
        .first()
        .and_then(|der| X509::from_der(der.as_ref()).ok())
        .and_then(|cert| {
            cert.subject_name().entries().find_map(|entry| {
                if entry.object().nid() == Nid::COMMONNAME {
                    entry
                        .data()
                        .to_string()
                        .ok()
                        .and_then(|s| s.split('.').next().map(String::from))
                } else {
                    None
                }
            })
        })
}

pub(crate) fn auth_blob(params: &TunnelParams, info: &GatewayInformation, machine_name: Option<String>) -> SExpression {
    let mut client_logging_data = params
        .client_logging_data
        .as_ref()
        .and_then(|path| ClientLoggingData::load(path).ok())
        .unwrap_or_default();

    client_logging_data.os_name.get_or_insert_with(|| "Windows".to_owned());
    client_logging_data.device_id.get_or_insert_with(util::get_device_id);
    if client_logging_data.machine_name.is_none() {
        client_logging_data.machine_name = machine_name;
    }

    debug!("Client logging data: {:?}", client_logging_data);

    let realm = AuthenticationRealm {
        client_type: params.tunnel_type.as_client_type().to_owned(),
        old_session_id: String::new(),
        protocol_version: 100,
        client_mode: params.client_mode.clone(),
        selected_realm_id: params.login_type.clone(),
        secondary_realm_hash: info
            .get_login_option(&params.login_type)
            .map(|o| o.secondary_realm_hash.clone()),
        client_logging_data: Some(client_logging_data),
    };

    SExpression::from(&realm)
}

/// The `msg_obj` S-expression that trails the challenge prompt, in both the
/// IKEv1 challenge attribute and the IKEv2 EAP request, and names the factor
/// the gateway is asking for.
pub(crate) fn challenge_from_msg_obj(msg_obj: &SExpression) -> anyhow::Result<MfaChallenge> {
    let state = msg_obj
        .get_value::<String>("msg_obj:authentication_state")
        .unwrap_or_else(|| "challenge".to_owned());

    if state != "challenge" && state != "new_factor" && state != "failed_attempt" {
        anyhow::bail!(tr!("error-not-challenge-state"));
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

    Ok(MfaChallenge {
        mfa_type: MfaType::from_id(&id),
        prompt,
    })
}
