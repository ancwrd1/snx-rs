use std::{net::Ipv4Addr, path::PathBuf, time::Duration};

use clap::Parser;
use ipnet::Ipv4Net;
use snxcore::model::params::{CertType, OperationMode, TransportType, TunnelParams, TunnelType};
use tracing::level_filters::LevelFilter;

#[derive(Parser)]
#[clap(about = "VPN client for Check Point security gateway", name = "snx-rs", version = env!("CARGO_PKG_VERSION"))]
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

    #[clap(
        long = "password-factor",
        short = 'Y',
        help = "Numerical index of the password factor, 1..N [default: 1]"
    )]
    pub password_factor: Option<usize>,

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
        long = "dns-servers",
        short = 'D',
        value_delimiter = ',',
        help = "Additional DNS servers"
    )]
    pub dns_servers: Vec<Ipv4Addr>,

    #[clap(
        long = "ignore-dns-servers",
        short = 'G',
        value_delimiter = ',',
        help = "Ignore specified DNS servers from the acquired list"
    )]
    pub ignore_dns_servers: Vec<Ipv4Addr>,

    #[clap(
        long = "set-routing-domains",
        short = 'Z',
        help = "Treat received search domains as routing domains"
    )]
    pub set_routing_domains: Option<bool>,

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
        long = "ignore-server-cert",
        short = 'X',
        help = "Disable all certificate validations (NOT SECURE!)"
    )]
    pub ignore_server_cert: Option<bool>,

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

    #[clap(long = "ike-lifetime", short = 'L', help = "IPSec IKE lifetime in seconds")]
    pub ike_lifetime: Option<u64>,

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

    #[clap(long = "no-keepalive", short = 'A', help = "Disable IPSec keepalive packets")]
    pub no_keepalive: Option<bool>,

    #[clap(
        long = "port-knock",
        short = 'R',
        help = "Enable port knock workaround for NAT-T probing"
    )]
    pub port_knock: Option<bool>,

    #[clap(long = "completions", help = "Generate shell completions for the given shell")]
    pub completions: Option<clap_complete::Shell>,

    #[clap(long = "ip-lease-time", short = 'P', help = "Custom IP lease time in seconds")]
    pub ip_lease_time: Option<u64>,

    #[clap(
        long = "disable-ipv6",
        short = 'Q',
        help = "Disable IPv6 when default route is enabled"
    )]
    pub disable_ipv6: Option<bool>,

    #[clap(long = "mtu", short = 'M', help = "Custom MTU for the tunnel interface")]
    pub mtu: Option<u16>,

    #[clap(
        long = "transport-type",
        short = 'T',
        help = "IPSec transport type [auto, kernel, tcpt, udp]"
    )]
    pub transport_type: Option<TransportType>,
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

        if let Some(password_factor) = self.password_factor {
            other.password_factor = password_factor;
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

        if !self.dns_servers.is_empty() {
            other.dns_servers = self.dns_servers;
        }

        if !self.ignore_dns_servers.is_empty() {
            other.ignore_dns_servers = self.ignore_dns_servers;
        }

        if let Some(set_routing_domains) = self.set_routing_domains {
            other.set_routing_domains = set_routing_domains;
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

        if let Some(ignore_server_cert) = self.ignore_server_cert {
            other.ignore_server_cert = ignore_server_cert;
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

        if let Some(ike_lifetime) = self.ike_lifetime {
            other.ike_lifetime = Duration::from_secs(ike_lifetime);
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

        if let Some(port_knock) = self.port_knock {
            other.port_knock = port_knock;
        }

        if let Some(ip_lease_time) = self.ip_lease_time {
            other.ip_lease_time = Some(Duration::from_secs(ip_lease_time));
        }

        if let Some(disable_ipv6) = self.disable_ipv6 {
            other.disable_ipv6 = disable_ipv6;
        }

        if let Some(mtu) = self.mtu {
            other.mtu = mtu;
        }

        if let Some(transport_type) = self.transport_type {
            other.transport_type = transport_type;
        }
    }
}
