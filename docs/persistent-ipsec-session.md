# Persistent IPSec Session

The `ike-persist` option will save IPSec session to disk and restore it after the service or computer restarts,
it will then attempt to automatically reconnect the tunnel without authentication. This parameter works best in combination with the `ike-lifetime` option:
for example, setting `ike-lifetime` to 604800 will keep the session for 7 days.

Note that most IPSec servers have shorter IKE duration configured, so it may be terminated earlier.
