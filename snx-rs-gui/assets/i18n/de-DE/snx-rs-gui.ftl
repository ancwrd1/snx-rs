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