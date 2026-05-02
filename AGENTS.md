# AGENTS.md

Guidance for AI coding agents (Claude Code, Copilot, Cursor, etc.) working in the `snx-rs` repository.

## Project overview

`snx-rs` is an unofficial open-source Linux client for Check Point VPN tunnels, written in Rust. It speaks both the IPsec and SSL flavors of Check Point's remote access protocol and supports the full matrix of authentication flows the official Windows client supports (password + MFA, certificate, HSM token, browser-based SSO / identity provider, Mobile Access web portal, hybrid machine-certificate + user).

Repository: https://github.com/ancwrd1/snx-rs  
License: AGPL-3.0 (see `COPYING`)  
Author: Dmitry Pankratov <dmitry@pankratov.net>  

Linux-only. Other platforms are not supported and not a target. However, all platform-specific code must be isolated under `platform` submodule.

## Workspace layout

This is a Cargo workspace (`resolver = "2"`, `edition = "2024"`) declared in the root `Cargo.toml`. Five members:

| Member       | Path              | Kind    | Purpose                                                                                    |
|--------------|-------------------|---------|--------------------------------------------------------------------------------------------|
| `snxcore`    | `crates/snxcore`  | library | All protocol, platform, tunnel, and controller logic. Everything non-trivial lives here.   |
| `i18n`       | `crates/i18n`     | library | Fluent-based localization. Wraps `fluent-templates` and exposes `tr!`/`translate` helpers. |
| `snx-rs`     | `apps/snx-rs`     | binary  | The service / daemon. Runs in standalone or command mode; needs root for tunnel setup.     |
| `snxctl`     | `apps/snxctl`     | binary  | Thin CLI that talks to `snx-rs` in command mode (connect/disconnect/status/info).          |
| `snx-rs-gui` | `apps/snx-rs-gui` | binary  | Slint-based GUI with tray icon (KSNI). Talks to `snx-rs` over IPC.                         |

Top-level directories:

* `crates/` — library crates.
* `apps/` — the three binary crates.
* `docs/` — user-facing Markdown docs (`docs/README.md` is the index). Keep in sync when changing user-visible behavior.
* `package/` — packaging assets: systemd unit (`snx-rs.service`), desktop file, Debian and RPM packaging, installer script, LTO build script.
* `.github/workflows/` — `ci.yml` (fmt + clippy + test), `release.yml`, `automerge.yml`, `pages.yml`.
* `i18n.md` — canonical instructions for translation tasks. Re-read before doing any locale work.
* `CHANGELOG.md` — hand-maintained. Add a user-visible entry for any user-visible change.

### `snxcore` internal modules

`crates/snxcore/src/`:

* `lib.rs` — has `#![deny(unsafe_code)]`. Keep it that way.
* `ccc.rs` — Check Point "CCC" HTTP(S) control protocol client.
* `server.rs` / `server_info.rs` — command-mode server (unix socket) and server-info queries.
* `controller.rs` — connection lifecycle orchestration.
* `browser.rs` — browser-based SAML/IdP flow.
* `otp.rs` — OTP listener (used by browser / MFA flows).
* `prompt.rs` — tty and secure prompt abstractions.
* `sexpr.rs` + `sexpr.pest` — parser for Check Point's S-expression wire format (`pest` grammar).
* `model/` — protocol types: `params.rs` (tunnel config / `TunnelParams`), `proto.rs` (wire structs), `wrappers.rs`.
* `platform/linux/` — Linux-only bits: `keychain.rs` (Secret Service), `resolver.rs` (systemd-resolved via D-Bus), `routing.rs` (rtnetlink), `xfrm.rs` (xfrmnetlink / netlink-packet-xfrm), `net.rs`, `stats.rs` (interface stats poller).
* `tunnel/` — transport implementations:
  * `ipsec/` — IPsec connector, NAT-T, keepalive, SCV policy emulation (`scv.rs`), plus the `imp/` impls selected at runtime (kernel XFRM vs. userspace TUN/TCPT).
  * `ssl/` — SSL tunnel codec, connector, keepalive (legacy fallback transport).
  * `device.rs` — tun device abstraction.
