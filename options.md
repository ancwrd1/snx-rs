## Configuration options

| Option                                    | Description                                                                                                                                           |
|-------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------|
| `server-name=<ip_or_address>`             | VPN server to connect to, this is a required parameter                                                                                                |
| `login-type=vpn_xxx`                      | Authentication method, acquired from the server, this is a required parameter                                                                         |
| `user-name=<username>`                    | User name to authenticate, not used for SAML or certificate authentication                                                                            |
| `password=<pass>`                         | Optional password in base64 encoding                                                                                                                  |
| `password-factor=<1..N>`                  | Index of the password authentication factor which is used for keychain storage and for reading the password from config file. Default is 1 (first).   |
| `cert-type=<cert_type>`                   | Enable certificate-based authentication using given type: pkcs8, pkcs11, pkcs12, none                                                                 |
| `cert-path=<cert_path>`                   | Path to PEM file for PKCS8, path to PFX file for PKCS12, path to driver file for PKCS11                                                               |
| `cert-password=<cert_password>`           | Password for PKCS12 or pin for PKCS11                                                                                                                 |
| `cert-id=<cert_id>`                       | Hexadecimal ID of PKCS11 certificate, bytes could be optionally separated with colon                                                                  |
| `search-domains=<search_domains>`         | Additional search domains for DNS resolver, comma-separated                                                                                           |
| `ignore-search-domains=<ignored_domains>` | Acquired search domains to ignore                                                                                                                     |
| `dns-servers=<dns_servers>`               | Additional DNS servers, comma-separated                                                                                                               |
| `ignore-dns-servers=<ignored_dns>`        | Acquired DNS servers to ignore, comma-separated                                                                                                       |
| `set-routing-domains=true\|false`         | Treat received search domains as routing domains. This option prevents DNS requests for unqualified domains to be sent through the tunnel.            |
| `default-route=true\|false`               | Set default route through the VPN tunnel, default is false                                                                                            |
| `no-routing=true\|false`                  | Ignore all routes acquired from the VPN server, default is false                                                                                      |
| `add-routes=<routes>`                     | Additional static routes, comma-separated, in the format of x.x.x.x/x                                                                                 |
| `ignore-routes=<routes>`                  | Ignore the specified routes acquired from the VPN server                                                                                              |
| `no-dns=true\|false`                      | Do not change DNS resolver configuration, default is false                                                                                            |
| `ignore-server-cert=true\|false`          | Disable all certificate checks, default is false                                                                                                      |
| `ca-cert=<ca_certs>`                      | One or more comma-separated custom CA root certificates used to validate TLS connection.                                                              |
| `tunnel-type=ipsec\|ssl`                  | Tunnel type, default is ipsec                                                                                                                         |
| `no-keychain=true\|false`                 | Do not store password in the OS keychain, default is false                                                                                            |
| `esp-lifetime=3600`                       | ESP SA lifetime in seconds, default is 3600                                                                                                           |
| `ike-lifetime=28800`                      | IKE SA lifetime in seconds, default is 28800. Set to higher value to extend IPSec session duration                                                    |
| `ike-persist=true\|false`                 | Save IKE session to disk and try to reconnect automatically after application restart                                                                 |
| `log-level=<log_level>`                   | Logging level: error, warn, debug, info, trace. Default is info. Note: trace-level log includes request and response dumps with sensitive information |
| `no-keepalive=true\|false`                | Disable keepalive packets for IPSec. Some Check Point servers block the keepalive requests.                                                           |
| `icon-theme=auto\|dark\|light`            | Set icon theme for the GUI app.                                                                                                                       |
