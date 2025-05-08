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