use std::path::PathBuf;

use clap::Parser;
use snxcore::model::params::TunnelParams;

use crate::tray::TrayEvent;

#[derive(Parser, Clone)]
#[clap(about = "VPN client for Check Point security gateway", name = "snx-rs-gui", version = env!("CARGO_PKG_VERSION"))]
pub struct CmdlineParams {
    #[clap(
        long = "config-file",
        short = 'c',
        help = "Configuration file to use [default: $HOME/.config/snx-rs/snx-rs.conf]"
    )]
    pub config_file: Option<PathBuf>,
    #[clap(
        long = "command",
        short = 'm',
        help = "Send command to the application [connect, disconnect, settings, status, exit, about]"
    )]
    pub command: Option<TrayEvent>,
    #[clap(long = "completions", help = "Generate shell completions for the given shell")]
    pub completions: Option<clap_complete::Shell>,
}

impl CmdlineParams {
    pub fn config_file(&self) -> PathBuf {
        self.config_file
            .clone()
            .unwrap_or_else(TunnelParams::default_config_path)
    }
}
