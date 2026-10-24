# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Linux: snx-rs checks for the `CAP_NET_ADMIN` capability instead of the root user, so it can run as a regular user with that capability, for example in a Docker container. The Dockerfile sets the capability on the binary.
- The PKCS12/PKCS8 client certificate is now read by the GUI or `snxctl` and sent to the service, so certificate login works when the service is sandboxed. The certificate file must be readable by the user running the frontend.

### Fixed

- Fixed a regression where mobile access authentication with webkit did not work.

## [6.5.4] - 2026-10-19

### Changed

- Change GPG signing key for packages from Ed25519 to RSA for compatibility with older enterprise distros.
  Existing APT users must re-download `/etc/apt/sources.list.d/snx-rs.sources` (it embeds the key); DNF and zypper prompt to import the new key.

### Fixed

- Fixed IPsec login failing with InvalidCookie when browser/MFA authentication takes longer than 20 seconds (#279)

## [6.5.3] - 2026-10-18

### Fixed

- Fixed a regression with IKEv1 tunnel rekeying.

## [6.5.2] - 2026-10-16

### Changed

- Linux x86_64: release packages (non-webkit) are now linked against glibc 2.28, so they also run on RHEL/Alma/Rocky/Oracle Linux 8.

### Fixed

- Fixed a problem with service crash when a stray packet is received on NAT-T UDP port that is not a valid IKE message (#274).
- Fixed a problem with TCPT IKE transport being disconnected after some idle timeout.
- Linux: fixed tunnel teardown right after connecting on systems with systemd-resolved older than 240 (e.g. RHEL 8), which lack the `SetLinkDefaultRoute` D-Bus method.

## [6.5.1] - 2026-10-12

### Fixed

- Bugfix for IKEv2 over TCPT (#273)

## [6.5.0] - 2026-10-12

### Added

- Added the `auto-disconnect` option which disconnects the tunnel when the GUI frontend exits.
- Added the `use-internal-gateway-ip` option, also in the GUI advanced settings, for gateways behind NAT. The gateway's internal address is detected from its MEP settings. With the `tcpt` and `udp` transports, keepalive and SCV are sent to it instead of the server address; with `tcpt`, ESP as well.
- IKEv1: gateways that assign no Office Mode address are supported. Like the Windows client, the tunnel interface gets a private placeholder address that is translated to the physical one. This needs the userspace ESP path, so `transport-type=auto` uses `udp` instead of `kernel` for such gateways.

### Changed

- Linux: the DEB, RPM and .run packages now install the systemd unit into `/usr/lib/systemd/system` instead of `/etc/systemd/system`. The service is re-enabled automatically on upgrade.

## [6.4.1] - 2026-09-24

### Added

- macOS: enabled IPsec keepalive, so a dead tunnel is detected without waiting for the next rekey.

### Fixed

- Fixed a problem with semicolon-separated DNS suffixes when using IKEv2 exchange.

## [6.4.0] - 2026-09-22

### Added

- Added experimental IKEv2 support for the IPsec tunnel, controlled by the new `ike-version` option. The default value is `auto` which selects IKEv2 when the server prefers it. See [Tunnel Types](https://github.com/ancwrd1/snx-rs/blob/main/docs/tunnel-types.md) for the current limitations.

## [6.3.1] - 2026-09-17

### Fixed

- Fixed a non-working certificate enrollment and renewal, see [Certificates](https://github.com/ancwrd1/snx-rs/blob/main/docs/certificates.md) section in the documentation.

## [6.3.0] - 2026-09-10

### Added

- Added the `notification-level` option to control the verbosity of GUI desktop notifications: off, minimal, standard or verbose.
- Added notifications for tunnel disconnection, connection attempts, IPsec rekeying and upcoming disconnection due to IKE SA expiry.
- Added the `ipsec ike-state` and `ipsec rekey` commands to `snxctl` to display and renew the IPsec security associations.
- Added the `list` command to `snxctl` to list the connection profiles.

### Changed

- When IKE SA expires, disconnect the tunnel automatically and remove the IKE persistent state.
- GUI: moved the IPsec-specific options into a separate "IPsec" section in the advanced settings.
- Linux: NAT-T traffic is now sent from the fixed port 4500 where possible, with a fallback to an ephemeral port.

### Fixed

- GUI: fixed a problem with menu translations being not updated after closing the settings dialog.

## [6.2.4] - 2026-08-12

### Added

- Added `mfa-code` config file option, honored by `snxctl` in command mode, not only the `--mfa-code` CLI flag of the standalone `snx-rs` binary.

### Fixed

- macOS: fixed the installer failing to upgrade over a running daemon (#241).
- macOS: fixed GUI application crash upon exit when windows are still opened (#242).
- Fixed excessive vertical spacing around the icon theme and certificate auth type settings rows.

## [6.2.3] - 2026-08-01

### Changed

- Linux: the x86_64 release build without the `mobile-access` feature is now linked against GLIBC 2.34, for compatibility with older distributions (#228).

### Fixed

- Windows, macOS: fixed the status window being displayed too narrow.
- Windows: several fixes in the MSI installer for better upgrade handling.

## [6.2.2] - 2026-07-25

### Added

- macOS: allow uninstallation by double-clicking the uninstallation script from the Finder.

### Changed

- Few optimizations and fixes in the Linux xfrm module.

### Fixed

- Fixed a glitch in the machine certificate UI section.

## [6.2.1] - 2026-07-14

### Changed

- Refactored macOS packaging to only build one dmg file which includes a full installer.

### Removed

- Removed unnecessary user signout call during tunnel disconnect (#234).

### Fixed

- Fixed a bug with routing rule cleanup after tun-based tunnel termination.

## [6.2.0] - 2026-07-10

This version adds experimental support to run snx-rs on macOS, on both Apple Silicon and Intel.
Networking is implemented using native macOS APIs (utun, PF_ROUTE routing sockets, SystemConfiguration, Keychain) instead of shelling out to external tools, the same approach used on Linux and Windows.
Distributed as a `.dmg` (GUI app bundle) and a `.pkg` (CLI plus a launchd service); binaries are ad-hoc signed and not notarized yet.

### Added

- IPsec uses a userspace TUN/ESP transport, same as on Windows, since macOS has no kernel xfrm module.
- Split DNS is implemented via SystemConfiguration (SCDynamicStore) scoped/supplemental match domains.
- The `snx-rs-gui` frontend runs on macOS: a native menu-bar tray built with the `tray-icon` crate, using template icons that follow the system appearance, and an embedded WebKit view for the Mobile Access portal login.
- Added macOS packaging: a `.dmg` with the GUI app bundle and a `.pkg` installing `snx-rs`/`snxctl` with a launchd LaunchDaemon (`com.github.snx-rs`). Ad-hoc signed only, not notarized yet.

### Fixed

- Fixed a problem with status window flickering, position and auto-resizing.
- Fixed a bug where the language selection dropdown was cut off.

## [6.1.2] - 2026-06-25

### Added

- Ask for HSM pin code if it is not provided by the user (#225)

### Fixed

- Fixed a problem with connectivity check on some corporate networks
- Fixed a bug in the UI where the XFRM transport choice was not shown

## [6.1.1] - 2026-06-09

### Changed

- Improve visual feedback and notification in the frontend when gateway address is not accessible.

### Fixed

- Fixed a problem with inconsistent caching of gateway information when certificate settings are changed.

## [6.1.0] - 2026-05-28

This version adds experimental support to run snx-rs on Windows, using wintun driver.
MSI installer can be downloaded from the [releases](https://github.com/ancwrd1/snx-rs/releases/latest) page.
All original functionality is available in the Windows version, except for the kernel-based (xfrm) IPsec support.
Important note: MSI and binaries are not signed yet.

### Changed

- More consistent UI for the machine certificate authentication settings.

### Fixed

- Fixed a bug where tray icon was updated too often.

## [6.0.6] - 2026-05-15

### Added

- Added `no-split-dns` option to disable split DNS for systemd-resolved, required in the container images.

### Fixed

- Fixed native kernel IPsec support detection when IPv6 is disabled.

## [6.0.5] - 2026-05-07

### Added

- Added interface statistics in the status window.

### Changed

- Improved performance when setting up the tunnel.
- Update the taskbar menu and Connect button in the status window after the profiles are reordered or modified.

### Fixed

- Fixed a problem with icon file permissions in the .run installer.

## [6.0.4] - 2026-04-30

### Added

- Added `tls-version-max` parameter to cap the maximum TLS version negotiated with the gateway. The default is now `1.2`, which avoids silent connect timeouts against gateways (notably Check Point SVN foundation) that fail to respond to TLS 1.3 ClientHello produced by recent OpenSSL releases. Set `tls-version-max=default` to keep the previous behavior.
- Added connection profiles reordering in the UI.
- Added connection profiles support to snxctl command line utility via the `--profile` option.

### Fixed

- Fixed a regression with the manually specified routes when `no-routing` option was enabled.

## [6.0.3] - 2026-04-27

Note to downstream package maintainers: this release requires to copy the `package/icons/*.svg` files to `/usr/share/icons/hicolor/symbolic/apps` directory.

### Added

- Added deb and rpm repositories, published at [ancwrd1.github.io/snx-rs](https://ancwrd1.github.io/snx-rs/).
- Added symbolic icons to automatically choose dark/light theme of the taskbar icon.
- Restart frontend automatically after upgrade (beginning with next version).

## [6.0.1] - 2026-04-24

### Fixed

- Fixed a bug with the .deb package which did not restart the service correctly during upgrade.
- Fixed labels alignment in the status dialog.
- Fixed dark/light theme detection on Ubuntu.

## [6.0.0] - 2026-04-21

Marking the project's three-year anniversary, this major release delivers a complete UI rewrite and a substantial overhaul of the routing stack.
What started as a reverse-engineering experiment born out of frustration with the official `snx` client has grown into a full-featured application
with feature parity with the Check Point Windows client, while offering better privacy and performance on Linux.

### Added

- The connection status dialog now shows the name of the active connection profile.

### Changed

- Rewritten GUI frontend based on the [Slint framework](https://slint.dev), replacing the previous GTK 4 implementation.
- Tray menu now shows either a Connect or a Disconnect item depending on the current tunnel state, instead of exposing both.
- Refactored split-tunnel routing setup to allow routes that contain the VPN gateway address. Fixes #199.
- Improved error handling and recovery during routing setup and teardown, so a partial failure no longer leaves stale routes behind.
- Keychain passwords are now stored per connection profile instead of globally. Existing users will be prompted to re-enter their password on first connect after the upgrade.

### Removed

- Dropped the GTK 4 dependency for generic builds. GTK 4 and WebKitGTK are still required when the `mobile-access` feature is enabled, to host the embedded webview.

### Fixed

- Fixed a regression where auto-connect did not trigger reliably on GUI startup.
- Several routing fixes for the SSL tunnel and the `default-route` option.
- Fixed a regression with additional search domains being incorrectly treated as routing domains. Fixes #198.

## [5.3.0] - 2026-04-11

### Added

- Added `allow-forwarding` option to enable packet forwarding for the tunnel interface. Forwarding is disabled by default.

### Changed

- Application now uses Linux APIs directly instead of running the external commands to configure OS networking stack.
- Switched to GTK 4.10, requiring Linux distros from 2023 and later.
- Flipped the `no-keychain` parameter to be `keychain` so that keychain usage is opt-in instead of opt-out.

## [5.2.4] - 2026-04-01

### Added

- Added SCV policy emulation, which may fix the long-standing issue of blocked access to Intranet resources.

### Fixed

- Fixed a potential problem with the SSL tunnel when connecting to gateway version R82.

## [5.2.3] - 2026-03-18

### Fixed

- Fixed a regression where the user certificate authentication stopped working when it is supplemented with 2FA.

## [5.2.2] - 2026-03-05

### Fixed

- Fixed a deadlock in the service when tunnel rekeying fails, requiring service restart.

## [5.2.1] - 2026-03-04

### Added

- Added `client-logging-data` parameter to fully impersonate the official Check Point client, documented [here](https://github.com/ancwrd1/snx-rs/blob/main/docs/options.md).

### Fixed

- Fixed a reported issue with machine certificates where the wrong machine name was extracted from the certificate subject.
- Fixed a problem with non-responsive service when TCPT packet sending stalls.

## [5.2.0] - 2026-03-02

### Added

- Added experimental support for [machine certificate authentication](https://github.com/ancwrd1/snx-rs/blob/main/README.md#machine-certificate-authentication).
- Added a file chooser dialog for the UI fields which require a file path.

### Fixed

- Fixed a problem reported by some users where OTP could not be acquired from the browser.

## [5.1.0] - 2026-02-17

### Added

- Added certificate enrollment and renewal operations via `snx-rs` command line interface. See the [documentation](https://github.com/ancwrd1/snx-rs/blob/main/README.md#certificate-enrollment) on how to use this feature.
- Added `--no-tray` option to the GUI application to disable tray notification icon for the desktop environments which do not have dbus SNI interface. The connection status window will be shown instead.

### Fixed

- Fixed a wrong Installed-Size field for the deb package.
- Fixed a problem with a potential UI deadlock under some conditions.
- Mobile Access: fixed a problem with the wrong URL of the VPN access portal.

## [5.0.5] - 2026-02-01

### Added

- Added Croatian localization.
- Added ARM64 builds to the CI release pipeline.

### Changed

- Better xfrm detection for custom monolithic kernels.
- Retry tray icon creation for 10 seconds to mitigate autostart race condition on some systems.
- Webkit/mobile-access is disabled by default in the CI release build to minimize system dependencies.

### Fixed

- Bugfix for IKE session storage.

### Security

- Internal: use `secrecy` crate for secure password handling.

## [5.0.2] - 2026-01-12

### Added

- Added `--mfa-code` command line option to specify the MFA code manually, typically a TOTP code, to be used in the scripts.
- Added a more clear error message in the settings UI when a certificate issue occurs.

### Changed

- Use `PasswordEntry` UI element for the password input which also shows the unhide button.

### Fixed

- Fixed snx-rs service being disabled by default on the Debian systems when installing from the DEB file.

## [5.0.1] - 2025-12-17

### Fixed

- Packaging fixes: removed libsqlite3.so dependency

## [5.0.0] - 2025-12-16

### Added

- Added Mobile Access authentication support using the embedded WebKit browser. Requires a libwebkitgtk-6.0 package and a cargo feature flag `mobile-access`. This feature emulates the native Check Point 'cshell' tunnel.
- Added DEB and RPM packaging assets.

### Changed

- Use SQLite database for storing the IKE sessions.

### Removed

- GUI app no longer accepts a command line parameter with a configuration file.

### Fixed

- Various bugfixes and improvements for connection profiles and the UI.

## [4.9.1] - 2025-11-22

### Fixed

- Fixed several UI issues with the connection profiles.
- Fixed a regression where the cancellation of pending browser SSO did not work correctly.

## [4.9.0] - 2025-11-12

### Added

- Added support for multiple connection profiles in the UI.

### Fixed

- Fixed a few internal stability issues.

### Security

- Disable DNSoverTLS for the tunnel interface.

## [4.8.3] - 2025-10-14

### Security

- Privacy fix: disable mDNS and LLMNR for the tunnel interface.

## [4.8.2] - 2025-10-01

### Changed

- The packaging assets have been moved to `package` directory.

### Fixed

- Fixed a problem with some VPN servers that do not advertise the default_authentication_method field.

## [4.8.1] - 2025-09-24

### Added

- Enabled SAML SSO authentication for an SSL tunnel.
- An installer is now created as part of the release pipeline, with `.run` extension.

### Changed

- Service and desktop files are modified to point to /usr/bin by default.

## [4.8.0] - 2025-09-08

### Added

- Added `transport-type` option to specify the IPSec transport type explicitly.
- Added WSL2 environment detection where the xfrm interface does not work.
- Added a logic to try to keep the same IP address during IP renewal. Might not work for all users, please report any issues.

### Changed

- Improved browser OTP listener.

### Fixed

- Fixed compilation for 32-bit targets (ARMv6/ARMv7).
- Fixed a problem with incorrectly constructed routes for some VPN servers that advertise routing policy as 0.0.0.1-255.255.255.254.

## [4.7.0] - 2025-08-28

### Added

- Added `mtu` option to specify the MTU size for the tunnel interface.

### Fixed

- Fixed an issue with SAML SSO authentication when the browser uses CORS preflight checks.

## [4.6.0] - 2025-08-19

### Added

- Added `disable-ipv6` option to help prevent IPv6 leaks. It disables IPv6 globally when the `default-route` setting is enabled.

### Fixed

- Fixed a problem with IP address renewal for some Linux distros which have `net.ipv4.conf.default.promote_secondaries` set to 0.

## [4.5.0] - 2025-07-27

### Added

- Added a possibility to specify a custom server port if it's different from 443.
- Added a new option `ip-lease-time`, to specify the custom IP lease time.

### Changed

- UI labels are made selectable in the connection status dialog.

### Fixed

- Fixed a problem with wrongly constructed keepalive packets causing DNS query delays.

## [4.4.5] - 2025-07-12

### Added

- Added a parameter to the command line tools to generate shell autocompletion.

### Changed

- For the routing options, allow specifying IP addresses without the CIDR /XX notation, in which case /32 is assumed.
- Wrap UI labels in the status dialog if they are too long.

### Fixed

- Fixed a bug for the incorrect parsing of hex-like strings in the server info response.

## [4.4.4] - 2025-06-17

### Fixed

- Fixed a regression in the UI where the empty login type list is displayed when the VPN server does not advertise a list of login methods.

## [4.4.3] - 2025-06-09

### Fixed

- Fixed a regression with a default-route option which did not work correctly.

## [4.4.2] - 2025-06-06

### Fixed

- Fixed a problem where a new IP address was returned after each renewal.

## [4.4.1] - 2025-05-30

### Added

- Added support for MD5, SHA384 and SHA512 hash algorithms for IKE SA.

### Changed

- Only allow one UI dialog of a given type to be shown.
- Reduced the number of NAT-T probes for faster IPSec transport detection.

### Fixed

- Fixed xfrm module detection with some kernels and distros.
- Fixed some missing translations.

## [4.4.0] - 2025-05-22

### Added

- Added `auto-connect` option which enables automatic tunnel connect when GUI frontend starts.
- Added translations for .desktop file actions.
- Added a new category in the configuration dialog, "UI Settings."
- Added automatic IP address renewal when it expires.

### Changed

- Send keepalive packets to the advertised server IP instead of the default public address.

### Fixed

- Fixed an issue with too short IP lease time.

## [4.3.1] - 2025-05-10

### Added

- Added Brazilian Portuguese localization

### Fixed

- Fixed a bug with the GUI application stopping responding and consuming 100% CPU.
- Added some missing translations

## [4.3.0] - 2025-05-09

### Added

- Added localization support.
- For older kernels or if xfrm module is not available, fall back to TUN-UDP transport.
- Added custom actions in the .desktop file to control the GUI frontend via the context menu.
- Show an authenticated username and login type in the status information output.
- Added "Connect," "Disconnect" and "Settings" buttons in the status dialog.

### Changed

- When showing a username prompt, fill it with a session username by default.
- Always allow the status dialog to be shown and update it dynamically.

### Fixed

- Various bugfixes for connection status, firewall and tunnel setup.
- Fixed a bug with server info display when multiple fingerprints are present in the response.

## [4.2.0] - 2025-05-02

### Added

- Show connection information in standalone mode.
- Added a logic to allow invalid ICMP state packets on the tunnel interface when firewalld/nftables is active, to avoid DNS query delays.

### Changed

- On desktop environments other than GNOME or KDE use standard icons instead of pixmaps.
- Better server information output with the "snxctl info" command.
- Set MTU size to 1350 on the tunnel interface.

### Fixed

- Fixed a bug with unnecessary network state polling when keepalive is disabled.

## [4.1.0] - 2025-04-25

### Added

- Added a graphical connection status window, activated via the "Connection status..." popup menu.
- Added extended connection information in the "snxctl status" command output.

### Fixed

- Fixed a problem with invalid search domains sent by some VPN servers.
- Fixed a bug with the same MFA being triggered multiple times under certain conditions.

## [4.0.0] - 2025-04-22

**Important note for downstream package maintainers**: this release replaces GTK 3 dependency with GTK 4.

### Added

- Added automatic detection of IPSec transport.
- Added `port-knock` option to try port knocking workaround for NAT-T port 4500 availability detection.
- It is now possible to cancel the pending connection, also in the MFA state.
- Show extended server information with the "snxctl info" command.

### Changed

- GUI frontend: refactored into GTK 4.
- GUI frontend: use `ksni` crate to show tray icon.
- Changed the internal communication between the frontend and the command service to use Unix domain sockets.
- Refactored internal IPSec certificate validation to use the advertised internal_ca_fingerprint.

### Removed

- Removed `no-cert-check` option which was used to disable hostname verification. Use `ignore-server-cert` instead.
- Removed `server-prompt` option. Server prompts are always enabled.
- Removed obsolete `ipsec-cert-check`, `ike-transport`, `esp-transport` and `ike-port` options.

### Fixed

- Fixed many issues related to a concurrent use of the GUI frontend and the snxctl utility.
- Fixed a bug with incomplete SSL tunnel shutdown.

## [3.1.2] - 2025-04-13

### Changed

- Improved desktop theme detection on Ubuntu.

### Removed

- Removed the invalid "VPN" category from the .desktop file.

### Fixed

- Fixed a problem with default IP address detection on some systems.

## [3.1.1] - 2025-03-15

### Fixed

- Fixed a problem with snx-rs-gui lock file in the multi-user environments.

## [3.1.0] - 2025-02-21

### Added

- Added `password-factor=N` option to determine which authentication factor is the password. Default is 1 (first).
- Added `set-routing-domains=true|false` option to treat the received search domains as [routing domains](https://systemd.io/RESOLVED-VPNS/).

### Changed

- Return non-zero exit code from snxctl when an error is encountered.
- Extended "-m info" to show a list of factors per login type

## [3.0.6] - 2025-02-16

### Added

- Added 3DES support for IKE SA exchange

### Changed

- Improved authentication prompts by displaying a header retrieved from the VPN server
- Lowered Rust compiler requirements to 1.79

### Fixed

- Fixed IPSec over TCPT for AES encryption ciphers with key length shorter than 256 bits

## [3.0.5] - 2025-02-11

### Fixed

- Fixed a bug with the password being not taken from the configuration file in some situations.

## [3.0.4] - 2025-02-10

### Changed

- Delete persistent IKE session if the tunnel cannot be created.

### Fixed

- Fixed a bug with the wrong MFA code being stored in the keychain.
- Fixed several issues with the loading and storing password in the keychain.

## [3.0.3] - 2025-02-05

### Changed

- Only do a sign-out call when manually disconnecting an IPSec tunnel

## [3.0.2] - 2025-02-04

### Fixed

- Fixed a problem with a persistent IKE session which wasn't restored for some VPN servers

## [3.0.1] - 2025-02-03

### Fixed

- Connectivity fixes

## [3.0.0] - 2025-02-03

### Added

- Added support for TCP Transport mode (TCPT) for IPSec tunnels. It can be selected by two options: `ike-transport=tcpt` and `esp-transport=tcpt`.
  Use the first option only if you encounter the "failed to fill whole buffer" error. Use the second option only if you encounter the "Probing failed, server is not reachable via ESPinUDP tunnel" error.
- Added a workaround to attempt to unblock IPSec port 4500 in some cases.
- Implemented a sign-out call after tunnel disconnection.

### Fixed

- Resolved a compatibility issue with OpenSSL 1.x when building on older hosts.

## [2.9.0] - 2025-01-07

### Added

- Added `dns-servers` and `ignore-dns-servers` options

### Changed

- UI: refactored authentication settings, moved certificate authentication options to the general tab

## [2.8.3] - 2024-12-18

### Added

- Added the "Apply" button in the settings dialog (validate and save settings without closing the dialog).

### Fixed

- Fixed a bug with incorrect resolver detection when /etc/resolv.conf is a relative symlink.

## [2.8.2] - 2024-12-13

### Removed

- Removed a superfluous title from the system tray icon (visible in some desktop environments).

### Fixed

- Fixed a problem with the invalid MFA prompt shown when password is stored in the keychain and `server-prompt` option is enabled.
- Fixed a problem with the incorrect resolver detection when /etc/resolv.conf is a multi-level symlink.

## [2.8.1] - 2024-12-02

### Fixed

- Fixed a bug with a tun device being not removed after an SSL tunnel is disconnected.

## [2.8.0] - 2024-12-01

### Added

- Added `icon-theme` config option and a GUI setting to choose the GUI icon theme, to work around an issue with desktop theme autodetection on some systems.

## [2.7.2] - 2024-11-13

### Added

- Added DNS resolver detection and support for the plain `/etc/resolv.conf` configuration without systemd-resolved. Note that this setup will forward all DNS requests to the corporate VPN.

### Changed

- Added improvements in config files parsing.

## [2.6.1] - 2024-10-29

### Fixed

- Fixed `default-route` option in combination with the SSL tunnel.
- Fixed an intermittent crash in the SSL tunnel due to a keepalive counter underflow.

## [2.6.0] - 2024-10-13

### Added

- Added `no-keepalive` option to disable IPSec keepalive packets, to work around some rare cases of tunnel disconnects.

### Changed

- Don't add duplicated routes.
- When attempting to connect, don't return an error if the tunnel is already connected.

### Removed

- Removed webkit from the project. It doesn't seem to bring any practical value.

## [2.5.0] - 2024-09-13

### Added

- Added experimental `ike-persist` option which will save IPSec session to disk, restore it after service or computer restart and automatically reconnect the tunnel without authentication. It works best in combination with the `ike-lifetime` option. For example, setting `ike-lifetime` to 604800 will keep the session for 7 days. Note that most IPSec servers have shorter IKE duration configured, so it may be terminated earlier. This option is also added to the GUI application under the "Misc settings" category. Automatic reconnection will happen when running in the standalone mode, when GUI application starts or when `snxctl` sends the "connect" command.

### Fixed

- Fixed some issues with added routes.
- Fixed a problem with SSL connection when the username is not specified.

## [2.4.2] - 2024-09-03

### Added

- Allow comma-separated values in the command line for the multi-value parameters.
- Added an informational message printed to stdout when the tunnel is connected in standalone mode.

### Changed

- Ignore stored or specified passwords for the SAML authentication.

### Fixed

- Fixed the `ignore-routes` option which wasn't working as expected.
- Fixed a problem with the `default-route=true` option in combination with the IPSec tunnel.

## [2.4.1] - 2024-08-26

### Changed

- Don't hard-fail the connection if there is an IP address mismatch in the IPSec ID payload. This seems to cause issues with some users. The warning will be logged instead.
- Don't require a username to be specified for password logins. The user will be prompted for it if needed.
- Improved MFA prompts retrieval from the server.

[unreleased]: https://github.com/ancwrd1/snx-rs/compare/v6.5.4...HEAD
[6.5.4]: https://github.com/ancwrd1/snx-rs/compare/v6.5.3...v6.5.4
[6.5.3]: https://github.com/ancwrd1/snx-rs/compare/v6.5.2...v6.5.3
[6.5.2]: https://github.com/ancwrd1/snx-rs/compare/v6.5.1...v6.5.2
[6.5.1]: https://github.com/ancwrd1/snx-rs/compare/v6.5.0...v6.5.1
[6.5.0]: https://github.com/ancwrd1/snx-rs/compare/v6.4.1...v6.5.0
[6.4.1]: https://github.com/ancwrd1/snx-rs/compare/v6.4.0...v6.4.1
[6.4.0]: https://github.com/ancwrd1/snx-rs/compare/v6.3.1...v6.4.0
[6.3.1]: https://github.com/ancwrd1/snx-rs/compare/v6.3.0...v6.3.1
[6.3.0]: https://github.com/ancwrd1/snx-rs/compare/v6.2.4...v6.3.0
[6.2.4]: https://github.com/ancwrd1/snx-rs/compare/v6.2.3...v6.2.4
[6.2.3]: https://github.com/ancwrd1/snx-rs/compare/v6.2.2...v6.2.3
[6.2.2]: https://github.com/ancwrd1/snx-rs/compare/v6.2.1...v6.2.2
[6.2.1]: https://github.com/ancwrd1/snx-rs/compare/v6.2.0...v6.2.1
[6.2.0]: https://github.com/ancwrd1/snx-rs/compare/v6.1.2...v6.2.0
[6.1.2]: https://github.com/ancwrd1/snx-rs/compare/v6.1.1...v6.1.2
[6.1.1]: https://github.com/ancwrd1/snx-rs/compare/v6.1.0...v6.1.1
[6.1.0]: https://github.com/ancwrd1/snx-rs/compare/v6.0.6...v6.1.0
[6.0.6]: https://github.com/ancwrd1/snx-rs/compare/v6.0.5...v6.0.6
[6.0.5]: https://github.com/ancwrd1/snx-rs/compare/v6.0.4...v6.0.5
[6.0.4]: https://github.com/ancwrd1/snx-rs/compare/v6.0.3...v6.0.4
[6.0.3]: https://github.com/ancwrd1/snx-rs/compare/v6.0.2...v6.0.3
[6.0.1]: https://github.com/ancwrd1/snx-rs/compare/v6.0.0...v6.0.1
[6.0.0]: https://github.com/ancwrd1/snx-rs/compare/v5.3.0...v6.0.0
[5.3.0]: https://github.com/ancwrd1/snx-rs/compare/v5.2.4...v5.3.0
[5.2.4]: https://github.com/ancwrd1/snx-rs/compare/v5.2.3...v5.2.4
[5.2.3]: https://github.com/ancwrd1/snx-rs/compare/v5.2.2...v5.2.3
[5.2.2]: https://github.com/ancwrd1/snx-rs/compare/v5.2.1...v5.2.2
[5.2.1]: https://github.com/ancwrd1/snx-rs/compare/v5.2.0...v5.2.1
[5.2.0]: https://github.com/ancwrd1/snx-rs/compare/v5.1.0...v5.2.0
[5.1.0]: https://github.com/ancwrd1/snx-rs/compare/v5.0.5...v5.1.0
[5.0.5]: https://github.com/ancwrd1/snx-rs/compare/v5.0.4...v5.0.5
[5.0.2]: https://github.com/ancwrd1/snx-rs/compare/v5.0.1...v5.0.2
[5.0.1]: https://github.com/ancwrd1/snx-rs/compare/v5.0.0...v5.0.1
[5.0.0]: https://github.com/ancwrd1/snx-rs/compare/v4.9.1...v5.0.0
[4.9.1]: https://github.com/ancwrd1/snx-rs/compare/v4.9.0...v4.9.1
[4.9.0]: https://github.com/ancwrd1/snx-rs/compare/v4.8.3...v4.9.0
[4.8.3]: https://github.com/ancwrd1/snx-rs/compare/v4.8.2...v4.8.3
[4.8.2]: https://github.com/ancwrd1/snx-rs/compare/v4.8.1...v4.8.2
[4.8.1]: https://github.com/ancwrd1/snx-rs/compare/v4.8.0...v4.8.1
[4.8.0]: https://github.com/ancwrd1/snx-rs/compare/v4.7.0...v4.8.0
[4.7.0]: https://github.com/ancwrd1/snx-rs/compare/v4.6.0...v4.7.0
[4.6.0]: https://github.com/ancwrd1/snx-rs/compare/v4.5.0...v4.6.0
[4.5.0]: https://github.com/ancwrd1/snx-rs/compare/v4.4.5...v4.5.0
[4.4.5]: https://github.com/ancwrd1/snx-rs/compare/v4.4.4...v4.4.5
[4.4.4]: https://github.com/ancwrd1/snx-rs/compare/v4.4.3...v4.4.4
[4.4.3]: https://github.com/ancwrd1/snx-rs/compare/v4.4.2...v4.4.3
[4.4.2]: https://github.com/ancwrd1/snx-rs/compare/v4.4.1...v4.4.2
[4.4.1]: https://github.com/ancwrd1/snx-rs/compare/v4.4.0...v4.4.1
[4.4.0]: https://github.com/ancwrd1/snx-rs/compare/v4.3.1...v4.4.0
[4.3.1]: https://github.com/ancwrd1/snx-rs/compare/v4.3.0...v4.3.1
[4.3.0]: https://github.com/ancwrd1/snx-rs/compare/v4.2.0...v4.3.0
[4.2.0]: https://github.com/ancwrd1/snx-rs/compare/v4.1.0...v4.2.0
[4.1.0]: https://github.com/ancwrd1/snx-rs/compare/v4.0.0...v4.1.0
[4.0.0]: https://github.com/ancwrd1/snx-rs/compare/v3.1.2...v4.0.0
[3.1.2]: https://github.com/ancwrd1/snx-rs/compare/v3.1.1...v3.1.2
[3.1.1]: https://github.com/ancwrd1/snx-rs/compare/v3.1.0...v3.1.1
[3.1.0]: https://github.com/ancwrd1/snx-rs/compare/v3.0.6...v3.1.0
[3.0.6]: https://github.com/ancwrd1/snx-rs/compare/v3.0.5...v3.0.6
[3.0.5]: https://github.com/ancwrd1/snx-rs/compare/v3.0.4...v3.0.5
[3.0.4]: https://github.com/ancwrd1/snx-rs/compare/v3.0.3...v3.0.4
[3.0.3]: https://github.com/ancwrd1/snx-rs/compare/v3.0.2...v3.0.3
[3.0.2]: https://github.com/ancwrd1/snx-rs/compare/v3.0.1...v3.0.2
[3.0.1]: https://github.com/ancwrd1/snx-rs/compare/v3.0.0...v3.0.1
[3.0.0]: https://github.com/ancwrd1/snx-rs/compare/v2.9.0...v3.0.0
[2.9.0]: https://github.com/ancwrd1/snx-rs/compare/v2.8.3...v2.9.0
[2.8.3]: https://github.com/ancwrd1/snx-rs/compare/v2.8.2...v2.8.3
[2.8.2]: https://github.com/ancwrd1/snx-rs/compare/v2.8.1...v2.8.2
[2.8.1]: https://github.com/ancwrd1/snx-rs/compare/v2.8.0...v2.8.1
[2.8.0]: https://github.com/ancwrd1/snx-rs/compare/v2.7.2...v2.8.0
[2.7.2]: https://github.com/ancwrd1/snx-rs/compare/v2.6.1...v2.7.2
[2.6.1]: https://github.com/ancwrd1/snx-rs/compare/v2.6.0...v2.6.1
[2.6.0]: https://github.com/ancwrd1/snx-rs/compare/v2.5.0...v2.6.0
[2.5.0]: https://github.com/ancwrd1/snx-rs/compare/v2.4.2...v2.5.0
[2.4.2]: https://github.com/ancwrd1/snx-rs/compare/v2.4.1...v2.4.2
[2.4.1]: https://github.com/ancwrd1/snx-rs/compare/v2.4.0...v2.4.1
