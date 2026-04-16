# Building from Sources

* Install the required dependencies:
  - Debian/Ubuntu: `sudo apt install build-essential libssl-dev libfontconfig1-dev libsqlite3-dev libgtk-4-dev libwebkitgtk-6.0-dev libsoup-3.0-dev libjavascriptcoregtk-6.0-dev`
  - openSUSE: `sudo zypper install libopenssl-3-devel sqlite3-devel fontconfig-devel gtk4-devel webkit2gtk4-devel`
  - Other distros: C compiler, OpenSSL, SQLite3, fontconfig, optionally GTK4 and WebKit6 development package
* Install a recent [Rust compiler](https://rustup.rs)
* Run `cargo build` to build the debug version, or `cargo build --release` to build the release version.
* To build a version with mobile access feature and webkit integration, pass the `--features=mobile-access` parameter.

NOTE: the minimal supported Rust version is 1.88.

## Static Build Recipe

The snx-rs command line application can be built and linked statically to use in containers or embedded environments.
System requirements: same as the normal build + docker or podman.

Static build instructions:

* Install `cross-rs` with `cargo install cross`
* Add `x86_64-unknown-linux-musl` target to the Rust compiler: `rustup target add x86_64-unknown-linux-musl`
* Build a static snx-rs executable with `cross build --target=x86_64-unknown-linux-musl --features snxcore/vendored-openssl,snxcore/vendored-sqlite -p snx-rs --profile lto`
