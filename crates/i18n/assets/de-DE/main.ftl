# Dialog and buttons
dialog-title = VPN-Einstellungen
button-ok = OK
button-apply = Anwenden
button-cancel = Abbrechen
button-fetch-info = Informationen abrufen

# Labels
label-server-address = VPN-Serveradresse
label-auth-method = Authentifizierungsmethode
label-tunnel-type = Tunneltyp
label-cert-auth-type = Zertifikatstyp
label-icon-theme = Symbolthema
label-username = Benutzername
label-username-required = Benutzername ist für die Authentifizierung erforderlich
label-password = Passwort
label-no-dns = DNS-Resolver-Konfiguration nicht ändern
label-dns-servers = Zusätzliche DNS-Server
label-ignored-dns-servers = Ignorierte DNS-Server
label-search-domains = Zusätzliche Suchdomänen
label-ignored-domains = Ignorierte Suchdomänen
label-routing-domains = Erhaltene Suchdomänen als Routing-Domänen behandeln
label-ca-cert = Server-CA-Stammzertifikate
label-no-cert-check = Alle TLS-Zertifikatsprüfungen deaktivieren (UNSICHER!)
label-password-factor = Index des Passwortfaktors, 1..N
label-no-keychain = Passwörter nicht im Schlüsselbund speichern
label-ike-lifetime = IPSec IKE SA-Lebensdauer, Sekunden
label-ike-persist = IPSec IKE-Sitzung speichern und automatisch neu verbinden
label-no-keepalive = IPSec Keepalive-Pakete deaktivieren
label-port-knock = NAT-T Port Knocking aktivieren
label-no-routing = Alle erworbenen Routen ignorieren
label-default-routing = Standardroute über den Tunnel setzen
label-add-routes = Zusätzliche statische Routen
label-ignored-routes = Zu ignorierende Routen
label-client-cert = Client-Zertifikat oder Treiberpfad (.pem, .pfx/.p12, .so)
label-cert-password = PFX-Passwort oder PKCS11-PIN
label-cert-id = Hex-ID des PKCS11-Zertifikats
label-language = Sprache
label-system-language = Systemstandard
label-username-password = Benutzername und Passwort
label-auto-connect = Automatisch beim Start verbinden
label-ip-lease-time = Benutzerdefinierte IP-Lease-Zeit, Sekunden
label-disable-ipv6 = IPv6 deaktivieren, wenn die Standardroute aktiviert ist
label-mtu = MTU

# Tabs and expanders
tab-general = Allgemein
tab-advanced = Erweitert
expand-dns = DNS
expand-routing = Routing
expand-certificates = Zertifikate
expand-misc = Weitere Einstellungen
expand-ui = Benutzeroberfläche

# Error messages
error-no-server-name = Keine Serveradresse angegeben
error-no-auth = Keine Authentifizierungsmethode ausgewählt
error-file-not-exist = Datei existiert nicht: {$path}
error-invalid-cert-id = Zertifikats-ID nicht im Hex-Format: {$id}
error-ca-root-not-exist = CA-Stammpfad existiert nicht: {$path}
error-validation = Validierungsfehler
error-user-input-canceled = Benutzereingabe abgebrochen
error-connection-cancelled = Verbindung abgebrochen
error-unknown-event = Unbekanntes Ereignis: {$event}
error-no-service-connection = Keine Verbindung zum Dienst
error-empty-input = Eingabe darf nicht leer sein
error-invalid-object = Ungültiges Objekt
error-no-connector = Kein Tunnel-Connector
error-tunnel-disconnected = Tunnel getrennt, letzte Nachricht: {$message}
error-unexpected-reply = Unerwartete Antwort
error-auth-failed = Authentifizierung fehlgeschlagen
error-no-login-type = Erforderlicher Parameter fehlt: login-type
error-connection-timeout = Verbindungszeitüberschreitung
error-invalid-response = Ungültige Antwort!
error-cannot-send-request = Anfrage kann nicht an den Dienst gesendet werden
error-cannot-read-reply = Antwort vom Dienst kann nicht gelesen werden
error-no-ipv4 = Keine IPv4-Adresse für {$server}
error-not-challenge-state = Kein Challenge-Status
error-no-challenge = Keine Challenge in den Daten
error-endless-challenges = Endlosschleife von Benutzernamen-Challenges
error-no-pkcs12 = Kein PKCS12-Pfad und Passwort angegeben
error-no-pkcs8 = Kein PKCS8 PEM-Pfad angegeben
error-no-pkcs11 = Kein PKCS11-PIN angegeben
error-no-ipsec-session = Keine IPSEC-Sitzung
error-request-failed-error-code = Anfrage fehlgeschlagen, Fehlercode: {$error_code}
error-no-root-privileges = Dieses Programm muss als Root-Benutzer ausgeführt werden!
error-missing-required-parameters = Erforderliche Parameter fehlen: Servername und/oder Anmeldetyp!
error-missing-server-name = Erforderlicher Parameter fehlt: Servername!
error-no-connector-for-challenge-code = Kein Connector zum Senden des Challenge-Codes!
error-probing-failed = Prüfung fehlgeschlagen, Server ist über NATT-Port nicht erreichbar!
error-invalid-sexpr = Ungültiger sexpr: {$value}
error-invalid-value = Ungültiger Wert
error-udp-request-failed = Fehler beim Senden der UDP-Anfrage
error-no-tty = Kein angeschlossenes TTY für Benutzereingabe
error-invalid-auth-response = Ungültige Authentifizierungsantwort
error-invalid-client-settings = Ungültige Client-Einstellungsantwort
error-invalid-otp-reply = Ungültige OTP-Antwort
error-udp-encap-failed = Socket-Option UDP_ENCAP konnte nicht gesetzt werden, Fehlercode: {$code}
error-so-no-check-failed = Socket-Option SO_NO_CHECK konnte nicht gesetzt werden, Fehlercode: {$code}
error-keepalive-failed = Keepalive fehlgeschlagen
error-receive-failed = Empfang fehlgeschlagen
error-unknown-color-scheme = Unbekannter Farb-Schema-Wert
error-cannot-determine-ip = Standard-IP kann nicht bestimmt werden
error-invalid-command = Ungültiger Befehl: {$command}
error-otp-browser-failed = OTP konnte nicht aus dem Browser abgerufen werden
error-invalid-operation-mode = Ungültiger Betriebsmodus
error-invalid-tunnel-type = Ungültiger Tunneltyp
error-invalid-cert-type = Ungültiger Zertifikatstyp
error-invalid-icon-theme = Ungültiges Symbolthema
error-no-natt-reply = Keine NATT-Antwort
error-not-implemented = Nicht implementiert
error-unknown-packet-type = Unbekannter Pakettyp
error-no-sender = Kein Absender
error-empty-ccc-session = Leere CCC-Sitzung
error-identity-timeout = Timeout beim Warten auf Identitätsantwort, ist der Anmeldetyp korrekt?
error-invalid-transport-type = Ungültiger Transporttyp

