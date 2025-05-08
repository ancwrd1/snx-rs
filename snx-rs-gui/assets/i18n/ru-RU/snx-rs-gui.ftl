# Dialog and buttons
dialog-title = Настройки VPN
button-ok = OK
button-apply = Применить
button-cancel = Отмена
button-fetch-info = Получить информацию

# Labels
label-server-address = Адрес VPN-сервера
label-auth-method = Метод аутентификации
label-tunnel-type = Тип туннеля
label-cert-auth-type = Тип сертификата
label-icon-theme = Тема иконок
label-username = Имя пользователя
label-password = Пароль
label-no-dns = Не изменять настройки DNS-серверов
label-dns-servers = Дополнительные DNS-серверы
label-ignored-dns-servers = Игнорируемые DNS-серверы
label-search-domains = Дополнительные домены поиска
label-ignored-domains = Игнорируемые домены поиска
label-routing-domains = Использовать полученные домены поиска как маршрутизируемые
label-ca-cert = Корневые сертификаты CA сервера
label-no-cert-check = Отключить все проверки TLS-сертификатов (НЕБЕЗОПАСНО!)
label-password-factor = Индекс фактора пароля, 1..N
label-no-keychain = Не хранить пароли в хранилище ключей
label-ike-lifetime = Время жизни IPSec IKE SA, секунды
label-ike-persist = Сохранять сессию IPSec IKE и переподключаться автоматически
label-no-keepalive = Отключить пакеты keepalive IPSec
label-port-knock = Включить NAT-T port knocking
label-no-routing = Игнорировать все полученные маршруты
label-default-routing = Установить маршрут по умолчанию через туннель
label-add-routes = Дополнительные статические маршруты
label-ignored-routes = Маршруты для игнорирования
label-client-cert = Клиентский сертификат или путь к драйверу (.pem, .pfx/.p12, .so)
label-cert-password = Пароль PFX или PIN-код PKCS11
label-cert-id = Шестнадцатеричный ID сертификата PKCS11

# Tabs and expanders
tab-general = Основные
tab-advanced = Дополнительно
expand-dns = DNS
expand-routing = Маршрутизация
expand-certificates = Сертификаты
expand-misc = Прочие настройки

# Error messages
error-no-server = Не указан адрес сервера
error-no-auth = Не выбран метод аутентификации
error-file-not-exist = Файл не существует: {$path}
error-invalid-cert-id = ID сертификата не в шестнадцатеричном формате: {$id}
error-ca-root-not-exist = Путь к корневому сертификату CA не существует: {$path}
error-validation = Ошибка проверки

# Placeholder texts
placeholder-domains = Домены через запятую
placeholder-ip-addresses = IP-адреса через запятую
placeholder-routes = Маршруты через запятую в формате x.x.x.x/x
placeholder-certs = PEM или DER файлы через запятую

# Tunnel types
tunnel-type-ipsec = IPSec
tunnel-type-ssl = SSL (устаревший)

# Certificate types
cert-type-none = Нет
cert-type-pfx = Файл PFX
cert-type-pem = Файл PEM
cert-type-hw = Аппаратный токен

# Icon themes
icon-theme-auto = Авто
icon-theme-dark = Тёмная
icon-theme-light = Светлая