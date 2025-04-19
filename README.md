# Open Source Linux Client for Check Point VPN Tunnels

[![github actions](https://github.com/ancwrd1/snx-rs/workflows/CI/badge.svg)](https://github.com/ancwrd1/snx-rs/actions)
[![license](https://img.shields.io/badge/License-AGPL-v3.svg)](https://opensource.org/license/agpl-v3)

This project contains the source code for an unofficial Linux client for Check Point VPN, written in Rust.

⚠️ Before creating an issue, please check the [FAQ section](#faq).

## Quick Start Guide

Run the service application in command mode, then start the GUI frontend which will display an icon in the taskbar.
**GNOME environment**: if the tray icon is not displayed, install the [AppIndicator](https://extensions.gnome.org/extension/615/appindicator-support/) extension. 

* `sudo ./snx-rs -m command`
* `./snx-rs-gui`

👇 Keep reading for additional information and command line usage.

## Advantages Over the Official SNX Client for Linux

* Open source
* IPSec support (provides a much faster tunnel)
* More authentication methods
* Better privacy for DNS requests: only requests for VPN-specific suffixes are routed through the tunnel
* Better integration with system DNS resolver
* Optional integration with GNOME Keyring or KDE KWallet
* Customizable routing and DNS settings

## Implemented Features

* Browser-based SSO authentication
* Username/password authentication with MFA support
* Certificate authentication via provided client certificate (PFX, PEM, or HW token)
* HW token support via PKCS11
* GTK frontend with tray icon
* IPSec tunnel via Linux native kernel XFRM interface or TCPT/TUN transport
* Automatic IPSec tunnel reconnection without authentication, via optional parameter
* SSL tunnel via Linux TUN device (deprecated)
* Store password factor in the OS keychain using Secret Service API

## Limitations

* Certificate enrollment and renewal is not supported

## System Requirements

* A recent Linux distribution with kernel version 4.19 or higher
* systemd-resolved is recommended as a global DNS resolver
* iproute2 (the `ip` command)
* D-Bus
* GTK4 for the GUI frontend

## DNS Resolver Configuration and Privacy

By default, if systemd-resolved is not detected as a global DNS resolver, snx-rs will fall back
to modify the /etc/resolv.conf file directly and DNS servers acquired from the tunnel will be used globally.
For better privacy, use the split DNS provided by systemd-resolved.

To find out whether it is already enabled, check the /etc/resolv.conf file:

`readlink /etc/resolv.conf`

If it is a symlink pointing to `/run/systemd/resolve/stub-resolv.conf` then it is already configured on your system.
Otherwise follow these steps:

1. `sudo ln -sf /run/systemd/resolve/stub-resolv.conf /etc/resolv.conf`
2. `sudo systemctl enable --now systemd-resolved`
3. `sudo systemctl restart NetworkManager`

With `systemd-resolved` it is also possible to use **routing domains** (as opposed to **search domains**).
Routing domains are prefixed with `~` character and when configured only requests for the fully qualified domains
will be forwarded through the tunnel. For further explanation, please check [this article](https://systemd.io/RESOLVED-VPNS/).

The `set-routing-domains=true|false` option controls whether to treat all acquired search domains as routing domains.

## Tunnel Transport Selection

IPSec is the default transport and is preferred because of it's performance and support for extended authentication types.
By default, it will use native kernel IPSec infrastructure with a UDP-based tunnel over port 4500.

In some environments those ports may be blocked by the firewall; in this case the application will fall back to the proprietary Check Point TCPT
transport via TCP port 443, which is slower than native UDP.

For older VPN servers or in case IPSec does not work for some reason, the legacy SSL tunnel can be used as well, selected with `tunnel-type=ssl`.
SSL tunnel has limited support for authentication types: no browser-based SSO, no hardware token support, no MFA in combination with the certificates.  

## Command Line Usage

Check the [Configuration Options](https://github.com/ancwrd1/snx-rs/blob/main/options.md) section for a list of all available options. Options can be specified in the configuration file
and the path of the file given via `-c /path/to/custom.conf` command line parameter.

Alternatively, in standalone mode, they can be specified via the command line of the `snx-rs` executable, prefixed with `--` (double dash).

Before the client can establish a connection, it must know the login (authentication) method to use (`--login-type` or `-o` option).
To find the supported login types, run it with the `-m info` parameter:

```sh
snx-rs -m info -s remote.acme.com
```

This command will display the supported login types. Use the `vpn_XXX` identifier as the login type.
If a certificate error is returned, try adding the `-X true` command line parameter to ignore certificate errors.

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

## Usage Examples

```bash
# standalone mode with trace logging and IPSec tunnel
sudo ./snx-rs -o vpn_Microsoft_Authenticator -s remote.company.com -e ipsec -l trace

# command mode with debug logging (use snxctl to establish a connection)
sudo ./snx-rs -m command -l debug
```

## Docker Usage

Check [this repository](https://github.com/leleobhz/snx-rs-docker) for a docker container.

## Certificate Validation

The following parameters control certificate validation during TLS and IKE exchanges:

* `ca-cert`: Comma-separated list of paths to PEM or DER files which contain custom CA root certificates
* `ignore-server-cert`: true|false. Disable all TLS certificate checks. Insecure and not recommended. Default is false.

Note that enabling the insecure option may compromise the channel security.

## Certificate Authentication

The following parameters control certificate-based authentication:

* `cert-type`: One of `none`, `pkcs12`, `pkcs8` or `pkcs11`. Choose `pkcs12` to read the certificate from an external PFX file. Choose `pkcs8` to read the certificate from an external PEM file (containing both private key and x509 cert). Choose `pkcs11` to use a hardware token via a PKCS11 driver.
* `cert-path`: Path to the PFX, PEM, or custom PKCS11 driver file, depending on the selected cert type. The default PKCS11 driver is `opensc-pkcs11.so`, which requires the opensc package to be installed.
* `cert-password`: Password for PKCS12 or PIN for PKCS11. Must be provided for those types.
* `cert-id`: Optional hexadecimal ID of the certificate for the PKCS11 type. Could be in the form of 'xx:xx:xx' or 'xxxxxx'.

## Persistent IPSec Session

The `ike-persist` option will save IPSec session to disk and restore it after the service or computer restarts,
it will then attempt to automatically reconnect the tunnel without authentication. This parameter works best in combination with the `ike-lifetime` option:
for example, setting `ike-lifetime` to 604800 will keep the session for 7 days.

Note that most IPSec servers have shorter IKE duration configured, so it may be terminated earlier.

## Additional Usage Notes

* If SAML SSO authentication is used in standalone mode, the browser URL will be printed to the console. In command mode, the browser will be opened automatically.
* If the password is not provided in the configuration file, the first entered MFA challenge code will be stored in the OS keychain unless the `no-keychain` parameter is specified. Keychain integration is provided only in command mode. The `password-factor` option controls which MFA factor to consider a password.

<a id="faq"></a>

## Troubleshooting Common Problems

| Problem                                                           | Solution                                                                                                                                                                     |
|-------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `deadline has elapsed`                                            | Try connecting again. Check if the correct login type is specified (one of the vpn_XXX identifiers returned from the "-m info" command).                                     |
| `Unknown device type`                                             | Make sure IPv6 protocol is enabled in the Linux kernel and 'xfrm' module can be loaded with `sudo modprobe xfrm`.                                                            |
| `error sending request for url (https://IP_OR_HOSTNAME/clients/)` | VPN server certificate is self-signed or untrusted. Use `ignore-server-cert` parameter to disable all HTTPS certificate checks.                                              |
| `No CCC session in reply!`                                        | Try the `client-mode` parameter with different values: `endpoint_security`, `secure_remote`, `secure_connect`                                                                | 
| A specific feature is missing or connection does not work         | Use the [cp-ikev1-proxy](https://github.com/ancwrd1/cp-ikev1-proxy) MITM proxy tool to capture the packets from the working Windows VPN client, then create an issue for it. | 

## Contributing

Pull requests, bug reports, and suggestions are welcome. This is a hobby project I maintain in my free time.

Before opening a PR, make sure to reformat the sources with the `cargo fmt` command and run it through the `cargo clippy` for any warnings.

## Building from Sources

* Install the required dependencies:
  - Debian/Ubuntu: `sudo apt install build-essential libssl-dev libgtk-4-dev`
  - openSUSE: `sudo zypper install libopenssl-3-devel gtk4-devel`
  - Other distros: C compiler, OpenSSL, GTK 4 development packages
* Install a recent [Rust compiler](https://rustup.rs)
* Run `cargo build` to build the debug version, or `cargo build --release` to build the release version
* If the GUI frontend is not needed, build it with `cargo build --release --workspace --exclude snx-rs-gui`

## Acknowledgements

Special thanks to the [cpyvpn](https://gitlab.com/cpvpn/cpyvpn) project for inspiration around SAML and IKEv1 exchange.

## License

Licensed under the [GNU Affero General Public License version 3](https://opensource.org/license/agpl-v3/).

