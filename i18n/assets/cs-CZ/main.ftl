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
label-language = Jazyk
label-system-language = Systémové výchozí

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
error-user-input-canceled = Vstup uživatele zrušen
error-connection-canceled = Připojení zrušeno
error-unknown-event = Neznámá událost: {$event}
error-no-service-connection = Žádné připojení ke službě
error-empty-input = Vstup nemůže být prázdný

# New error messages
error-invalid-object = Neplatný objekt
error-no-connector = Žádný konektor tunelu
error-connection-cancelled = Připojení zrušeno
error-tunnel-disconnected = Tunel odpojen, poslední zpráva: {$message}
error-unexpected-reply = Neočekávaná odpověď
error-auth-failed = Ověření selhalo
error-no-server-name = Chybí povinný parametr: server-name
error-no-login-type = Chybí povinný parametr: login-type
error-connection-timeout = Časový limit připojení
error-invalid-response = Neplatná odpověď
error-cannot-send-request = Nelze odeslat požadavek na službu
error-cannot-read-reply = Nelze přečíst odpověď ze služby
error-no-ipv4 = Žádná IPv4 adresa pro {$server}
error-not-challenge-state = Není stav výzvy
error-no-challenge = Žádná výzva v datech
error-endless-challenges = Nekonečná smyčka výzev uživatelského jména
error-no-pkcs12 = Žádná cesta PKCS12 a heslo nejsou poskytnuty
error-no-pkcs8 = Žádná cesta PKCS8 PEM není poskytnuta
error-no-pkcs11 = Žádný PIN PKCS11 není poskytnut
error-no-ipsec-session = Žádná IPSEC relace

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

# Connection info
info-connected-since = Připojeno od
info-server-name = Název serveru
info-user-name = Uživatelské jméno
info-login-type = Typ přihlášení
info-tunnel-type = Typ tunelu
info-transport-type = Typ transportu
info-ip-address = IP adresa
info-dns-servers = DNS servery
info-search-domains = Vyhledávací domény
info-interface = Rozhraní
info-dns-configured = DNS nakonfigurováno
info-routing-configured = Směrování nakonfigurováno
info-default-route = Výchozí trasa

# Application
app-title = SNX-RS VPN klient pro Linux
app-connection-error = Chyba připojení
app-connection-success = Připojení úspěšné

# Authentication
auth-dialog-title = VPN autentizační faktor
auth-dialog-message = Zadejte váš autentizační faktor:

# Status dialog
status-dialog-title = Informace o připojení
status-button-copy = Kopírovat
status-button-settings = Nastavení
status-button-connect = Připojit
status-button-disconnect = Odpojit

# Tray menu
tray-menu-connect = Připojit
tray-menu-disconnect = Odpojit
tray-menu-status = Stav připojení...
tray-menu-settings = Nastavení...
tray-menu-about = O aplikaci...
tray-menu-exit = Ukončit

# CLI Messages
cli-identity-provider-auth = Pro ověření přes poskytovatele identity otevřete následující URL ve vašem prohlížeči:
cli-tunnel-connected = Tunel připojen, stiskněte Ctrl+C pro ukončení.
cli-tunnel-disconnected = Tunel odpojen
cli-another-instance-running = Jiná instance snx-rs již běží
cli-app-terminated = Aplikace ukončena signálem

# Connection Messages
connection-connected-to = Připojeno k {$server}

# Languages
language-cs-CZ = Čeština
language-da-DK = Dánština
language-de-DE = Němčina
language-en-US = Angličtina
language-es-ES = Španělština
language-fi-FI = Finština
language-fr-FR = Francouzština
language-it-IT = Italština
language-nl-NL = Nizozemština
language-no-NO = Norština
language-pl-PL = Polština
language-pt-PT = Portugalština
language-ru-RU = Ruština
language-sk-SK = Slovenština
language-sv-SE = Švédština
