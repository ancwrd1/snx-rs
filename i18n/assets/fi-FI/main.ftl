# Dialog and buttons
dialog-title = VPN-asetukset
button-ok = OK
button-apply = Käytä
button-cancel = Peruuta
button-fetch-info = Hae tiedot

# Labels
label-server-address = VPN-palvelimen osoite
label-auth-method = Tunnistusmenetelmä
label-tunnel-type = Tunnelityyppi
label-cert-auth-type = Varmennetyyppi
label-icon-theme = Kuvaketeema
label-username = Käyttäjätunnus
label-username-required = Käyttäjätunnus vaaditaan todennukseen
label-password = Salasana
label-no-dns = Älä muuta DNS-konfiguraatiota
label-dns-servers = Lisä-DNS-palvelimet
label-ignored-dns-servers = Ohitettavat DNS-palvelimet
label-search-domains = Lisähakualueet
label-ignored-domains = Ohitettavat hakualueet
label-routing-domains = Käsittele vastaanotetut hakualueet reititysalueina
label-ca-cert = Palvelimen CA-juurivarmenne
label-no-cert-check = Poista kaikki TLS-varmennetarkistukset käytöstä (TURVATON!)
label-password-factor = Salasanatekijäindeksi, 1..N
label-no-keychain = Älä tallenna salasanoja avainketjuun
label-ike-lifetime = IPSec IKE SA -elinaika, sekuntia
label-ike-persist = Tallenna IPSec IKE -istunto ja yhdistä automaattisesti
label-no-keepalive = Poista IPSec keepalive-paketit käytöstä
label-port-knock = Ota NAT-T port knocking käyttöön
label-no-routing = Ohita kaikki vastaanotetut reitit
label-default-routing = Aseta oletusreitti tunnelin kautta
label-add-routes = Lisästaattiset reitit
label-ignored-routes = Ohitettavat reitit
label-client-cert = Asiakasvarmenne tai ajuripolku (.pem, .pfx/.p12, .so)
label-cert-password = PFX-salasana tai PKCS11-PIN
label-cert-id = PKCS11-varmennuksen heksadesimaalinen tunniste
label-language = Kieli
label-system-language = Järjestelmän oletus
label-username-password = Käyttäjätunnus ja salasana
label-auto-connect = Yhdistä automaattisesti käynnistyksessä
label-ip-lease-time = Mukautettu IP-vuokra-aika, sekuntia
label-disable-ipv6 = Poista IPv6 käytöstä, kun oletusreitti on käytössä
label-mtu = MTU

# Tabs and expanders
tab-general = Yleiset
tab-advanced = Lisäasetukset
expand-dns = DNS
expand-routing = Reititys
expand-certificates = Varmenne
expand-misc = Muut asetukset
expand-ui = Käyttöliittymän asetukset

# Error messages
error-no-server-name = Palvelimen osoitetta ei ole määritetty
error-no-auth = Tunnistusmenetelmää ei ole valittu
error-file-not-exist = Tiedostoa ei löydy: {$path}
error-invalid-cert-id = Varmennetunniste ei ole heksadesimaalimuodossa: {$id}
error-ca-root-not-exist = CA-juuripolkua ei löydy: {$path}
error-validation = Validoinnin virhe
error-user-input-canceled = Käyttäjän syöte peruttu
error-connection-cancelled = Yhteys peruttu
error-unknown-event = Tuntematon tapahtuma: {$event}
error-no-service-connection = Ei yhteyttä palveluun
error-empty-input = Syöte ei voi olla tyhjä
error-invalid-response = Virheellinen vastaus!
error-invalid-object = Virheellinen objekti
error-no-connector = Ei tunneliyhteyttä
error-tunnel-disconnected = Tunneli katkaistu, viimeisin viesti: {$message}
error-unexpected-reply = Odottamaton vastaus
error-auth-failed = Tunnistus epäonnistui
error-no-login-type = Pakollinen parametri puuttuu: login-type
error-connection-timeout = Yhteyden aikakatkaisu
error-cannot-send-request = Pyyntöä ei voi lähettää palveluun
error-cannot-read-reply = Vastausta ei voi lukea palvelusta
error-no-ipv4 = Ei IPv4-osoitetta kohteelle {$server}
error-not-challenge-state = Ei haasteiden tilaa
error-no-challenge = Ei haastetta tiedoissa
error-endless-challenges = Loputon silmukka käyttäjätunnushaasteita
error-no-pkcs12 = Ei PKCS12-polku ja salasana annettu
error-no-pkcs8 = Ei PKCS8 PEM-polku annettu
error-no-pkcs11 = Ei PKCS11 PIN-koodia annettu
error-no-ipsec-session = Ei IPSEC-istuntoa
error-request-failed-error-code = Pyyntö epäonnistui, virhekoodi: {$error_code}
error-no-root-privileges = Tämä ohjelma pitää ajaa root-käyttäjänä!
error-missing-required-parameters = Pakolliset parametrit puuttuvat: palvelimen nimi ja/tai kirjautumistyyppi!
error-missing-server-name = Pakollinen parametri puuttuu: palvelimen nimi!
error-no-connector-for-challenge-code = Ei konektoria haastekoodin lähettämiseen!
error-probing-failed = Tarkistus epäonnistui, palvelimeen ei pääse NATT-portin kautta!
error-invalid-sexpr = Virheellinen sexpr: {$value}
error-invalid-value = Virheellinen arvo
error-udp-request-failed = Virhe UDP-pyynnön lähetyksessä
error-no-tty = Ei TTY-yhteyttä käyttäjän syötteeseen
error-invalid-auth-response = Virheellinen tunnistusvastaus
error-invalid-client-settings = Virheelliset asiakasasetukset
error-invalid-otp-reply = Virheellinen OTP-vastaus
error-udp-encap-failed = UDP_ENCAP-soketin asetusta ei voi asettaa, virhekoodi: {$code}
error-so-no-check-failed = SO_NO_CHECK-soketin asetusta ei voi asettaa, virhekoodi: {$code}
error-keepalive-failed = Keepalive epäonnistui
error-receive-failed = Vastaanotto epäonnistui
error-unknown-color-scheme = Tuntematon väriteeman arvo
error-cannot-determine-ip = Oletus-IP-osoitetta ei voi määrittää
error-invalid-command = Virheellinen komento: {$command}
error-otp-browser-failed = OTP:ta ei voi hakea selaimesta
error-invalid-operation-mode = Virheellinen toimintatila
error-invalid-tunnel-type = Virheellinen tunnelityyppi
error-invalid-cert-type = Virheellinen varmennetyyppi
error-invalid-icon-theme = Virheellinen kuvaketeema
error-no-natt-reply = Ei NATT-vastausta
error-not-implemented = Ei toteutettu
error-unknown-packet-type = Tuntematon pakettityyppi
error-no-sender = Ei lähettäjää
error-empty-ccc-session = Tyhjä CCC-istunto
error-identity-timeout = Aikakatkaisu odottaessa identiteettivastausta, onko kirjautumistyyppi oikea?
error-invalid-transport-type = Virheellinen kuljetustyyppi

