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
label-username-required = O nome de utilizador é necessário para autenticação
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
label-language = Idioma
label-system-language = Predefinição do sistema
label-username-password = Nome de utilizador e palavra-passe
label-auto-connect = Ligar automaticamente ao iniciar
label-ip-lease-time = Tempo de concessão de IP personalizado, segundos
label-disable-ipv6 = Desativar o IPv6 quando a rota predefinida estiver ativa
label-mtu = MTU
label-connection-profile = Perfil de ligação
label-profile-name = Nome do perfil
label-confirmation = Por favor, confirme
label-mobile-access = Acesso móvel

# Tabs and expanders
tab-general = Geral
tab-advanced = Avançado
expand-dns = DNS
expand-routing = Encaminhamento
expand-certificates = Certificados
expand-misc = Definições adicionais
expand-ui = Interface do utilizador

# Error messages
error-no-server-name = Nenhum endereço de servidor especificado
error-no-auth = Nenhum método de autenticação selecionado
error-file-not-exist = O ficheiro não existe: {$path}
error-invalid-cert-id = ID do certificado não está em formato hexadecimal: {$id}
error-ca-root-not-exist = O caminho raiz CA não existe: {$path}
error-validation = Erro de validação
error-user-input-canceled = Entrada do utilizador cancelada
error-connection-cancelled = Ligação cancelada
error-unknown-event = Evento desconhecido: {$event}
error-no-service-connection = Sem ligação ao serviço
error-empty-input = A entrada não pode estar vazia
error-invalid-object = Objeto inválido
error-no-connector = Sem conector de túnel
error-tunnel-disconnected = Túnel desligado, última mensagem: {$message}
error-unexpected-reply = Resposta inesperada
error-auth-failed = Autenticação falhou
error-no-login-type = Parâmetro obrigatório em falta: login-type
error-connection-timeout = Tempo limite de ligação
error-invalid-response = Resposta inválida!
error-cannot-acquire-access-cookie = Não é possível obter a cookie de acesso!
error-cannot-send-request = Não é possível enviar pedido ao serviço
error-cannot-read-reply = Não é possível ler resposta do serviço
error-no-ipv4 = Sem endereço IPv4 para {$server}
error-not-challenge-state = Não é um estado de desafio
error-no-challenge = Sem desafio nos dados
error-endless-challenges = Ciclo infinito de desafios de nome de utilizador
error-no-pkcs12 = Sem caminho PKCS12 e palavra-passe fornecidos
error-no-pkcs8 = Sem caminho PKCS8 PEM fornecido
error-no-pkcs11 = Sem PIN PKCS11 fornecido
error-no-ipsec-session = Sem sessão IPSEC
error-request-failed-error-code = Pedido falhou, código de erro: {$error_code}
error-no-root-privileges = Este programa deve ser executado como utilizador root!
error-missing-required-parameters = Parâmetros obrigatórios em falta: nome do servidor e/ou tipo de acesso!
error-missing-server-name = Parâmetro obrigatório em falta: nome do servidor!
error-no-connector-for-challenge-code = Sem conector para enviar o código de desafio!
error-probing-failed = Sondagem falhou, o servidor não está acessível através da porta NATT!
error-invalid-sexpr = sexpr inválido: {$value}
error-invalid-value = Valor inválido
error-udp-request-failed = Erro ao enviar pedido UDP
error-no-tty = Sem TTY ligado para entrada do utilizador
error-invalid-auth-response = Resposta de autenticação inválida
error-invalid-client-settings = Definições do cliente inválidas
error-invalid-otp-reply = Resposta OTP inválida
error-udp-encap-failed = Não é possível definir a opção de socket UDP_ENCAP, código de erro: {$code}
error-so-no-check-failed = Não é possível definir a opção de socket SO_NO_CHECK, código de erro: {$code}
error-keepalive-failed = Keepalive falhou
error-receive-failed = Receção falhou
error-unknown-color-scheme = Valor de esquema de cores desconhecido
error-cannot-determine-ip = Não é possível determinar o IP predefinido
error-invalid-command = Comando inválido: {$command}
error-otp-browser-failed = Não é possível obter o OTP do navegador
error-invalid-operation-mode = Modo de operação inválido
error-invalid-tunnel-type = Tipo de túnel inválido
error-invalid-cert-type = Tipo de certificado inválido
error-invalid-icon-theme = Tema de ícones inválido
error-no-natt-reply = Sem resposta NATT
error-not-implemented = Não implementado
error-unknown-packet-type = Tipo de pacote desconhecido
error-no-sender = Sem remetente
error-empty-ccc-session = Sessão CCC vazia
error-identity-timeout = Tempo limite ao aguardar resposta de identidade, o tipo de acesso está correto?
error-invalid-transport-type = Tipo de transporte inválido
error-certificate-verify-failed = Validação do certificado TLS falhou. O certificado do servidor é inválido, expirou ou não é de confiança.

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

# Transport types
transport-type-autodetect = Deteção automática
transport-type-kernel = UDP XFRM
transport-type-tcpt = TCPT TUN
transport-type-udp = UDP TUN

# Icon themes
icon-theme-autodetect = Deteção automática
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

# CLI Messages
cli-identity-provider-auth = Para autenticação através do fornecedor de identidade, abra o seguinte URL no seu navegador:
cli-tunnel-connected = Túnel conectado, prima Ctrl+C para sair.
cli-tunnel-disconnected = Túnel desconectado
cli-another-instance-running = Outra instância do snx-rs já está em execução
cli-app-terminated = Aplicação terminada por sinal
cli-mobile-access-auth = Para autenticação de acesso móvel, inicie sessão usando o seguinte URL, depois procure uma palavra-passe de utilizador em formato hexadecimal no código-fonte HTML da página e introduza-a aqui:

# Connection Messages
connection-connected-to = Ligado a {$server}

# Languages
language-cs-CZ = Checo
language-da-DK = Dinamarquês
language-de-DE = Alemão
language-en-US = Inglês
language-es-ES = Espanhol
language-fi-FI = Finlandês
language-fr-FR = Francês
language-hr-HR = Croata
language-it-IT = Italiano
language-nl-NL = Holandês
language-no-NO = Norueguês
language-pl-PL = Polaco
language-pt-PT = Português
language-pt-BR = Português Brasileiro
language-ru-RU = Russo
language-sk-SK = Eslovaco
language-sv-SE = Sueco

# Connection status messages
connection-status-disconnected = Desligado
connection-status-connecting = A ligar
connection-status-connected-since = Ligado desde: {$since}
connection-status-mfa-pending = À espera de MFA: {$mfa_type}

# Login options
login-options-server-address = Endereço do servidor
login-options-server-ip = IP do servidor
login-options-client-enabled = Cliente ativado
login-options-supported-protocols = Protocolos suportados
login-options-preferred-protocol = Protocolo preferido
login-options-tcpt-port = Porta TCPT
login-options-natt-port = Porta NATT
login-options-internal-ca-fingerprint = Impressão digital CA interna

# Connection profiles
profile-new = Novo
profile-rename = Mudar nome
profile-delete = Eliminar
profile-delete-prompt = Tem a certeza de que deseja eliminar o perfil selecionado?
profile-default-name = Predefinição
profile-new-title = Novo perfil de ligação
profile-rename-title = Mudar nome do perfil de ligação
