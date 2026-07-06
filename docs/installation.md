# Installation

NOTE: artifacts with `-webkit` tag require gtk4 and webkit6 packages and are compiled with `mobile-access` feature, see below.

Download the latest binary and source release [here](https://github.com/ancwrd1/snx-rs/releases/latest).<br/>
For Arch Linux and derivatives, the [AUR package](https://aur.archlinux.org/packages/snx-rs) can be used.<br/>
For NixOS follow the specific [configuration instructions](https://github.com/ancwrd1/snx-rs/blob/main/docs/nixos.md).<br/>
For Ubuntu/Debian, a DEB package is provided in the release assets.<br/>
For RPM-based distros (Fedora, CentOS, openSUSE) use the provided RPM package.<br/>
For Windows, use the msi installer from the release page.<br/>
For macOS, use the `.pkg` installer from the release page (see below).<br/>
For manual installation using .run installer:

1. Download the installer, then: `chmod +x snx-rs-*-linux-x86_64.run`
2. Install the application: `sudo ./snx-rs-*-linux-x86_64.run`

For macOS, install the CLI (`snx-rs`, `snxctl`) and the `com.github.snx-rs` LaunchDaemon from the `.pkg`:

1. Download `snx-rs-<version>-aarch64-apple-darwin.pkg` from the [releases](https://github.com/ancwrd1/snx-rs/releases/latest) page.
2. Install it: `sudo installer -pkg snx-rs-*.pkg -target /`. This installs the `snx-rs`/`snxctl` tools and loads the LaunchDaemon (runs as root from a root-owned location, restarts on failure, logs to `/var/log/snx-rs.log`).
3. The package is ad-hoc signed only (no Apple Developer ID) and not notarized. If Gatekeeper blocks it, right-click → Open once to approve it; a signed and notarized build opens with no prompt.
4. To uninstall, run the bundled `uninstall.sh` as root: `sudo /Applications/SNX-RS.app/Contents/Resources/uninstall.sh` (it is also included on the `.dmg`).
5. To build from source instead, see [Building from Sources](building.md). The `.dmg` contains the `SNX-RS` menu-bar app; drag it to Applications.

Signed APT and DNF repositories with the latest release builds are published at [ancwrd1.github.io/snx-rs](https://ancwrd1.github.io/snx-rs/).
