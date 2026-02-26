# Dialog and buttons
dialog-title = VPN-inställningar
button-ok = OK
button-apply = Tillämpa
button-cancel = Avbryt
button-fetch-info = Hämta information

# Labels
label-server-address = VPN-serveradress
label-auth-method = Autentiseringsmetod
label-tunnel-type = Tunneltyp
label-cert-auth-type = Certifikattyp
label-icon-theme = Ikon-tema
label-username = Användarnamn
label-username-required = Användarnamn krävs för autentisering
label-password = Lösenord
label-no-dns = Ändra inte DNS-konfigurationen
label-dns-servers = Ytterligare DNS-servrar
label-ignored-dns-servers = Ignorerade DNS-servrar
label-search-domains = Ytterligare sökdomäner
label-ignored-domains = Ignorerade sökdomäner
label-routing-domains = Behandla mottagna sökdomäner som routningsdomäner
label-ca-cert = Server CA-rotcertifikat
label-no-cert-check = Inaktivera alla TLS-certifikatkontroller (OSÄKERT!)
label-password-factor = Lösenordsfaktorindex, 1..N
label-no-keychain = Spara inte lösenord i nyckelringen
label-ike-lifetime = IPSec IKE SA-livstid, sekunder
label-ike-persist = Spara IPSec IKE-session och återanslut automatiskt
label-no-keepalive = Inaktivera IPSec keepalive-paket
label-port-knock = Aktivera NAT-T port knocking
label-no-routing = Ignorera alla erhållna rutter
label-default-routing = Ange standardrutt genom tunneln
label-add-routes = Ytterligare statiska rutter
label-ignored-routes = Rutter att ignorera
label-client-cert = Klientcertifikat eller drivrutinspath (.pem, .pfx/.p12, .so)
label-cert-password = PFX-lösenord eller PKCS11-PIN
label-cert-id = PKCS11-certifikatets hexadecimella ID
label-language = Språk
label-system-language = Systemstandard
label-username-password = Användarnamn och lösenord
label-auto-connect = Anslut automatiskt vid start
label-ip-lease-time = Anpassad IP-leasetid, sekunder
label-disable-ipv6 = Inaktivera IPv6 när standardrutt är aktiverad
label-mtu = MTU
label-connection-profile = Anslutningsprofil
label-profile-name = Profilnamn
label-confirmation = Vänligen bekräfta
label-mobile-access = Mobilåtkomst
label-machine-cert-auth = Maskinscertifikatautentisering
label-browse = Bläddra...
label-keychain-files = Nyckelringsfiler
label-all-files = Alla filer
label-cancel = Avbryt
label-open = Öppna
label-select-file = Välj en fil
label-ca-cert-files = X.509-certifikat

# Tabs and expanders
tab-general = Allmänt
tab-advanced = Avancerat
expand-dns = DNS
expand-routing = Routning
expand-certificates = Certifikat
expand-misc = Ytterligare inställningar
expand-ui = Användargränssnitt

