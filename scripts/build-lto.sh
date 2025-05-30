#!/usr/bin/bash

export RUSTFLAGS="-C link-arg=-L/usr/lib64"
if [ -z "$1" ]; then
    targets="x86_64-unknown-linux-gnu"
else
    targets="$1"
fi
for target in $targets; do
    cargo zigbuild --target=${target}.2.17 --profile=lto --features vendored-openssl
done
