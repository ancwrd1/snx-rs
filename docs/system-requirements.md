# System Requirements

* A recent Linux distribution with kernel version 4.19 or higher.
* `systemd-resolved` is highly recommended as a global DNS resolver, to avoid sending all DNS traffic to the corporate VPN servers.
* Optional: GTK 4.10+ and WebKit 6.0+ for the `mobile-access` feature.
* GNOME desktop: [AppIndicator](https://extensions.gnome.org/extension/615/appindicator-support/) extension. Not needed for Ubuntu.

## macOS

* macOS 11 (Big Sur) or later, Apple Silicon (`aarch64-apple-darwin`) or Intel (`x86_64-apple-darwin`).
* Root privileges (`sudo`) are required for tunnel setup; the `.pkg` installer's LaunchDaemon runs as root.
* Release binaries are ad-hoc signed only (no Apple Developer ID) and not notarized. Gatekeeper may block first run; clear the quarantine attribute (`xattr -dr com.apple.quarantine <path>`) or right-click → Open.
* The GUI frontend runs on macOS 11+ as a menu-bar app; the `snx-rs`/`snxctl` command-line tools are also available.
