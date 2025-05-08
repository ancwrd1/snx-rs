# Dialog and buttons
dialog-title = Impostazioni VPN
button-ok = OK
button-apply = Applica
button-cancel = Annulla
button-fetch-info = Recupera informazioni

# Labels
label-server-address = Indirizzo server VPN
label-auth-method = Metodo di autenticazione
label-tunnel-type = Tipo di tunnel
label-cert-auth-type = Tipo di certificato
label-icon-theme = Tema icone
label-username = Nome utente
label-password = Password
label-no-dns = Non modificare la configurazione del resolver DNS
label-dns-servers = Server DNS aggiuntivi
label-ignored-dns-servers = Server DNS ignorati
label-search-domains = Domini di ricerca aggiuntivi
label-ignored-domains = Domini di ricerca ignorati
label-routing-domains = Tratta i domini di ricerca ricevuti come domini di routing
label-ca-cert = Certificati root CA del server
label-no-cert-check = Disabilita tutti i controlli dei certificati TLS (INSICURO!)
label-password-factor = Indice del fattore password, 1..N
label-no-keychain = Non memorizzare le password nel portachiavi
label-ike-lifetime = Durata IPSec IKE SA, secondi
label-ike-persist = Salva sessione IPSec IKE e riconnetti automaticamente
label-no-keepalive = Disabilita pacchetti keepalive IPSec
label-port-knock = Abilita port knocking NAT-T
label-no-routing = Ignora tutte le route acquisite
label-default-routing = Imposta route predefinita attraverso il tunnel
label-add-routes = Route statiche aggiuntive
label-ignored-routes = Route da ignorare
label-client-cert = Certificato client o percorso driver (.pem, .pfx/.p12, .so)
label-cert-password = Password PFX o PIN PKCS11
label-cert-id = ID esadecimale del certificato PKCS11

# Tabs and expanders
tab-general = Generale
tab-advanced = Avanzate
expand-dns = DNS
expand-routing = Routing
expand-certificates = Certificati
expand-misc = Impostazioni varie

# Error messages
error-no-server = Nessun indirizzo server specificato
error-no-auth = Nessun metodo di autenticazione selezionato
error-file-not-exist = Il file non esiste: {$path}
error-invalid-cert-id = ID certificato non in formato esadecimale: {$id}
error-ca-root-not-exist = Il percorso root CA non esiste: {$path}
error-validation = Errore di validazione

# Placeholder texts
placeholder-domains = Domini separati da virgole
placeholder-ip-addresses = Indirizzi IP separati da virgole
placeholder-routes = Route separate da virgole nel formato x.x.x.x/x
placeholder-certs = File PEM o DER separati da virgole

# Tunnel types
tunnel-type-ipsec = IPSec
tunnel-type-ssl = SSL (obsoleto)

# Certificate types
cert-type-none = Nessuno
cert-type-pfx = File PFX
cert-type-pem = File PEM
cert-type-hw = Token hardware

# Icon themes
icon-theme-auto = Auto
icon-theme-dark = Scuro
icon-theme-light = Chiaro
