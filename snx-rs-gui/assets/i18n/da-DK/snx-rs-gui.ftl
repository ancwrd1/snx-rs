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
