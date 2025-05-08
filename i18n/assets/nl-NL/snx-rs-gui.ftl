# Dialog and buttons
dialog-title = VPN-instellingen
button-ok = OK
button-apply = Toepassen
button-cancel = Annuleren
button-fetch-info = Informatie ophalen

# Labels
label-server-address = VPN-serveradres
label-auth-method = Authenticatiemethode
label-tunnel-type = Tunneltype
label-cert-auth-type = Certificaattype
label-icon-theme = Pictogramthema
label-username = Gebruikersnaam
label-password = Wachtwoord
label-no-dns = DNS-resolverconfiguratie niet wijzigen
label-dns-servers = Extra DNS-servers
label-ignored-dns-servers = Genegeerde DNS-servers
label-search-domains = Extra zoekdomeinen
label-ignored-domains = Genegeerde zoekdomeinen
label-routing-domains = Ontvangen zoekdomeinen als routeringsdomeinen behandelen
label-ca-cert = Server CA-basiscertificaten
label-no-cert-check = Alle TLS-certificaatcontroles uitschakelen (ONVEILIG!)
label-password-factor = Index van wachtwoordfactor, 1..N
label-no-keychain = Wachtwoorden niet in sleutelhanger opslaan
label-ike-lifetime = IPSec IKE SA-levensduur, seconden
label-ike-persist = IPSec IKE-sessie opslaan en automatisch opnieuw verbinden
label-no-keepalive = IPSec keepalive-pakketten uitschakelen
label-port-knock = NAT-T port knocking inschakelen
label-no-routing = Alle verkregen routes negeren
label-default-routing = Standaardroute via tunnel instellen
label-add-routes = Extra statische routes
label-ignored-routes = Te negeren routes
label-client-cert = Clientcertificaat of stuurprogrammapad (.pem, .pfx/.p12, .so)
label-cert-password = PFX-wachtwoord of PKCS11-pin
label-cert-id = Hexadecimale ID van PKCS11-certificaat

# Tabs and expanders
tab-general = Algemeen
tab-advanced = Geavanceerd
expand-dns = DNS
expand-routing = Routering
expand-certificates = Certificaten
expand-misc = Overige instellingen

# Error messages
error-no-server = Geen serveradres opgegeven
error-no-auth = Geen authenticatiemethode geselecteerd
error-file-not-exist = Bestand bestaat niet: {$path}
error-invalid-cert-id = Certificaat-ID niet in hexadecimaal formaat: {$id}
error-ca-root-not-exist = CA-rootpad bestaat niet: {$path}
error-validation = Validatiefout

# Placeholder texts
placeholder-domains = Door komma's gescheiden domeinen
placeholder-ip-addresses = Door komma's gescheiden IP-adressen
placeholder-routes = Door komma's gescheiden x.x.x.x/x
placeholder-certs = Door komma's gescheiden PEM- of DER-bestanden

# Tunnel types
tunnel-type-ipsec = IPSec
tunnel-type-ssl = SSL (verouderd)

# Certificate types
cert-type-none = Geen
cert-type-pfx = PFX-bestand
cert-type-pem = PEM-bestand
cert-type-hw = Hardware-token

# Icon themes
icon-theme-auto = Auto
icon-theme-dark = Donker
icon-theme-light = Licht

# Connection info
info-connected-since = Verbonden sinds
info-server-name = Servernaam
info-user-name = Gebruikersnaam
info-login-type = Inlogtype
info-tunnel-type = Tunneltype
info-transport-type = Transporttype
info-ip-address = IP-adres
info-dns-servers = DNS-servers
info-search-domains = Zoekdomeinen
info-interface = Interface
info-dns-configured = DNS geconfigureerd
info-routing-configured = Routering geconfigureerd
info-default-route = Standaardroute

# Application
app-title = SNX-RS VPN-client voor Linux
app-connection-error = Verbindingsfout
app-connection-success = Verbinding geslaagd

# Authentication
auth-dialog-title = VPN-authenticatiefactor
auth-dialog-message = Voer uw authenticatiefactor in:

# Status dialog
status-dialog-title = Verbindingsinformatie
status-button-copy = Kopiëren
status-button-settings = Instellingen
status-button-connect = Verbinden
status-button-disconnect = Verbreken

# Tray menu
tray-menu-connect = Verbinden
tray-menu-disconnect = Verbreken
tray-menu-status = Verbindingsstatus...
tray-menu-settings = Instellingen...
tray-menu-about = Over...
tray-menu-exit = Afsluiten
