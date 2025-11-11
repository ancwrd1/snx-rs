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
label-username-required = Для аутентификации требуется имя пользователя
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
label-language = Язык
label-system-language = Системный по умолчанию
label-username-password = Имя пользователя и пароль
label-auto-connect = Автоматически подключаться при запуске
label-ip-lease-time = Пользовательское время аренды IP, секунды
label-disable-ipv6 = Отключать IPv6, когда включён маршрут по умолчанию
label-mtu = MTU
label-connection-profile = Профиль подключения
label-profile-name = Имя профиля
label-confirmation = Пожалуйста, подтвердите

# Tabs and expanders
tab-general = Основные
tab-advanced = Дополнительно
expand-dns = DNS
expand-routing = Маршрутизация
expand-certificates = Сертификаты
expand-misc = Прочие настройки
expand-ui = Настройки интерфейса

# Error messages
error-no-server-name = Не указан адрес сервера
error-no-auth = Не выбран метод аутентификации
error-file-not-exist = Файл не существует: {$path}
error-invalid-cert-id = ID сертификата не в шестнадцатеричном формате: {$id}
error-ca-root-not-exist = Путь к корневому сертификату CA не существует: {$path}
error-validation = Ошибка проверки
error-user-input-canceled = Ввод пользователя отменён
error-connection-cancelled = Соединение отменено
error-unknown-event = Неизвестное событие: {$event}
error-no-service-connection = Нет соединения со службой
error-empty-input = Ввод не может быть пустым
error-invalid-response = Недопустимый ответ!
error-invalid-object = Недопустимый объект
error-no-connector = Нет коннектора туннеля
error-tunnel-disconnected = Туннель отключен, последнее сообщение: {$message}
error-unexpected-reply = Неожиданный ответ
error-auth-failed = Ошибка аутентификации
error-no-login-type = Отсутствует обязательный параметр: login-type
error-connection-timeout = Таймаут соединения
error-cannot-send-request = Невозможно отправить запрос в службу
error-cannot-read-reply = Невозможно прочитать ответ от службы
error-no-ipv4 = Нет IPv4 адреса для {$server}
error-not-challenge-state = Не состояние запроса
error-no-challenge = Нет запроса в данных
error-endless-challenges = Бесконечный цикл запросов имени пользователя
error-no-pkcs12 = Не указан путь к PKCS12 и пароль
error-no-pkcs8 = Не указан путь к PKCS8 PEM
error-no-pkcs11 = Не указан PIN-код PKCS11
error-no-ipsec-session = Нет сессии IPSEC
error-request-failed-error-code = Ошибка запроса, код ошибки: {$error_code}
error-no-root-privileges = Эта программа должна быть запущена с правами root!
error-missing-required-parameters = Отсутствуют обязательные параметры: имя сервера и/или тип входа!
error-missing-server-name = Отсутствует обязательный параметр: имя сервера!
error-no-connector-for-challenge-code = Нет коннектора для отправки кода запроса!
error-probing-failed = Ошибка проверки, сервер недоступен через порт NATT!
error-invalid-sexpr = Недопустимый sexpr: {$value}
error-invalid-value = Недопустимое значение
error-udp-request-failed = Ошибка отправки UDP-запроса
error-no-tty = Нет подключенного TTY для получения ввода пользователя
error-invalid-auth-response = Недопустимый ответ аутентификации
error-invalid-client-settings = Недопустимый ответ настроек клиента
error-invalid-otp-reply = Недопустимый ответ OTP
error-udp-encap-failed = Не удалось установить опцию сокета UDP_ENCAP, код ошибки: {$code}
error-so-no-check-failed = Не удалось установить опцию сокета SO_NO_CHECK, код ошибки: {$code}
error-keepalive-failed = Ошибка keepalive
error-receive-failed = Ошибка получения
error-unknown-color-scheme = Неизвестное значение цветовой схемы
error-cannot-determine-ip = Не удалось определить IP по умолчанию
error-invalid-command = Недопустимая команда: {$command}
error-otp-browser-failed = Не удалось получить OTP из браузера
error-invalid-operation-mode = Недопустимый режим работы
error-invalid-tunnel-type = Недопустимый тип туннеля
error-invalid-cert-type = Недопустимый тип сертификата
error-invalid-icon-theme = Недопустимая тема иконок
error-no-natt-reply = Нет ответа NAT-T
error-not-implemented = Не реализовано
error-unknown-packet-type = Неизвестный тип пакета
error-no-sender = Нет отправителя
error-empty-ccc-session = Пустая сессия CCC
error-identity-timeout = Таймаут при ожидании ответа идентификации, правильный ли тип входа?
error-invalid-transport-type = Неверный тип транспорта

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

