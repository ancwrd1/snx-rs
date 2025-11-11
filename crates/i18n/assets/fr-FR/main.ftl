# Dialog and buttons
dialog-title = Paramètres VPN
button-ok = OK
button-apply = Appliquer
button-cancel = Annuler
button-fetch-info = Récupérer les informations

# Labels
label-server-address = Adresse du serveur VPN
label-auth-method = Méthode d'authentification
label-tunnel-type = Type de tunnel
label-cert-auth-type = Type de certificat
label-icon-theme = Thème d'icônes
label-username = Nom d'utilisateur
label-username-required = Le nom d'utilisateur est requis pour l'authentification
label-password = Mot de passe
label-no-dns = Ne pas modifier la configuration du résolveur DNS
label-dns-servers = Serveurs DNS supplémentaires
label-ignored-dns-servers = Serveurs DNS ignorés
label-search-domains = Domaines de recherche supplémentaires
label-ignored-domains = Domaines de recherche ignorés
label-routing-domains = Traiter les domaines de recherche reçus comme domaines de routage
label-ca-cert = Certificats racine CA du serveur
label-no-cert-check = Désactiver toutes les vérifications de certificats TLS (INSÉCURISÉ !)
label-password-factor = Index du facteur de mot de passe, 1..N
label-no-keychain = Ne pas stocker les mots de passe dans le trousseau
label-ike-lifetime = Durée de vie IPSec IKE SA, secondes
label-ike-persist = Sauvegarder la session IPSec IKE et se reconnecter automatiquement
label-no-keepalive = Désactiver les paquets keepalive IPSec
label-port-knock = Activer le port knocking NAT-T
label-no-routing = Ignorer toutes les routes acquises
label-default-routing = Définir la route par défaut via le tunnel
label-add-routes = Routes statiques supplémentaires
label-ignored-routes = Routes à ignorer
label-client-cert = Certificat client ou chemin du pilote (.pem, .pfx/.p12, .so)
label-cert-password = Mot de passe PFX ou code PIN PKCS11
label-cert-id = ID hexadécimal du certificat PKCS11
label-language = Langue
label-system-language = Par défaut du système
label-username-password = Nom d'utilisateur et mot de passe
label-auto-connect = Se connecter automatiquement au démarrage
label-ip-lease-time = Durée de bail IP personnalisée, secondes
label-disable-ipv6 = Désactiver IPv6 lorsque la route par défaut est activée
label-mtu = MTU
label-connection-profile = Profil de connexion
label-profile-name = Nom du profil
label-confirmation = Veuillez confirmer

# Tabs and expanders
tab-general = Général
tab-advanced = Avancé
expand-dns = DNS
expand-routing = Routage
expand-certificates = Certificats
expand-misc = Paramètres divers
expand-ui = Interface utilisateur

# Error messages
error-no-server-name = Aucune adresse de serveur spécifiée
error-no-auth = Aucune méthode d'authentification sélectionnée
error-file-not-exist = Le fichier n'existe pas : {$path}
error-invalid-cert-id = ID de certificat non au format hexadécimal : {$id}
error-ca-root-not-exist = Le chemin racine CA n'existe pas : {$path}
error-validation = Erreur de validation
error-user-input-canceled = Saisie utilisateur annulée
error-connection-cancelled = Connexion annulée
error-unknown-event = Événement inconnu : {$event}
error-no-service-connection = Pas de connexion au service
error-empty-input = La saisie ne peut pas être vide
error-invalid-object = Objet invalide
error-no-connector = Pas de connecteur de tunnel
error-tunnel-disconnected = Tunnel déconnecté, dernier message : {$message}
error-unexpected-reply = Réponse inattendue
error-auth-failed = Échec de l'authentification
error-no-login-type = Paramètre obligatoire manquant : login-type
error-connection-timeout = Délai de connexion dépassé
error-invalid-response = Réponse invalide !
error-cannot-send-request = Impossible d'envoyer la requête au service
error-cannot-read-reply = Impossible de lire la réponse du service
error-no-ipv4 = Pas d'adresse IPv4 pour {$server}
error-not-challenge-state = Pas un état de défi
error-no-challenge = Pas de défi dans les données
error-endless-challenges = Boucle infinie de défis de nom d'utilisateur
error-no-pkcs12 = Pas de chemin PKCS12 et mot de passe fournis
error-no-pkcs8 = Pas de chemin PKCS8 PEM fourni
error-no-pkcs11 = Pas de code PIN PKCS11 fourni
error-no-ipsec-session = Pas de session IPSEC
error-request-failed-error-code = Échec de la requête, code d'erreur : {$error_code}
error-no-root-privileges = Ce programme doit être exécuté en tant qu'utilisateur root !
error-missing-required-parameters = Paramètres obligatoires manquants : nom du serveur et/ou type de connexion !
error-missing-server-name = Paramètre obligatoire manquant : nom du serveur !
error-no-connector-for-challenge-code = Pas de connecteur pour envoyer le code de défi !
error-probing-failed = Échec de la sonde, le serveur n'est pas accessible via le port NATT !
error-invalid-sexpr = sexpr invalide : {$value}
error-invalid-value = Valeur invalide
error-udp-request-failed = Erreur lors de l'envoi de la requête UDP
error-no-tty = Pas de TTY attaché pour obtenir la saisie utilisateur
error-invalid-auth-response = Réponse d'authentification invalide
error-invalid-client-settings = Réponse des paramètres client invalide
error-invalid-otp-reply = Réponse OTP invalide
error-udp-encap-failed = Impossible de définir l'option de socket UDP_ENCAP, code d'erreur : {$code}
error-so-no-check-failed = Impossible de définir l'option de socket SO_NO_CHECK, code d'erreur : {$code}
error-keepalive-failed = Échec du keepalive
error-receive-failed = Échec de la réception
error-unknown-color-scheme = Valeur de schéma de couleur inconnue
error-cannot-determine-ip = Impossible de déterminer l'IP par défaut
error-invalid-command = Commande invalide : {$command}
error-otp-browser-failed = Impossible d'obtenir l'OTP depuis le navigateur
error-invalid-operation-mode = Mode d'opération invalide
error-invalid-tunnel-type = Type de tunnel invalide
error-invalid-cert-type = Type de certificat invalide
error-invalid-icon-theme = Thème d'icônes invalide
error-no-natt-reply = Pas de réponse NATT
error-not-implemented = Non implémenté
error-unknown-packet-type = Type de paquet inconnu
error-no-sender = Pas d'expéditeur
error-empty-ccc-session = Session CCC vide
error-identity-timeout = Délai d'attente lors de l'attente de la réponse d'identité, le type de connexion est-il correct ?
error-invalid-transport-type = Type de transport invalide

