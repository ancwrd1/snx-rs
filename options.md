## Configuration options

| Option | Description |
| ------ | ----------- |
| `server-name=<ip_or_address>` | VPN server to connect to, this is a required parameter |
| `login-type=vpn_xxx` | authentication method, acquired from the server, this is a required parameter |
| `user-name=<username>` | user name to authenticate, not used for SAML or certificate authentication |
| `password=<pass>` | optional password in base64 encoding |
| `cert-type=<cert_type>` | enable certificate-based authentication using given type: pkcs8, pkcs11, pkcs12, none |
| `cert-path=<cert_path>` | path to PEM file for PKCS8, path to PFX file for PKCS12, path to driver file for PKCS11 |
| `cert-password=<cert_password>` | password for PKCS12 or pin for PKCS11 |
| `cert-id=<cert_id>` | hexadecimal ID of PKCS11 certificate, bytes could be optionally separated with colon |
| `search-domains=<search_domains>` | additional search domains for DNS resolver, comma-separated |
| `ignore-search-domains=<ignored_domains>` | acquired search domains to ignore |
| `default-route=true\|false` | set default route through the VPN tunnel, default is false |
| `no-routing=true\|false` | ignore all routes acquired from the VPN server, default is false |
| `add-routes=<routes>` | additional static routes, comma-separated, in the format of x.x.x.x/x |
| `ignore-routes=<routes>` | ignore the specified routes acquired from the VPN server |
| `no-dns=true\|false` | do not change DNS resolver configuration, default is false |
| `no-cert-check=true\|false` | do not check server certificate common name, default is false |
| `ignore-server-cert=true\|false` | disable all certificate checks, default is false |
| `tunnel-type=ipsec\|ssl` | tunnel type, default is ipsec |
| `no-keychain=true\|false` | do not store password in the OS keychain, default is false |
| `server-prompt=true\|false` | retrieve MFA prompts from the server, default is false |
| `esp-lifetime=3600` | ESP SA lifetime in seconds, default is 3600 |
| `ike-lifetime=28800` | IKE SA lifetime in seconds, default is 28800. Set to higher value to extend IPSec session duration |
| `ike-port=500` | IKE communication port, either 500 or 4500, default is 500 |
| `log-level=<log_level>` | Logging level: error, warn, debug, info, trace. Default is info. Note: trace-level log includes request and response dumps with sensitive information |
