use clap::Parser;

use crate::tray::TrayEvent;

#[derive(Parser, Clone)]
#[clap(about = "VPN client for Check Point security gateway", name = "snx-rs-gui", version = env!("CARGO_PKG_VERSION"))]
pub struct CmdlineParams {
    #[clap(
        long = "command",
        short = 'm',
        help = "Send command to the application [connect, disconnect, settings, status, exit, about]"
    )]
    pub command: Option<TrayEvent>,
    #[clap(long = "completions", help = "Generate shell completions for the given shell")]
    pub completions: Option<clap_complete::Shell>,
}
