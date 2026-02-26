# Dialog and buttons
dialog-title = VPN postavke
button-ok = U redu
button-apply = Primijeni
button-cancel = Odustani
button-fetch-info = Dohvati podatke

# Labels
label-server-address = Adresa VPN poslužitelja
label-auth-method = Metoda autentifikacije
label-tunnel-type = Vrsta tunela
label-cert-auth-type = Vrsta autentifikacije certifikatom
label-icon-theme = Tema ikona
label-username = Korisničko ime
label-username-required = Korisničko ime je potrebno za autentifikaciju
label-password = Lozinka
label-no-dns = Ne mijenjaj konfiguraciju DNS razrješitelja
label-dns-servers = Dodatni DNS poslužitelji
label-ignored-dns-servers = Zanemareni DNS poslužitelji
label-search-domains = Dodatne domene za pretraživanje
label-ignored-domains = Zanemarene domene za pretraživanje
label-routing-domains = Tretiraj primljene domene za pretraživanje kao domene za usmjeravanje
label-ca-cert = Korijenski CA certifikati poslužitelja
label-no-cert-check = Onemogući sve provjere TLS certifikata (NESIGURNO!)
label-password-factor = Indeks faktora lozinke, 1..N
label-no-keychain = Ne spremaj lozinke u privjesak za ključeve
label-ike-lifetime = Životni vijek IPSec IKE SA, sekunde
label-ike-persist = Spremi IPSec IKE sesiju i automatski se ponovno poveži
label-no-keepalive = Onemogući IPSec keepalive pakete
label-port-knock = Omogući NAT-T port knocking
label-no-routing = Zanemari sve dobivene rute
label-default-routing = Postavi zadanu rutu kroz tunel
label-add-routes = Dodatne statičke rute
label-ignored-routes = Rute za zanemariti
label-client-cert = Certifikat klijenta ili putanja upravljačkog programa (.pem, .pfx/.p12, .so)
label-cert-password = PFX lozinka ili PKCS11 pin
label-cert-id = Hex ID PKCS11 certifikata
label-language = Jezik
label-system-language = Zadano sustava
label-username-password = Korisničko ime i lozinka
label-auto-connect = Automatski se poveži pri pokretanju
label-ip-lease-time = Prilagođeno vrijeme zakupa IP adrese, sekunde
label-disable-ipv6 = Onemogući IPv6 kada je zadana ruta omogućena
label-mtu = MTU
label-connection-profile = Profil veze
label-profile-name = Naziv profila
label-confirmation = Molimo potvrdite
label-mobile-access = Mobilni pristup
label-machine-cert-auth = Provjera autentičnosti strojnim certifikatom
label-browse = Pregledaj...
label-keychain-files = Datoteke privjeska za ključeve
label-all-files = Sve datoteke
label-cancel = Odustani
label-open = Otvori
label-select-file = Odaberi datoteku
label-ca-cert-files = X.509 certifikati

# Tabs and expanders
tab-general = Općenito
tab-advanced = Napredno
expand-dns = DNS
expand-routing = Usmjeravanje
expand-certificates = Certifikati
expand-misc = Razne postavke
expand-ui = Postavke sučelja

