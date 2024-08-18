#!/bin/bash

basedir="$(cd $(dirname $0)/.. && pwd -P)"
target="$basedir/target"
version="$(git -C "$basedir" describe)"
arches="x86_64"
apps="snx-rs snxctl snx-rs-gui"
assets="snx-rs.conf snx-rs.service snx-rs-gui.desktop"

for arch in $arches; do
    name="snx-rs-$version-linux-$arch"
    triple="$arch-unknown-linux-gnu"

    rm -rf "$target/$name"
    mkdir "$target/$name"

    for app in $apps; do
        if ! cp "$target/$triple/release/$app" "$target/$name/"; then
            exit 1
        fi
    done

    for asset in $assets; do
        cp "$basedir/assets/$asset" "$target/$name/"
    done

    cd "$target"
    tar cJf "$name.tar.xz" "$name"
    echo "$target/$name.tar.xz"
done