* `util.rs` — misc helpers.
* `tests/` — integration fixtures (`*.txt` captured wire payloads) and integration tests.

The external `isakmp` crate (git dep: `https://github.com/ancwrd1/isakmp.git`) provides IKE/ISAKMP primitives used by the IPsec tunnel.

## Build & verify commands

Minimum supported Rust: **1.88**. Workspace edition is 2024.

```bash
# Debug build (whole workspace)
cargo build

# Release build
cargo build --release

# Release build with mobile-access (embedded WebKit for Mobile Access portal flow)
cargo build --release --features mobile-access

# Static musl build (matches release pipeline; requires cross-rs and docker/podman)
cross build --target=x86_64-unknown-linux-musl \
  --features snxcore/vendored-openssl,snxcore/vendored-sqlite \
  -p snx-rs --profile lto

# CI gate — always run these three before declaring work done:
cargo fmt --check
cargo clippy --workspace --features mobile-access -- -D warnings
cargo test  --workspace --features mobile-access
```

CI runs on `ubuntu-latest` / `x86_64-unknown-linux-gnu` / stable. Clippy is `-D warnings` — do not land new warnings.

### System dependencies

Required for the default build: C toolchain, OpenSSL, SQLite3, fontconfig. GTK 4.10+ and WebKit 6.0+ are required only when the `mobile-access` feature is enabled.

* Debian/Ubuntu: `build-essential libssl-dev libfontconfig1-dev libsqlite3-dev libgtk-4-dev libwebkitgtk-6.0-dev libsoup-3.0-dev libjavascriptcoregtk-6.0-dev`
* openSUSE: `libopenssl-3-devel sqlite3-devel fontconfig-devel gtk4-devel webkit2gtk4-devel`

### Cargo features

Defined on `snxcore`:
* `vendored-openssl` — build OpenSSL from source (used for static/musl builds).
* `vendored-sqlite` — bundle SQLite via `rusqlite/bundled`.

Defined on `snx-rs-gui`:
* `mobile-access` — pulls in `gtk4` and `webkit6`, enabling the embedded browser used for Mobile Access portal login. Off by default in release CI to minimize runtime deps.

### Build profiles

`release` has `panic = "abort"`. There is also an `lto` profile (inherits `release`) that adds `strip = true`, `lto = true`, `codegen-units = 1` — used for packaging.

## Code style and conventions

* `rustfmt.toml` sets `max_width = 120`. Run `cargo fmt` before committing.
* `#![deny(unsafe_code)]` is set in `crates/snxcore/src/lib.rs` and `apps/snx-rs/src/main.rs`. Do not introduce `unsafe` there. If you genuinely need FFI, discuss with maintainers first — the preference is to use safe wrapper crates (`nix`, `rtnetlink`, `xfrmnetlink`, `tun`, etc.).
* Errors flow through `anyhow::Result` at app boundaries; library code also uses `anyhow` (there is no custom error type at present).
* Async runtime is `tokio` (multi-thread). Prefer `tokio::spawn` + channels over ad-hoc threading.
* Secrets go through `secrecy::SecretString` / `ExposeSecret`. Do not log, format, or embed passwords / tokens into error strings.
* Logging uses `tracing` + `tracing-subscriber`. Respect the `log-level` config option. `trace` level is documented as sensitive — do not downgrade that warning.
* Platform-specific code belongs under `crates/snxcore/src/platform/linux/` behind `#[cfg(target_os = "linux")]` where applicable. The `Platform` trait in `platform.rs` is the abstraction seam.
* Comments: the maintainer's stated AI policy (`docs/contributing.md`) is explicit — *no redundant per-line or per-function comments*. Only comment the non-obvious *why*. Machine-generated boilerplate comments will be rejected.

## User-facing strings must be localized