# Error messages
error-no-server-name = Nije navedena adresa poslužitelja
error-no-auth = Nije odabrana metoda autentifikacije
error-file-not-exist = Datoteka ne postoji: {$path}
error-invalid-cert-id = ID certifikata nije u hex formatu: {$id}
error-ca-root-not-exist = CA korijenski put ne postoji: {$path}
error-validation = Greška validacije
error-user-input-canceled = Korisnički unos otkazan
error-connection-cancelled = Veza otkazana
error-unknown-event = Nepoznat događaj: {$event}
error-no-service-connection = Nema veze s uslugom
error-empty-input = Unos ne može biti prazan
error-invalid-response = Nevažeći odgovor!
error-cannot-acquire-access-cookie = Nije moguće dobiti pristupni kolačić!
error-invalid-object = Nevažeći objekt
error-no-connector = Nema konektora tunela
error-tunnel-disconnected = Tunel prekinut, zadnja poruka: {$message}
error-unexpected-reply = Neočekivani odgovor
error-auth-failed = Autentifikacija nije uspjela
error-no-login-type = Nedostaje obavezni parametar: login-type
error-connection-timeout = Istek vremena veze
error-cannot-send-request = Nije moguće poslati zahtjev usluzi
error-cannot-read-reply = Nije moguće pročitati odgovor od usluge
error-no-ipv4 = Nema IPv4 adrese za {$server}
error-not-challenge-state = Nije stanje izazova
error-no-challenge = Nema izazova u sadržaju
error-endless-challenges = Beskonačna petlja izazova korisničkog imena
error-no-pkcs12 = Nije navedena PKCS12 putanja i lozinka
error-no-pkcs8 = Nije navedena PKCS8 PEM putanja
error-no-pkcs11 = Nije naveden PKCS11 pin
error-no-ipsec-session = Nema IPSEC sesije
error-request-failed-error-code = Zahtjev nije uspio, kod greške: {$error_code}
error-no-root-privileges = Ovaj program treba pokrenuti kao root korisnik!
error-missing-required-parameters = Nedostaju obavezni parametri: naziv poslužitelja i/ili tip prijave!
error-missing-server-name = Nedostaje obavezni parametar: naziv poslužitelja!
error-no-connector-for-challenge-code = Nema konektora za slanje koda izazova!
error-probing-failed = Ispitivanje nije uspjelo, poslužitelj nije dostupan putem NATT porta!
error-invalid-sexpr = Nevažeći sexpr: {$value}
error-invalid-value = Nevažeća vrijednost
error-udp-request-failed = Greška pri slanju UDP zahtjeva
error-no-tty = Nema priključenog TTY-a za korisnički unos
error-invalid-auth-response = Nevažeći odgovor autentifikacije
error-invalid-client-settings = Nevažeći odgovor postavki klijenta
error-invalid-cert-response = Nevažeći odgovor certifikata
error-certificate-enrollment-failed = Registracija certifikata nije uspjela, kod greške: {$code}
error-missing-cert-path = Nedostaje putanja certifikata do PKCS12 datoteke!
error-missing-cert-password = Nedostaje PKCS12 lozinka!
error-missing-reg-key = Nedostaje ključ za registraciju!
error-invalid-otp-reply = Nevažeći OTP odgovor
error-udp-encap-failed = Nije moguće postaviti UDP_ENCAP opciju socketa, kod greške: {$code}
error-so-no-check-failed = Nije moguće postaviti SO_NO_CHECK opciju socketa, kod greške: {$code}
error-keepalive-failed = Keepalive nije uspio
error-receive-failed = Primanje nije uspjelo
error-unknown-color-scheme = Nepoznata vrijednost color-scheme
error-cannot-determine-ip = Nije moguće odrediti zadanu IP adresu
error-invalid-command = Nevažeća naredba: {$command}
error-otp-browser-failed = Nije moguće dobiti OTP iz preglednika
error-invalid-operation-mode = Nevažeći način rada
error-invalid-tunnel-type = Nevažeća vrsta tunela
error-invalid-cert-type = Nevažeća vrsta certifikata
error-invalid-icon-theme = Nevažeća tema ikona
error-no-natt-reply = Nema NAT-T odgovora
error-not-implemented = Nije implementirano
error-unknown-packet-type = Nepoznata vrsta paketa
error-no-sender = Nema pošiljatelja
error-empty-ccc-session = Prazna CCC sesija
error-identity-timeout = Istek vremena čekanja na odgovor identiteta, je li tip prijave ispravan?
error-invalid-transport-type = Nevažeća vrsta prijenosa
error-certificate-verify-failed = TLS validacija certifikata nije uspjela. Certifikat poslužitelja je nevažeći, istekao ili nije pouzdan.

# Placeholder texts
placeholder-domains = Domene odvojene zarezom
placeholder-ip-addresses = IP adrese odvojene zarezom
placeholder-routes = Odvojeno zarezom x.x.x.x/x
placeholder-certs = PEM ili DER datoteke odvojene zarezom

# Tunnel types
tunnel-type-ipsec = IPSec
tunnel-type-ssl = SSL

