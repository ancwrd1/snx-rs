#!/usr/bin/bash

arch="$(uname -m)"

cargo build --target="$arch-unknown-linux-gnu" --profile=lto "$@"
