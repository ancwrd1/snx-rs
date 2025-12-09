#!/usr/bin/bash

if [ -z "$1" ]; then
    targets="x86_64-unknown-linux-gnu"
else
    targets="$1"
fi
for target in $targets; do
    cargo build --target="$target" --profile=lto --features vendored-openssl,mobile-access
done
