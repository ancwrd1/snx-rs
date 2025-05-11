# Dialog and buttons
dialog-title = Configurações da VPN
button-ok = OK
button-apply = Aplicar
button-cancel = Cancelar
button-fetch-info = Buscar informações

# Labels
label-server-address = Endereço do servidor de VPN
label-auth-method = Método de autenticação
label-tunnel-type = Tipo de túnel
label-cert-auth-type = Tipo de autenticação do certificado
label-icon-theme = Tema dos ícones
label-username = Usuário
label-password = Senha
label-no-dns = Não modificar a configuração de DNS
label-dns-servers = Servidores DNS adicionais
label-ignored-dns-servers = Servidores DNS ignorados
label-search-domains = Domínios adicionais para busca
label-ignored-domains = Domínios adicionais ignorados
label-routing-domains = Trate os domínios recebidos como domínios de roteamento
label-ca-cert = Certificados raiz do servidor
label-no-cert-check = Desabilitar todos as verificações de certificados TLS (PERIGOSO!)
label-password-factor = Índice do fator de senha, 1..N
label-no-keychain = Não armazenar senhas no chaveiro
label-ike-lifetime = Tempo de vida da IPSec IKE SA - em segundos
label-ike-persist = Salvar sessão IPSec IKE e reconectar automaticamente
label-no-keepalive = Desabilitar pacotes keepalive IPSec
label-port-knock = Habilitar port knocking no NAT-T
label-no-routing = Ignorar todas as rotas recebidas
label-default-routing = Habilitar rota padrão via túnel
label-add-routes = Rotas estáticas adicionais
label-ignored-routes = Rotas para ignorar
label-client-cert = Caminho para certificado de cliente ou caminho do driver (.pem, .pfx/.p12, .so)
label-cert-password = Senha do arquivo PFX ou pin PKCS11
label-cert-id = ID hexadecimal do certificado PKCS11
label-language = Idioma
label-system-language = Padrão do sistema
label-username-password = Nome de usuário e senha

# Tabs and expanders
tab-general = Geral
tab-advanced = Avançado
expand-dns = DNS
expand-routing = Roteamento
expand-certificates = Certificados
expand-misc = Outras configurações

# Error messages
error-no-server-name = Nenhum endereço de servidor especificado
error-no-auth = Nenhum método de autenticação selecionado
error-file-not-exist = O arquivo não existe: {$path}
error-invalid-cert-id = ID do certificado não está em formato hexadecimal: {$id}
error-ca-root-not-exist = O caminho do certificado raiz não existe: {$path}
error-validation = Erro de validação
error-user-input-canceled = Entrada de usuário cancelada
error-connection-cancelled = Conexão cancelada
error-unknown-event = Evento desconhecido: {$event}
error-no-service-connection = Sem conexão ao serviço
error-empty-input = O campo não pode estar vazio
error-invalid-object = Objeto inválido
error-no-connector = Sem conector de túnel
error-tunnel-disconnected = Túnel desconectado, última mensagem: {$message}
error-unexpected-reply = Resposta inesperada
error-auth-failed = A autenticação falhou
error-no-login-type = Falta um parâmetro obrigatório: login-type
error-connection-timeout = Tempo limite da conexão
error-invalid-response = Resposta inválida!
error-cannot-send-request = Impossível enviar pedido ao serviço
error-cannot-read-reply = Impossível ler a resposta do serviço
error-no-ipv4 = Sem endereço IPv4 para o servidor {$server}
error-not-challenge-state = Não é um estado de negociação
error-no-challenge = Sem negociação no payload
error-endless-challenges = Loop infinito na negociação do nome de usuário
error-no-pkcs12 = Não foram informados o caminho PKCS12 e a senha do certificado
error-no-pkcs8 = Não foi informado um caminho para o certificado PEM PKCS8
error-no-pkcs11 = Não foi informado um PIN PKCS11
error-no-ipsec-session = Sem sessão IPSec
error-request-failed-error-code = Falha na requisição, código de erro: {$error_code}
error-no-root-privileges = Este programa deve ser executado como usuário root!
error-missing-required-parameters = Parâmetros obrigatórios ausentes: nome do servidor e/ou tipo de acesso!
error-missing-server-name = Parâmetro obrigatório ausente: nome do servidor!
error-no-connector-for-challenge-code = Sem conector para enviar o código de desafio!
error-probing-failed = Sondagem falhou, o servidor não está acessível através da porta NATT!
error-invalid-sexpr = sexpr inválido: {$value}
error-invalid-value = Valor inválido
error-udp-request-failed = Erro ao enviar requisição UDP
error-no-tty = Sem TTY conectado para entrada do usuário
error-invalid-auth-response = Resposta de autenticação inválida
error-invalid-client-settings = Configurações do cliente inválidas
error-invalid-otp-reply = Resposta OTP inválida
error-udp-encap-failed = Não foi possível definir a opção de socket UDP_ENCAP, código de erro: {$code}
error-so-no-check-failed = Não foi possível definir a opção de socket SO_NO_CHECK, código de erro: {$code}
error-keepalive-failed = Keepalive falhou
error-receive-failed = Recebimento falhou
error-unknown-color-scheme = Valor de esquema de cores desconhecido
error-cannot-determine-ip = Não foi possível determinar o IP padrão
error-invalid-command = Comando inválido: {$command}
error-otp-browser-failed = Não foi possível obter o OTP do navegador
error-invalid-operation-mode = Modo de operação inválido
error-invalid-tunnel-type = Tipo de túnel inválido
error-invalid-cert-type = Tipo de certificado inválido
error-invalid-icon-theme = Tema de ícones inválido
error-no-natt-reply = Sem resposta NATT
error-not-implemented = Não implementado
error-unknown-packet-type = Tipo de pacote desconhecido
error-no-sender = Sem remetente
error-empty-ccc-session = Sessão CCC vazia
error-identity-timeout = Timeout ao aguardar resposta de identidade, o tipo de acesso está correto?

