# Rust client for Checkpoint VPN tunnels

This project contains a Rust source code of the unofficial client for Checkpoint VPN.
Based on the reverse engineered protocol used by the vendor application.

## Advantages over the 'official' snx client for Linux:

* IPSec support (faster tunnel)
* Better privacy for DNS requests: only requests for VPN-specific suffixes are routed through the tunnel
* Better integration with NetworkManager and systemd-resolved
* Integration with libsecret and OS keychain (only in command mode)

## Implemented features

* SSL tunnel
* IPSec tunnel
* Username/password authentication
* Certificate authentication via the provided client certificate
* Microsoft Authenticator app support
* Multi-factor codes input via TTY (SMS/SecurID/TOTP)
* Store password in the keychain using libsecret
* Tray icon and menu support (optional via 'tray-icon' flag)

## Roadmap
 
* SAML SSO support

## Usage

There are two ways to use the application:

* Standalone service mode, selected by `-m standalone` parameter. This is the default mode. Run `snx-rs --help` to get a help with all command line parameters. In this mode the application takes connection parameters either from the command line or from the specified configuration file.
* Command mode, selected by `-m command` parameter. In this mode the application runs as a service without
 establishing a connection and awaits for the commands from the external client. Use `snxctl` utility
 to send commands to the service. The following commands are accepted:
  - `connect` - establish a connection. Parameters are taken from the `~/.config/snx-rs/snx-rs.conf` file.
  - `disconnect` - disconnect a tunnel
  - `reconnect` - drop a connection and then connect again
  - `status` - show connection status
  - `info` - dump server information in JSON format

## Notes

* If additional MFA steps are required a prompt will be shown on the terminal to enter the codes.
  If the application has no attached terminal an authentication error will be triggered.
* If password is not provided in the configuration file or command line it will be prompted for and stored
  in the OS keychain (this will only work when using command mode and `snxctl` because the main application runs
  as a root user without access to keychain/kwallet).

## License

Licensed under [GNU Affero General Public License version 3](https://opensource.org/license/agpl-v3/)
