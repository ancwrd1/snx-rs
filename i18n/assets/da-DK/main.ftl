# Dialog and buttons
dialog-title = VPN-indstillinger
button-ok = OK
button-apply = Anvend
button-cancel = Annuller
button-fetch-info = Hent information

# Labels
label-server-address = VPN-serveradresse
label-auth-method = Godkendelsesmetode
label-tunnel-type = Tunneltype
label-cert-auth-type = Certifikattype
label-icon-theme = Ikon-tema
label-username = Brugernavn
label-password = Adgangskode
label-no-dns = Ændr ikke DNS-konfigurationen
label-dns-servers = Yderligere DNS-servere
label-ignored-dns-servers = Ignorerede DNS-servere
label-search-domains = Yderligere søgedomæner
label-ignored-domains = Ignorerede søgedomæner
label-routing-domains = Behandl modtagne søgedomæner som routingdomæner
label-ca-cert = Server CA-rodcertifikat
label-no-cert-check = Deaktiver alle TLS-certifikatkontroller (USIKKERT!)
label-password-factor = Adgangskodefaktorindeks, 1..N
label-no-keychain = Gem ikke adgangskoder i nøglering
label-ike-lifetime = IPSec IKE SA-levetid, sekunder
label-ike-persist = Gem IPSec IKE-session og genopret automatisk
label-no-keepalive = Deaktiver IPSec keepalive-pakker
label-port-knock = Aktiver NAT-T port knocking
label-no-routing = Ignorer alle modtagne ruter
label-default-routing = Angiv standardrute gennem tunnellen
label-add-routes = Yderligere statiske ruter
label-ignored-routes = Ruter der skal ignoreres
label-client-cert = Klientcertifikat eller driversti (.pem, .pfx/.p12, .so)
label-cert-password = PFX-adgangskode eller PKCS11-PIN
label-cert-id = PKCS11-certifikatets hexadecimale ID
label-language = Sprog
label-system-language = Systemstandard

# Tabs and expanders
tab-general = Generelt
tab-advanced = Avanceret
expand-dns = DNS
expand-routing = Routing
expand-certificates = Certifikater
expand-misc = Yderligere indstillinger

# Error messages
error-no-server = Ingen serveradresse angivet
error-no-auth = Ingen godkendelsesmetode valgt
error-file-not-exist = Filen findes ikke: {$path}
error-invalid-cert-id = Certifikat-ID er ikke i hexadecimalt format: {$id}
error-ca-root-not-exist = CA-rodsti findes ikke: {$path}
error-validation = Valideringsfejl
error-user-input-canceled = Brugerinput annulleret
error-connection-canceled = Forbindelse annulleret
error-unknown-event = Ukendt begivenhed: {$event}
error-no-service-connection = Ingen forbindelse til tjenesten
error-empty-input = Input kan ikke være tomt

# New error messages
error-invalid-object = Ugyldigt objekt
error-no-connector = Ingen tunnelforbindelse
error-connection-cancelled = Forbindelse annulleret
error-tunnel-disconnected = Tunnel afbrudt, sidste besked: {$message}
error-unexpected-reply = Uventet svar
error-auth-failed = Godkendelse mislykkedes
error-no-server-name = Manglende påkrævet parameter: server-name
error-no-login-type = Manglende påkrævet parameter: login-type
error-connection-timeout = Forbindelsestimeout
error-invalid-response = Ugyldigt svar
error-cannot-send-request = Kan ikke sende anmodning til tjenesten
error-cannot-read-reply = Kan ikke læse svar fra tjenesten
error-no-ipv4 = Ingen IPv4-adresse for {$server}
error-not-challenge-state = Ikke en udfordringsstatus
error-no-challenge = Ingen udfordring i data
error-endless-challenges = Uendelig løkke af brugernavnudfordringer
error-no-pkcs12 = Ingen PKCS12-sti og adgangskode angivet
error-no-pkcs8 = Ingen PKCS8 PEM-sti angivet
error-no-pkcs11 = Ingen PKCS11 PIN angivet
error-no-ipsec-session = Ingen IPSEC-session

# Placeholder texts
placeholder-domains = Domæner adskilt med komma
placeholder-ip-addresses = IP-adresser adskilt med komma
placeholder-routes = Ruter adskilt med komma i formatet x.x.x.x/x
placeholder-certs = PEM- eller DER-filer adskilt med komma

# Tunnel types
tunnel-type-ipsec = IPSec
tunnel-type-ssl = SSL (forældet)

# Certificate types
cert-type-none = Ingen
cert-type-pfx = PFX-fil
cert-type-pem = PEM-fil
cert-type-hw = Hardware-token

# Icon themes
icon-theme-auto = Automatisk
icon-theme-dark = Mørk
icon-theme-light = Lys

# Connection info
info-connected-since = Forbundet siden
info-server-name = Servernavn
info-user-name = Brugernavn
info-login-type = Logintype
info-tunnel-type = Tunneltype
info-transport-type = Transporttype
info-ip-address = IP-adresse
info-dns-servers = DNS-servere
info-search-domains = Søgedomæner
info-interface = Interface
info-dns-configured = DNS konfigureret
info-routing-configured = Routing konfigureret
info-default-route = Standardrute

# Application
app-title = SNX-RS VPN-klient til Linux
app-connection-error = Forbindelsesfejl
app-connection-success = Forbindelse lykkedes

# Authentication
auth-dialog-title = VPN-godkendelsesfaktor
auth-dialog-message = Indtast din godkendelsesfaktor:

# Status dialog
status-dialog-title = Forbindelsesinformation
status-button-copy = Kopiér
status-button-settings = Indstillinger
status-button-connect = Forbind
status-button-disconnect = Afbryd

# Tray menu
tray-menu-connect = Forbind
tray-menu-disconnect = Afbryd
tray-menu-status = Forbindelsesstatus...
tray-menu-settings = Indstillinger...
tray-menu-about = Om...
tray-menu-exit = Afslut

# CLI Messages
cli-identity-provider-auth = For godkendelse via identitetsudbyder, åbn følgende URL i din browser:
cli-tunnel-connected = Tunnel forbundet, tryk Ctrl+C for at afslutte.
cli-tunnel-disconnected = Tunnel forbindelse afbrudt
cli-another-instance-running = En anden forekomst af snx-rs kører allerede
cli-app-terminated = Applikation afsluttet af signal

# Connection Messages
connection-connected-to = Forbundet til {$server}

# Languages
language-cs-CZ = Tjekkisk
language-da-DK = Dansk
language-de-DE = Tysk
language-en-US = Engelsk
language-es-ES = Spansk
language-fi-FI = Finsk
language-fr-FR = Fransk
language-it-IT = Italiensk
language-nl-NL = Hollandsk
language-no-NO = Norsk
language-pl-PL = Polsk
language-pt-PT = Portugisisk
language-ru-RU = Russisk
language-sk-SK = Slovakisk
language-sv-SE = Svensk

# Connection status messages
connection-status-disconnected = Afbrudt
connection-status-connecting = Forbinder
connection-status-connected-since = Forbundet siden: {$since}
connection-status-mfa-pending = Afventer MFA: {$mfa_type}

# Login options
login-options-server-address = Serveradresse
login-options-server-ip = Server-IP
login-options-client-enabled = Klient aktiveret
login-options-supported-protocols = Understøttede protokoller
login-options-preferred-protocol = Foretrukken protokol
login-options-tcpt-port = TCPT-port
login-options-natt-port = NATT-port
login-options-internal-ca-fingerprint = Internt CA-fingeraftryk
