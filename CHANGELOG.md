## v4.0.0 (TBD)

**Important note for downstream package maintainers**: this release replaces GTK 3 dependency with GTK 4.

- Added automatic detection of IPSec transport.
- GUI frontend: refactored into GTK 4.
- GUI frontend: use `ksni` crate to show tray icon.
- Removed `no-cert-check` option which was used to disable hostname verification. Use `ignore-server-cert` instead.
- Removed `server-prompt` option. Server prompts are always enabled.
- Removed obsolete `ipsec-cert-check`, `ike-transport`, `esp-transport` and `ike-port` options.
- Changed the internal communication between the frontend and the command service to use Unix domain sockets.
- Fixed many issues related to a concurrent use of the GUI frontend and the snxctl utility.
- It is now possible to cancel the pending connection, also in the MFA state.
- Refactored internal IPSec certificate validation to use the advertised internal_ca_fingerprint.
- Show extended server information with the 'snxctl info` command.

## v3.1.2 (2025-04-13)
- Fixed a problem with default IP address detection on some systems.
- Removed invalid "VPN" category from the .desktop file.
- Improved desktop theme detection on Ubuntu.

## v3.1.1 (2025-03-15)
- Fixed a problem with snx-rs-gui lock file in the multi-user environments.

## v3.1.0 (2025-02-21)
- Added `password-factor=N` option to determine which authentication factor is the password. Default is 1 (first).
- Added `set-routing-domains=true|false` option to treat the received search domains as [routing domains](https://systemd.io/RESOLVED-VPNS/).
- Return non-zero exit code from snxctl when an error is euncountered.
- Extended "-m info" to show list of factors per login type

## v3.0.6 (2025-02-16)
- Improved authentication prompts by displaying a header retrieved from VPN server
- Fixed IPSec over TCPT for AES encryption ciphers with key length shorter than 256 bits
- Added 3DES support for IKE SA exchange
- Lowered Rust compiler requirements to 1.79

## v3.0.5 (2025-02-11)
- Fixed a bug with the password being not taken from the configuration file in some situations.

## v3.0.4 (2025-02-10)
- Fixed a bug with the wrong MFA code being stored in the keychain.
- Fixed several issues with the loading and storing password in the keychain.
- Delete persistent IKE session if the tunnel cannot be created.

## v3.0.3 (2025-02-05)
- Only do a signout call when manually disconnecting an IPSec tunnel

## v3.0.2 (2025-02-04)
- Fixed a problem with persistent IKE session which wasn't restored for some VPN servers

## v3.0.1 (2025-02-03)
- Connectivity fixes

## v3.0.0 (2025-02-03)
- Added support for TCP Transport mode (TCPT) for IPSec tunnels. It can be selected by two options: `ike-transport=tcpt` and `esp-transport=tcpt`.
  Use the first option only if you encounter the "failed to fill whole buffer" error. Use the second option only if you encounter the "Probing failed, server is not reachable via ESPinUDP tunnel" error.
- Added a workaround to attempt to unblock IPSec port 4500 in some cases.
- Implemented a sign-out call after tunnel disconnection.
- Resolved a compatibility issue with OpenSSL 1.x when building on older hosts.

## v2.9.0 (2025-01-07)
- Added `dns-servers` and `ignore-dns-servers` options
- UI: refactored authentication settings, moved certificate authentication options to general tab

## v2.8.3 (2024-12-18)
- Fixed a bug with incorrect resolver detection when /etc/resolv.conf is a relative symlink.
- Added "Apply" button in the settings dialog (validate and save settings without closing the dialog).

## v2.8.2 (2024-12-13)
- Removed a superfluous title from the system tray icon (visible on some desktop environments).
- Fixed a problem with the invalid MFA prompt shown when password is stored in the keychain and `server-prompt` option is enabled.
- Fixed a problem with the incorrect resolver detection when /etc/resolv.conf is a multi-level symlink.

## v2.8.1 (2024-12-02)
- Fixed a bug with a tun device being not removed after SSL tunnel is disconnected.

## v2.8.0 (2024-12-01)
- Added `icon-theme` config option and a GUI setting to choose the GUI icon theme, to workaround an issue with desktop theme autodetection on some systems.

## v2.7.2 (2024-11-13)
- Added DNS resolver detection and support for the plain `/etc/resolv.conf` configuration without systemd-resolved. Note that this setup will forward all DNS requests to the corporate VPN.
- Added improvements in config files parsing.

## v2.6.1 (2024-10-29)
- Fixed `default-route` option in combination with the SSL tunnel.
- Fixed intermittent crash in the SSL tunnel due to a keepalive counter underflow.

## v2.6.0 (2024-10-13)
- Added `no-keepalive` option to disable IPSec keepalive packets, to workaround some rare cases of tunnel disconnects.
- Removed webkit from the project. It doesn't seem to bring any practical value.
- Don't add duplicated routes.
- When attempting to connect, don't return an error if the tunnel is already connected.

## v2.5.0 (2024-09-13)
- Added experimental `ike-persist` option which will save IPSec session to disk, restore it after service or computer restart and automatically reconnect the tunnel without authentication. It works best in combination with the `ike-lifetime` option. For example, setting `ike-lifetime` to 604800 will keep the session for 7 days. Note that most IPSec servers have shorter IKE duration configured, so it may be terminated earlier. This option is also added to the GUI application under "Misc settings" category. Automatic reconnection will happen when running in the standalone mode, when GUI application starts or when `snxctl` sends the "connect" command.
- Fixed some issues with added routes.
- Fixed a problem with SSL connection when username is not specified.

## v2.4.2 (2024-09-03)
- Fixed the `ignore-routes` option which wasn't working as expected.
- Fixed a problem with the `default-route=true` option in combination with the IPSec tunnel.
- Allow comma-separated values in the command line for the multi-value parameters.
- Added informational message printed to stdout when the tunnel is connected in standalone mode.
- Ignore stored or specified passwords for the SAML authentication.

## v2.4.1 (2024-08-26)
- Don't hard-fail the connection if there is IP address mismatch in the IPSec ID payload. This seems to cause issues with some users. The warning will be logged instead.
- Don't require user name to be specified for password logins. The user will be prompted for it if needed.
- Improved MFA prompts retrieval from the server.
