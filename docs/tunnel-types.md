# Tunnel Type Selection

snx-rs supports two tunnel types: IPsec and SSL. IPsec tunnel is a default option if not specified in the configuration.
Depending on the availability of the kernel `xfrm` module, it will use either a native kernel IPsec infrastructure or a TUN device
with userspace ESP packet encoding.

IPsec ESP traffic is encapsulated in the UDP packets sent via port 4500 which may be blocked in some environments.
In this case the application will fall back to the proprietary Check Point TCPT transport via TCP port 443, which is slower than UDP.

The `transport-type` option can be used to choose the IPsec transport type manually. The default value is `auto` which will perform autodetection.

macOS and Windows have no kernel `xfrm` support, so IPsec always uses the userspace TUN/ESP path there.
The `kernel` value for `transport-type` is not available on those platforms; valid values are `auto`, `udp` and `tcpt`.

## Gateways Without Office Mode

Some gateways assign no Office Mode address. With IKEv1, snx-rs then does what the Windows client does: the tunnel
carries the computer's own IPv4 address, while the TUN device gets a private placeholder address that overlaps neither
the VPN routes nor the local network, and the userspace ESP path translates between the two. The kernel transport cannot
do that translation, so `transport-type=auto` selects `udp` instead of `kernel` for such gateways, and an explicit
`transport-type=kernel` fails with an error.

## IKE Protocol Version

The IKE protocol version is autodetected by default: IKEv2 is used when the server advertises the
`Prefer_IKEv2_Support_IKEv1` or `IKEv2_Only` data tunnel protocol, otherwise IKEv1 is used. The `ike-version` option overrides this with
`1` or `2`. Both versions use the same transports, the same authentication methods and the same office mode configuration.

Unlike IKEv1, the IKEv2 exchange itself runs over UDP on the NAT-T port rather than over TCPT, because the server will
not carry ESP over UDP for a session it negotiated over TCPT. If that exchange does not get through — UDP blocked, or a
path that drops the IP fragments the server's certificate round needs — the client retries the whole login over TCPT and
moves ESP there with it. That fallback applies only to the default `transport-type=auto`; an explicit `transport-type`
is left as chosen.

## SSL Tunnel

For older VPN servers or in case IPsec does not work for some reason, the legacy SSL tunnel can be used as well, selected with `tunnel-type=ssl`.
SSL tunnel has some limitations: it is slower, has no hardware token support and no MFA in combination with the certificates.