# Error messages
error-no-server-name = Ingen serveradress angiven
error-no-auth = Ingen autentiseringsmetod vald
error-file-not-exist = Filen finns inte: {$path}
error-invalid-cert-id = Certifikat-ID är inte i hexadecimalt format: {$id}
error-ca-root-not-exist = CA-rotpath finns inte: {$path}
error-validation = Valideringsfel
error-user-input-canceled = Användarinput avbruten
error-connection-cancelled = Anslutning avbruten
error-unknown-event = Okänd händelse: {$event}
error-no-service-connection = Ingen anslutning till tjänsten
error-empty-input = Input kan inte vara tom
error-invalid-object = Ogiltigt objekt
error-no-connector = Ingen tunnelanslutning
error-tunnel-disconnected = Tunnel frånkopplad, sista meddelande: {$message}
error-unexpected-reply = Oväntat svar
error-auth-failed = Autentisering misslyckades
error-no-login-type = Saknad obligatorisk parameter: login-type
error-connection-timeout = Anslutningstimeout
error-invalid-response = Ogiltigt svar!
error-cannot-acquire-access-cookie = Kan inte hämta åtkomstcookie!
error-cannot-send-request = Kan inte skicka förfrågan till tjänsten
error-cannot-read-reply = Kan inte läsa svar från tjänsten
error-no-ipv4 = Ingen IPv4-adress för {$server}
error-not-challenge-state = Inte ett utmaningstillstånd
error-no-challenge = Ingen utmaning i data
error-endless-challenges = Oändlig loop av användarnamnsutmaningar
error-no-pkcs12 = Ingen PKCS12-sökväg och lösenord angivna
error-no-pkcs8 = Ingen PKCS8 PEM-sökväg angiven
error-no-pkcs11 = Ingen PKCS11 PIN angiven
error-no-ipsec-session = Ingen IPSEC-session
error-request-failed-error-code = Förfrågan misslyckades, felkod: {$error_code}
error-no-root-privileges = Detta program måste köras som root-användare!
error-missing-required-parameters = Saknade obligatoriska parametrar: servernamn och/eller åtkomsttyp!
error-missing-server-name = Saknad obligatorisk parameter: servernamn!
error-no-connector-for-challenge-code = Ingen anslutning för att skicka utmaningskod!
error-probing-failed = Sondering misslyckades, servern är inte tillgänglig via NATT-porten!
error-invalid-sexpr = Ogiltig sexpr: {$value}
error-invalid-value = Ogiltigt värde
error-udp-request-failed = Fel vid sändning av UDP-förfrågan
error-no-tty = Ingen TTY ansluten för användarinput
error-invalid-auth-response = Ogiltigt autentiseringssvar
error-invalid-client-settings = Ogiltiga klientinställningar
error-invalid-cert-response = Ogiltigt certifikatsvar
error-certificate-enrollment-failed = Certifikatregistrering misslyckades, felkod: {$code}
error-missing-cert-path = Sökväg till PKCS12-fil saknas!
error-missing-cert-password = PKCS12-lösenord saknas!
error-missing-reg-key = Registreringsnyckel saknas!
error-invalid-otp-reply = Ogiltigt OTP-svar
error-udp-encap-failed = Kan inte ställa in UDP_ENCAP socket-option, felkod: {$code}
error-so-no-check-failed = Kan inte ställa in SO_NO_CHECK socket-option, felkod: {$code}
error-keepalive-failed = Keepalive misslyckades
error-receive-failed = Mottagning misslyckades
error-unknown-color-scheme = Okänt färgschema-värde
error-cannot-determine-ip = Kan inte bestämma standard-IP
error-invalid-command = Ogiltigt kommando: {$command}
error-otp-browser-failed = Kan inte få OTP från webbläsaren
error-invalid-operation-mode = Ogiltigt driftläge
error-invalid-tunnel-type = Ogiltig tunneltyp
error-invalid-cert-type = Ogiltig certifikattyp
error-invalid-icon-theme = Ogiltigt ikon-tema
error-no-natt-reply = Inget NATT-svar
error-not-implemented = Inte implementerat
error-unknown-packet-type = Okänd pakettyp
error-no-sender = Ingen avsändare
error-empty-ccc-session = Tom CCC-session
error-identity-timeout = Timeout vid väntan på identitetssvar, är åtkomsttypen korrekt?
error-invalid-transport-type = Ogiltig transporttyp
error-certificate-verify-failed = TLS-certifikatvalidering misslyckades. Serverns certifikat är ogiltigt, utgånget eller inte betrott.

# Placeholder texts
placeholder-domains = Domäner separerade med kommatecken
placeholder-ip-addresses = IP-adresser separerade med kommatecken
placeholder-routes = Rutter separerade med kommatecken i formatet x.x.x.x/x
placeholder-certs = PEM- eller DER-filer separerade med kommatecken

# Tunnel types
tunnel-type-ipsec = IPSec
tunnel-type-ssl = SSL (föråldrat)