# Placeholder texts
placeholder-domains = Pilkulla erotetut verkkotunnukset
placeholder-ip-addresses = Pilkulla erotetut IP-osoitteet
placeholder-routes = Pilkulla erotetut reitit muodossa x.x.x.x/x
placeholder-certs = Pilkulla erotetut PEM- tai DER-tiedostot

# Tunnel types
tunnel-type-ipsec = IPSec
tunnel-type-ssl = SSL (vanhentunut)

# Certificate types
cert-type-none = Ei mitään
cert-type-pfx = PFX-tiedosto
cert-type-pem = PEM-tiedosto
cert-type-hw = Laiteavain

# Transport types
transport-type-auto-detect = Automaattinen tunnistus
transport-type-kernel = Kernel XFRM
transport-type-tcpt = TCPT TUN
transport-type-udp = UDP TUN

# Icon themes
icon-theme-auto = Automaattinen
icon-theme-dark = Tumma
icon-theme-light = Vaalea

# Connection info
info-connected-since = Yhdistetty alkaen
info-server-name = Palvelimen nimi
info-user-name = Käyttäjänimi
info-login-type = Kirjautumistyyppi
info-tunnel-type = Tunnelityyppi
info-transport-type = Kuljetustyyppi
info-ip-address = IP-osoite
info-dns-servers = DNS-palvelimet
info-search-domains = Hakualueet
info-interface = Käyttöliittymä
info-dns-configured = DNS määritetty
info-routing-configured = Reititys määritetty
info-default-route = Oletusreitti

# Application
app-title = SNX-RS VPN-asiakasohjelma Linuxille
app-connection-error = Yhteysvirhe
app-connection-success = Yhteys onnistui

# Authentication
auth-dialog-title = VPN-todennustekijä
auth-dialog-message = Syötä todennustekijäsi:

# Status dialog
status-dialog-title = Yhteystiedot
status-button-copy = Kopioi
status-button-settings = Asetukset
status-button-connect = Yhdistä
status-button-disconnect = Katkaise yhteys

# Tray menu
tray-menu-connect = Yhdistä
tray-menu-disconnect = Katkaise yhteys
tray-menu-status = Yhteyden tila...
tray-menu-settings = Asetukset...
tray-menu-about = Tietoja...
tray-menu-exit = Lopeta

# CLI Messages
cli-identity-provider-auth = Tunnistautumista varten tunnistepalvelun kautta, avaa seuraava URL-selaimessasi:
cli-tunnel-connected = Tunneli yhdistetty, paina Ctrl+C lopettaaksesi.
cli-tunnel-disconnected = Tunneli katkaistu
cli-another-instance-running = Toinen snx-rs-esiintymä on jo käynnissä
cli-app-terminated = Sovellus päättyi signaalin vuoksi

# Connection Messages
connection-connected-to = Yhdistetty palvelimeen {$server}

# Languages
language-cs-CZ = Tšekki
language-da-DK = Tanska
language-de-DE = Saksa
language-en-US = Englanti
language-es-ES = Espanja
language-fi-FI = Suomi
language-fr-FR = Ranska
language-it-IT = Italia
language-nl-NL = Hollanti
language-no-NO = Norja
language-pl-PL = Puola
language-pt-PT = Portugali
language-pt-BR = Brasilianportugali
language-ru-RU = Venäjä
language-sk-SK = Slovakia
language-sv-SE = Ruotsi

# Connection status messages
connection-status-disconnected = Yhteys katkaistu
connection-status-connecting = Yhdistetään
connection-status-connected-since = Yhdistetty: {$since}
connection-status-mfa-pending = Odotetaan MFA: {$mfa_type}

# Login options
login-options-server-address = Palvelimen osoite
login-options-server-ip = Palvelimen IP
login-options-client-enabled = Asiakasohjelma käytössä
login-options-supported-protocols = Tuetut protokollat
login-options-preferred-protocol = Suositeltu protokolla
login-options-tcpt-port = TCPT-portti
login-options-natt-port = NATT-portti
login-options-internal-ca-fingerprint = Sisäisen CA:n sormenjälki
