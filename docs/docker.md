# Docker Usage

This project includes a Dockerfile for building and running snx-rs as a containerized Check Point VPN client.

## Using the Project's Dockerfile

The root `Dockerfile` builds a static binary using musl cross-compilation and supports multiple base images (alpine by default, or archlinux).

### Building the Image

```bash
# Build with default alpine base
docker build -t snx-rs .

# Build with archlinux base
docker build -t snx-rs --build-arg BASE=archlinux .
```

### Running the Container

The container runs as a non-root user (`snxuser`, UID 1000) with a configuration directory at `/home/snxuser/.config/snx-rs`.

To connect to a VPN server:

```bash
# Basic usage (replace placeholders with your server and credentials)
docker run -it --rm \
  --net=host \
  --cap-add=NET_ADMIN \
  --cap-add=NET_RAW \
  snx-rs \
  snx-rs -s vpn.example.com -o vpn_Username_Password -u username
```

To persist configuration across runs, mount a volume:

```bash
# With configuration volume
docker run -it --rm \
  --net=host \
  --cap-add=NET_ADMIN \
  --cap-add=NET_RAW \
  -v $PWD/config:/home/snxuser/.config/snx-rs \
  snx-rs \
  snx-rs <command>
```

> **Note:** The `--net=host` and network capabilities (`NET_ADMIN`, `NET_RAW`) are required for VPN tunnel establishment.

### Privileges

On Linux, snx-rs does not require the root user: it checks for the `CAP_NET_ADMIN` capability instead.
The image sets this capability on the binary (`setcap cap_net_admin+ep`), so `snxuser` gets it as long as
the container is started with `--cap-add=NET_ADMIN`. Without that flag, snx-rs refuses to start, even when the
container runs as root, because Docker does not grant `NET_ADMIN` by default.

The file capability is ignored when the container runs with `--security-opt no-new-privileges`. In that case,
run the container as root (`--user root`) instead.

Because the container user cannot modify `/etc/resolv.conf`, use the `no-dns=true` option and configure name
resolution for the VPN domains separately, or run the container as root.

### Running as a regular user on the host

`CAP_NET_ADMIN` covers the kernel side of the tunnel, but systemd-resolved and NetworkManager authorize D-Bus
callers through polkit, which only trusts the root user. When snx-rs runs as a regular user with
`CAP_NET_ADMIN` on a host that uses systemd-resolved, setting up DNS fails with
`org.freedesktop.DBus.Error.InteractiveAuthorizationRequired`.

The repository ships a polkit rule, [`package/50-snx-rs.rules`](../package/50-snx-rs.rules), that grants
these permissions to the members of the `snx-rs` group. It is not installed by the packages. To use it:

```bash
sudo groupadd --system snx-rs
sudo usermod -aG snx-rs <user running snx-rs>
sudo install -m 0644 package/50-snx-rs.rules /etc/polkit-1/rules.d/50-snx-rs.rules
```

polkit checks the groups of the running snx-rs process, not the user database, so the new group membership must be
active in the process that starts snx-rs: `id` must list `snx-rs`. Logging out may not be enough because the
`systemd --user` manager keeps running while any session of the user is open, and applications it launches inherit
its old groups. Reboot, or start snx-rs with `sg snx-rs -c "snx-rs ..."`.

Alternatively, run snx-rs as root or use the `no-dns=true` option.

### Dockerfile Details

- **Build stage:** Uses `rust:1.92-slim` with musl cross-compilation to produce a static binary
- **Runtime stage:** Configurable base image (alpine or archlinux) with required dependencies (iproute2, openssl, ca-certificates, bash, sudo)
- **Security:** Runs as non-root user with a dedicated configuration directory
- **Entry point:** `snx-rs` with default command `--help`

## Alternative: Pre-built Container

For a ready-to-use Docker container, check [leleobhz/snx-rs-docker](https://github.com/leleobhz/snx-rs-docker).
