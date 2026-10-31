# Security Policy

`snx-rs` handles VPN credentials, IKE/ESP key material and the host's routing and DNS configuration, and its
service runs with elevated privileges. Security reports are taken seriously and are welcome.

## Supported versions

Only the latest release receives security fixes. Releases are frequent, so fixes are not backported to older
versions; upgrade to the newest release from the
[releases page](https://github.com/ancwrd1/snx-rs/releases/latest) or the
[package repositories](https://ancwrd1.github.io/snx-rs/).

| Version        | Supported |
|----------------|-----------|
| Latest release | Yes       |
| Older releases | No        |

## Reporting a vulnerability

**Please do not open a public issue, discussion or pull request for a security vulnerability.**

Report it privately in one of these ways:

1. Preferred: [GitHub private vulnerability reporting](https://github.com/ancwrd1/snx-rs/security/advisories/new).
2. Email: dmitry@pankratov.net, with `[snx-rs security]` in the subject line.

Please include:

* the affected version, platform (Linux, Windows or macOS) and installation method;
* the tunnel type (IPsec IKEv1, IKEv2 or SSL) and authentication method, if relevant;
* steps to reproduce, a proof of concept, or the affected code location;
* the impact you believe it has.

Logs help, but `trace`-level logs contain credentials and session data. Redact passwords, tokens, cookies, keys,
gateway addresses and usernames before sending them.

## What to expect

`snx-rs` is maintained by a single volunteer, so the timelines below are best effort:

* acknowledgement of the report within 7 days;
* an initial assessment within 14 days;
* a fix released as soon as practical, depending on severity and complexity.

Fixed vulnerabilities are disclosed through a GitHub security advisory (with a CVE where appropriate) and noted
in the `### Security` section of the [changelog](CHANGELOG.md). Reporters are credited unless they ask not to be.
Please keep the details private until the fix is released or 90 days have passed, whichever comes first;
the deadline can be extended by agreement.

## Scope

In scope:

* the `snx-rs` service, `snxctl` and `snx-rs-gui`, and the `snxcore` and `i18n` crates in this repository;
* the release artifacts and packaging in this repository: installers, `.deb`/`.rpm` packages, the systemd unit,
  the polkit rule, the Dockerfile and the signed package repositories.

Examples of issues that are in scope:

* local privilege escalation through the service or its command socket;
* disclosure of passwords, OTPs, tokens, certificate passwords or IKE/ESP keys, for example in logs, error
  messages, the persisted IKE session store or files with insecure permissions;
* bypass of server certificate validation when `ignore-server-cert` is not set;
* flaws in the IKEv1/IKEv2, ESP or SSL tunnel implementations that let an on-path attacker decrypt, inject
  or redirect traffic;
* memory-safety or denial-of-service bugs triggered by data from the gateway or the network;
* DNS or routing configuration that leaks traffic outside the tunnel contrary to the configured policy.

Out of scope:

* vulnerabilities in Check Point gateways or in the Check Point protocol itself, which should be reported to
  Check Point directly;
* behavior that follows from explicitly insecure configuration, such as `ignore-server-cert=true` or sharing
  `trace`-level logs;
* attacks that require root or administrator access on the client machine;
* vulnerabilities in third-party dependencies that do not affect `snx-rs`; for those that do, a report here is
  welcome alongside one upstream;
* the Homebrew, AUR and NixOS packages, which are maintained by third parties.

The [`isakmp`](https://github.com/ancwrd1/isakmp) crate is maintained by the same author; issues in it can be
reported through the same channels.

## Verifying releases

The `.deb` and `.rpm` packages and the APT, DNF and zypper repositories are signed with the project's GPG key,
published at [ancwrd1.github.io/snx-rs](https://ancwrd1.github.io/snx-rs/). The macOS package is ad-hoc signed
only and is not notarized.
