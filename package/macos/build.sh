#!/usr/bin/env bash
set -euo pipefail

# Builds snx-rs, snxctl and snx-rs-gui for macOS with the lto profile.
# Set TARGETS to build additional archs, e.g. TARGETS="aarch64-apple-darwin
# x86_64-apple-darwin"; package.sh can then lipo them into a universal binary.
#
# The GUI is built with the mobile-access feature (embedded WebKit for the
# Mobile Access portal login); it needs no extra system libraries on macOS.

targets="${TARGETS:-aarch64-apple-darwin}"

for target in $targets; do
    rustup target add "$target"
    cargo build --target="$target" --profile=lto \
        -p snx-rs -p snxctl -p snx-rs-gui \
        --features snxcore/vendored-openssl,snx-rs-gui/mobile-access "$@"
done
