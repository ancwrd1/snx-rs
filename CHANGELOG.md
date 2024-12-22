## v2,9.0 (unreleased)
- Added `dns-servers` and `ignore-dns-servers` options

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
