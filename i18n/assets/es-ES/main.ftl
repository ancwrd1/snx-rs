# Dialog and buttons
dialog-title = Configuración de VPN
button-ok = OK
button-apply = Aplicar
button-cancel = Cancelar
button-fetch-info = Obtener información

# Labels
label-server-address = Dirección del servidor VPN
label-auth-method = Método de autenticación
label-tunnel-type = Tipo de túnel
label-cert-auth-type = Tipo de certificado
label-icon-theme = Tema de iconos
label-username = Nombre de usuario
label-username-required = Se requiere nombre de usuario para la autenticación
label-password = Contraseña
label-no-dns = No modificar la configuración del resolvedor DNS
label-dns-servers = Servidores DNS adicionales
label-ignored-dns-servers = Servidores DNS ignorados
label-search-domains = Dominios de búsqueda adicionales
label-ignored-domains = Dominios de búsqueda ignorados
label-routing-domains = Tratar los dominios de búsqueda recibidos como dominios de enrutamiento
label-ca-cert = Certificados raíz CA del servidor
label-no-cert-check = Desactivar todas las comprobaciones de certificados TLS (¡INSEGURO!)
label-password-factor = Índice del factor de contraseña, 1..N
label-no-keychain = No almacenar contraseñas en el llavero
label-ike-lifetime = Tiempo de vida de IPSec IKE SA, segundos
label-ike-persist = Guardar sesión IPSec IKE y reconectar automáticamente
label-no-keepalive = Desactivar paquetes keepalive IPSec
label-port-knock = Activar port knocking NAT-T
label-no-routing = Ignorar todas las rutas adquiridas
label-default-routing = Establecer ruta predeterminada a través del túnel
label-add-routes = Rutas estáticas adicionales
label-ignored-routes = Rutas a ignorar
label-client-cert = Certificado de cliente o ruta del controlador (.pem, .pfx/.p12, .so)
label-cert-password = Contraseña PFX o PIN PKCS11
label-cert-id = ID hexadecimal del certificado PKCS11
label-language = Idioma
label-system-language = Predeterminado del sistema
label-username-password = Nombre de usuario y contraseña
label-auto-connect = Conectar automáticamente al inicio
label-ip-lease-time = Tiempo de concesión IP personalizado, segundos
label-disable-ipv6 = Desactivar IPv6 cuando la ruta predeterminada esté habilitada
label-mtu = MTU

# Tabs and expanders
tab-general = General
tab-advanced = Avanzado
expand-dns = DNS
expand-routing = Enrutamiento
expand-certificates = Certificados
expand-misc = Configuración adicional
expand-ui = Interfaz de usuario

# Error messages
error-no-server-name = No se ha especificado dirección de servidor
error-no-auth = No se ha seleccionado método de autenticación
error-file-not-exist = El archivo no existe: {$path}
error-invalid-cert-id = ID de certificado no en formato hexadecimal: {$id}
error-ca-root-not-exist = La ruta raíz CA no existe: {$path}
error-validation = Error de validación
error-user-input-canceled = Entrada de usuario cancelada
error-connection-cancelled = Conexión cancelada
error-unknown-event = Evento desconocido: {$event}
error-no-service-connection = No hay conexión al servicio
error-empty-input = La entrada no puede estar vacía
error-invalid-response = ¡Respuesta inválida!
error-invalid-object = Objeto inválido
error-no-connector = No hay conector de túnel
error-tunnel-disconnected = Túnel desconectado, último mensaje: {$message}
error-unexpected-reply = Respuesta inesperada
error-auth-failed = Error de autenticación
error-no-login-type = Falta el parámetro obligatorio: login-type
error-connection-timeout = Tiempo de conexión agotado
error-request-failed-error-code = Error en la solicitud, código de error: {$error_code}
error-no-root-privileges = ¡Este programa debe ejecutarse como usuario root!
error-missing-required-parameters = ¡Faltan parámetros obligatorios: nombre del servidor y/o tipo de inicio de sesión!
error-missing-server-name = ¡Falta el parámetro obligatorio: nombre del servidor!
error-no-connector-for-challenge-code = ¡No hay conector para enviar el código de desafío!
error-probing-failed = ¡Error en la prueba, el servidor no es accesible a través del puerto NATT!
error-invalid-sexpr = sexpr inválido: {$value}
error-invalid-value = Valor inválido
error-udp-request-failed = Error al enviar la solicitud UDP
error-no-tty = No hay TTY conectado para obtener la entrada del usuario
error-invalid-auth-response = Respuesta de autenticación inválida
error-invalid-client-settings = Respuesta de configuración del cliente inválida
error-invalid-otp-reply = Respuesta OTP inválida
error-udp-encap-failed = No se pudo establecer la opción de socket UDP_ENCAP, código de error: {$code}
error-so-no-check-failed = No se pudo establecer la opción de socket SO_NO_CHECK, código de error: {$code}
error-keepalive-failed = Error en keepalive
error-receive-failed = Error en la recepción
error-unknown-color-scheme = Valor de esquema de color desconocido
error-cannot-determine-ip = No se puede determinar la IP predeterminada
error-invalid-command = Comando inválido: {$command}
error-otp-browser-failed = No se pudo obtener el OTP desde el navegador
error-invalid-operation-mode = Modo de operación inválido
error-invalid-tunnel-type = Tipo de túnel inválido
error-invalid-cert-type = Tipo de certificado inválido
error-invalid-icon-theme = Tema de iconos inválido
error-no-natt-reply = No hay respuesta NATT
error-not-implemented = No implementado
error-unknown-packet-type = Tipo de paquete desconocido
error-no-sender = No hay remitente
error-empty-ccc-session = Sesión CCC vacía
error-identity-timeout = Tiempo de espera al esperar la respuesta de identidad, ¿es correcto el tipo de inicio de sesión?
error-not-challenge-state = No es un estado de desafío
error-no-pkcs8 = No se ha proporcionado la ruta PEM PKCS8
error-no-pkcs12 = No se ha proporcionado la ruta y contraseña PKCS12
error-no-pkcs11 = No se ha proporcionado el PIN PKCS11
error-no-ipv4 = No hay dirección IPv4 para {$server}
error-no-ipsec-session = No hay sesión IPSEC
error-no-challenge = No hay desafío en la carga útil
error-endless-challenges = Bucle infinito de desafíos de nombre de usuario
error-cannot-send-request = No se puede enviar la solicitud al servicio
error-cannot-read-reply = No se puede leer la respuesta del servicio
error-invalid-transport-type = Tipo de transporte no válido

