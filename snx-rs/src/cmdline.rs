use std::{path::PathBuf, time::Duration};

use clap::Parser;
use ipnet::Ipv4Net;
use tracing::level_filters::LevelFilter;

use snxcore::model::params::{CertType, OperationMode, TunnelParams, TunnelType};

#[derive(Parser)]
#[clap(about = "VPN client for Checkpoint security gateway", name = "snx-rs")]
pub struct CmdlineParams {
    #[clap(long = "server-name", short = 's', help = "Server name")]
    pub server_name: Option<String>,

    #[clap(
        long = "mode",
        short = 'm',
        default_value = "standalone",
        help = "Operation mode, one of: standalone, command, info"
    )]
    pub mode: OperationMode,

    #[clap(long = "user-name", short = 'u', help = "User name")]
    pub user_name: Option<String>,

    #[clap(long = "password", short = 'p', help = "Password in base64-encoded form")]
    pub password: Option<String>,

    #[clap(long = "config-file", short = 'c', help = "Read parameters from config file")]
    pub config_file: Option<PathBuf>,

    #[clap(
        long = "log-level",
        short = 'l',
        help = "Enable logging to stdout, one of: off, info, warn, error, debug, trace"
    )]
    pub log_level: Option<LevelFilter>,

    #[clap(
        long = "search-domains",
        short = 'd',
        value_delimiter = ',',
        help = "Additional search domains"
    )]
    pub search_domains: Vec<String>,

    #[clap(
        long = "ignore-search-domains",
        short = 'i',
        value_delimiter = ',',
        help = "Ignore specified search domains from the acquired list"
    )]
    pub ignore_search_domains: Vec<String>,

    #[clap(
        long = "default-route",
        short = 't',
        help = "Set the default route through the tunnel"
    )]
    pub default_route: Option<bool>,

    #[clap(long = "no-routing", short = 'n', help = "Ignore all routes from the acquired list")]
    pub no_routing: Option<bool>,

    #[clap(
        long = "add-routes",
        short = 'a',
        value_delimiter = ',',
        help = "Additional routes through the tunnel"
    )]
    pub add_routes: Vec<Ipv4Net>,

    #[clap(
        long = "ignore-routes",
        short = 'I',
        value_delimiter = ',',
        help = "Ignore specified routes from the acquired list"
    )]
    pub ignore_routes: Vec<Ipv4Net>,

    #[clap(long = "no-dns", short = 'N', help = "Do not change DNS resolver configuration")]
    pub no_dns: Option<bool>,

    #[clap(
        long = "no-cert-check",
        short = 'H',
        help = "Do not validate server common name in the certificate"
    )]
    pub no_cert_check: Option<bool>,

    #[clap(
        long = "ignore-server-cert",
        short = 'X',
        help = "Disable all certificate validations (NOT SECURE!)"
    )]
    pub ignore_server_cert: Option<bool>,

    #[clap(
        long = "ipsec-cert-check",
        short = 'S',
        help = "Validate IPSec certificates acquired during IKE identity protection phase"
    )]
    pub ipsec_cert_check: Option<bool>,

    #[clap(long = "tunnel-type", short = 'e', help = "Tunnel type, one of: ssl, ipsec")]
    pub tunnel_type: Option<TunnelType>,

    #[clap(
        long = "ca-cert",
        short = 'k',
        value_delimiter = ',',
        help = "Custom CA certificates in PEM or DER format"
    )]
    pub ca_cert: Vec<PathBuf>,

    #[clap(
        long = "login-type",
        short = 'o',
        help = "Login type, obtained from running the 'snx-rs -m info -s address', login_options_list::id field"
    )]
    pub login_type: Option<String>,

    #[clap(
        long = "cert-type",
        short = 'y',
        help = "Enable certificate authentication via the provided method, one of: pkcs8, pkcs11, pkcs12, none"
    )]
    pub cert_type: Option<CertType>,

    #[clap(
        long = "cert-path",
        short = 'z',
        help = "Path to PEM file for PKCS8, path to PFX file for PKCS12, path to driver file for PKCS11 token"
    )]
    pub cert_path: Option<PathBuf>,

    #[clap(
        long = "cert-password",
        short = 'x',
        help = "Password for PKCS12 file or PIN for PKCS11 token"
    )]
    pub cert_password: Option<String>,

    #[clap(long = "cert-id", short = 'w', help = "Certificate ID in hexadecimal form")]
    pub cert_id: Option<String>,

    #[clap(long = "if-name", short = 'f', help = "Interface name for tun or xfrm device")]
    pub if_name: Option<String>,

    #[clap(
        long = "no-keychain",
        short = 'K',
        help = "Do not use OS keychain to store or retrieve user password"
    )]
    pub no_keychain: Option<bool>,

    #[clap(
        long = "server-prompt",
        short = 'P',
        help = "Ask server for authentication data prompt values"
    )]
    pub server_prompt: Option<bool>,

    #[clap(long = "esp-lifetime", short = 'E', help = "IPSec ESP lifetime in seconds")]
    pub esp_lifetime: Option<u64>,

    #[clap(long = "ike-lifetime", short = 'L', help = "IPSec IKE lifetime in seconds")]
    pub ike_lifetime: Option<u64>,

    #[clap(long = "ike-port", short = 'R', help = "IPSec IKE communication port [default: 500]")]
    pub ike_port: Option<u16>,

    #[clap(
        long = "ike-persist",
        short = 'W',
        help = "Store IKE session to disk and load it automatically"
    )]
    pub ike_persist: Option<bool>,

    #[clap(
        long = "client-mode",
        short = 'C',
        help = "Custom client mode [default: secure_connect]"
    )]
    pub client_mode: Option<String>,

    #[clap(long = "no-keepalive", short = 'A', help = "Disable keepalive packets")]
    pub no_keepalive: Option<bool>,
}

