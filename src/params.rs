use std::path::PathBuf;

use clap::Parser;
use tracing::Level;

#[derive(Parser)]
#[clap(about = "VPN client for Checkpoint security gateway", name = "snx-rs")]
pub struct SnxParams {
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

    #[clap(long = "tun-name", short = 't', help = "Custom name for tun interface")]
    pub tun_name: Option<String>,

    #[clap(
        long = "config-file",
        short = 'c',
        help = "Read parameters from config file"
    )]
    pub config_file: Option<PathBuf>,

    #[clap(
        long = "log-level",
        short = 'l',
        help = "Enable logging to stdout [info, warn, error, debug, trace]"
    )]
    pub log_level: Option<Level>,

    #[clap(
        long = "reauth",
        short = 'r',
        help = "Enable automatic re-authentication"
    )]
    pub reauth: Option<bool>,
}

impl SnxParams {
    pub fn load(&mut self) -> anyhow::Result<()> {
        if let Some(ref config) = self.config_file {
            let data = std::fs::read_to_string(config)?;
            for line in data.lines() {
                if let Some((k, v)) = line.split_once('=') {
                    match k.trim() {
                        "user-name" => {
                            if self.user_name.is_none() {
                                self.user_name = Some(v.to_owned())
                            }
                        }
                        "password" => {
                            if self.password.is_none() {
                                self.password = Some(v.to_owned())
                            }
                        }
                        "server-name" => {
                            if self.server_name.is_none() {
                                self.server_name = Some(v.to_owned())
                            }
                        }
                        "tun-name" => {
                            if self.tun_name.is_none() {
                                self.tun_name = Some(v.to_owned())
                            }
                        }
                        "log-level" => {
                            if self.log_level.is_none() {
                                self.log_level = v.parse().ok()
                            }
                        }
                        "reauth" => {
                            if self.reauth.is_none() {
                                self.reauth = v.parse().ok()
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }
}