# Placeholder texts
placeholder-domains = Dominios separados por comas
placeholder-ip-addresses = Direcciones IP separadas por comas
placeholder-routes = Rutas separadas por comas en formato x.x.x.x/x
placeholder-certs = Archivos PEM o DER separados por comas

# Tunnel types
tunnel-type-ipsec = IPSec
tunnel-type-ssl = SSL (obsoleto)

# Certificate types
cert-type-none = Ninguno
cert-type-pfx = Archivo PFX
cert-type-pem = Archivo PEM
cert-type-hw = Token de hardware

# Transport types
transport-type-auto-detect = Detección automática
transport-type-kernel = Kernel XFRM
transport-type-tcpt = TCPT TUN
transport-type-udp = UDP TUN

# Icon themes
icon-theme-auto = Auto
icon-theme-dark = Oscuro
icon-theme-light = Claro

# Connection info
info-connected-since = Conectado desde
info-server-name = Nombre del servidor
info-user-name = Nombre de usuario
info-login-type = Tipo de inicio de sesión
info-tunnel-type = Tipo de túnel
info-transport-type = Tipo de transporte
info-ip-address = Dirección IP
info-dns-servers = Servidores DNS
info-search-domains = Dominios de búsqueda
info-interface = Interfaz
info-dns-configured = DNS configurado
info-routing-configured = Enrutamiento configurado
info-default-route = Ruta predeterminada

# Application
app-title = Cliente VPN SNX-RS para Linux
app-connection-error = Error de conexión
app-connection-success = Conexión exitosa

# Authentication
auth-dialog-title = Factor de autenticación VPN
auth-dialog-message = Introduzca su factor de autenticación:

# Status dialog
status-dialog-title = Información de conexión
status-button-copy = Copiar
status-button-settings = Configuración
status-button-connect = Conectar
status-button-disconnect = Desconectar

# Tray menu
tray-menu-connect = Conectar
tray-menu-disconnect = Desconectar
tray-menu-status = Estado de conexión...
tray-menu-settings = Configuración...
tray-menu-about = Acerca de...
tray-menu-exit = Salir

# CLI Messages
cli-identity-provider-auth = Para la autenticación a través del proveedor de identidad, abra la siguiente URL en su navegador:
cli-tunnel-connected = Túnel conectado, presione Ctrl+C para salir.
cli-tunnel-disconnected = Túnel desconectado
cli-another-instance-running = Ya hay otra instancia de snx-rs en ejecución
cli-app-terminated = Aplicación terminada por señal

# Connection Messages
connection-connected-to = Conectado a {$server}

# Languages
language-cs-CZ = Checo
language-da-DK = Danés
language-de-DE = Alemán
language-en-US = Inglés
language-es-ES = Español
language-fi-FI = Finés
language-fr-FR = Francés
language-it-IT = Italiano
language-nl-NL = Neerlandés
language-no-NO = Noruego
language-pl-PL = Polaco
language-pt-PT = Portugués
language-ru-RU = Ruso
language-sk-SK = Eslovaco
language-sv-SE = Sueco
language-pt-BR = Portugués brasileño

# Connection status messages
connection-status-disconnected = Desconectado
connection-status-connecting = Conectando
connection-status-connected-since = Conectado desde: {$since}
connection-status-mfa-pending = Esperando MFA: {$mfa_type}

# Login options
login-options-server-address = Dirección del servidor
login-options-server-ip = IP del servidor
login-options-client-enabled = Cliente habilitado
login-options-supported-protocols = Protocolos soportados
login-options-preferred-protocol = Protocolo preferido
login-options-tcpt-port = Puerto TCPT
login-options-natt-port = Puerto NATT
login-options-internal-ca-fingerprint = Huella digital CA interna