# Certificate types
cert-type-none = Ingen
cert-type-pfx = PFX-fil
cert-type-pem = PEM-fil
cert-type-hw = Hårdvarutoken

# Transport types
transport-type-autodetect = Automatisk identifiering
transport-type-kernel = UDP XFRM
transport-type-tcpt = TCPT TUN
transport-type-udp = UDP TUN

# Icon themes
icon-theme-autodetect = Automatisk identifiering
icon-theme-dark = Mörkt
icon-theme-light = Ljust

# Connection info
info-connected-since = Ansluten sedan
info-server-name = Servernamn
info-user-name = Användarnamn
info-login-type = Inloggningstyp
info-tunnel-type = Tunneltyp
info-transport-type = Transporttyp
info-ip-address = IP-adress
info-dns-servers = DNS-servrar
info-search-domains = Sökdomäner
info-interface = Gränssnitt
info-dns-configured = DNS konfigurerad
info-routing-configured = Routning konfigurerad
info-default-route = Standardrutt

# Application
app-title = SNX-RS VPN-klient för Linux
app-connection-error = Anslutningsfel
app-connection-success = Anslutning lyckades

# Authentication
auth-dialog-title = VPN-autentiseringsfaktor
auth-dialog-message = Ange din autentiseringsfaktor:

# Status dialog
status-dialog-title = Anslutningsinformation
status-button-copy = Kopiera
status-button-settings = Inställningar
status-button-connect = Anslut
status-button-disconnect = Koppla från

# Tray menu
tray-menu-connect = Anslut
tray-menu-disconnect = Koppla från
tray-menu-status = Anslutningsstatus...
tray-menu-settings = Inställningar...
tray-menu-about = Om...
tray-menu-exit = Avsluta

# CLI Messages
cli-identity-provider-auth = För autentisering via identitetsleverantören, öppna följande URL i din webbläsare:
cli-tunnel-connected = Tunnel ansluten, tryck Ctrl+C för att avsluta.
cli-tunnel-disconnected = Tunnel frånkopplad
cli-another-instance-running = En annan instans av snx-rs körs redan
cli-app-terminated = Applikation avslutad av signal
cli-mobile-access-auth = För autentisering av mobil åtkomst, logga in via följande URL, leta sedan upp ett användarlösenord i hex-format i sidans HTML-källkod och skriv in det här:
cli-certificate-enrolled = Certifikatet har registrerats.

# Connection Messages
connection-connected-to = Ansluten till {$server}

# Languages
language-cs-CZ = Tjeckiska
language-da-DK = Danska
language-de-DE = Tyska
language-en-US = Engelska
language-es-ES = Spanska
language-fi-FI = Finska
language-fr-FR = Franska
language-hr-HR = Kroatiska
language-it-IT = Italienska
language-nl-NL = Nederländska
language-no-NO = Norska
language-pl-PL = Polska
language-pt-PT = Portugisiska
language-pt-BR = Brasiliansk Portugisiska
language-ru-RU = Ryska
language-sk-SK = Slovakiska
language-sv-SE = Svenska

# Connection status messages
connection-status-disconnected = Frånkopplad
connection-status-connecting = Ansluter
connection-status-connected-since = Ansluten sedan: {$since}
connection-status-mfa-pending = Väntar på MFA: {$mfa_type}

# Login options
login-options-server-address = Serveradress
login-options-server-ip = Server-IP
login-options-client-enabled = Klient aktiverad
login-options-supported-protocols = Protokoll som stöds
login-options-preferred-protocol = Föredragen protokoll
login-options-tcpt-port = TCPT-port
login-options-natt-port = NATT-port
login-options-internal-ca-fingerprint = Internt CA-fingeravtryck

# Connection profiles
profile-new = Ny
profile-rename = Byt namn
profile-delete = Ta bort
profile-delete-prompt = Är du säker på att du vill ta bort den valda profilen?
profile-default-name = Standard
profile-new-title = Ny anslutningsprofil
profile-rename-title = Byt namn på anslutningsprofil
