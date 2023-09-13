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

## Contributing

If it doesn't work with your particular Checkpoint server please run the following command:

`snx-rs -m info -s <serveraddress>`

and paste the JSON output into the new issue.

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

Licensed under [GNU Affero General Public License version 3](https://opensource.org/license/agpl-v3/)
