#!/usr/bin/env bash
set -euo pipefail

# Assembles SNX-RS.app, ad-hoc codesigns it, and builds a single .dmg artifact
# containing the installer .pkg (GUI app + CLI + LaunchDaemon) and an
# "Uninstall SNX-RS Application" script. Binaries must already be built, e.g.
# with package/macos/build.sh. Mirrors package/package.sh.
#
# Usage: package/macos/package.sh [version]
# version defaults to `git describe` (same convention as package/package.sh).
#
# Set TRIPLE=x86_64-apple-darwin to package that arch instead of the default
# aarch64-apple-darwin, or TRIPLE=universal to lipo both together (both must
# already be built under target/<triple>/lto/ first).
#
# Ad-hoc signing only (codesign --sign -): there is no Apple Developer ID in
# this pipeline. Set DEVELOPER_ID_IDENTITY to a real "Developer ID
# Application: ..." identity to sign for real; a signed and notarized build
# opens with no prompt. Otherwise the first launch is gated and the user
# right-clicks -> Open once, which records the approval without disabling
# Gatekeeper.

basedir="$(cd "$(dirname "$0")/../.." && pwd)"
target="$basedir/target"
macos_dir="$basedir/package/macos"
bundle_id="com.github.snx-rs"
identity="${DEVELOPER_ID_IDENTITY:--}"
triple="${TRIPLE:-aarch64-apple-darwin}"

version="${1:-$(git -C "$basedir" describe)}"
pkg_version="${version#v}"

stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT

if [ "$triple" = "universal" ]; then
    bindir="$stage/universal-bin"
    mkdir -p "$bindir"
    for app in snx-rs snxctl snx-rs-gui; do
        lipo -create \
            "$target/aarch64-apple-darwin/lto/$app" \
            "$target/x86_64-apple-darwin/lto/$app" \
            -output "$bindir/$app"
    done
else
    bindir="$target/$triple/lto"
fi

build_iconset() {
    local png="$basedir/package/wix/snx-rs.png"
    if [ ! -f "$png" ]; then
        echo "no icon source found ($png), packaging SNX-RS.app without an icon" >&2
        return
    fi
    local iconset="$stage/snx-rs.iconset"
    mkdir -p "$iconset"
    for size in 16 32 128 256 512; do
        sips -z "$size" "$size" "$png" --out "$iconset/icon_${size}x${size}.png" >/dev/null
        sips -z $((size * 2)) $((size * 2)) "$png" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
    done
    iconutil -c icns "$iconset" -o "$1"
}

create_app() {
    echo "Assembling SNX-RS.app" >&2

    if [ ! -f "$bindir/snx-rs-gui" ]; then
        echo "missing $bindir/snx-rs-gui, build it first (see package/macos/build.sh)" >&2
        exit 1
    fi

    local app="$stage/SNX-RS.app"
    local contents="$app/Contents"
    mkdir -p "$contents/MacOS" "$contents/Resources"

    install -m 755 "$bindir/snx-rs-gui" "$contents/MacOS/"
    sed "s/{{version}}/$pkg_version/" "$macos_dir/Info.plist" > "$contents/Info.plist"
    build_iconset "$contents/Resources/snx-rs.icns"
    # Ship the uninstaller inside the bundle so users don't have to fetch it from GitHub.
    install -m 755 "$macos_dir/uninstall.sh" "$contents/Resources/uninstall.sh"

    codesign --force --deep --sign "$identity" "$app"

    echo "$app"
}

create_pkg() {
    echo "Packaging .pkg for $triple" >&2

    local app="$1"
    local payload="$stage/pkg-payload"
    local scripts="$stage/pkg-scripts"
    local libexec="$payload/Library/Application Support/snx-rs"

    mkdir -p "$payload/usr/local/bin" "$libexec" "$payload/Library/LaunchDaemons" "$payload/Applications" "$scripts"

    for bin in snx-rs snxctl; do
        if [ ! -f "$bindir/$bin" ]; then
            echo "missing $bindir/$bin, build it first (see package/macos/build.sh)" >&2
            exit 1
        fi
    done

    # The root LaunchDaemon runs snx-rs from a root-owned directory rather than the user-writable
    # Homebrew /usr/local prefix, so it cannot be swapped out to escalate to root. snxctl runs as the
    # user; the /usr/local/bin/snx-rs symlink is only a convenience (root uses the absolute path).
    install -m 755 "$bindir/snx-rs" "$libexec/snx-rs"
    install -m 755 "$bindir/snxctl" "$payload/usr/local/bin/snxctl"
    ln -s "/Library/Application Support/snx-rs/snx-rs" "$payload/usr/local/bin/snx-rs"

    install -m 644 "$macos_dir/com.github.snx-rs.plist" "$payload/Library/LaunchDaemons/"
    install -m 755 "$macos_dir/scripts/postinstall" "$scripts/"

    # Ship the GUI app in the same installer so it lands under /Applications.
    cp -R "$app" "$payload/Applications/"

    pkgbuild --root "$payload" \
        --scripts "$scripts" \
        --identifier "$bundle_id" \
        --version "$pkg_version" \
        --install-location / \
        "$stage/snx-rs-component.pkg" >&2

    productbuild --package "$stage/snx-rs-component.pkg" "$stage/SNX-RS.pkg" >&2

    echo "$stage/SNX-RS.pkg"
}

create_dmg() {
    echo "Packaging .dmg for $triple" >&2

    local pkg="$1"
    local name="snx-rs-${version}-${triple}"
    local dmg_stage="$stage/dmg"

    mkdir -p "$dmg_stage"
    cp "$pkg" "$dmg_stage/SNX-RS.pkg"
    # Drop the uninstaller next to the installer so it is visible when the .dmg is mounted.
    # The .command extension lets Finder run it in Terminal on double-click.
    install -m 755 "$macos_dir/uninstall.sh" "$dmg_stage/Uninstall SNX-RS Application.command"

    hdiutil create -volname "SNX-RS" -srcfolder "$dmg_stage" -ov -format UDZO "$target/$name.dmg"
}

mkdir -p "$target"
app_path="$(create_app)"
pkg_path="$(create_pkg "$app_path")"
create_dmg "$pkg_path"
