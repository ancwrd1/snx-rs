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
label-language = Jazyk
label-system-language = Systémové predvolené

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
error-user-input-canceled = Vstup používateľa zrušený
error-connection-canceled = Pripojenie zrušené
error-unknown-event = Neznáma udalosť: {$event}
error-no-service-connection = Žiadne pripojenie k službe
error-empty-input = Vstup nemôže byť prázdny

# New error messages
error-invalid-object = Neplatný objekt
error-no-connector = Žiadny konektor tunela
error-connection-cancelled = Pripojenie zrušené
error-tunnel-disconnected = Tunel odpojený, posledná správa: {$message}
error-unexpected-reply = Neočakávaná odpoveď
error-auth-failed = Overenie zlyhalo
error-no-server-name = Chýba povinný parameter: server-name
error-no-login-type = Chýba povinný parameter: login-type
error-connection-timeout = Časový limit pripojenia
error-invalid-response = Neplatná odpoveď
error-cannot-send-request = Nie je možné odoslať požiadavku na službu
error-cannot-read-reply = Nie je možné prečítať odpoveď zo služby
error-no-ipv4 = Žiadna IPv4 adresa pre {$server}
error-not-challenge-state = Nie je stav výzvy
error-no-challenge = Žiadna výzva v dátach
error-endless-challenges = Nekonečná slučka výziev používateľského mena
error-no-pkcs12 = Žiadna cesta PKCS12 a heslo nie sú poskytnuté
error-no-pkcs8 = Žiadna cesta PKCS8 PEM nie je poskytnutá
error-no-pkcs11 = Žiadny PIN PKCS11 nie je poskytnutý
error-no-ipsec-session = Žiadna IPSEC relácia

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

# CLI Messages
cli-identity-provider-auth = Pre autentifikáciu cez poskytovateľa identity otvorte nasledujúcu URL adresu vo vašom prehliadači:
cli-tunnel-connected = Tunel pripojený, stlačte Ctrl+C pre ukončenie.
cli-tunnel-disconnected = Tunel odpojený
cli-another-instance-running = Iná inštancia snx-rs už beží
cli-app-terminated = Aplikácia ukončená signálom

# Connection Messages
connection-connected-to = Pripojené k {$server}

# Languages
language-cs-CZ = Čeština
language-da-DK = Dánčina
language-de-DE = Nemčina
language-en-US = Angličtina
language-es-ES = Španielčina
language-fi-FI = Fínčina
language-fr-FR = Francúzština
language-it-IT = Taliančina
language-nl-NL = Holandčina
language-no-NO = Nórčina
language-pl-PL = Poľština
language-pt-PT = Portugalčina
language-pt-BR = Brazilska portugalščina
language-ru-RU = Ruština
language-sk-SK = Slovenčina
language-sv-SE = Švédčina

# Connection status messages
connection-status-disconnected = Odpojené
connection-status-connecting = Prebieha pripájanie
connection-status-connected-since = Pripojené od: {$since}
connection-status-mfa-pending = Čakanie na MFA: {$mfa_type}

# Login options
login-options-server-address = Adresa servera
login-options-server-ip = IP servera
login-options-client-enabled = Klient povolený
login-options-supported-protocols = Podporované protokoly
login-options-preferred-protocol = Preferovaný protokol
login-options-tcpt-port = Port TCPT
login-options-natt-port = Port NATT
login-options-internal-ca-fingerprint = Odtlačok interného CA
