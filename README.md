# Rust client for Checkpoint VPN tunnels

This project implements a client for Checkpoint VPN written in Rust programming language.
Based on the reverse engineered protocol used by the vendor application.

## Implemented features

* SSL tunnel
* IPSec tunnel
* Microsoft MFA Authenticator

## Roadmap
 
* GUI with tray icon
* Connection stats
* SAML SSO support with IKE Phase 2 exchange

## Usage

Run `snx-rs --help` to get a help with all command line parameters.

Run `assets/install.sh` to install the release build to the host system as a systemd service (Linux only).

## License

Licensed under MIT or Apache license ([LICENSE-MIT](https://opensource.org/licenses/MIT) or [LICENSE-APACHE](https://opensource.org/licenses/Apache-2.0))
