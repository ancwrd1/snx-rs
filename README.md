# Open source Linux client for Checkpoint VPN tunnels

This project contains a Rust source code of the unofficial Linux client for Checkpoint VPN.
Based on the reverse engineered protocol from the vendor application.

## Advantages over the official snx client for Linux:

* Open source
* IPSec support (faster tunnel)
* Better privacy for DNS requests: only requests for VPN-specific suffixes are routed through the tunnel
* Better integration with NetworkManager and systemd-resolved
* Optional integration with Gnome Keyring or KDE KWallet via libsecret

## Implemented features

* **NEW**: SAML SSO authentication (only with IPSec tunnel)
* **NEW**: GTK frontend with tray icon and webkit webview for SAML authentication
* Username/password authentication with MFA support
* Certificate authentication via the provided client certificate
* SSL tunnel via Linux TUN device
* IPSec tunnel via Linux native kernel XFRM interface
  Supported hash and encryption algorithms: SHA1, SHA256, AES-CBC. Unsupported algorithms: MD5, 3DES.
* Store password in the keychain using libsecret

## System requirements

* Recent Linux distribution with kernel version 4.19 or higher
* NetworkManager
* systemd-resolved configured as a global DNS resolver
* iproute2
* DBus
* libsecret
* gtk 3 and webkit2gtk for the GUI frontend 

## GUI usage

Run the `snx-rs-gui` application which will display a tray icon with a menu

## Command line usage

Before the client can establish a connection it must know the login (authentication) method to use
 (`--login-type` or `-o` option). In order to find the supported login types run it with "-m info" parameter:

 `snx-rs -m info -s remote.acme.com` 

 This command will dump the supported login types. Use the `vpn_XXX` identifier as the login type.

 Example output (may differ for your server):

 ```text
 Supported tunnel protocols:
        IPSec
        SSL
        L2TP
Available login types:
        vpn_Microsoft_Authenticator (Microsoft Authenticator)
        vpn_Emergency_Access (Emergency Access)
        vpn_Username_Password (Username Password)
        vpn_Azure_Authentication (Azure Authentication)
        vpn (Standard)
```

There are two ways to use the application:

* Standalone service mode, selected by `-m standalone` parameter. This is the default mode. 
  Run `snx-rs --help` to get a help with all command line parameters.
  In this mode the application takes connection parameters either from the command line or from the specified configuration file.
  Recommended for headless usage.
* Command mode, selected by `-m command` parameter. In this mode the application runs as a service without
  establishing a connection and awaits for the commands from the external client. Use `snxctl` utility
  to send commands to the service. Recommended for desktop usage. The following commands are accepted:
  - `connect` - establish a connection. Parameters are taken from the `~/.config/snx-rs/snx-rs.conf` file.
  - `disconnect` - disconnect a tunnel
  - `reconnect` - drop a connection and then connect again
  - `status` - show connection status
  - `info` - dump server information in JSON format
  - run without parameters: print usage help or show tray icon

Configuration file may contain all options which are accepted via the command line, without the leading double dashes.

## Authentication types

* For authentications which require additional password or challenge codes the `user-name` option must be provided in the configuration. 
  If the `password` option is provided (base64-encoded) it will be used for the first MFA challenge.
* For SAML SSO authentication the `user-name` and `password` options should NOT be specified.

## Additional usage notes

* If SAML SSO authentication is used in standalone mode, the browser URL will be printed to the console.
  In command mode the browser will be opened automatically.
* If password is not provided in the configuration file the first entered MFA challenge code will be stored
  in the OS keychain unless `no-keychain` parameter is specified. Keychain integration is provided only in the
  command mode.

## Building from sources

Recent Rust compiler (https://rustup.rs) is required. Run `cargo build --release` to build the release version.

## Credits

Special thanks to [cpyvpn](https://gitlab.com/cpvpn/cpyvpn) project for inspiration around SAML and IKEv1 authentication

## License

Licensed under [GNU Affero General Public License version 3](https://opensource.org/license/agpl-v3/)
