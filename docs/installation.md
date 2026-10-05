# Installation

There are several ways to install `snx-rs`:

- Download the latest binary and source release [here](https://github.com/ancwrd1/snx-rs/releases/latest).
  > NOTE: artifacts with `-webkit` tag require gtk4 and webkit6 packages and are compiled with the `mobile-access` feature (the embedded Mobile Access portal).
- For Arch Linux and derivatives, the [AUR package](https://aur.archlinux.org/packages/snx-rs) can be used.
- For NixOS follow the specific [configuration instructions](https://github.com/ancwrd1/snx-rs/blob/main/docs/nixos.md).
- For Ubuntu/Debian, a DEB package is provided in the release assets.
- For RPM-based distros (Fedora, CentOS, openSUSE) use the provided RPM package.
- For Windows, use the msi installer from the release page.
- For macOS, install the Homebrew [formula](https://formulae.brew.sh/formula/snx-rs) or use the `.dmg` package from the release page (see [below](#macos)).
- For manual installation using the `.run` installer:
  1. Download the installer, then: `chmod +x snx-rs-*-linux-x86_64.run`
  2. Install the application: `sudo ./snx-rs-*-linux-x86_64.run`
- Signed repositories for APT (Debian/Ubuntu), DNF (Fedora/RHEL) and zypper (openSUSE) with the latest release builds are published at [ancwrd1.github.io/snx-rs](https://ancwrd1.github.io/snx-rs/).

## macOS

1. Download `snx-rs-<version>-aarch64-apple-darwin.dmg` from the [releases](https://github.com/ancwrd1/snx-rs/releases/latest) page.
2. Mount it and install the contained package.
3. The package is ad-hoc signed only (no Apple Developer ID) and not notarized. If Gatekeeper blocks it, right-click → Open once to approve it; a signed and notarized build opens with no prompt.
4. To uninstall, run the bundled `uninstall.sh` as root: `sudo /Applications/SNX-RS.app/Contents/Resources/uninstall.sh` (it is also included on the `.dmg`).
5. To build from source instead, see [Building from Sources](building.md).

