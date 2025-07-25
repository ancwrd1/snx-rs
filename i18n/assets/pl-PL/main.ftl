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
label-username-required = Nazwa użytkownika jest wymagana do uwierzytelnienia
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
label-language = Język
label-system-language = Domyślny systemowy
label-username-password = Nazwa użytkownika i hasło
label-auto-connect = Automatycznie łącz przy starcie
label-ip-lease-time = Niestandardowy czas dzierżawy IP, sekundy

# Tabs and expanders
tab-general = Ogólne
tab-advanced = Zaawansowane
expand-dns = DNS
expand-routing = Routing
expand-certificates = Certyfikaty
expand-misc = Ustawienia dodatkowe
expand-ui = Interfejs użytkownika

# Error messages
error-no-server-name = Nie podano adresu serwera
error-no-auth = Nie wybrano metody uwierzytelniania
error-file-not-exist = Plik nie istnieje: {$path}
error-invalid-cert-id = Identyfikator certyfikatu nie w formacie szesnastkowym: {$id}
error-ca-root-not-exist = Ścieżka główna CA nie istnieje: {$path}
error-validation = Błąd walidacji
error-user-input-canceled = Wprowadzanie danych przez użytkownika anulowane
error-connection-cancelled = Połączenie anulowane
error-unknown-event = Nieznane zdarzenie: {$event}
error-no-service-connection = Brak połączenia z usługą
error-empty-input = Dane wejściowe nie mogą być puste
error-invalid-object = Nieprawidłowy obiekt
error-no-connector = Brak łącznika tunelu
error-tunnel-disconnected = Tunel rozłączony, ostatnia wiadomość: {$message}
error-unexpected-reply = Nieoczekiwana odpowiedź
error-auth-failed = Uwierzytelnianie nie powiodło się
error-no-login-type = Brak wymaganego parametru: login-type
error-connection-timeout = Przekroczenie czasu połączenia
error-invalid-response = Nieprawidłowa odpowiedź
error-cannot-send-request = Nie można wysłać żądania do usługi
error-cannot-read-reply = Nie można odczytać odpowiedzi z usługi
error-no-ipv4 = Brak adresu IPv4 dla {$server}
error-not-challenge-state = Nie jest stanem wyzwania
error-no-challenge = Brak wyzwania w danych
error-endless-challenges = Nieskończona pętla wyzwań nazwy użytkownika
error-no-pkcs12 = Nie podano ścieżki PKCS12 i hasła
error-no-pkcs8 = Nie podano ścieżki PKCS8 PEM
error-no-pkcs11 = Nie podano kodu PIN PKCS11
error-no-ipsec-session = Brak sesji IPSEC
error-request-failed-error-code = Żądanie nie powiodło się, kod błędu: {$error_code}
error-no-root-privileges = Ten program musi być uruchomiony jako użytkownik root!
error-missing-required-parameters = Brak wymaganych parametrów: nazwa serwera i/lub typ dostępu!
error-missing-server-name = Brak wymaganego parametru: nazwa serwera!
error-no-connector-for-challenge-code = Brak łącznika do wysłania kodu wyzwania!
error-probing-failed = Sondowanie nie powiodło się, serwer nie jest dostępny przez port NATT!
error-invalid-sexpr = Nieprawidłowy sexpr: {$value}
error-invalid-value = Nieprawidłowa wartość
error-udp-request-failed = Błąd podczas wysyłania żądania UDP
error-no-tty = Brak podłączonego TTY do wprowadzania danych przez użytkownika
error-invalid-auth-response = Nieprawidłowa odpowiedź uwierzytelniania
error-invalid-client-settings = Nieprawidłowe ustawienia klienta
error-invalid-otp-reply = Nieprawidłowa odpowiedź OTP
error-udp-encap-failed = Nie można ustawić opcji gniazda UDP_ENCAP, kod błędu: {$code}
error-so-no-check-failed = Nie można ustawić opcji gniazda SO_NO_CHECK, kod błędu: {$code}
error-keepalive-failed = Keepalive nie powiódł się
error-receive-failed = Odbieranie nie powiodło się
error-unknown-color-scheme = Nieznana wartość schematu kolorów
error-cannot-determine-ip = Nie można określić domyślnego IP
error-invalid-command = Nieprawidłowe polecenie: {$command}
error-otp-browser-failed = Nie można uzyskać OTP z przeglądarki
error-invalid-operation-mode = Nieprawidłowy tryb działania
error-invalid-tunnel-type = Nieprawidłowy typ tunelu
error-invalid-cert-type = Nieprawidłowy typ certyfikatu
error-invalid-icon-theme = Nieprawidłowy motyw ikon
error-no-natt-reply = Brak odpowiedzi NATT
error-not-implemented = Nie zaimplementowano
error-unknown-packet-type = Nieznany typ pakietu
error-no-sender = Brak nadawcy
error-empty-ccc-session = Pusta sesja CCC
error-identity-timeout = Przekroczenie czasu oczekiwania na odpowiedź tożsamości, czy typ dostępu jest poprawny?

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

