# Tunnel Type Selection

snx-rs supports two tunnel types: IPsec and SSL. IPsec tunnel is a default option if not specified in the configuration.
Depending on the availability of the kernel `xfrm` module, it will use either a native kernel IPsec infrastructure or a TUN device
with userspace ESP packet encoding.

IPsec ESP traffic is encapsulated in the UDP packets sent via port 4500 which may be blocked in some environments.
In this case the application will fall back to the proprietary Check Point TCPT transport via TCP port 443, which is slower than UDP.

The `transport-type` option can be used to choose the IPsec transport type manually. The default value is `auto` which will perform autodetection.

For older VPN servers or in case IPsec does not work for some reason, the legacy SSL tunnel can be used as well, selected with `tunnel-type=ssl`.
SSL tunnel has some limitations: it is slower, has no hardware token support and no MFA in combination with the certificates.
