# Open Source Linux Client for Check Point VPN Tunnels

[![github actions](https://github.com/ancwrd1/snx-rs/workflows/CI/badge.svg)](https://github.com/ancwrd1/snx-rs/actions)
[![license](https://img.shields.io/badge/License-AGPL-v3.svg)](https://opensource.org/license/agpl-v3)

This project contains the source code for an unofficial Linux client for Check Point VPN, written in Rust.

## Key Features

* IPSec and SSL tunnel support
* Browser-based SSO, username/password, certificate, HSM token and MFA authentication
* GTK frontend with tray icon
* Split DNS via systemd-resolved for better privacy
* OS keychain integration (GNOME Keyring, KDE KWallet)
* Multiple connection profiles

## Documentation

See the full documentation in the [docs](docs/README.md) directory.

## Quick Links

* [Installation](docs/installation.md)
* [Quick Start Guide](docs/quick-start.md)
* [Configuration Options](docs/options.md)
* [Troubleshooting](docs/troubleshooting.md)
* [Building from Sources](docs/building.md)
* [Contributing](docs/contributing.md)

## License

Licensed under the [GNU Affero General Public License version 3](https://opensource.org/license/agpl-v3/).
