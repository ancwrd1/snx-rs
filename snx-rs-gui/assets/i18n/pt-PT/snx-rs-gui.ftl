# Dialog and buttons
dialog-title = Definições VPN
button-ok = OK
button-apply = Aplicar
button-cancel = Cancelar
button-fetch-info = Obter informações

# Labels
label-server-address = Endereço do servidor VPN
label-auth-method = Método de autenticação
label-tunnel-type = Tipo de túnel
label-cert-auth-type = Tipo de certificado
label-icon-theme = Tema de ícones
label-username = Nome de utilizador
label-password = Palavra-passe
label-no-dns = Não alterar a configuração do resolvedor DNS
label-dns-servers = Servidores DNS adicionais
label-ignored-dns-servers = Servidores DNS ignorados
label-search-domains = Domínios de pesquisa adicionais
label-ignored-domains = Domínios de pesquisa ignorados
label-routing-domains = Tratar domínios de pesquisa recebidos como domínios de encaminhamento
label-ca-cert = Certificados raiz CA do servidor
label-no-cert-check = Desativar todas as verificações de certificados TLS (INSEGURO!)
label-password-factor = Índice do fator de palavra-passe, 1..N
label-no-keychain = Não armazenar palavras-passe no porta-chaves
label-ike-lifetime = Tempo de vida IPSec IKE SA, segundos
label-ike-persist = Guardar sessão IPSec IKE e reconectar automaticamente
label-no-keepalive = Desativar pacotes keepalive IPSec
label-port-knock = Ativar port knocking NAT-T
label-no-routing = Ignorar todas as rotas adquiridas
label-default-routing = Definir rota predefinida através do túnel
label-add-routes = Rotas estáticas adicionais
label-ignored-routes = Rotas a ignorar
label-client-cert = Certificado do cliente ou caminho do controlador (.pem, .pfx/.p12, .so)
label-cert-password = Palavra-passe PFX ou PIN PKCS11
label-cert-id = ID hexadecimal do certificado PKCS11

# Tabs and expanders
tab-general = Geral
tab-advanced = Avançado
expand-dns = DNS
expand-routing = Encaminhamento
expand-certificates = Certificados
expand-misc = Definições adicionais

# Error messages
error-no-server = Nenhum endereço de servidor especificado
error-no-auth = Nenhum método de autenticação selecionado
error-file-not-exist = O ficheiro não existe: {$path}
error-invalid-cert-id = ID do certificado não está em formato hexadecimal: {$id}
error-ca-root-not-exist = O caminho raiz CA não existe: {$path}
error-validation = Erro de validação

# Placeholder texts
placeholder-domains = Domínios separados por vírgulas
placeholder-ip-addresses = Endereços IP separados por vírgulas
placeholder-routes = Rotas separadas por vírgulas no formato x.x.x.x/x
placeholder-certs = Ficheiros PEM ou DER separados por vírgulas

# Tunnel types
tunnel-type-ipsec = IPSec
tunnel-type-ssl = SSL (obsoleto)

# Certificate types
cert-type-none = Nenhum
cert-type-pfx = Ficheiro PFX
cert-type-pem = Ficheiro PEM
cert-type-hw = Token de hardware

# Icon themes
icon-theme-auto = Automático
icon-theme-dark = Escuro
icon-theme-light = Claro

# Connection info
info-connected-since = Conectado desde
info-server-name = Nome do servidor
info-user-name = Nome de utilizador
info-login-type = Tipo de início de sessão
info-tunnel-type = Tipo de túnel
info-transport-type = Tipo de transporte
info-ip-address = Endereço IP
info-dns-servers = Servidores DNS
info-search-domains = Domínios de pesquisa
info-interface = Interface
info-dns-configured = DNS configurado
info-routing-configured = Encaminhamento configurado
info-default-route = Rota predefinida

# Application
app-title = Cliente VPN SNX-RS para Linux
app-connection-error = Erro de ligação
app-connection-success = Ligação bem-sucedida

# Authentication
auth-dialog-title = Fator de autenticação VPN
auth-dialog-message = Introduza o seu fator de autenticação:

# Status dialog
status-dialog-title = Informação de ligação
status-button-copy = Copiar
status-button-settings = Definições
status-button-connect = Ligar
status-button-disconnect = Desligar

# Tray menu
tray-menu-connect = Ligar
tray-menu-disconnect = Desligar
tray-menu-status = Estado da ligação...
tray-menu-settings = Definições...
tray-menu-about = Acerca de...
tray-menu-exit = Sair
