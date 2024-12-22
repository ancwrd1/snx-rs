#!/usr/bin/bash

export RUSTFLAGS="-C link-arg=-L/usr/lib64"
cargo zigbuild --target=x86_64-unknown-linux-gnu.2.17 --profile=lto --features reqwest/native-tls-vendored