# Placeholder texts
placeholder-domains = Domínios separados por vírgulas
placeholder-ip-addresses = Endereços IP separados por vírgulas
placeholder-routes = Rotas separadas por vírgulas no formato x.x.x.x/x
placeholder-certs = Arquivos PEM ou DER separados por vírgulas

# Tunnel types
tunnel-type-ipsec = IPSec
tunnel-type-ssl = SSL (obsoleto)

# Certificate types
cert-type-none = Nenhum
cert-type-pfx = Arquivo PFX
cert-type-pem = Arquivo PEM
cert-type-hw = Token em hardware

# Icon themes
icon-theme-auto = Automático
icon-theme-dark = Escuro
icon-theme-light = Claro

# Application
app-title = Cliente VPN SNX-RS para Linux
app-connection-error = Erro de conexã́o
app-connection-success = Conexão estabelecida com sucesso

# Authentication
auth-dialog-title = Fator de autenticação da VPN
auth-dialog-message = Por gentileza, entre com seu fator de autenticação:

# Status dialog
status-dialog-title = Informação da conexão
status-button-copy = Copiar
status-button-settings = Configurações
status-button-connect = Conectar
status-button-disconnect = Desconectar

# Tray menu
tray-menu-connect = Conectar
tray-menu-disconnect = Desconectar
tray-menu-status = Estado da conexão...
tray-menu-settings = Configurações...
tray-menu-about = Sobre...
tray-menu-exit = Sair

# Connection info
info-connected-since = Conectado desde
info-server-name = Nome do servidor
info-user-name = Usuário
info-login-type = Tipo de login
info-tunnel-type = Tipo de túnel
info-transport-type = Tipo de transporte
info-ip-address = Endereço IP
info-dns-servers = Servidores DNS
info-search-domains = Domínios para busca
info-interface = Interface
info-dns-configured = DNS configurado
info-routing-configured = Roteamento configurado
info-default-route = Rota padrão

# CLI Messages
cli-identity-provider-auth = Para autenticação com o provedor de identidade, abra a seguinte URL no seu navegador:
cli-tunnel-connected = Túnel conectado, pressione Control+c para sair.
cli-tunnel-disconnected = Túnel desconectado
cli-another-instance-running = Outra instância do SNX-RS está em execução
cli-app-terminated = A aplicação terminou devido a um sinal

# Connection Messages
connection-connected-to = Conectado à {$server}

# Languages
language-cs-CZ = Checo
language-da-DK = Dinamarquês
language-de-DE = Alemão
language-en-US = Inglês
language-es-ES = Espanhol
language-fi-FI = Finlandês
language-fr-FR = Francês
language-it-IT = Italiano
language-nl-NL = Holandês
language-no-NO = Norueguês
language-pl-PL = Polonês
language-pt-PT = Português de portugal
language-pt-BR = Português Brasileiro
language-ru-RU = Russo
language-sk-SK = Esloveno
language-sv-SE = Sueco

# Connection status messages
connection-status-disconnected = Desconectado
connection-status-connecting = Conexão em progresso
connection-status-connected-since = Conectado desde: {$since}
connection-status-mfa-pending = Autenticação multifator pendente: {$mfa_type}

# Login options
login-options-server-address = Endereço do servidor
login-options-server-ip = IP do servidor
login-options-client-enabled = Cliente habilitado
login-options-supported-protocols = Protocolos suportados
login-options-preferred-protocol = Protocolos preferidos
login-options-tcpt-port = Porta TCPT
login-options-natt-port = Porta NAT-T
login-options-internal-ca-fingerprint = Impressão digital da CA interna
