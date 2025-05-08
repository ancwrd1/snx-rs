# Dialog and buttons
dialog-title = VPN settings
button-ok = OK
button-apply = Apply
button-cancel = Cancel
button-fetch-info = Fetch info

# Labels
label-server-address = VPN server address
label-auth-method = Authentication method
label-tunnel-type = Tunnel type
label-cert-auth-type = Certificate auth type
label-icon-theme = Icon theme
label-username = User name
label-password = Password
label-no-dns = Do not change DNS resolver configuration
label-dns-servers = Additional DNS servers
label-ignored-dns-servers = Ignored DNS servers
label-search-domains = Additional search domains
label-ignored-domains = Ignored search domains
label-routing-domains = Treat received search domains as routing domains
label-ca-cert = Server CA root certificates
label-no-cert-check = Disable all TLS certificate checks (INSECURE!)
label-password-factor = Index of password factor, 1..N
label-no-keychain = Do not store passwords in the keychain
label-ike-lifetime = IPSec IKE SA lifetime, seconds
label-ike-persist = Save IPSec IKE session and reconnect automatically
label-no-keepalive = Disable IPSec keepalive packets
label-port-knock = Enable NAT-T port knocking
label-no-routing = Ignore all acquired routes
label-default-routing = Set default route through the tunnel
label-add-routes = Additional static routes
label-ignored-routes = Routes to ignore
label-client-cert = Client certificate or driver path (.pem, .pfx/.p12, .so)
label-cert-password = PFX password or PKCS11 pin
label-cert-id = Hex ID of PKCS11 certificate

# Tabs and expanders
tab-general = General
tab-advanced = Advanced
expand-dns = DNS
expand-routing = Routing
expand-certificates = Certificates
expand-misc = Misc settings

# Error messages
error-no-server = No server address specified
error-no-auth = No authentication method selected
error-file-not-exist = File does not exist: {$path}
error-invalid-cert-id = Certificate ID not in hex format: {$id}
error-ca-root-not-exist = CA root path does not exist: {$path}
error-validation = Validation error
error-user-input-canceled = User input canceled
error-connection-canceled = Connection canceled
error-unknown-event = Unknown event: {$event}
error-no-service-connection = No connection to service
error-empty-input = Input cannot be empty

# Placeholder texts
placeholder-domains = Comma-separated domains
placeholder-ip-addresses = Comma-separated IP addresses
placeholder-routes = Comma-separated x.x.x.x/x
placeholder-certs = Comma-separated PEM or DER files

# Tunnel types
tunnel-type-ipsec = IPSec
tunnel-type-ssl = SSL (deprecated)

# Certificate types
cert-type-none = None
cert-type-pfx = PFX file
cert-type-pem = PEM file
cert-type-hw = Hardware token

# Icon themes
icon-theme-auto = Auto
icon-theme-dark = Dark
icon-theme-light = Light

# Application
app-title = SNX-RS VPN Client for Linux
app-connection-error = Connection error
app-connection-success = Connection succeeded

# Authentication
auth-dialog-title = VPN Authentication Factor
auth-dialog-message = Please enter your authentication factor:

# Status dialog
status-dialog-title = Connection information
status-button-copy = Copy
status-button-settings = Settings
status-button-connect = Connect
status-button-disconnect = Disconnect

# Tray menu
tray-menu-connect = Connect
tray-menu-disconnect = Disconnect
tray-menu-status = Connection status...
tray-menu-settings = Settings...
tray-menu-about = About...
tray-menu-exit = Exit

# Connection info
info-connected-since = Connected since
info-server-name = Server name
info-user-name = User name
info-login-type = Login type
info-tunnel-type = Tunnel type
info-transport-type = Transport type
info-ip-address = IP address
info-dns-servers = DNS servers
info-search-domains = Search domains
info-interface = Interface
info-dns-configured = DNS configured
info-routing-configured = Routing configured
info-default-route = Default route
