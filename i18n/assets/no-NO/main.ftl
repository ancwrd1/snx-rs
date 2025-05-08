# Dialog and buttons
dialog-title = VPN-innstillinger
button-ok = OK
button-apply = Bruk
button-cancel = Avbryt
button-fetch-info = Hent informasjon

# Labels
label-server-address = VPN-serveradresse
label-auth-method = Autentiseringsmetode
label-tunnel-type = Tunneltype
label-cert-auth-type = Sertifikattype
label-icon-theme = Ikon-tema
label-username = Brukernavn
label-password = Passord
label-no-dns = Ikke endre DNS-konfigurasjonen
label-dns-servers = Tilleggs-DNS-servere
label-ignored-dns-servers = Ignorerte DNS-servere
label-search-domains = Tilleggssøkedomener
label-ignored-domains = Ignorerte søkedomener
label-routing-domains = Behandle mottatte søkedomener som rutedomener
label-ca-cert = Server CA-rotsertifikat
label-no-cert-check = Deaktiver alle TLS-sertifikatkontroller (USIKKERT!)
label-password-factor = Passordfaktorindeks, 1..N
label-no-keychain = Ikke lagre passord i nøkkelring
label-ike-lifetime = IPSec IKE SA-levetid, sekunder
label-ike-persist = Lagre IPSec IKE-økt og koble til automatisk
label-no-keepalive = Deaktiver IPSec keepalive-pakker
label-port-knock = Aktiver NAT-T port knocking
label-no-routing = Ignorer alle mottatte ruter
label-default-routing = Angi standardrute gjennom tunnelen
label-add-routes = Tilleggsstatiske ruter
label-ignored-routes = Ruter som skal ignoreres
label-client-cert = Klientsertifikat eller driversti (.pem, .pfx/.p12, .so)
label-cert-password = PFX-passord eller PKCS11-PIN
label-cert-id = PKCS11-sertifikatets heksadesimale ID

# Tabs and expanders
tab-general = Generelt
tab-advanced = Avansert
expand-dns = DNS
expand-routing = Ruting
expand-certificates = Sertifikater
expand-misc = Tilleggsinnstillinger

# Error messages
error-no-server = Ingen serveradresse er angitt
error-no-auth = Ingen autentiseringsmetode er valgt
error-file-not-exist = Filen finnes ikke: {$path}
error-invalid-cert-id = Sertifikat-ID er ikke i heksadesimalt format: {$id}
error-ca-root-not-exist = CA-rotsti finnes ikke: {$path}
error-validation = Valideringsfeil
error-user-input-canceled = Brukerinndata avbrutt
error-connection-canceled = Tilkobling avbrutt
error-unknown-event = Ukjent hendelse: {$event}
error-no-service-connection = Ingen tilkobling til tjenesten
error-empty-input = Inndata kan ikke være tom

# New error messages
error-invalid-object = Ugyldig objekt
error-no-connector = Ingen tunnelkobling
error-connection-cancelled = Tilkobling avbrutt
error-tunnel-disconnected = Tunnel frakoblet, siste melding: {$message}
error-unexpected-reply = Uventet svar
error-auth-failed = Autentisering mislyktes
error-no-server-name = Manglende påkrevd parameter: server-name
error-no-login-type = Manglende påkrevd parameter: login-type
error-connection-timeout = Tilkoblingstimeout
error-invalid-response = Ugyldig svar
error-cannot-send-request = Kan ikke sende forespørsel til tjenesten
error-cannot-read-reply = Kan ikke lese svar fra tjenesten
error-no-ipv4 = Ingen IPv4-adresse for {$server}
error-not-challenge-state = Ikke en utfordringsstatus
error-no-challenge = Ingen utfordring i dataene
error-endless-challenges = Uendelig løkke av brukernavnutfordringer
error-no-pkcs12 = Ingen PKCS12-sti og passord oppgitt
error-no-pkcs8 = Ingen PKCS8 PEM-sti oppgitt
error-no-pkcs11 = Ingen PKCS11 PIN oppgitt
error-no-ipsec-session = Ingen IPSEC-økt

# Placeholder texts
placeholder-domains = Domener separert med komma
placeholder-ip-addresses = IP-adresser separert med komma
placeholder-routes = Ruter separert med komma i formatet x.x.x.x/x
placeholder-certs = PEM- eller DER-filer separert med komma

# Tunnel types
tunnel-type-ipsec = IPSec
tunnel-type-ssl = SSL (foreldet)

# Certificate types
cert-type-none = Ingen
cert-type-pfx = PFX-fil
cert-type-pem = PEM-fil
cert-type-hw = Maskinvare-token

# Icon themes
icon-theme-auto = Automatisk
icon-theme-dark = Mørk
icon-theme-light = Lys

# Connection info
info-connected-since = Tilkoblet siden
info-server-name = Servernavn
info-user-name = Brukernavn
info-login-type = Innloggingstype
info-tunnel-type = Tunneltype
info-transport-type = Transporttype
info-ip-address = IP-adresse
info-dns-servers = DNS-servere
info-search-domains = Søkedomener
info-interface = Grensesnitt
info-dns-configured = DNS konfigurert
info-routing-configured = Ruting konfigurert
info-default-route = Standardrute

# Application
app-title = SNX-RS VPN-klient for Linux
app-connection-error = Tilkoblingsfeil
app-connection-success = Tilkobling vellykket

# Authentication
auth-dialog-title = VPN-autentiseringsfaktor
auth-dialog-message = Skriv inn din autentiseringsfaktor:

# Status dialog
status-dialog-title = Tilkoblingsinformasjon
status-button-copy = Kopier
status-button-settings = Innstillinger
status-button-connect = Koble til
status-button-disconnect = Koble fra

# Tray menu
tray-menu-connect = Koble til
tray-menu-disconnect = Koble fra
tray-menu-status = Tilkoblingsstatus...
tray-menu-settings = Innstillinger...
tray-menu-about = Om...
tray-menu-exit = Avslutt

# CLI Messages
cli-identity-provider-auth = For autentisering via identitetsleverandør, åpne følgende URL i nettleseren:
cli-tunnel-connected = Tunnel tilkoblet, trykk Ctrl+C for å avslutte.
cli-tunnel-disconnected = Tunnel frakoblet
cli-another-instance-running = En annen forekomst av snx-rs kjører allerede
cli-app-terminated = Applikasjon avsluttet av signal

# Connection Messages
connection-connected-to = Tilkoblet {$server}
