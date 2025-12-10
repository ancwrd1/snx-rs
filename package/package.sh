#!/bin/bash

suffix="$1"
basedir="$(dirname $(readlink -f $0))/.."
target="$basedir/target"
version="$(git -C "$basedir" describe)"
arches="x86_64"
apps="snx-rs snxctl snx-rs-gui"
assets="snx-rs.service snx-rs-gui.desktop install.sh"

for arch in $arches; do
    name="snx-rs-${version}${suffix}-linux-$arch"
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
        cp "$basedir/package/$asset" "$target/$name/"
    done

    cd "$target"
    tar c "$name" | xz -9 > "$name.tar.xz"

    makeself --quiet --tar-quietly --xz --needroot --sha256 "$name" "$name.run" "SNX-RS VPN Client version $version" ./install.sh
done