# Placeholder texts
placeholder-domains = Durch Komma getrennte Domänen
placeholder-ip-addresses = Durch Komma getrennte IP-Adressen
placeholder-routes = Durch Komma getrennte x.x.x.x/x
placeholder-certs = Durch Komma getrennte PEM- oder DER-Dateien

# Tunnel types
tunnel-type-ipsec = IPSec
tunnel-type-ssl = SSL (veraltet)

# Certificate types
cert-type-none = Keine
cert-type-pfx = PFX-Datei
cert-type-pem = PEM-Datei
cert-type-hw = Hardware-Token

# Transport types
transport-type-autodetect = Automatisch erkennen
transport-type-kernel = Kernel XFRM
transport-type-tcpt = TCPT TUN
transport-type-udp = UDP TUN

# Icon themes
icon-theme-autodetect = Automatisch
icon-theme-dark = Dunkel
icon-theme-light = Hell

# Application
app-title = SNX-RS VPN-Client für Linux
app-connection-error = Verbindungsfehler
app-connection-success = Verbindung erfolgreich

# Authentication
auth-dialog-title = VPN-Authentifizierungsfaktor
auth-dialog-message = Bitte geben Sie Ihren Authentifizierungsfaktor ein:

# Status dialog
status-dialog-title = Verbindungsinformationen
status-button-copy = Kopieren
status-button-settings = Einstellungen
status-button-connect = Verbinden
status-button-disconnect = Trennen

# Tray menu
tray-menu-connect = Verbinden
tray-menu-disconnect = Trennen
tray-menu-status = Verbindungsstatus...
tray-menu-settings = Einstellungen...
tray-menu-about = Über...
tray-menu-exit = Beenden

# Connection info
info-connected-since = Verbunden seit
info-server-name = Servername
info-user-name = Benutzername
info-login-type = Anmeldetyp
info-tunnel-type = Tunneltyp
info-transport-type = Transporttyp
info-ip-address = IP-Adresse
info-dns-servers = DNS-Server
info-search-domains = Suchdomänen
info-interface = Schnittstelle
info-dns-configured = DNS konfiguriert
info-routing-configured = Routing konfiguriert
info-default-route = Standardroute

# CLI Messages
cli-identity-provider-auth = Für die Authentifizierung über den Identitätsanbieter öffnen Sie die folgende URL in Ihrem Browser:
cli-tunnel-connected = Tunnel verbunden, drücken Sie Strg+C zum Beenden.
cli-tunnel-disconnected = Tunnel getrennt
cli-another-instance-running = Eine andere Instanz von snx-rs läuft bereits
cli-app-terminated = Anwendung durch Signal beendet

# Connection Messages
connection-connected-to = Verbunden mit {$server}

# Languages
language-cs-CZ = Tschechisch
language-da-DK = Dänisch
language-de-DE = Deutsch
language-en-US = Englisch
language-es-ES = Spanisch
language-fi-FI = Finnisch
language-fr-FR = Französisch
language-it-IT = Italienisch
language-nl-NL = Niederländisch
language-no-NO = Norwegisch
language-pl-PL = Polnisch
language-pt-PT = Portugiesisch
language-pt-BR = Brasilianisches Portugiesisch
language-ru-RU = Russisch
language-sk-SK = Slowakisch
language-sv-SE = Schwedisch

# Connection status messages
connection-status-disconnected = Getrennt
connection-status-connecting = Verbindung wird hergestellt
connection-status-connected-since = Verbunden seit: {$since}
connection-status-mfa-pending = Warte auf MFA: {$mfa_type}

# Login options
login-options-server-address = Serveradresse
login-options-server-ip = Server-IP
login-options-client-enabled = Client aktiviert
login-options-supported-protocols = Unterstützte Protokolle
login-options-preferred-protocol = Bevorzugtes Protokoll
login-options-tcpt-port = TCPT-Port
login-options-natt-port = NATT-Port
login-options-internal-ca-fingerprint = Interner CA-Fingerabdruck
