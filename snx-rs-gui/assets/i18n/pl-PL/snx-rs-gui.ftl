# Dialog and buttons
dialog-title = Ustawienia VPN
button-ok = OK
button-apply = Zastosuj
button-cancel = Anuluj
button-fetch-info = Pobierz informacje

# Labels
label-server-address = Adres serwera VPN
label-auth-method = Metoda uwierzytelniania
label-tunnel-type = Typ tunelu
label-cert-auth-type = Typ certyfikatu
label-icon-theme = Motyw ikon
label-username = Nazwa użytkownika
label-password = Hasło
label-no-dns = Nie zmieniaj konfiguracji resolvera DNS
label-dns-servers = Dodatkowe serwery DNS
label-ignored-dns-servers = Ignorowane serwery DNS
label-search-domains = Dodatkowe domeny wyszukiwania
label-ignored-domains = Ignorowane domeny wyszukiwania
label-routing-domains = Traktuj otrzymane domeny wyszukiwania jako domeny routingu
label-ca-cert = Certyfikaty główne CA serwera
label-no-cert-check = Wyłącz wszystkie kontrole certyfikatów TLS (NIEBEZPIECZNE!)
label-password-factor = Indeks współczynnika hasła, 1..N
label-no-keychain = Nie przechowuj haseł w schowku kluczy
label-ike-lifetime = Czas życia IPSec IKE SA, sekundy
label-ike-persist = Zapisz sesję IPSec IKE i połącz ponownie automatycznie
label-no-keepalive = Wyłącz pakiety keepalive IPSec
label-port-knock = Włącz port knocking NAT-T
label-no-routing = Ignoruj wszystkie nabyte trasy
label-default-routing = Ustaw trasę domyślną przez tunel
label-add-routes = Dodatkowe trasy statyczne
label-ignored-routes = Trasy do ignorowania
label-client-cert = Certyfikat klienta lub ścieżka sterownika (.pem, .pfx/.p12, .so)
label-cert-password = Hasło PFX lub PIN PKCS11
label-cert-id = Identyfikator szesnastkowy certyfikatu PKCS11

# Tabs and expanders
tab-general = Ogólne
tab-advanced = Zaawansowane
expand-dns = DNS
expand-routing = Routing
expand-certificates = Certyfikaty
expand-misc = Ustawienia dodatkowe

# Error messages
error-no-server = Nie podano adresu serwera
error-no-auth = Nie wybrano metody uwierzytelniania
error-file-not-exist = Plik nie istnieje: {$path}
error-invalid-cert-id = Identyfikator certyfikatu nie w formacie szesnastkowym: {$id}
error-ca-root-not-exist = Ścieżka główna CA nie istnieje: {$path}
error-validation = Błąd walidacji

# Placeholder texts
placeholder-domains = Domeny oddzielone przecinkami
placeholder-ip-addresses = Adresy IP oddzielone przecinkami
placeholder-routes = Trasy oddzielone przecinkami w formacie x.x.x.x/x
placeholder-certs = Pliki PEM lub DER oddzielone przecinkami

# Tunnel types
tunnel-type-ipsec = IPSec
tunnel-type-ssl = SSL (przestarzały)

# Certificate types
cert-type-none = Brak
cert-type-pfx = Plik PFX
cert-type-pem = Plik PEM
cert-type-hw = Token sprzętowy

# Icon themes
icon-theme-auto = Auto
icon-theme-dark = Ciemny
icon-theme-light = Jasny
