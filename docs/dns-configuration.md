# DNS Resolver Configuration and Privacy

On Linux, if systemd-resolved is not detected as a global DNS resolver, snx-rs will fall back
to modify the /etc/resolv.conf file directly and DNS servers acquired from the tunnel will be used globally.
For better privacy, use the split DNS provided by systemd-resolved.

To find out whether it is already enabled, check the /etc/resolv.conf file:

`readlink /etc/resolv.conf`

If it is a symlink pointing to `/run/systemd/resolve/stub-resolv.conf` then it is already configured on your system,
otherwise follow these steps:

1. Install `systemd-resolved` package if is not already installed.
2. `sudo ln -sf /run/systemd/resolve/stub-resolv.conf /etc/resolv.conf`
3. `sudo systemctl enable --now systemd-resolved`
4. `sudo systemctl restart NetworkManager`

With `systemd-resolved` it is also possible to use **routing domains** (as opposed to **search domains**).
Routing domains are prefixed with `~` character and when configured only requests for the fully qualified domains
will be forwarded through the tunnel. For further explanation, please check [this article](https://systemd.io/RESOLVED-VPNS/).

The `set-routing-domains=true|false` option controls whether to treat all acquired search domains as routing domains.

For container images which have `systemd-resovled` enabled, it may be necessary to disable split DNS via the `no-split-dns=true` option.

### Choosing the resolver backend

The `dns-resolver` option selects how DNS is configured on Linux:

* `auto` (default): use systemd-resolved if `/etc/resolv.conf` points to it, otherwise modify `/etc/resolv.conf`.
* `systemd-resolved`: always use systemd-resolved.
* `resolv-conf`: always modify `/etc/resolv.conf` (or the file it links to).

### Network namespaces

systemd-resolved can only configure interfaces in its own network namespace, so it cannot be used when snx-rs
runs in another one, for example via `ip netns exec`. In `auto` mode snx-rs detects this and modifies the
namespace's own `/etc/resolv.conf` instead. Create one for the namespace before connecting, it is bind-mounted
by `ip netns exec`:

```sh
sudo mkdir -p /etc/netns/vpn
echo "nameserver 1.1.1.1" | sudo tee /etc/netns/vpn/resolv.conf
```

Without it, `/etc/resolv.conf` would be shared with the host, and snx-rs refuses to modify it. Note that the
command socket is also bound to the network namespace, so `snxctl` and the GUI must run in the same namespace
as the service.

## macOS

DNS is configured via the macOS SystemConfiguration framework (`SCDynamicStore`). Split DNS is implemented using scoped and supplemental match domains, the macOS equivalent of systemd-resolved's routing domains.
