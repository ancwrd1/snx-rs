# System Requirements

* A recent Linux distribution with kernel version 4.19 or higher.
* `systemd-resolved` is highly recommended as a global DNS resolver, to avoid sending all DNS traffic to the corporate VPN servers.
* Optional: GTK 4.10+ and WebKit 6.0+ for the `mobile-access` feature.
* GNOME desktop: [AppIndicator](https://extensions.gnome.org/extension/615/appindicator-support/) extension. Not needed for Ubuntu.

## macOS

* macOS 11 (Big Sur) or later, Apple Silicon or Intel.
* Root privileges (`sudo`) are required for tunnel setup.
* The GUI frontend is not available on macOS yet; use `snx-rs`/`snxctl` from the command line.
