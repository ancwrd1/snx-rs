# Linux client for Checkpoint VPN written in Rust

This project contains a Linux client for Checkpoint VPN written in Rust programming language.
Based on the reverse engineered protocol used by the vendor 'snx' application.

## Todo
 
* GUI with tray icon
* Connection stats
* macOS support (dns and routing setup)

## Usage

Run `snx-rs --help` to get a help with all command line parameters.

Run `assets/install.sh` to install the release build to the host system as a systemd service.

## License

Licensed under MIT or Apache license ([LICENSE-MIT](https://opensource.org/licenses/MIT) or [LICENSE-APACHE](https://opensource.org/licenses/Apache-2.0))
