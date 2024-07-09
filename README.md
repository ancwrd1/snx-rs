# Open Source Linux Client for Checkpoint VPN Tunnels

This project contains the source code for an unofficial Linux client for Checkpoint VPN, written in Rust. It is based on a reverse-engineered protocol from the vendor's application.

## Why This Project

Unfortunately, my employer uses the popular-but-not-so-Linux-friendly Checkpoint VPN for remote access. This prompted me to create my own client.

## Advantages Over the Official SNX Client for Linux

* Open source
* IPSec support (provides a much faster tunnel)
* More authentication methods
* Better privacy for DNS requests: only requests for VPN-specific suffixes are routed through the tunnel
* Better integration with NetworkManager and systemd-resolved
* Optional integration with GNOME Keyring or KDE KWallet
* Customizable routing and DNS settings

## Implemented Features

* SAML SSO authentication (only with IPSec tunnel)
* Username/password authentication with MFA support
* Certificate authentication via provided client certificate (PFX, PEM, or HW token)
* HW token support via PKCS11 (only with IPSec tunnel)
* GTK frontend with tray icon and WebKit webview for SAML authentication
* SSL tunnel via Linux TUN device
* IPSec tunnel via Linux native kernel XFRM interface
* Store passwords in the keychain using Secret Service API

## System Requirements

* A recent Linux distribution with kernel version 4.19 or higher. For the IPSec tunnel, the IPv6 protocol must be enabled in the kernel.
* systemd-resolved [configured](https://wiki.archlinux.org/title/Systemd-resolved) as a global DNS resolver
* iproute2 (the `ip` utility, which should be standard for all distros)
* D-Bus
* GTK3, webkit2gtk, and libappindicator3 for the GUI frontend

## GUI Usage

* For GNOME environment: install the [Appindicator](https://extensions.gnome.org/extension/615/appindicator-support/) extension
* Run the main application in command mode: `sudo snx-rs -m command` or install it as a systemd service
* Run the `snx-rs-gui` application, which will display a tray icon with a menu

## Command Line Usage

Check the [Configuration Options](https://github.com/ancwrd1/snx-rs/blob/main/options.md) section for a list of all available options.

Before the client can establish a connection, it must know the login (authentication) method to use (`--login-type` or `-o` option). To find the supported login types, run it with the `-m info` parameter:

```sh
snx-rs -m info -s remote.acme.com
```

This command will display the supported login types. Use the `vpn_XXX` identifier as the login type. If a certificate error is returned, try adding the `-X true` command line parameter to ignore certificate errors.

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

* **Command Mode**: Selected by the `-m command` parameter. In this mode, the application runs as a service without establishing a connection and awaits commands from the external client. Use the `snxctl` utility to send commands to the service. This mode is recommended for desktop usage. The following commands are accepted:
  - `connect`: Establish a connection. Parameters are taken from the `~/.config/snx-rs/snx-rs.conf` file.
  - `disconnect`: Disconnect a tunnel.
  - `reconnect`: Drop the connection and then reconnect.
  - `status`: Show connection status.
  - `info`: Show server authentication methods and supported tunnel types.
  - Run it with the `--help` option to get usage help.
* **Standalone Service Mode**: Selected by the `-m standalone` parameter. This is the default mode if no parameters are specified. Run `snx-rs --help` to get help with all command line parameters. In this mode, the application takes connection parameters either from the command line or from the specified configuration file. This mode is recommended for headless usage.

## Certificate Authentication

There are four parameters that control certificate-based authentication:

* `cert-type`: One of "none", "pkcs12", "pkcs8", or "pkcs11". Choose "pkcs12" to read the certificate from an external PFX file. Choose "pkcs8" to read the certificate from an external PEM file (containing both private key and x509 cert). Choose "pkcs11" to use a hardware token via a PKCS11 driver.
* `cert-path`: Path to the PFX, PEM, or custom PKCS11 driver file, depending on the selected cert type. The default PKCS11 driver is `opensc-pkcs11.so`, which requires the opensc package to be installed.
* `cert-password`: Password for PKCS12 or PIN for PKCS11. Must be provided for those types.
* `cert-id`: Optional hexadecimal ID of the certificate for the PKCS11 type. Could be in the form of 'xx:xx:xx' or 'xxxxxx'.

## Additional Usage Notes

* If SAML SSO authentication is used in standalone mode, the browser URL will be printed to the console. In command mode, the browser will be opened automatically.
* If the password is not provided in the configuration file, the first entered MFA challenge code will be stored in the OS keychain unless the `no-keychain` parameter is specified. Keychain integration is provided only in command mode.

## Troubleshooting common problems

| Error | Solution |
| ----- | -------- |
| `deadline has elapsed` | Check if the correct login type is specified (one of the vpn_XXX identifiers returned from the "-m info" command). |
| `Unknown device type` | Check if the IPv6 protocol is enabled in the Linux kernel. |
| `[0020] The user is not defined properly` | Application failed to negotiate IPSec encryption parameters. Usually it means that Checkpoint server is misconfigured with the obsolete insecure ciphers. **Do not connect to it.** |
| `error sending request for url (https://IP_OR_HOSTNAME/clients/)` + SSL-related stack trace | VPN server certificate is self-signed or untrusted. Use `ignore-server-cert` parameter to disable all HTTPS certificate checks. Use `no-cert-check` to only disable hostname validation. |
| How do I logout from SAML SSO? | Delete the `~/.config/snx-rs/cookies.db` file |

## Contributing

Pull requests, bug reports, and suggestions are welcome. This is a hobby project I maintain in my free time. Adding a missing feature can be considered on a case-by-case basis but will require some testing efforts from the requester.

## Building from Sources

The recommended way to build it is via distrobox, which is available for all distros.

* Provision distrobox container: `distrobox create --image ubuntu:22.04 --name snx-ubuntu`
* Enter the container: `distrobox enter snx-ubuntu`
* Install the required dependencies: `sudo apt install build-essential pkg-config libssl-dev libgtk-3-dev libsoup-3.0-dev libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev`
* Install a recent [Rust compiler](https://rustup.rs)
* Run `cargo build --release` to build the release version
* If the GUI frontend is not needed, build it with `cargo build --release --workspace --exclude snx-rs-gui`.

## Credits

Special thanks to the [cpyvpn](https://gitlab.com/cpvpn/cpyvpn) project for inspiration around SAML and IKEv1 exchange.

## License

Licensed under the [GNU Affero General Public License version 3](https://opensource.org/license/agpl-v3/).
