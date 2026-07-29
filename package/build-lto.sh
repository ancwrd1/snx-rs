#!/usr/bin/bash

arch="$(uname -m)"
target="$arch-unknown-linux-gnu"

# GLIBC_VERSION pins the minimum glibc to link against, for compatibility with older distros.
# It requires cargo-zigbuild and the zig compiler.
if [ -n "$GLIBC_VERSION" ]; then
    cargo zigbuild --target="$target.$GLIBC_VERSION" --profile=lto "$@"
else
    cargo build --target="$target" --profile=lto "$@"
fi
