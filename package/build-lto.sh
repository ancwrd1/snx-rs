#!/usr/bin/bash

targets="x86_64-unknown-linux-gnu"

for target in $targets; do
    cargo build --target="$target" --profile=lto "$@"
done