# Certificate types
cert-type-none = Ništa
cert-type-pfx = PFX datoteka
cert-type-pem = PEM datoteka
cert-type-hw = Hardverski token

# Transport types
transport-type-autodetect = Automatsko otkrivanje
transport-type-kernel = UDP XFRM
transport-type-tcpt = TCPT TUN
transport-type-udp = UDP TUN

# Icon themes
icon-theme-autodetect = Automatsko otkrivanje
icon-theme-dark = Tamna
icon-theme-light = Svijetla

# Application
app-title = SNX-RS VPN klijent za Linux
app-connection-error = Greška veze
app-connection-success = Veza uspješna

# Authentication
auth-dialog-title = VPN faktor autentifikacije
auth-dialog-message = Molimo unesite vaš faktor autentifikacije:

# Status dialog
status-dialog-title = Informacije o vezi
status-button-copy = Kopiraj
status-button-settings = Postavke
status-button-connect = Poveži
status-button-disconnect = Prekini vezu

# Tray menu
tray-menu-connect = Poveži
tray-menu-disconnect = Prekini vezu
tray-menu-status = Status veze...
tray-menu-settings = Postavke...
tray-menu-about = O programu...
tray-menu-exit = Izlaz

# Connection info
info-connected-since = Povezano od
info-server-name = Naziv poslužitelja
info-user-name = Korisničko ime
info-login-type = Tip prijave
info-tunnel-type = Vrsta tunela
info-transport-type = Vrsta prijenosa
info-ip-address = IP adresa
info-dns-servers = DNS poslužitelji
info-search-domains = Domene za pretraživanje
info-interface = Sučelje
info-dns-configured = DNS konfiguriran
info-routing-configured = Usmjeravanje konfigurirano
info-default-route = Zadana ruta

# CLI Messages
cli-identity-provider-auth = Za autentifikaciju putem pružatelja identiteta, otvorite sljedeću URL adresu u pregledniku:
cli-tunnel-connected = Tunel povezan, pritisnite Ctrl-C za izlaz.
cli-tunnel-disconnected = Tunel prekinut
cli-another-instance-running = Druga instanca snx-rs već je pokrenuta
cli-app-terminated = Aplikacija prekinuta zbog signala
cli-mobile-access-auth = Za autentifikaciju mobilnog pristupa, prijavite se putem sljedeće URL adrese, zatim pronađite korisničku lozinku u hex obliku u HTML izvoru stranice i unesite je ovdje:
cli-certificate-enrolled = Certifikat je uspješno registriran.

# Connection Messages
connection-connected-to = Povezano na {$server}

# Languages
language-cs-CZ = Češki
language-da-DK = Danski
language-de-DE = Njemački
language-en-US = Engleski
language-es-ES = Španjolski
language-fi-FI = Finski
language-fr-FR = Francuski
language-hr-HR = Hrvatski
language-it-IT = Talijanski
language-nl-NL = Nizozemski
language-no-NO = Norveški
language-pl-PL = Poljski
language-pt-PT = Portugalski
language-pt-BR = Brazilski portugalski
language-ru-RU = Ruski
language-sk-SK = Slovački
language-sv-SE = Švedski

# Connection status messages
connection-status-disconnected = Prekinuta veza
connection-status-connecting = Povezivanje u tijeku
connection-status-connected-since = Povezano od: {$since}
connection-status-mfa-pending = MFA na čekanju: {$mfa_type}

# Login options
login-options-server-address = Adresa poslužitelja
login-options-server-ip = IP poslužitelja
login-options-client-enabled = Klijent omogućen
login-options-supported-protocols = Podržani protokoli
login-options-preferred-protocol = Preferirani protokol
login-options-tcpt-port = TCPT port
login-options-natt-port = NATT port
login-options-internal-ca-fingerprint = Otisak internog CA

# Connection profiles
profile-new = Novi
profile-rename = Preimenuj
profile-delete = Obriši
profile-delete-prompt = Jeste li sigurni da želite obrisati odabrani profil?
profile-default-name = Zadano
profile-new-title = Novi profil veze
profile-rename-title = Preimenuj profil veze
