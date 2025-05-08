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

# Tabs and expanders
tab-general = General
tab-advanced = Avanzado
expand-dns = DNS
expand-routing = Enrutamiento
expand-certificates = Certificados
expand-misc = Configuración adicional

# Error messages
error-no-server = No se ha especificado dirección de servidor
error-no-auth = No se ha seleccionado método de autenticación
error-file-not-exist = El archivo no existe: {$path}
error-invalid-cert-id = ID de certificado no en formato hexadecimal: {$id}
error-ca-root-not-exist = La ruta raíz CA no existe: {$path}
error-validation = Error de validación
error-user-input-canceled = Entrada de usuario cancelada
error-connection-canceled = Conexión cancelada
error-unknown-event = Evento desconocido: {$event}
error-no-service-connection = No hay conexión al servicio
error-empty-input = La entrada no puede estar vacía

# New error messages
error-invalid-object = Objeto inválido
error-no-connector = No hay conector de túnel
error-connection-cancelled = Conexión cancelada
error-tunnel-disconnected = Túnel desconectado, último mensaje: {$message}
error-unexpected-reply = Respuesta inesperada
error-auth-failed = Error de autenticación
error-no-server-name = Falta el parámetro obligatorio: server-name
error-no-login-type = Falta el parámetro obligatorio: login-type
error-connection-timeout = Tiempo de conexión agotado
error-invalid-response = Respuesta inválida
error-cannot-send-request = No se puede enviar la solicitud al servicio
error-cannot-read-reply = No se puede leer la respuesta del servicio
error-no-ipv4 = No hay dirección IPv4 para {$server}
error-not-challenge-state = No es un estado de desafío
error-no-challenge = No hay desafío en los datos
error-endless-challenges = Bucle infinito de desafíos de nombre de usuario
error-no-pkcs12 = No se proporcionó ruta PKCS12 y contraseña
error-no-pkcs8 = No se proporcionó ruta PKCS8 PEM
error-no-pkcs11 = No se proporcionó código PIN PKCS11
error-no-ipsec-session = No hay sesión IPSEC

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