All human-readable English strings that reach the user go through the `i18n` crate's `tr!` macro or `i18n::translate`. The canonical source file is `crates/i18n/assets/en-US/main.ftl`; 16 other locales live alongside it.

Rules (from `i18n.md`):

* Keys are prefixed by category: `error-*`, `label-*`, `info-*`, `language-*`, etc.
* When adding a new string: add the `en-US` entry **and** a translated entry in every other locale's `main.ftl`. Preserve file grouping and comments. Leave `{$placeholder}` tokens untranslated.
* Never modify existing entries (reword via a new key instead).
* Prefer small Python scripts for bulk locale work over re-reading every file by hand.
* If you add a new locale, also add a `language-<locale>` entry to **every existing** `main.ftl`.

Read `i18n.md` in full before performing any of its three named tasks — it is the source of truth.

## Documentation

User-facing docs are in `docs/`. If you change a config option, CLI flag, tunnel behavior, or supported auth flow, update the relevant file there:

* `options.md` — all config-file / CLI options table.
* `command-line-usage.md` — modes and invocation.
* `features.md` — the capability bullet list.
* `tunnel-types.md`, `dns-configuration.md`, `certificates.md`, etc. — topic deep-dives.
* `troubleshooting.md` — add new entries when you diagnose a recurring issue.

Also add a short bullet to `CHANGELOG.md` for any user-visible change, under the `v6.0.0 (TBD)` section (or whatever the current in-flight version is).

## Runtime modes (useful context for changes)

The `snx-rs` binary runs in one of two operating modes selected by `-m`:

* `standalone` — daemon + tunnel in one process. Needs root. Config comes from the CLI or a `-c` config file.
* `command` — service mode. Daemon only; establishes tunnels on demand when `snxctl` (or the GUI) sends commands over a local unix socket. This is the desktop path.

Plus one-shot utility modes: `info` (query server for auth methods), certificate enrollment, etc.

The GUI (`snx-rs-gui`) always runs unprivileged and talks to a `command`-mode `snx-rs` service over IPC (`interprocess` crate) + D-Bus (`zbus`) for desktop integration. The GUI is **not** a replacement for the service; it's a client of it.

## Things agents commonly get wrong here

* **Don't add redundant comments.** The contributing guide says so explicitly; this is the single most common cause of rejected AI patches in this repo.
* **Don't hard-code user-visible English strings.** Route them through `tr!` and update the Fluent files.
* **Don't run external commands to configure networking.** Since v5.3.0 the project deliberately uses Linux APIs directly (`rtnetlink`, `xfrmnetlink`, systemd-resolved over D-Bus, `sysctl` crate). Do not regress this by shelling out to `ip`, `resolvectl`, `iptables`, etc.
* **Don't introduce `unsafe` in `snxcore` or `snx-rs`.** Both crates `#![deny(unsafe_code)]`.
* **Don't add backwards-compat shims** for config flags that were renamed (e.g. `no-keychain` → `keychain` was flipped deliberately in 5.3.0). If you're changing a flag, change it cleanly and document in `CHANGELOG.md`.
* **Don't bypass `secrecy`.** Passwords, IKE keys, tokens, and cert passwords must stay inside `SecretString` until the exact point of use.
* **Don't forget MSRV.** Edition 2024 features are fine; `rustc 1.88+` features are fine; anything newer is not.

## Git / PR hygiene

* Branch from `main`. PRs target `main`.
* Before pushing: `cargo fmt --check && cargo clippy --workspace --features mobile-access -- -D warnings && cargo test --workspace --features mobile-access`.
* Keep commits focused. The recent history is small, well-scoped commits (`git log --oneline` to see the style).
* Add a `CHANGELOG.md` entry for anything a user would notice.
* If a change affects translations, update all locales or explicitly note the gap.

## Useful external references

* Upstream repo and issue tracker: https://github.com/ancwrd1/snx-rs
* Sister crate for ISAKMP/IKE: https://github.com/ancwrd1/isakmp
* Slint (GUI framework): https://slint.dev
* Fluent localization: https://projectfluent.org
