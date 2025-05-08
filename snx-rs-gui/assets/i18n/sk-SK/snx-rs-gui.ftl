# Dialog and buttons
dialog-title = Nastavenia VPN
button-ok = OK
button-apply = Použiť
button-cancel = Zrušiť
button-fetch-info = Získať informácie

# Labels
label-server-address = Adresa VPN servera
label-auth-method = Metóda overovania
label-tunnel-type = Typ tunela
label-cert-auth-type = Typ certifikátu
label-icon-theme = Motív ikon
label-username = Používateľské meno
label-password = Heslo
label-no-dns = Nemeniť konfiguráciu DNS resolvera
label-dns-servers = Ďalšie DNS servery
label-ignored-dns-servers = Ignorované DNS servery
label-search-domains = Ďalšie vyhľadávacie domény
label-ignored-domains = Ignorované vyhľadávacie domény
label-routing-domains = Považovať prijaté vyhľadávacie domény za smerovacie domény
label-ca-cert = Koreňové certifikáty CA servera
label-no-cert-check = Zakázať všetky kontroly TLS certifikátov (NEBEZPEČNÉ!)
label-password-factor = Index hesla, 1..N
label-no-keychain = Neukladať heslá do úložiska kľúčov
label-ike-lifetime = Životnosť IPSec IKE SA, sekundy
label-ike-persist = Uložiť IPSec IKE reláciu a automaticky sa znova pripojiť
label-no-keepalive = Zakázať pakety keepalive IPSec
label-port-knock = Povoliť NAT-T port knocking
label-no-routing = Ignorovať všetky získané trasy
label-default-routing = Nastaviť predvolenú trasu cez tunel
label-add-routes = Ďalšie statické trasy
label-ignored-routes = Trasy na ignorovanie
label-client-cert = Klientský certifikát alebo cesta k ovládaču (.pem, .pfx/.p12, .so)
label-cert-password = Heslo PFX alebo PIN PKCS11
label-cert-id = Hexadecimálne ID certifikátu PKCS11

# Tabs and expanders
tab-general = Všeobecné
tab-advanced = Rozšírené
expand-dns = DNS
expand-routing = Smerovanie
expand-certificates = Certifikáty
expand-misc = Ďalšie nastavenia

# Error messages
error-no-server = Nie je zadaná adresa servera
error-no-auth = Nie je vybraná metóda overovania
error-file-not-exist = Súbor neexistuje: {$path}
error-invalid-cert-id = ID certifikátu nie je v hexadecimálnom formáte: {$id}
error-ca-root-not-exist = Cesta ku koreňovému certifikátu CA neexistuje: {$path}
error-validation = Chyba overenia

# Placeholder texts
placeholder-domains = Domény oddelené čiarkami
placeholder-ip-addresses = IP adresy oddelené čiarkami
placeholder-routes = Trasy oddelené čiarkami vo formáte x.x.x.x/x
placeholder-certs = Súbory PEM alebo DER oddelené čiarkami

# Tunnel types
tunnel-type-ipsec = IPSec
tunnel-type-ssl = SSL (zastaralé)

# Certificate types
cert-type-none = Žiadny
cert-type-pfx = Súbor PFX
cert-type-pem = Súbor PEM
cert-type-hw = Hardvérový token

# Icon themes
icon-theme-auto = Automaticky
icon-theme-dark = Tmavý
icon-theme-light = Svetlý

# Connection info
info-connected-since = Pripojené od
info-server-name = Názov servera
info-user-name = Používateľské meno
info-login-type = Typ prihlásenia
info-tunnel-type = Typ tunela
info-transport-type = Typ transportu
info-ip-address = IP adresa
info-dns-servers = DNS servery
info-search-domains = Vyhľadávacie domény
info-interface = Rozhranie
info-dns-configured = DNS nakonfigurované
info-routing-configured = Smerovanie nakonfigurované
info-default-route = Predvolená trasa

# Application
app-title = SNX-RS VPN klient pre Linux
app-connection-error = Chyba pripojenia
app-connection-success = Pripojenie úspešné

# Authentication
auth-dialog-title = VPN autentifikačný faktor
auth-dialog-message = Zadajte váš autentifikačný faktor:

# Status dialog
status-dialog-title = Informácie o pripojení
status-button-copy = Kopírovať
status-button-settings = Nastavenia
status-button-connect = Pripojiť
status-button-disconnect = Odpojiť

# Tray menu
tray-menu-connect = Pripojiť
tray-menu-disconnect = Odpojiť
tray-menu-status = Stav pripojenia...
tray-menu-settings = Nastavenia...
tray-menu-about = O aplikácii...
tray-menu-exit = Ukončiť