impl CmdlineParams {
    pub fn merge_into_tunnel_params(self, other: &mut TunnelParams) {
        if let Some(server_name) = self.server_name {
            other.server_name = server_name;
        }

        if let Some(user_name) = self.user_name {
            other.user_name = user_name;
        }

        if let Some(password) = self.password {
            other.password = password;
            let _ = other.decode_password();
        }

        if let Some(log_level) = self.log_level {
            other.log_level = log_level.to_string();
        }

        if !self.search_domains.is_empty() {
            other.search_domains = self.search_domains;
        }

        if !self.ignore_search_domains.is_empty() {
            other.ignore_search_domains = self.ignore_search_domains;
        }

        if let Some(default_route) = self.default_route {
            other.default_route = default_route;
        }

        if let Some(no_routing) = self.no_routing {
            other.no_routing = no_routing;
        }

        if let Some(no_dns) = self.no_dns {
            other.no_dns = no_dns;
        }

        if !self.add_routes.is_empty() {
            other.add_routes = self.add_routes;
        }

        if !self.ignore_routes.is_empty() {
            other.ignore_routes = self.ignore_routes;
        }

        if let Some(tunnel_type) = self.tunnel_type {
            other.tunnel_type = tunnel_type;
        }

        if !self.ca_cert.is_empty() {
            other.ca_cert = self.ca_cert;
        }

        if let Some(no_cert_check) = self.no_cert_check {
            other.no_cert_check = no_cert_check;
        }

        if let Some(ignore_server_cert) = self.ignore_server_cert {
            other.ignore_server_cert = ignore_server_cert;
        }

        if let Some(ipsec_cert_check) = self.ipsec_cert_check {
            other.ipsec_cert_check = ipsec_cert_check;
        }

        if let Some(login_type) = self.login_type {
            other.login_type = login_type;
        }

        if let Some(cert_type) = self.cert_type {
            other.cert_type = cert_type;
        }

        if let Some(cert_path) = self.cert_path {
            other.cert_path = Some(cert_path);
        }

        if let Some(cert_password) = self.cert_password {
            other.cert_password = Some(cert_password);
        }

        if let Some(cert_id) = self.cert_id {
            other.cert_id = Some(cert_id);
        }

        if let Some(if_name) = self.if_name {
            other.if_name = Some(if_name);
        }

        if let Some(no_keychain) = self.no_keychain {
            other.no_keychain = no_keychain;
        }

        if let Some(server_prompt) = self.server_prompt {
            other.server_prompt = server_prompt;
        }

        if let Some(esp_lifetime) = self.esp_lifetime {
            other.esp_lifetime = Duration::from_secs(esp_lifetime);
        }

        if let Some(ike_lifetime) = self.ike_lifetime {
            other.ike_lifetime = Duration::from_secs(ike_lifetime);
        }

        if let Some(ike_port) = self.ike_port {
            other.ike_port = ike_port;
        }

        if let Some(ike_persist) = self.ike_persist {
            other.ike_persist = ike_persist;
        }

        if let Some(client_mode) = self.client_mode {
            other.client_mode = client_mode;
        }

        if let Some(no_keepalive) = self.no_keepalive {
            other.no_keepalive = no_keepalive;
        }
    }
}