# Transport types
transport-type-autodetect = Автоопределение
transport-type-kernel = UDP XFRM
transport-type-tcpt = TCPT TUN
transport-type-udp = UDP TUN

# Icon themes
icon-theme-autodetect = Автоопределение
icon-theme-dark = Тёмная
icon-theme-light = Светлая

# Application
app-title = VPN-клиент SNX-RS для Linux
app-connection-error = Ошибка соединения
app-connection-success = Соединение установлено

# Authentication
auth-dialog-title = Фактор аутентификации VPN
auth-dialog-message = Пожалуйста, введите ваш фактор аутентификации:

# Status dialog
status-dialog-title = Информация о соединении
status-button-copy = Копировать
status-button-settings = Настройки
status-button-connect = Подключить
status-button-disconnect = Отключить

# Tray menu
tray-menu-connect = Подключить
tray-menu-disconnect = Отключить
tray-menu-status = Статус соединения...
tray-menu-settings = Настройки...
tray-menu-about = О программе...
tray-menu-exit = Выход

# Connection info
info-connected-since = Подключено с
info-server-name = Имя сервера
info-user-name = Имя пользователя
info-login-type = Тип входа
info-tunnel-type = Тип туннеля
info-transport-type = Тип транспорта
info-ip-address = IP-адрес
info-dns-servers = DNS-серверы
info-search-domains = Домены поиска
info-interface = Интерфейс
info-dns-configured = DNS настроен
info-routing-configured = Маршрутизация настроена
info-default-route = Маршрут по умолчанию

# CLI Messages
cli-identity-provider-auth = Для аутентификации через провайдера идентификации откройте следующий URL в браузере:
cli-tunnel-connected = Туннель подключен, нажмите Ctrl-C для выхода.
cli-tunnel-disconnected = Туннель отключен
cli-another-instance-running = Другая копия snx-rs уже запущена
cli-app-terminated = Приложение завершено по сигналу

# Connection Messages
connection-connected-to = Подключено к {$server}

# Languages
language-cs-CZ = Чешский
language-da-DK = Датский
language-de-DE = Немецкий
language-en-US = Английский
language-es-ES = Испанский
language-fi-FI = Финский
language-fr-FR = Французский
language-it-IT = Итальянский
language-nl-NL = Голландский
language-no-NO = Норвежский
language-pl-PL = Польский
language-pt-PT = Португальский
language-pt-BR = Бразильский португальский
language-ru-RU = Русский
language-sk-SK = Словацкий
language-sv-SE = Шведский

# Connection status messages
connection-status-disconnected = Отключено
connection-status-connecting = Выполняется подключение
connection-status-connected-since = Подключено с: {$since}
connection-status-mfa-pending = Ожидание MFA: {$mfa_type}

# Login options
login-options-server-address = Адрес сервера
login-options-server-ip = IP сервера
login-options-client-enabled = Клиент включен
login-options-supported-protocols = Поддерживаемые протоколы
login-options-preferred-protocol = Предпочтительный протокол
login-options-tcpt-port = Порт TCPT
login-options-natt-port = Порт NATT
login-options-internal-ca-fingerprint = Отпечаток внутреннего CA

# Connection profiles
profile-new = Новый
profile-rename = Переименовать
profile-delete = Удалить
profile-delete-prompt = Вы уверены, что хотите удалить выбранный профиль?
profile-default-name = По умолчанию
profile-new-title = Новый профиль подключения
profile-rename-title = Переименовать профиль подключения