# Placeholder texts
placeholder-domains = Domaines séparés par des virgules
placeholder-ip-addresses = Adresses IP séparées par des virgules
placeholder-routes = Routes séparées par des virgules au format x.x.x.x/x
placeholder-certs = Fichiers PEM ou DER séparés par des virgules

# Tunnel types
tunnel-type-ipsec = IPSec
tunnel-type-ssl = SSL (obsolète)

# Certificate types
cert-type-none = Aucun
cert-type-pfx = Fichier PFX
cert-type-pem = Fichier PEM
cert-type-hw = Jeton matériel

# Transport types
transport-type-autodetect = Détection automatique
transport-type-kernel = UDP XFRM
transport-type-tcpt = TCPT TUN
transport-type-udp = UDP TUN

# Icon themes
icon-theme-autodetect = Détection automatique
icon-theme-dark = Sombre
icon-theme-light = Clair

# Application
app-title = Client VPN SNX-RS pour Linux
app-connection-error = Erreur de connexion
app-connection-success = Connexion réussie

# Authentication
auth-dialog-title = Facteur d'authentification VPN
auth-dialog-message = Veuillez saisir votre facteur d'authentification :

# Status dialog
status-dialog-title = Informations de connexion
status-button-copy = Copier
status-button-settings = Paramètres
status-button-connect = Se connecter
status-button-disconnect = Se déconnecter

# Tray menu
tray-menu-connect = Se connecter
tray-menu-disconnect = Se déconnecter
tray-menu-status = État de la connexion...
tray-menu-settings = Paramètres...
tray-menu-about = À propos...
tray-menu-exit = Quitter

# Connection info
info-connected-since = Connecté depuis
info-server-name = Nom du serveur
info-user-name = Nom d'utilisateur
info-login-type = Type de connexion
info-tunnel-type = Type de tunnel
info-transport-type = Type de transport
info-ip-address = Adresse IP
info-dns-servers = Serveurs DNS
info-search-domains = Domaines de recherche
info-interface = Interface
info-dns-configured = DNS configuré
info-routing-configured = Routage configuré
info-default-route = Route par défaut

# CLI Messages
cli-identity-provider-auth = Pour l'authentification via le fournisseur d'identité, ouvrez l'URL suivante dans votre navigateur :
cli-tunnel-connected = Tunnel connecté, appuyez sur Ctrl+C pour quitter.
cli-tunnel-disconnected = Tunnel déconnecté
cli-another-instance-running = Une autre instance de snx-rs est déjà en cours d'exécution
cli-app-terminated = Application terminée par un signal

# Connection Messages
connection-connected-to = Connecté à {$server}

# Languages
language-cs-CZ = Tchèque
language-da-DK = Danois
language-de-DE = Allemand
language-en-US = Anglais
language-es-ES = Espagnol
language-fi-FI = Finnois
language-fr-FR = Français
language-it-IT = Italien
language-nl-NL = Néerlandais
language-no-NO = Norvégien
language-pl-PL = Polonais
language-pt-PT = Portugais
language-pt-BR = Portugais brésilien
language-ru-RU = Russe
language-sk-SK = Slovaque
language-sv-SE = Suédois

# Connection status messages
connection-status-disconnected = Déconnecté
connection-status-connecting = Connexion en cours
connection-status-connected-since = Connecté depuis: {$since}
connection-status-mfa-pending = En attente de MFA: {$mfa_type}

# Login options
login-options-server-address = Adresse du serveur
login-options-server-ip = IP du serveur
login-options-client-enabled = Client activé
login-options-supported-protocols = Protocoles pris en charge
login-options-preferred-protocol = Protocole préféré
login-options-tcpt-port = Port TCPT
login-options-natt-port = Port NATT
login-options-internal-ca-fingerprint = Empreinte CA interne

# Connection profiles
profile-new = Nouveau
profile-rename = Renommer
profile-delete = Supprimer
profile-delete-prompt = Êtes-vous sûr de vouloir supprimer le profil sélectionné ?
profile-default-name = Par défaut
profile-new-title = Nouveau profil de connexion
profile-rename-title = Renommer le profil de connexion
