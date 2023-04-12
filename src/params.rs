use std::path::{Path, PathBuf};

use base64::Engine;
use clap::Parser;
use tracing::metadata::LevelFilter;

#[derive(Parser)]
#[clap(about = "VPN client for Checkpoint security gateway", name = "snx-rs")]
pub struct CmdlineParams {
    #[clap(long = "server-name", short = 's', help = "Server name")]
    pub server_name: Option<String>,

    #[clap(long = "user-name", short = 'u', help = "User name")]
    pub user_name: Option<String>,

    #[clap(
        long = "password",
        short = 'p',
        help = "Password in base64-encoded form"
    )]
    pub password: Option<String>,

    #[clap(
        long = "config-file",
        short = 'c',
        help = "Read parameters from config file"
    )]
    pub config_file: Option<PathBuf>,

    #[clap(
        long = "log-level",
        short = 'l',
        help = "Enable logging to stdout [off, info, warn, error, debug, trace]"
    )]
    pub log_level: Option<LevelFilter>,

    #[clap(
        long = "reauth",
        short = 'r',
        help = "Enable automatic re-authentication"
    )]
    pub reauth: Option<bool>,

    #[clap(
        long = "search-domains",
        short = 'd',
        help = "Additional search domains"
    )]
    pub search_domains: Vec<String>,

    #[clap(
        long = "default-route",
        short = 't',
        help = "Set the default route through the tunnel"
    )]
    pub default_route: Option<bool>,

    #[clap(long = "no-routing", short = 'n', help = "Do not change routing table")]
    pub no_routing: Option<bool>,

    #[clap(
        long = "no-dns",
        short = 'N',
        help = "Do not change DNS resolver configuration"
    )]
    pub no_dns: Option<bool>,
}

#[derive(Clone)]
pub struct TunnelParams {
    pub server_name: String,
    pub user_name: String,
    pub password: String,
    pub log_level: LevelFilter,
    pub reauth: bool,
    pub search_domains: Vec<String>,
    pub default_route: bool,
    pub no_routing: bool,
    pub no_dns: bool,
}

impl Default for TunnelParams {
    fn default() -> Self {
        Self {
            server_name: String::new(),
            user_name: String::new(),
            password: String::new(),
            log_level: LevelFilter::OFF,
            reauth: false,
            search_domains: Vec::new(),
            default_route: false,
            no_routing: false,
            no_dns: false,
        }
    }
}

impl TunnelParams {
    pub fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let mut params = Self::default();
        let data = std::fs::read_to_string(path)?;
        for line in data.lines() {
            if !line.trim().starts_with('#') {
                if let Some((k, v)) = line.split_once('=').map(|(k, v)| (k.trim(), v.trim())) {
                    match k {
                        "server-name" => params.server_name = v.to_string(),
                        "user-name" => params.user_name = v.to_string(),
                        "password" => {
                            params.password = String::from_utf8_lossy(
                                &base64::engine::general_purpose::STANDARD.decode(v)?,
                            )
                            .into_owned();
                        }
                        "log-level" => params.log_level = v.parse().unwrap_or(LevelFilter::OFF),
                        "reauth" => params.reauth = v.parse().unwrap_or_default(),
                        "search-domains" => {
                            params.search_domains =
                                v.split(',').map(|s| s.trim().to_owned()).collect()
                        }
                        "default-route" => params.default_route = v.parse().unwrap_or_default(),
                        "no-routing" => params.no_routing = v.parse().unwrap_or_default(),
                        "no-dns" => params.no_dns = v.parse().unwrap_or_default(),
                        _ => {}
                    }
                }
            }
        }
        Ok(params)
    }

    pub fn merge(&mut self, other: CmdlineParams) {
        if let Some(server_name) = other.server_name {
            self.server_name = server_name;
        }

        if let Some(user_name) = other.user_name {
            self.user_name = user_name;
        }

        if let Some(password) = other.password {
            self.password = password;
        }

        if let Some(reauth) = other.reauth {
            self.reauth = reauth;
        }

        if let Some(log_level) = other.log_level {
            self.log_level = log_level;
        }

        if !other.search_domains.is_empty() {
            self.search_domains = other.search_domains;
        }

        if let Some(default_route) = other.default_route {
            self.default_route = default_route;
        }

        if let Some(no_routing) = other.no_routing {
            self.no_routing = no_routing;
        }

        if let Some(no_dns) = other.no_dns {
            self.no_dns = no_dns;
        }
    }
}
