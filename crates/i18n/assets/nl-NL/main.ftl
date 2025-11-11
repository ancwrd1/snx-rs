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
label-username-required = Gebruikersnaam is vereist voor authenticatie
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
label-language = Taal
label-system-language = Systeemstandaard
label-username-password = Gebruikersnaam en wachtwoord
label-auto-connect = Automatisch verbinden bij opstarten
label-ip-lease-time = Aangepaste IP-leasetijd, seconden
label-disable-ipv6 = IPv6 uitschakelen wanneer de standaardroute is ingeschakeld
label-mtu = MTU
label-connection-profile = Verbindingsprofiel
label-profile-name = Profielnaam
label-confirmation = Bevestig alstublieft

# Tabs and expanders
tab-general = Algemeen
tab-advanced = Geavanceerd
expand-dns = DNS
expand-routing = Routering
expand-certificates = Certificaten
expand-misc = Overige instellingen
expand-ui = Gebruikersinterface

# Error messages
error-no-server-name = Geen serveradres opgegeven
error-no-auth = Geen authenticatiemethode geselecteerd
error-file-not-exist = Bestand bestaat niet: {$path}
error-invalid-cert-id = Certificaat-ID niet in hexadecimaal formaat: {$id}
error-ca-root-not-exist = CA-rootpad bestaat niet: {$path}
error-validation = Validatiefout
error-user-input-canceled = Gebruikersinvoer geannuleerd
error-connection-cancelled = Verbinding geannuleerd
error-unknown-event = Onbekende gebeurtenis: {$event}
error-no-service-connection = Geen verbinding met de service
error-empty-input = Invoer mag niet leeg zijn
error-invalid-object = Ongeldig object
error-no-connector = Geen tunnelconnector
error-tunnel-disconnected = Tunnel verbroken, laatste bericht: {$message}
error-unexpected-reply = Onverwachte reactie
error-auth-failed = Authenticatie mislukt
error-no-login-type = Verplichte parameter ontbreekt: login-type
error-connection-timeout = Verbindingstimeout
error-invalid-response = Ongeldige reactie!
error-request-failed-error-code = Verzoek mislukt, foutcode: {$error_code}
error-no-root-privileges = Dit programma moet als root-gebruiker worden uitgevoerd!
error-missing-required-parameters = Verplichte parameters ontbreken: servernaam en/of toegangstype!
error-missing-server-name = Verplichte parameter ontbreekt: servernaam!
error-no-connector-for-challenge-code = Geen connector voor het verzenden van de uitdagingscode!
error-probing-failed = Sondering mislukt, de server is niet bereikbaar via de NATT-poort!
error-invalid-sexpr = Ongeldige sexpr: {$value}
error-invalid-value = Ongeldige waarde
error-udp-request-failed = Fout bij het verzenden van UDP-verzoek
error-no-tty = Geen TTY aangesloten voor gebruikersinvoer
error-invalid-auth-response = Ongeldige authenticatierespons
error-invalid-client-settings = Ongeldige clientinstellingen
error-invalid-otp-reply = Ongeldige OTP-reactie
error-udp-encap-failed = Kan UDP_ENCAP socketoptie niet instellen, foutcode: {$code}
error-so-no-check-failed = Kan SO_NO_CHECK socketoptie niet instellen, foutcode: {$code}
error-keepalive-failed = Keepalive mislukt
error-receive-failed = Ontvangst mislukt
error-unknown-color-scheme = Onbekende kleurenschema-waarde
error-cannot-determine-ip = Kan standaard-IP niet bepalen
error-invalid-command = Ongeldig commando: {$command}
error-otp-browser-failed = Kan OTP niet van browser krijgen
error-invalid-operation-mode = Ongeldige bedrijfsmodus
error-invalid-tunnel-type = Ongeldig tunneltype
error-invalid-cert-type = Ongeldig certificaattype
error-invalid-icon-theme = Ongeldig pictogramthema
error-no-natt-reply = Geen NATT-reactie
error-not-implemented = Niet geïmplementeerd
error-unknown-packet-type = Onbekend pakkettype
error-no-sender = Geen afzender
error-empty-ccc-session = Lege CCC-sessie
error-identity-timeout = Timeout tijdens wachten op identiteitsreactie, is het toegangstype correct?
error-cannot-send-request = Kan verzoek niet naar service sturen
error-cannot-read-reply = Kan antwoord van service niet lezen
error-no-ipv4 = Geen IPv4-adres voor {$server}
error-not-challenge-state = Geen uitdagingsstatus
error-no-challenge = Geen uitdaging in gegevens
error-endless-challenges = Oneindige lus van gebruikersnaamuitdagingen
error-no-pkcs12 = Geen PKCS12-pad en wachtwoord opgegeven
error-no-pkcs8 = Geen PKCS8 PEM-pad opgegeven
error-no-pkcs11 = Geen PKCS11 PIN opgegeven
error-no-ipsec-session = Geen IPSEC-sessie
error-invalid-transport-type = Ongeldig transporttype

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

# Transport types
transport-type-autodetect = Automatisch detecteren
transport-type-kernel = UDP XFRM
transport-type-tcpt = TCPT TUN
transport-type-udp = UDP TUN

# Icon themes
icon-theme-autodetect = Automatisch detecteren
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

# CLI Messages
cli-identity-provider-auth = Voor authenticatie via de identiteitsprovider, open de volgende URL in uw browser:
cli-tunnel-connected = Tunnel verbonden, druk op Ctrl+C om af te sluiten.
cli-tunnel-disconnected = Tunnel verbroken
cli-another-instance-running = Er draait al een andere instantie van snx-rs
cli-app-terminated = Applicatie beëindigd door signaal

# Connection Messages
connection-connected-to = Verbonden met {$server}

# Languages
language-cs-CZ = Tsjechisch
language-da-DK = Deens
language-de-DE = Duits
language-en-US = Engels
language-es-ES = Spaans
language-fi-FI = Fins
language-fr-FR = Frans
language-it-IT = Italiaans
language-nl-NL = Nederlands
language-no-NO = Noors
language-pl-PL = Pools
language-pt-PT = Portugees
language-pt-BR = Braziliaans-Portugees
language-ru-RU = Russisch
language-sk-SK = Slowaaks
language-sv-SE = Zweeds

# Connection status messages
connection-status-disconnected = Verbinding verbroken
connection-status-connecting = Verbinding maken
connection-status-connected-since = Verbonden sinds: {$since}
connection-status-mfa-pending = Wachten op MFA: {$mfa_type}

# Login options
login-options-server-address = Serveradres
login-options-server-ip = Server-IP
login-options-client-enabled = Client ingeschakeld
login-options-supported-protocols = Ondersteunde protocollen
login-options-preferred-protocol = Voorkeursprotocol
login-options-tcpt-port = TCPT-poort
login-options-natt-port = NATT-poort
login-options-internal-ca-fingerprint = Interne CA-vingerafdruk

# Connection profiles
profile-new = Nieuw
profile-rename = Hernoemen
profile-delete = Verwijderen
profile-delete-prompt = Weet u zeker dat u het geselecteerde profiel wilt verwijderen?
profile-default-name = Standaard
profile-new-title = Nieuw verbindingsprofiel
profile-rename-title = Verbindingsprofiel hernoemen
