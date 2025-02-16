## Configuration options

| Option                                    | Description                                                                                                                                           |
|-------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------|
| `server-name=<ip_or_address>`             | VPN server to connect to, this is a required parameter                                                                                                |
| `login-type=vpn_xxx`                      | authentication method, acquired from the server, this is a required parameter                                                                         |
| `user-name=<username>`                    | user name to authenticate, not used for SAML or certificate authentication                                                                            |
| `password=<pass>`                         | optional password in base64 encoding                                                                                                                  |
| `password-factor=<1..N>`                  | index of the password authentication factor which is used for keychain storage and for reading the password from config file. Default is 1 (first).   |
| `cert-type=<cert_type>`                   | enable certificate-based authentication using given type: pkcs8, pkcs11, pkcs12, none                                                                 |
| `cert-path=<cert_path>`                   | path to PEM file for PKCS8, path to PFX file for PKCS12, path to driver file for PKCS11                                                               |
| `cert-password=<cert_password>`           | password for PKCS12 or pin for PKCS11                                                                                                                 |
| `cert-id=<cert_id>`                       | hexadecimal ID of PKCS11 certificate, bytes could be optionally separated with colon                                                                  |
| `search-domains=<search_domains>`         | additional search domains for DNS resolver, comma-separated                                                                                           |
| `ignore-search-domains=<ignored_domains>` | acquired search domains to ignore                                                                                                                     |
| `dns-servers=<dns_servers>`               | additional DNS servers, comma-separated                                                                                                               |
| `ignore-dns-servers=<ignored_dns>`        | acquired DNS servers to ignore, comma-separated                                                                                                       |
| `default-route=true\|false`               | set default route through the VPN tunnel, default is false                                                                                            |
| `no-routing=true\|false`                  | ignore all routes acquired from the VPN server, default is false                                                                                      |
| `add-routes=<routes>`                     | additional static routes, comma-separated, in the format of x.x.x.x/x                                                                                 |
| `ignore-routes=<routes>`                  | ignore the specified routes acquired from the VPN server                                                                                              |
| `no-dns=true\|false`                      | do not change DNS resolver configuration, default is false                                                                                            |
| `no-cert-check=true\|false`               | do not check server certificate common name, default is false                                                                                         |
| `ignore-server-cert=true\|false`          | disable all certificate checks, default is false                                                                                                      |
| `ca-cert=<ca_certs>`                      | One or more comma-separated custom CA root certificates used to validate TLS connection and optionally IPSec certificates.                            |
| `ipsec-cert-check=true\|false`            | enable IPSec certificate check during IKE identity protection phase. Requires custom CA root certificate to be specified.                             |
| `tunnel-type=ipsec\|ssl`                  | tunnel type, default is ipsec                                                                                                                         |
| `no-keychain=true\|false`                 | do not store password in the OS keychain, default is false                                                                                            |
| `server-prompt=true\|false`               | retrieve MFA prompts from the server, default is false                                                                                                |
| `esp-lifetime=3600`                       | ESP SA lifetime in seconds, default is 3600                                                                                                           |
| `esp-transport=udp\|tcpt`                 | Select network transport for ESP packets. UDP is the default and standard, TCPT is the Check Point proprietary protocol and is much slower.           |
| `ike-lifetime=28800`                      | IKE SA lifetime in seconds, default is 28800. Set to higher value to extend IPSec session duration                                                    |
| `ike-port=500`                            | IKE communication port, either 500 or 4500, default is 500                                                                                            |
| `ike-persist=true\|false`                 | Save IKE session to disk and try to reconnect automatically after application restart                                                                 |
| `ike-transport=udp\|tcpt`                 | Select network transport for IKE exchange. UDP is the default and standard, TCPT is the Check Point proprietary protocol.                             |
| `log-level=<log_level>`                   | Logging level: error, warn, debug, info, trace. Default is info. Note: trace-level log includes request and response dumps with sensitive information |
| `no-keepalive=true\|false`                | Disable keepalive packets for IPSec. Some Check Point servers block the keepalive requests.                                                           |
| `icon-theme=auto\|dark\|light`            | Set icon theme for the GUI app.                                                                                                                       |
