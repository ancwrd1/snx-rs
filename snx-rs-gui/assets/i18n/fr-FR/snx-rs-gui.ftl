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

# Tabs and expanders
tab-general = Général
tab-advanced = Avancé
expand-dns = DNS
expand-routing = Routage
expand-certificates = Certificats
expand-misc = Paramètres divers

# Error messages
error-no-server = Aucune adresse de serveur spécifiée
error-no-auth = Aucune méthode d'authentification sélectionnée
error-file-not-exist = Le fichier n'existe pas : {$path}
error-invalid-cert-id = ID de certificat non au format hexadécimal : {$id}
error-ca-root-not-exist = Le chemin racine CA n'existe pas : {$path}
error-validation = Erreur de validation

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

# Icon themes
icon-theme-auto = Auto
icon-theme-dark = Sombre
icon-theme-light = Clair