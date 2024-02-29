# Open source Linux client for Checkpoint VPN tunnels

This project contains a Rust source code of the unofficial Linux client for Checkpoint VPN.
Based on the reverse engineered protocol from the vendor application.

## Advantages over the official snx client for Linux:

* Open source
* IPSec support (faster tunnel)
* Better privacy for DNS requests: only requests for VPN-specific suffixes are routed through the tunnel
* Better integration with NetworkManager and systemd-resolved
* Optional integration with Gnome Keyring or KDE KWallet via libsecret (only when using snxctl in command mode)

## Implemented features

* **NEW**: SAML SSO authentication (only with IPSec tunnel)
* SSL tunnel via Linux TUN device
* IPSec tunnel via Linux native kernel XFRM interface and VTI device with the following features:
  * `AES-CBC-256` encryption algorithm
  * `HMAC-SHA-256-128` authentication algorithm
  * `ESPinUDP` tunnel encapsulation via UDP port 4500
* Username/password authentication with MFA support
* Certificate authentication via the provided client certificate
* Microsoft Authenticator app support
* Multi-factor codes input via TTY/GUI (SMS/SecurID/TOTP)
* Store password in the keychain using libsecret
* Tray icon and menu support (optional via 'tray-icon' compile-time feature)

## System requirements

* Recent Linux distribution
* NetworkManager
* systemd-resolved configured as a global DNS resolver
* iproute2
* DBus
* libsecret
* For tray-icon build-time feature: Adwaita theme (icons), zenity or kdialog utility (user prompts)

## Usage

Before the client can establish a connection it must know the login (authentication) type to use
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

* Standalone service mode, selected by `-m standalone` parameter. This is the default mode. Run `snx-rs --help` to get a help with all command line parameters. In this mode the application takes connection parameters either from the command line or from the specified configuration file. Recommended for headless usage.
* Command mode, selected by `-m command` parameter. In this mode the application runs as a service without
 establishing a connection and awaits for the commands from the external client. Use `snxctl` utility
 to send commands to the service. Recommended for desktop usage. The following commands are accepted:
  - `connect` - establish a connection. Parameters are taken from the `~/.config/snx-rs/snx-rs.conf` file.
  - `disconnect` - disconnect a tunnel
  - `reconnect` - drop a connection and then connect again
  - `status` - show connection status
  - `info` - dump server information in JSON format

Configuration file may contain all options which are accepted via the command line, without the leading double dashes.

## Authentication types

* For authentications which require additional password or challenge codes the `user-name` option must be provided in the configuration.
* For SAML SSO authentication the `user-name` and `password` options should NOT be specified.

## Tray icon and UI

* The application can be built with `tray-icon` feature. In this case if `snxctl` utility runs without parameters
 it displays the notification icon with the popup menu. Prompts for MFA codes are displayed using zenity or kdialog.

 ## Additional usage notes

* If SAML SSO authentication is used in standalone mode, the browser URL will be printed to the console.
  The user must open this URL manually. In command mode the browser will be opened automtically.
* If additional MFA steps are required a prompt will be shown to enter the codes.
  If the application has no attached terminal an authentication error will be triggered.
* If password is not provided in the configuration file or command line it will be prompted for and stored
  in the OS keychain unless `no-keychain` parameter is specified. Keychain integration is provided only when
  using command mode and `snxctl` because the main application runs as a root user.

## Building from sources

Rust compiler 1.75 or later (https://rustup.rs) is required. Run `cargo build --release --all-features`
 to build the release version with tray icon support.

## License

Licensed under [GNU Affero General Public License version 3](https://opensource.org/license/agpl-v3/)