# Connection info
info-connected-since = Połączono od
info-server-name = Nazwa serwera
info-user-name = Nazwa użytkownika
info-login-type = Typ logowania
info-tunnel-type = Typ tunelu
info-transport-type = Typ transportu
info-ip-address = Adres IP
info-dns-servers = Serwery DNS
info-search-domains = Domeny wyszukiwania
info-interface = Interfejs
info-dns-configured = DNS skonfigurowany
info-routing-configured = Routing skonfigurowany
info-default-route = Trasa domyślna

# Application
app-title = Klient VPN SNX-RS dla Linux
app-connection-error = Błąd połączenia
app-connection-success = Połączenie udane

# Authentication
auth-dialog-title = Czynnik uwierzytelniania VPN
auth-dialog-message = Wprowadź swój czynnik uwierzytelniania:

# Status dialog
status-dialog-title = Informacje o połączeniu
status-button-copy = Kopiuj
status-button-settings = Ustawienia
status-button-connect = Połącz
status-button-disconnect = Rozłącz

# Tray menu
tray-menu-connect = Połącz
tray-menu-disconnect = Rozłącz
tray-menu-status = Status połączenia...
tray-menu-settings = Ustawienia...
tray-menu-about = O programie...
tray-menu-exit = Zakończ

# CLI Messages
cli-identity-provider-auth = Aby uwierzytelnić się przez dostawcę tożsamości, otwórz następujący adres URL w przeglądarce:
cli-tunnel-connected = Tunel połączony, naciśnij Ctrl+C aby zakończyć.
cli-tunnel-disconnected = Tunel rozłączony
cli-another-instance-running = Inna instancja snx-rs jest już uruchomiona
cli-app-terminated = Aplikacja zakończona przez sygnał

# Connection Messages
connection-connected-to = Połączono z {$server}

# Languages
language-cs-CZ = Czeski
language-da-DK = Duński
language-de-DE = Niemiecki
language-en-US = Angielski
language-es-ES = Hiszpański
language-fi-FI = Fiński
language-fr-FR = Francuski
language-it-IT = Włoski
language-nl-NL = Holenderski
language-no-NO = Norweski
language-pl-PL = Polski
language-pt-PT = Portugalski
language-pt-BR = Brazylijski portugalski
language-ru-RU = Rosyjski
language-sk-SK = Słowacki
language-sv-SE = Szwedzki

# Connection status messages
connection-status-disconnected = Rozłączono
connection-status-connecting = Łączenie
connection-status-connected-since = Połączono od: {$since}
connection-status-mfa-pending = Oczekiwanie na MFA: {$mfa_type}

# Login options
login-options-server-address = Adres serwera
login-options-server-ip = IP serwera
login-options-client-enabled = Klient włączony
login-options-supported-protocols = Obsługiwane protokoły
login-options-preferred-protocol = Preferowany protokół
login-options-tcpt-port = Port TCPT
login-options-natt-port = Port NATT
login-options-internal-ca-fingerprint = Odcisk wewnętrznego CA
