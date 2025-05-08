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

# Tabs and expanders
tab-general = Allgemein
tab-advanced = Erweitert
expand-dns = DNS
expand-routing = Routing
expand-certificates = Zertifikate
expand-misc = Weitere Einstellungen

# Error messages
error-no-server = Keine Serveradresse angegeben
error-no-auth = Keine Authentifizierungsmethode ausgewählt
error-file-not-exist = Datei existiert nicht: {$path}
error-invalid-cert-id = Zertifikats-ID nicht im Hex-Format: {$id}
error-ca-root-not-exist = CA-Stammpfad existiert nicht: {$path}
error-validation = Validierungsfehler
error-user-input-canceled = Benutzereingabe abgebrochen
error-connection-canceled = Verbindung abgebrochen
error-unknown-event = Unbekanntes Ereignis: {$event}
error-no-service-connection = Keine Verbindung zum Dienst
error-empty-input = Eingabe darf nicht leer sein

# New error messages
error-invalid-object = Ungültiges Objekt
error-no-connector = Kein Tunnel-Connector
error-connection-cancelled = Verbindung abgebrochen
error-tunnel-disconnected = Tunnel getrennt, letzte Nachricht: {$message}
error-unexpected-reply = Unerwartete Antwort
error-auth-failed = Authentifizierung fehlgeschlagen
error-no-server-name = Erforderlicher Parameter fehlt: server-name
error-no-login-type = Erforderlicher Parameter fehlt: login-type
error-connection-timeout = Verbindungszeitüberschreitung
error-invalid-response = Ungültige Antwort
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

# Icon themes
icon-theme-auto = Auto
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
