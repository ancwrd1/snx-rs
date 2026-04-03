# DNS Resolver Configuration and Privacy

By default, if systemd-resolved is not detected as a global DNS resolver, snx-rs will fall back
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
