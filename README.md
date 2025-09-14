# Open Source Linux Client for Check Point VPN Tunnels

[![github actions](https://github.com/ancwrd1/snx-rs/workflows/CI/badge.svg)](https://github.com/ancwrd1/snx-rs/actions)
[![license](https://img.shields.io/badge/License-AGPL-v3.svg)](https://opensource.org/license/agpl-v3)

This project contains the source code for an unofficial Linux client for Check Point VPN, written in Rust.

⚠️ Before creating an issue, please check the [FAQ section](#faq).

Download the latest binary and source release [here](https://github.com/ancwrd1/snx-rs/releases/latest).

For Arch Linux and derivatives the [AUR package](https://aur.archlinux.org/packages/snx-rs) can be used as well.

## Quick Start Guide (GUI)

Run the service application in command mode and start the GUI frontend which will display an icon in the taskbar.
**GNOME environment**: if the tray icon is not displayed, install the [AppIndicator](https://extensions.gnome.org/extension/615/appindicator-support/) extension.

```bash
# run the service in command mode, requires root privileges
sudo ./snx-rs -m command
# run the GUI frontend as a user application
./snx-rs-gui
```

## Quick Start Guide (CLI)

```bash
# get the list of supported login types
./snx-rs -m info -s remote.company.com
# run the service in standalone mode, requires root privileges
sudo ./snx-rs -o vpn_Microsoft_Authenticator -s remote.company.com
```

⚠️ **Note about certificate errors**: if the connection fails with the certificate error, the `ignore-server-cert` option can be used to disable certificate checks (not recommended).

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

* Browser-based identity provider authentication
* Username/password authentication with MFA support
* Certificate authentication via provided client certificate (PFX, PEM, or HW token)
* HW token support via PKCS11
* GTK frontend with tray icon
* IPSec tunnel via Linux native kernel XFRM interface or TCPT/TUN transport
* Automatic IPSec tunnel reconnection without authentication, via optional parameter
* SSL tunnel via Linux TUN device (deprecated)
* Store a password factor in the OS keychain using Secret Service API
* Localization support, see i18n/assets directory for a list of supported locales

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

If it is a symlink pointing to `/run/systemd/resolve/stub-resolv.conf` then it is already configured on your system,
otherwise follow these steps:

1. `sudo ln -sf /run/systemd/resolve/stub-resolv.conf /etc/resolv.conf`
2. `sudo systemctl enable --now systemd-resolved`
3. `sudo systemctl restart NetworkManager`

With `systemd-resolved` it is also possible to use **routing domains** (as opposed to **search domains**).
Routing domains are prefixed with `~` character and when configured only requests for the fully qualified domains
will be forwarded through the tunnel. For further explanation, please check [this article](https://systemd.io/RESOLVED-VPNS/).

The `set-routing-domains=true|false` option controls whether to treat all acquired search domains as routing domains.

## Tunnel Type Selection

snx-rs supports two tunnel types: IPSec and SSL. IPSec tunnel is a default option if not specified in the configuration.
Depending on the availability of the kernel `xfrm` module, it will use either a native kernel IPSec infrastructure or a TUN device
with userspace ESP packet encoding.

IPSec ESP traffic is encapsulated in the UDP packets sent via port 4500 which may be blocked in some environments.
In this case the application will fall back to the proprietary Check Point TCPT transport via TCP port 443, which is slower than UDP.

The `transport-type` option can be used to choose the IPSec transport type manually. The default value is `auto` which will perform autodetection.

For older VPN servers or in case IPSec does not work for some reason, the legacy SSL tunnel can be used as well, selected with `tunnel-type=ssl`.
SSL tunnel has some limitations: it is slower, has no hardware token support and no MFA in combination with the certificates.

## Command Line Usage

Check the [Configuration Options](https://github.com/ancwrd1/snx-rs/blob/main/options.md) section for a list of all available options. Options can be specified in the configuration file
and the path of the file given via `-c /path/to/custom.conf` command line parameter.

Alternatively, in standalone mode, they can be specified via the command line of the `snx-rs` executable, prefixed with `--` (double dash).

Before the client can establish a connection, it must know the login (authentication) method to use (`--login-type` or `-o` option).
To find the supported login types, run it with the `-m info` parameter:

```sh
# in standalone mode
snx-rs -m info -s remote.acme.com
# in command mode
snxctl info
```

This command will display the supported login types. Use the `vpn_XXX` identifier as the login type.
If a certificate error is returned, try adding the `-X true` command line parameter to ignore certificate errors.

Example output (may differ for your server):

```text
           Server address: remote.company.com
                Server IP: 1.2.3.4
           Client enabled: true
      Supported protocols: IPSec, SSL, L2TP
       Preferred protocol: IPSec
                TCPT port: 443
                NATT port: 4500
  Internal CA fingerprint: MATE FRED PEN RANK LIP HUGH BEAD WET CAGE DEW FULL EDIT
[Microsoft Authenticator]: vpn_Microsoft_Authenticator (password)
       [Emergency Access]: vpn_Emergency_Access (password)
      [Username Password]: vpn_Username_Password (password)
   [Azure Authentication]: vpn_Azure_Authentication (identity_provider)
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
* `cert-id`: Optional hexadecimal ID of the certificate for the PKCS11 type. Could be in the form of `xx:xx:xx` or `xxxxxx`.

## Persistent IPSec Session

The `ike-persist` option will save IPSec session to disk and restore it after the service or computer restarts,
it will then attempt to automatically reconnect the tunnel without authentication. This parameter works best in combination with the `ike-lifetime` option:
for example, setting `ike-lifetime` to 604800 will keep the session for 7 days.

Note that most IPSec servers have shorter IKE duration configured, so it may be terminated earlier.

## Additional Usage Notes

* If identity provider SSO authentication is used in standalone mode, the browser URL will be printed to the console. In command mode, the browser will be opened automatically.
* If the password is not provided in the configuration file, the first entered MFA challenge code will be stored in the OS keychain unless the `no-keychain` parameter is specified. Keychain integration is provided only in command mode. The `password-factor` option controls which MFA factor to consider a password.

<a id="faq"></a>

## Troubleshooting Common Problems

| Problem                                                                       | Solution                                                                                                                                                                                |
|-------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `Timeout while waiting for identity response`                                 | Check if the correct login type is specified (one of the vpn_XXX identifiers returned from the "-m info" command).                                                                      |
| `Error sending request for url (https://IP_OR_HOSTNAME/clients/)`             | VPN server host is not reachable or certificate is untrusted. Use `ignore-server-cert` parameter to disable all HTTPS certificate checks (not recommended).                             |
| `No session in reply`                                                         | Usually happens when Check Point server runs out of Office Mode licenses. Try the `client-mode` parameter with different values: `endpoint_security`, `secure_remote`, `secure_connect` | 
| Unable to reach remote sites by their fully qualified names                   | VPN server does not return back the proper search domains. Use the `search-domains` option to specify DNS domains manually.                                                             | 
| Connections to remote sites are unstable, IP address changes every 10 minutes | VPN server has a short IP lease policy configured. Try the `ip-lease-time` option to manually extend it. Value must be specified in seconds.                                            | 

## Contributing

Pull requests, bug reports, and suggestions are welcome. This is a hobby project I maintain in my free time.

Before opening a PR, make sure to reformat the sources with the `cargo fmt` command and run it through the `cargo clippy` for any warnings.

## Additional Translations

The provided [sample AI prompt](https://github.com/ancwrd1/snx-rs/blob/main/llm-localization-prompt.txt) can be used
to perform automated translation via the AI agent of choice. Tested with Zed editor and GPT-4.1 model.

## Building from Sources

* Install the required dependencies:
  - Debian/Ubuntu: `sudo apt install build-essential libssl-dev libgtk-4-dev`
  - openSUSE: `sudo zypper install libopenssl-3-devel gtk4-devel`
  - Other distros: C compiler, OpenSSL, GTK 4 development packages
* Install a recent [Rust compiler](https://rustup.rs)
* Run `cargo build` to build the debug version, or `cargo build --release` to build the release version

NOTE: the minimal supported Rust version is 1.88.

## Acknowledgements

Special thanks to the [cpyvpn](https://gitlab.com/cpvpn/cpyvpn) project for inspiration around SAML and IKEv1 exchange.

## License

Licensed under the [GNU Affero General Public License version 3](https://opensource.org/license/agpl-v3/).

