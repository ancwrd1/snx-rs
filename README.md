# Open Source Linux Client for Check Point VPN Tunnels

[![github actions](https://github.com/ancwrd1/snx-rs/workflows/CI/badge.svg)](https://github.com/ancwrd1/snx-rs/actions)
[![license](https://img.shields.io/badge/License-AGPL-v3.svg)](https://opensource.org/license/agpl-v3)

This project contains the source code for an unofficial Linux client for Check Point VPN, written in Rust.

* [System Requirements](#system-requirements)
* [Installation](#installation)
* [Quick Start Guide](#quick-start-guide)
* [Quick Start Guide (CLI, standalone mode)](#quick-start-guide-cli-standalone-mode)
* [Quick Start Guide (CLI, command mode)](#quick-start-guide-cli-command-mode)
* [Advantages Over the Official SNX Client for Linux](#advantages-over-the-official-snx-client-for-linux)
* [Implemented Features](#implemented-features)
* [DNS Resolver Configuration and Privacy](#dns-resolver-configuration-and-privacy)
* [Tunnel Type Selection](#tunnel-type-selection)
* [Command Line Usage](#command-line-usage)
* [Usage Examples](#usage-examples)
* [Mobile Access Authentication](#mobile-access-authentication)
* [Certificate Validation](#certificate-validation)
* [Certificate Authentication](#certificate-authentication)
* [Machine Certificate Authentication](#machine-certificate-authentication)
* [Certificate Enrollment](#certificate-enrollment)
* [Persistent IPSec Session](#persistent-ipsec-session)
* [Additional Usage Notes](#additional-usage-notes)
* [Troubleshooting Common Problems](#troubleshooting-common-problems)
* [Contributing](#contributing)
* [Additional Translations](#additional-translations)
* [Building from Sources](#building-from-sources)
* [Docker Usage](#docker-usage)
* [Acknowledgements](#acknowledgements)
* [License](#license)

<!-- TOC --><a name="system-requirements"></a>
## System Requirements

* A recent Linux distribution with kernel version 4.19 or higher.
* `systemd-resolved` is highly recommended as a global DNS resolver, to avoid sending all DNS traffic to the corporate VPN servers.
* GTK 4.10+ for the UI frontend.
* Optional: WebKit 6.0+ for the `mobile-access` feature.
* GNOME desktop: [AppIndicator](https://extensions.gnome.org/extension/615/appindicator-support/) extension. Not needed for Ubuntu.

<!-- TOC --><a name="installation"></a>
## Installation

Download the latest binary and source release [here](https://github.com/ancwrd1/snx-rs/releases/latest).<br/>
For Arch Linux and derivatives, the [AUR package](https://aur.archlinux.org/packages/snx-rs) can be used.<br/>
For NixOS follow the specific [configuration instructions](https://github.com/ancwrd1/snx-rs/blob/main/nixos.md).

1. Download the installer from the releases section, then: `chmod +x snx-rs-*-linux-x86_64.run`
2. Install the application: `sudo ./snx-rs-*-linux-x86_64.run`

For Ubuntu/Debian, a DEB package is provided in the release assets. For RPM-based distros (Fedora, CentOS, openSUSE) use the provided RPM package.

<!-- TOC --><a name="quick-start-guide"></a>
## Quick Start Guide

1. Run the GUI frontend from the application menu of the desktop manager.
2. Click on the application tray icon, choose "Settings."
3. In the opened dialog, type the server address and press "Fetch info" to retrieve a list of supported login types.
4. Select the login type and save settings. Username and password fields are optional.
5. Click on the application tray icon and choose "Connect."

If the desktop environment does not have a dbus SNI interface, use the `--no-tray` command line parameter to the snx-rs-gui application to show the status window instead of the notification icon.

<!-- TOC --><a name="quick-start-guide-cli-standalone-mode"></a>
## Quick Start Guide (CLI, standalone mode)

1. Get the list of supported login types: `snx-rs -m info -s remote.company.com`
2. Run the tunnel: `sudo snx-rs -o vpn_Microsoft_Authenticator -s remote.company.com`

⚠️ **Note about certificate errors**: if the connection fails with the certificate error, add the `-X true` parameter (insecure and not recommended).

<!-- TOC --><a name="quick-start-guide-cli-command-mode"></a>
## Quick Start Guide (CLI, command mode)

1. Get the list of supported login types: `snx-rs -m info -s remote.company.com`
2. Create a configuration file: `$HOME/.config/snx-rs/snx-rs.conf`, with desired [options](https://github.com/ancwrd1/snx-rs/blob/main/options.md).
3. Connect the tunnel with `snxctl connect` command.

<!-- TOC --><a name="advantages-over-the-official-snx-client-for-linux"></a>
## Advantages Over the Official SNX Client for Linux

* Open source
* IPSec support (provides a much faster tunnel)
* More authentication methods
* Better privacy for DNS requests: only requests for VPN-specific suffixes are routed through the tunnel
* Better integration with system DNS resolver
* Optional integration with GNOME Keyring or KDE KWallet
* Customizable routing and DNS settings

<!-- TOC --><a name="implemented-features"></a>
## Implemented Features

* Browser-based identity provider authentication
* Username/password authentication with MFA support
* Certificate authentication via provided client certificate (PFX, PEM, or HW token)
* Hybrid authentication using machine certificate and user credentials
* Mobile Access authentication using VPN web portal (experimental)
* HSM token authentication
* GTK frontend with tray icon
* IPSec tunnel via Linux native kernel XFRM interface or TCPT/TUN transport
* Automatic IPSec tunnel reconnection without authentication, via optional parameter
* SSL tunnel via Linux TUN device
* Store a password factor in the OS keychain using Secret Service API
* Multiple connection profiles
* Certificate enrollment and renewal using command line interface

<!-- TOC --><a name="dns-resolver-configuration-and-privacy"></a>
## DNS Resolver Configuration and Privacy

By default, if systemd-resolved is not detected as a global DNS resolver, snx-rs will fall back
to modify the /etc/resolv.conf file directly and DNS servers acquired from the tunnel will be used globally.
For better privacy, use the split DNS provided by systemd-resolved.

To find out whether it is already enabled, check the /etc/resolv.conf file:

`readlink /etc/resolv.conf`

If it is a symlink pointing to `/run/systemd/resolve/stub-resolv.conf` then it is already configured on your system,
otherwise follow these steps:

1. Install `systemd-resolved` package if is not already installed.
2. `sudo ln -sf /run/systemd/resolve/stub-resolv.conf /etc/resolv.conf`
3. `sudo systemctl enable --now systemd-resolved`
4. `sudo systemctl restart NetworkManager`

With `systemd-resolved` it is also possible to use **routing domains** (as opposed to **search domains**).
Routing domains are prefixed with `~` character and when configured only requests for the fully qualified domains
will be forwarded through the tunnel. For further explanation, please check [this article](https://systemd.io/RESOLVED-VPNS/).

The `set-routing-domains=true|false` option controls whether to treat all acquired search domains as routing domains.

<!-- TOC --><a name="tunnel-type-selection"></a>
## Tunnel Type Selection

snx-rs supports two tunnel types: IPSec and SSL. IPSec tunnel is a default option if not specified in the configuration.
Depending on the availability of the kernel `xfrm` module, it will use either a native kernel IPSec infrastructure or a TUN device
with userspace ESP packet encoding.

IPSec ESP traffic is encapsulated in the UDP packets sent via port 4500 which may be blocked in some environments.
In this case the application will fall back to the proprietary Check Point TCPT transport via TCP port 443, which is slower than UDP.

The `transport-type` option can be used to choose the IPSec transport type manually. The default value is `auto` which will perform autodetection.

For older VPN servers or in case IPSec does not work for some reason, the legacy SSL tunnel can be used as well, selected with `tunnel-type=ssl`.
SSL tunnel has some limitations: it is slower, has no hardware token support and no MFA in combination with the certificates.

<!-- TOC --><a name="command-line-usage"></a>
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

<!-- TOC --><a name="usage-examples"></a>
## Usage Examples

```bash
# standalone mode with trace logging and IPSec tunnel
sudo ./snx-rs -o vpn_Microsoft_Authenticator -s remote.company.com -e ipsec -l trace

# command mode with debug logging (use snxctl to establish a connection)
sudo ./snx-rs -m command -l debug
```

<!-- TOC --><a name="mobile-access-authentication"></a>
## Mobile Access Authentication

Mobile Access authentication is an experimental feature that replaces the official `cshell` connection method using the web access.
It is helpful in cases when a DynamicID MFA code is required via the web portal login and the normal VPN connection via the advertised
authentication types does not work.
To use it, the snx-rs-gui application must be built with the `mobile-access` feature enabled. It requires the `webkit6` package to be installed.
Depending on the distribution, it may be called `libwebkitgtk-6.0-dev`, `webkit2gtk4-devel` or similar.
Additionally, it may require to install `libsoup-3.0-dev` and `libjavascriptcoregtk-6.0-dev`.

A new login type called "Mobile Access" will be visible from the dropdown list in the GUI settings. When connecting using that method,
the application will open a browser window and navigate to the VPN web portal. After successful authentication, the application will attempt
to find the password cookie in the web page. Then the browser will be closed and the tunnel will be established.

<!-- TOC --><a name="certificate-validation"></a>
## Certificate Validation

The following parameters control certificate validation during TLS and IKE exchanges:

* `ca-cert`: Comma-separated list of paths to PEM or DER files which contain custom CA root certificates
* `ignore-server-cert`: true|false. Disable all TLS certificate checks. Insecure and not recommended. Default is false.

Note that enabling the insecure option may compromise the channel security.

<!-- TOC --><a name="certificate-authentication"></a>
## Certificate Authentication

The following parameters control certificate-based authentication:

* `cert-type`: One of `none`, `pkcs12`, `pkcs8` or `pkcs11`. Choose `pkcs12` to read the certificate from an external PFX file. Choose `pkcs8` to read the certificate from an external PEM file (containing both private key and x509 cert). Private key must come first in this file. Choose `pkcs11` to use a hardware token via a PKCS11 driver.
* `cert-path`: Path to the PFX, PEM, or custom PKCS11 driver file, depending on the selected cert type. The default PKCS11 driver is `opensc-pkcs11.so`, which requires the opensc package to be installed.
* `cert-password`: Password for PKCS12 or PIN for PKCS11. Must be provided for those types.
* `cert-id`: Optional hexadecimal ID of the certificate for the PKCS11 type. Could be in the form of `xx:xx:xx` or `xxxxxx`.

Certificate authentication should be used with the appropriate vpn_XXX login type which has a "certificate" as its authentication factor.

<!-- TOC --><a name="machine-certificate-authentication"></a>
## Machine Certificate Authentication

With the machine certificate authentication it is possible to combine the certificate with the normal authentication methods.
To enable it, specify the certificate authentication options as described in the previous section and use one of the normal
vpn_XXX login types. The machine certificate authentication must be enabled on the VPN server side.
The certificate subject must have an entry for the machine name: `CN=<machinename>`. It does not have to match the Linux hostname.

When using a GUI frontend, there is a switch in the settings dialog to enable this option.

<!-- TOC --><a name="certificate-enrollment"></a>
## Certificate Enrollment

`snx-rs` supports certificate enrollment and renewal for those configurations which require certificate-based authentication.
It is implemented as a command-line interface, with two additional operation modes of the `snx-rs` application: `enroll` and `renew`.
Enrollment operation requires a registration key which the user should receive from the IT department. Renewal requires an existing certificate in PKCS12 format.

Usage:

```bash
# Enrollment into identity.p12 file using registration key 12345678
snx-rs --mode enroll \
       --reg-key=12345678 \
       --cert-path=identity.p12 \
       --cert-password=password \
       --server-name=remote.company.com
```

```bash
# Renewal using existing identity.p12
snx-rs --mode renew \
       --cert-path=identity.p12 \
       --cert-password=password \
       --server-name=remote.company.com
```

After enrollment or renewal, the obtained PKCS12 keystore can be used for tunnel authentication.

<!-- TOC --><a name="persistent-ipsec-session"></a>
## Persistent IPSec Session

The `ike-persist` option will save IPSec session to disk and restore it after the service or computer restarts,
it will then attempt to automatically reconnect the tunnel without authentication. This parameter works best in combination with the `ike-lifetime` option:
for example, setting `ike-lifetime` to 604800 will keep the session for 7 days.

Note that most IPSec servers have shorter IKE duration configured, so it may be terminated earlier.

<!-- TOC --><a name="additional-usage-notes"></a>
## Additional Usage Notes

* If identity provider SSO authentication is used in standalone mode, the browser URL will be printed to the console. In command mode, the browser will be opened automatically.
* If the password is not provided in the configuration file, the first entered MFA challenge code will be stored in the OS keychain unless the `no-keychain` parameter is specified. Keychain integration is provided only in command mode. The `password-factor` option controls which MFA factor to consider a password.

<a id="faq"></a>

<!-- TOC --><a name="troubleshooting-common-problems"></a>
## Troubleshooting Common Problems

| Problem                                                                       | Solution                                                                                                                                                                                |
|-------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `Timeout while waiting for identity response`                                 | Check if the correct login type is specified (one of the vpn_XXX identifiers returned from the "-m info" command).                                                                      |
| `Error sending request for url (https://IP_OR_HOSTNAME/clients/)`             | VPN server host is not reachable or certificate is untrusted. Use `ignore-server-cert` parameter to disable all HTTPS certificate checks (not recommended).                             |
| `No session in reply`                                                         | Usually happens when Check Point server runs out of Office Mode licenses. Try the `client-mode` parameter with different values: `endpoint_security`, `secure_remote`, `secure_connect` | 
| Unable to reach remote sites by their fully qualified names                   | VPN server does not return back the proper search domains. Use the `search-domains` option to specify DNS domains manually.                                                             | 
| VPN tunnel is unstable and disconnects quickly                                | Set the `mtu` option to a lower value like 1280.                                                                                                                                        | 
| Connections to remote sites are unstable, IP address changes every 10 minutes | VPN server has a short IP lease policy configured. Try the `ip-lease-time` option to manually extend it. Value must be specified in seconds.                                            | 

<!-- TOC --><a name="contributing"></a>
## Contributing

Pull requests, bug reports, and suggestions are welcome. Before opening a PR, make sure to reformat the sources with the `cargo fmt` command and run it through the `cargo clippy` for any warnings.

### AI Policy

AI-generated code may be accepted if it implements a useful feature, fixes a critical bug, adds a translation or helps with the CI/CD workflows. Please test and review the generated code before submission:

* There are no redundant ELI5-style comments added for every line of code or every function
* The code is maintainable and can be easily understood

I reserve the right to reject the AI slop without discussion.

<!-- TOC --><a name="additional-translations"></a>
## Additional Translations

The provided [sample AI prompt](https://github.com/ancwrd1/snx-rs/blob/main/i18n.md) can be used
to perform automated translation via the AI agent of choice.

<!-- TOC --><a name="building-from-sources"></a>
## Building from Sources

* Install the required dependencies:
  - Debian/Ubuntu: `sudo apt install build-essential libssl-dev libgtk-4-dev libwebkitgtk-6.0-dev libsoup-3.0-dev libjavascriptcoregtk-6.0-dev libsqlite3-dev`
  - openSUSE: `sudo zypper install libopenssl-3-devel gtk4-devel webkit2gtk4-devel sqlite3-devel`
  - Other distros: C compiler, OpenSSL, SQLite3, GTK 4 development packages, optionally WebKit 6 development package
* Install a recent [Rust compiler](https://rustup.rs)
* Run `cargo build` to build the debug version, or `cargo build --release` to build the release version.
* To build a version with mobile access feature and webkit integration, pass the `--features=mobile-access` parameter.

NOTE: the minimal supported Rust version is 1.88.

### Static Build Recipe

The snx-rs command line application can be built and linked statically to use in containers or embedded environments.
System requirements: same as the normal build + docker or podman.

Static build instructions:

* Install `cross-rs` with `cargo install cross`
* Add `x86_64-unknown-linux-musl` target to the Rust compiler: `rustup target add x86_64-unknown-linux-musl`
* Build a static snx-rs executable with `cross build --target=x86_64-unknown-linux-musl --features snxcore/vendored-openssl,snxcore/vendored-sqlite -p snx-rs --profile lto`

<!-- TOC --><a name="docker-usage"></a>
## Docker Usage

Check [this repository](https://github.com/leleobhz/snx-rs-docker) for a docker container.

<!-- TOC --><a name="acknowledgements"></a>
## Acknowledgements

Special thanks to the [cpyvpn](https://gitlab.com/cpvpn/cpyvpn) project for inspiration around SAML and IKEv1 exchange.

<!-- TOC --><a name="license"></a>
## License

Licensed under the [GNU Affero General Public License version 3](https://opensource.org/license/agpl-v3/).
