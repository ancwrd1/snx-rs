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
label-language = Language
label-system-language = System default
label-username-password = Username and password
label-username-required = Username is required for authentication

# Tabs and expanders
tab-general = General
tab-advanced = Advanced
expand-dns = DNS
expand-routing = Routing
expand-certificates = Certificates
expand-misc = Misc settings

# Error messages
error-no-server-name = No server address specified
error-no-auth = No authentication method selected
error-file-not-exist = File does not exist: {$path}
error-invalid-cert-id = Certificate ID not in hex format: {$id}
error-ca-root-not-exist = CA root path does not exist: {$path}
error-validation = Validation error
error-user-input-canceled = User input canceled
error-connection-cancelled = Connection cancelled
error-unknown-event = Unknown event: {$event}
error-no-service-connection = No connection to service
error-empty-input = Input cannot be empty
error-invalid-response = Invalid response!

# New error messages
error-invalid-object = Invalid object
error-no-connector = No tunnel connector
error-tunnel-disconnected = Tunnel disconnected, last message: {$message}
error-unexpected-reply = Unexpected reply
error-auth-failed = Authentication failed
error-no-login-type = Missing required parameter: login-type
error-connection-timeout = Connection timeout
error-cannot-send-request = Cannot send request to the service
error-cannot-read-reply = Cannot read reply from the service
error-no-ipv4 = No IPv4 address for {$server}
error-not-challenge-state = Not a challenge state
error-no-challenge = No challenge in payload
error-endless-challenges = Endless loop of username challenges
error-no-pkcs12 = No PKCS12 path and password provided
error-no-pkcs8 = No PKCS8 PEM path provided
error-no-pkcs11 = No PKCS11 pin provided
error-no-ipsec-session = No IPSEC session
error-request-failed-error-code = Request failed, error code: {$error_code}
error-no-root-privileges = This program should be run as a root user!
error-missing-required-parameters = Missing required parameters: server name and/or login type!
error-missing-server-name = Missing required parameter: server name!
error-no-connector-for-challenge-code = No connector to send the challenge code to!
error-probing-failed = Probing failed, server is not reachable via NATT port!
error-invalid-sexpr = Invalid sexpr: {$value}
error-invalid-value = Invalid value
error-udp-request-failed = Error sending UDP request
error-no-tty = No attached TTY to get user input
error-invalid-auth-response = Invalid authentication response
error-invalid-client-settings = Invalid client settings response
error-invalid-otp-reply = Invalid OTP reply
error-udp-encap-failed = Cannot set UDP_ENCAP socket option, error code: {$code}
error-so-no-check-failed = Cannot set SO_NO_CHECK socket option, error code: {$code}
error-keepalive-failed = Keepalive failed
error-receive-failed = Receive failed
error-unknown-color-scheme = Unknown color-scheme value
error-cannot-determine-ip = Cannot determine default IP
error-invalid-command = Invalid command: {$command}
error-otp-browser-failed = Unable to acquire OTP from the browser
error-invalid-operation-mode = Invalid operation mode
error-invalid-tunnel-type = Invalid tunnel type
error-invalid-cert-type = Invalid cert type
error-invalid-icon-theme = Invalid icon theme
error-no-natt-reply = No NAT-T reply
error-not-implemented = Not implemented
error-unknown-packet-type = Unknown packet type
error-no-sender = No sender
error-empty-ccc-session = Empty CCC session
error-identity-timeout = Timeout while waiting for identity response, is the login type correct?

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

# CLI Messages
cli-identity-provider-auth = For identity provider authentication, open the following URL in your browser:
cli-tunnel-connected = Tunnel connected, press Ctrl-C to exit.
cli-tunnel-disconnected = Tunnel disconnected
cli-another-instance-running = Another instance of snx-rs is already running
cli-app-terminated = Application terminated due to a signal

# Connection Messages
connection-connected-to = Connected to {$server}

# Languages
language-cs-CZ = Czech
language-da-DK = Danish
language-de-DE = German
language-en-US = English
language-es-ES = Spanish
language-fi-FI = Finnish
language-fr-FR = French
language-it-IT = Italian
language-nl-NL = Dutch
language-no-NO = Norwegian
language-pl-PL = Polish
language-pt-PT = Portuguese
language-pt-BR = Brazillian Portuguese
language-ru-RU = Russian
language-sk-SK = Slovak
language-sv-SE = Swedish

# Connection status messages
connection-status-disconnected = Disconnected
connection-status-connecting = Connecting in progress
connection-status-connected-since = Connected since: {$since}
connection-status-mfa-pending = MFA pending: {$mfa_type}

# Login options
login-options-server-address = Server address
login-options-server-ip = Server IP
login-options-client-enabled = Client enabled
login-options-supported-protocols = Supported protocols
login-options-preferred-protocol = Preferred protocol
login-options-tcpt-port = TCPT port
login-options-natt-port = NATT port
login-options-internal-ca-fingerprint = Internal CA fingerprint
