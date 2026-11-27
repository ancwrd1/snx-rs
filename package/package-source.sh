#!/bin/bash
#
# Builds the Debian source package (.dsc, .orig.tar.xz, .debian.tar.xz) and the source RPM.
# Both are made from one tarball holding the git tree plus the vendored crates, so that they
# build offline. With cargo-vendor-filterer installed, only the crates needed on Linux are vendored.

set -euo pipefail

basedir="$(dirname "$(readlink -f "$0")")/.."
target="$basedir/target"
version="${1:-$(git -C "$basedir" describe)}"
upstream="$(echo "${version#v}" | sed 's/-/~/g')"
deb_version="$upstream-1"
name="snx-rs-$upstream"
orig="snx-rs_$upstream.orig.tar.xz"
platforms="x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu"

commit_time="$(git -C "$basedir" log -1 --format=%ct HEAD)"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
srcdir="$tmpdir/$name"

echo "Creating source tarball for $upstream"

mkdir -p "$target"
git -C "$basedir" archive --format=tar --prefix="$name/" HEAD | tar x -C "$tmpdir"

cd "$srcdir"
if cargo vendor-filterer --help >/dev/null 2>&1; then
    platform_args=()
    for platform in $platforms; do
        platform_args+=("--platform=$platform")
    done
    cargo vendor-filterer "${platform_args[@]}" vendor >/dev/null
else
    echo "cargo-vendor-filterer not found, vendoring crates for all platforms"
    cargo vendor --quiet --locked vendor >/dev/null
fi

mkdir -p .cargo
{
    printf '[source.crates-io]\nreplace-with = "vendored-sources"\n'
    grep -o '^source = "git+[^"#]*' Cargo.lock | sed 's/^source = "//' | sort -u | while read -r src; do
        url="${src#git+}"
        printf '\n[source."%s"]\ngit = "%s"\n' "$src" "${url%%\?*}"
        if [ "$url" != "${url%%\?*}" ]; then
            echo "${url#*\?}" | tr '&' '\n' | sed 's/^\([^=]*\)=\(.*\)$/\1 = "\2"/'
        fi
        echo 'replace-with = "vendored-sources"'
    done
    printf '\n[source.vendored-sources]\ndirectory = "vendor"\n'
} > .cargo/config.toml

cd "$tmpdir"
tar c --sort=name --owner=0 --group=0 --numeric-owner --mtime="@$commit_time" "$name" | xz -T0 -9 > "$orig"

echo "Packaging Debian source for $deb_version"

mkdir "$srcdir/debian"
cp -a "$basedir/package/debian/src/." "$srcdir/debian/"
rm "$srcdir/debian/changelog.in"
for script in preinst postinst prerm postrm; do
    install -m 755 "$basedir/package/debian/$script" "$srcdir/debian/"
done
sed "s/{{version}}/$deb_version/;s/{{upstream}}/$upstream/;s/{{date}}/$(LC_ALL=C date -R -d "@$commit_time")/" \
    "$basedir/package/debian/src/changelog.in" > "$srcdir/debian/changelog"

SOURCE_DATE_EPOCH="$commit_time" dpkg-source -b "$name"
cp "$orig" "snx-rs_$deb_version.debian.tar.xz" "snx-rs_$deb_version.dsc" "$target/"

echo "Packaging source RPM for $upstream"

rpm="$tmpdir/rpm"
mkdir -p "$rpm/SOURCES" "$rpm/SPECS"
mv "$orig" "$rpm/SOURCES/"
cat "$basedir/package/rpm/source.spec.in" "$basedir/package/rpm/common.spec" \
    | sed "s/{{version}}/$upstream/" > "$rpm/SPECS/snx-rs.spec"
rpmbuild --define "_topdir $rpm" --define "dist %{nil}" -bs "$rpm/SPECS/snx-rs.spec"
cp "$rpm/SRPMS"/*.src.rpm "$target/"
