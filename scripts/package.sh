#!/bin/bash

basedir="$(cd $(dirname $0)/.. && pwd -P)"
target="$basedir/target"
version="$(git -C "$basedir" describe)"
arches="x86_64 aarch64"
apps="snx-rs snxctl snx-rs-gui"
assets="snx-rs.service snx-rs-gui.desktop"

for arch in $arches; do
    name="snx-rs-$version-linux-$arch"
    triple="$arch-unknown-linux-gnu"

    if [ ! -f "$target/$triple/lto/snx-rs" ]; then
        continue
    fi

    echo "Packaging for $arch"

    rm -rf "$target/$name"
    mkdir "$target/$name"

    for app in $apps; do
        if ! cp "$target/$triple/lto/$app" "$target/$name/"; then
            exit 1
        fi
    done

    for asset in $assets; do
        cp "$basedir/assets/$asset" "$target/$name/"
    done
    cp "$basedir/scripts/install.sh" "$target/$name/"

    cd "$target"
    tar c "$name" | xz -9 > "$name.tar.xz"

    makeself --quiet --tar-quietly --xz --needroot "$name" "$name.run" "SNX-RS VPN Client for Linux" ./install.sh
done
