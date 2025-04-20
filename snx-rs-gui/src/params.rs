use std::path::PathBuf;

use clap::Parser;
use snxcore::model::params::TunnelParams;

#[derive(Parser, Clone)]
#[clap(about = "VPN client for Check Point security gateway", name = "snx-rs-gui", version = env!("CARGO_PKG_VERSION"))]
pub struct CmdlineParams {
    #[clap(
        long = "config-file",
        short = 'c',
        global = true,
        help = "Configuration file to use [default: $HOME/.config/snx-rs/snx-rs.conf]"
    )]
    pub config_file: Option<PathBuf>,
}

impl CmdlineParams {
    pub fn config_file(&self) -> PathBuf {
        self.config_file
            .clone()
            .unwrap_or_else(TunnelParams::default_config_path)
    }
}
