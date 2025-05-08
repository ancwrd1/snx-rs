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
