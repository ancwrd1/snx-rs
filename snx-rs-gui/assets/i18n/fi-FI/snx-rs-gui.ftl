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

# Tabs and expanders
tab-general = Yleiset
tab-advanced = Lisäasetukset
expand-dns = DNS
expand-routing = Reititys
expand-certificates = Varmenne
expand-misc = Muut asetukset

# Error messages
error-no-server = Palvelimen osoitetta ei ole määritetty
error-no-auth = Tunnistusmenetelmää ei ole valittu
error-file-not-exist = Tiedostoa ei löydy: {$path}
error-invalid-cert-id = Varmennetunniste ei ole heksadesimaalimuodossa: {$id}
error-ca-root-not-exist = CA-juuripolkua ei löydy: {$path}
error-validation = Validoinnin virhe

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

# Icon themes
icon-theme-auto = Automaattinen
icon-theme-dark = Tumma
icon-theme-light = Vaalea
