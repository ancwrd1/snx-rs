# Rust client for Checkpoint VPN tunnels

This project implements a client for Checkpoint VPN written in Rust programming language.
Based on the reverse engineered protocol used by the vendor application.

## Implemented features

* SSL tunnel
* IPSec tunnel
* Username/password authentication with Microsoft MFA Authenticator

## Roadmap
 
* GUI with tray icon
* Connection stats
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

## License

Licensed under MIT or Apache license ([LICENSE-MIT](https://opensource.org/licenses/MIT) or [LICENSE-APACHE](https://opensource.org/licenses/Apache-2.0))
