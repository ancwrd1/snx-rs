# Rust client for Checkpoint VPN tunnels

This project implements a client for Checkpoint VPN written in Rust programming language.
Based on the reverse engineered protocol used by the vendor 'snx' application.

## Todo
 
* GUI with tray icon
* Connection stats
* Complete macOS support (DNS servers and suffixes for tun interface)

## Usage

Run `snx-rs --help` to get a help with all command line parameters.

Run `assets/install.sh` to install the release build to the host system as a systemd service (Linux only).

## License

Licensed under MIT or Apache license ([LICENSE-MIT](https://opensource.org/licenses/MIT) or [LICENSE-APACHE](https://opensource.org/licenses/Apache-2.0))
