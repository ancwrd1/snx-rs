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
label-username-required = Il nome utente è richiesto per l'autenticazione
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
label-language = Lingua
label-system-language = Predefinito di sistema
label-username-password = Nome utente e password

# Tabs and expanders
tab-general = Generale
tab-advanced = Avanzate
expand-dns = DNS
expand-routing = Routing
expand-certificates = Certificati
expand-misc = Impostazioni varie

# Error messages
error-no-server-name = Nessun indirizzo server specificato
error-no-auth = Nessun metodo di autenticazione selezionato
error-file-not-exist = Il file non esiste: {$path}
error-invalid-cert-id = ID certificato non in formato esadecimale: {$id}
error-ca-root-not-exist = Il percorso root CA non esiste: {$path}
error-validation = Errore di validazione
error-user-input-canceled = Input utente annullato
error-connection-cancelled = Connessione annullata
error-unknown-event = Evento sconosciuto: {$event}
error-no-service-connection = Nessuna connessione al servizio
error-empty-input = L'input non può essere vuoto
error-invalid-object = Oggetto non valido
error-no-connector = Nessun connettore tunnel
error-tunnel-disconnected = Tunnel disconnesso, ultimo messaggio: {$message}
error-unexpected-reply = Risposta inaspettata
error-auth-failed = Autenticazione fallita
error-no-login-type = Parametro obbligatorio mancante: login-type
error-connection-timeout = Timeout della connessione
error-invalid-response = Risposta non valida!
error-request-failed-error-code = Richiesta fallita, codice di errore: {$error_code}
error-no-root-privileges = Questo programma deve essere eseguito come utente root!
error-missing-required-parameters = Parametri obbligatori mancanti: nome server e/o tipo di accesso!
error-missing-server-name = Parametro obbligatorio mancante: nome server!
error-no-connector-for-challenge-code = Nessun connettore per inviare il codice di sfida!
error-probing-failed = Sondaggio fallito, il server non è raggiungibile tramite la porta NATT!
error-invalid-sexpr = sexpr non valido: {$value}
error-invalid-value = Valore non valido
error-udp-request-failed = Errore nell'invio della richiesta UDP
error-no-tty = Nessun TTY collegato per ottenere l'input dell'utente
error-invalid-auth-response = Risposta di autenticazione non valida
error-invalid-client-settings = Risposta delle impostazioni client non valida
error-invalid-otp-reply = Risposta OTP non valida
error-udp-encap-failed = Impossibile impostare l'opzione socket UDP_ENCAP, codice di errore: {$code}
error-so-no-check-failed = Impossibile impostare l'opzione socket SO_NO_CHECK, codice di errore: {$code}
error-keepalive-failed = Keepalive fallito
error-receive-failed = Ricezione fallita
error-unknown-color-scheme = Valore schema colori sconosciuto
error-cannot-determine-ip = Impossibile determinare l'IP predefinito
error-invalid-command = Comando non valido: {$command}
error-otp-browser-failed = Impossibile ottenere l'OTP dal browser
error-invalid-operation-mode = Modalità operativa non valida
error-invalid-tunnel-type = Tipo di tunnel non valido
error-invalid-cert-type = Tipo di certificato non valido
error-invalid-icon-theme = Tema icone non valido
error-no-natt-reply = Nessuna risposta NATT
error-not-implemented = Non implementato
error-unknown-packet-type = Tipo di pacchetto sconosciuto
error-no-sender = Nessun mittente
error-empty-ccc-session = Sessione CCC vuota
error-identity-timeout = Timeout durante l'attesa della risposta di identità, il tipo di accesso è corretto?

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

# Connection info
info-connected-since = Connesso da
info-server-name = Nome server
info-user-name = Nome utente
info-login-type = Tipo di accesso
info-tunnel-type = Tipo di tunnel
info-transport-type = Tipo di trasporto
info-ip-address = Indirizzo IP
info-dns-servers = Server DNS
info-search-domains = Domini di ricerca
info-interface = Interfaccia
info-dns-configured = DNS configurato
info-routing-configured = Routing configurato
info-default-route = Route predefinita

# Application
app-title = Client VPN SNX-RS per Linux
app-connection-error = Errore di connessione
app-connection-success = Connessione riuscita

# Authentication
auth-dialog-title = Fattore di autenticazione VPN
auth-dialog-message = Inserisci il tuo fattore di autenticazione:

# Status dialog
status-dialog-title = Informazioni di connessione
status-button-copy = Copia
status-button-settings = Impostazioni
status-button-connect = Connetti
status-button-disconnect = Disconnetti

# Tray menu
tray-menu-connect = Connetti
tray-menu-disconnect = Disconnetti
tray-menu-status = Stato connessione...
tray-menu-settings = Impostazioni...
tray-menu-about = Informazioni...
tray-menu-exit = Esci

# CLI Messages
cli-identity-provider-auth = Per l'autenticazione tramite il provider di identità, apri il seguente URL nel tuo browser:
cli-tunnel-connected = Tunnel connesso, premi Ctrl+C per uscire.
cli-tunnel-disconnected = Tunnel disconnesso
cli-another-instance-running = Un'altra istanza di snx-rs è già in esecuzione
cli-app-terminated = Applicazione terminata da un segnale

# Connection Messages
connection-connected-to = Connesso a {$server}

# Languages
language-cs-CZ = Ceco
language-da-DK = Danese
language-de-DE = Tedesco
language-en-US = Inglese
language-es-ES = Spagnolo
language-fi-FI = Finlandese
language-fr-FR = Francese
language-it-IT = Italiano
language-nl-NL = Olandese
language-no-NO = Norvegese
language-pl-PL = Polacco
language-pt-PT = Portoghese
language-pt-BR = Portoghese Brasiliano
language-ru-RU = Russo
language-sk-SK = Slovacco
language-sv-SE = Svedese

# Connection status messages
connection-status-disconnected = Disconnesso
connection-status-connecting = Connessione in corso
connection-status-connected-since = Connesso da: {$since}
connection-status-mfa-pending = In attesa di MFA: {$mfa_type}

# Login options
login-options-server-address = Indirizzo server
login-options-server-ip = IP server
login-options-client-enabled = Client abilitato
login-options-supported-protocols = Protocolli supportati
login-options-preferred-protocol = Protocollo preferito
login-options-tcpt-port = Porta TCPT
login-options-natt-port = Porta NATT
login-options-internal-ca-fingerprint = Impronta CA interna
