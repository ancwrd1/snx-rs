#!/usr/bin/bash

BASEDIR=$(dirname $(readlink -f $0))
REMAPDIRS="$BASEDIR=/project $HOME=/"

for rd in $REMAPDIRS; do
    export RUSTFLAGS="$RUSTFLAGS --remap-path-prefix=$rd"
done

if [ -z "$1" ]; then
    targets="x86_64-unknown-linux-gnu"
else
    targets="$1"
fi
for target in $targets; do
    cargo build --target=${target} --profile=lto --features vendored-openssl
done
