# Dialog and buttons
dialog-title = Nastavení VPN
button-ok = OK
button-apply = Použít
button-cancel = Zrušit
button-fetch-info = Získat informace

# Labels
label-server-address = Adresa VPN serveru
label-auth-method = Metoda ověřování
label-tunnel-type = Typ tunelu
label-cert-auth-type = Typ certifikátu
label-icon-theme = Motiv ikon
label-username = Uživatelské jméno
label-password = Heslo
label-no-dns = Neměnit konfiguraci DNS resolveru
label-dns-servers = Další DNS servery
label-ignored-dns-servers = Ignorované DNS servery
label-search-domains = Další vyhledávací domény
label-ignored-domains = Ignorované vyhledávací domény
label-routing-domains = Považovat přijaté vyhledávací domény za směrovací domény
label-ca-cert = Kořenové certifikáty CA serveru
label-no-cert-check = Zakázat všechny kontroly TLS certifikátů (NEBEZPEČNÉ!)
label-password-factor = Index hesla, 1..N
label-no-keychain = Neukládat hesla do úložiště klíčů
label-ike-lifetime = Životnost IPSec IKE SA, sekundy
label-ike-persist = Uložit IPSec IKE relaci a automaticky se znovu připojit
label-no-keepalive = Zakázat pakety keepalive IPSec
label-port-knock = Povolit NAT-T port knocking
label-no-routing = Ignorovat všechny získané trasy
label-default-routing = Nastavit výchozí trasu přes tunel
label-add-routes = Další statické trasy
label-ignored-routes = Trasy k ignorování
label-client-cert = Klientský certifikát nebo cesta k ovladači (.pem, .pfx/.p12, .so)
label-cert-password = Heslo PFX nebo PIN PKCS11
label-cert-id = Hexadecimální ID certifikátu PKCS11

# Tabs and expanders
tab-general = Obecné
tab-advanced = Rozšířené
expand-dns = DNS
expand-routing = Směrování
expand-certificates = Certifikáty
expand-misc = Další nastavení

# Error messages
error-no-server = Není zadána adresa serveru
error-no-auth = Není vybrána metoda ověřování
error-file-not-exist = Soubor neexistuje: {$path}
error-invalid-cert-id = ID certifikátu není v hexadecimálním formátu: {$id}
error-ca-root-not-exist = Cesta ke kořenovému certifikátu CA neexistuje: {$path}
error-validation = Chyba ověření

# Placeholder texts
placeholder-domains = Domény oddělené čárkami
placeholder-ip-addresses = IP adresy oddělené čárkami
placeholder-routes = Trasy oddělené čárkami ve formátu x.x.x.x/x
placeholder-certs = Soubory PEM nebo DER oddělené čárkami

# Tunnel types
tunnel-type-ipsec = IPSec
tunnel-type-ssl = SSL (zastaralé)

# Certificate types
cert-type-none = Žádný
cert-type-pfx = Soubor PFX
cert-type-pem = Soubor PEM
cert-type-hw = Hardwarový token

# Icon themes
icon-theme-auto = Automaticky
icon-theme-dark = Tmavý
icon-theme-light = Světlý