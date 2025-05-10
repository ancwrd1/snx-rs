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

# Tabs and expanders
tab-general = Allmänt
tab-advanced = Avancerat
expand-dns = DNS
expand-routing = Routning
expand-certificates = Certifikat
expand-misc = Ytterligare inställningar

# Error messages
error-no-server = Ingen serveradress angiven
error-no-auth = Ingen autentiseringsmetod vald
error-file-not-exist = Filen finns inte: {$path}
error-invalid-cert-id = Certifikat-ID är inte i hexadecimalt format: {$id}
error-ca-root-not-exist = CA-rotpath finns inte: {$path}
error-validation = Valideringsfel
error-user-input-canceled = Användarinput avbruten
error-connection-canceled = Anslutning avbruten
error-unknown-event = Okänd händelse: {$event}
error-no-service-connection = Ingen anslutning till tjänsten
error-empty-input = Input kan inte vara tom

# New error messages
error-invalid-object = Ogiltigt objekt
error-no-connector = Ingen tunnelanslutning
error-connection-cancelled = Anslutning avbruten
error-tunnel-disconnected = Tunnel frånkopplad, sista meddelande: {$message}
error-unexpected-reply = Oväntat svar
error-auth-failed = Autentisering misslyckades
error-no-server-name = Saknad obligatorisk parameter: server-name
error-no-login-type = Saknad obligatorisk parameter: login-type
error-connection-timeout = Anslutningstimeout
error-invalid-response = Ogiltigt svar
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

# Icon themes
icon-theme-auto = Automatiskt
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
