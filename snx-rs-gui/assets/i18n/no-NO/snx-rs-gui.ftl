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
