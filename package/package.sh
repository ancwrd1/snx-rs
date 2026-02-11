#!/bin/bash

suffix="$1"
basedir="$(dirname $(readlink -f $0))/.."
target="$basedir/target"
version="$(git -C "$basedir" describe)"
deb_version="${version:1}"
rpm_version="$(echo $version | sed 's/-/~/g')"
arch="$(uname -m)"
apps="snx-rs snxctl snx-rs-gui"
assets="snx-rs.service snx-rs-gui.desktop install.sh"

case $arch in
    aarch64)
      build_arch=arm64
      ;;
    *)
      build_arch=$arch
      ;;
esac

create_run() {
    echo "Packaging .run for $build_arch"

    name="snx-rs-${version}${suffix}-linux-$build_arch"
    triple="$arch-unknown-linux-gnu"

    if [ ! -f "$target/$triple/lto/snx-rs" ]; then
        return
    fi

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
}

create_deb() {
    echo "Packaging .deb for $build_arch"

    case $build_arch in
      x86_64)
        deb_arch=amd64
        ;;
      *)
        deb_arch=$build_arch
        ;;
    esac

    name="snx-rs-${version}${suffix}-linux-$build_arch"
    tmpdir="$(mktemp -d)"
    debian="$tmpdir/debian/DEBIAN"

    mkdir -p "$debian"
    install -m 755 "$basedir/package/debian/postinst" "$debian/"
    install -m 755 "$basedir/package/debian/preinst" "$debian/"
    install -m 755 "$basedir/package/debian/prerm" "$debian/"

    mkdir -p "$tmpdir/debian/usr/bin"
    mkdir -p "$tmpdir/debian/etc/systemd/system"
    mkdir -p "$tmpdir/debian/usr/share/applications"

    for app in $apps; do
      install -m 755 "$target/$triple/lto/$app" "$tmpdir/debian/usr/bin/"
    done

    size=$(du -sk $tmpdir/debian/usr/bin | cut -f1)

    sed "s/{{version}}/$deb_version/;s/{{arch}}/$deb_arch/;s/{{size}}/$size/" "$basedir/package/debian/control.in" > "$debian/control"

    cp "$basedir/package/snx-rs.service" "$tmpdir/debian/etc/systemd/system/"
    cp "$basedir/package/snx-rs-gui.desktop" "$tmpdir/debian/usr/share/applications"

    if ! fakeroot dpkg-deb --build "$tmpdir/debian" "$target/$name.deb"; then
        exit 1
    fi

    rm -rf "$tmpdir"
}

create_rpm() {
    echo "Packaging .rpm for $build_arch"

    name="snx-rs-${version}${suffix}-linux-$build_arch"
    tmpdir="$(mktemp -d)"
    rpm="$tmpdir/rpm"

    export RPM_BUILDROOT="$rpm/root"

    mkdir -p "$rpm/BUILD"
    mkdir -p "$rpm/RPMS"
    mkdir -p "$rpm/SOURCES"
    mkdir -p "$rpm/SPECS"
    mkdir -p "$rpm/SRPMS"
    mkdir -p "$rpm/BUILDROOT"

    sed "s/{{version}}/$rpm_version/;s/{{arch}}/$arch/" "$basedir/package/rpm/package.spec.in" > "$rpm/SPECS/package.spec"

    mkdir -p "$RPM_BUILDROOT/usr/bin"
    mkdir -p "$RPM_BUILDROOT/etc/systemd/system"
    mkdir -p "$RPM_BUILDROOT/usr/share/applications"

    for app in $apps; do
      install -m 755 "$target/$triple/lto/$app" "$RPM_BUILDROOT/usr/bin/"
    done

    cp "$basedir/package/snx-rs.service" "$RPM_BUILDROOT/etc/systemd/system/"
    cp "$basedir/package/snx-rs-gui.desktop" "$RPM_BUILDROOT/usr/share/applications"

    if ! rpmbuild --define "_topdir $rpm" \
             --define "_buildroot $rpm/BUILDROOT" \
             --buildroot "$rpm/BUILDROOT" \
             -bb "$rpm/SPECS/package.spec"; then
      exit 1
    fi

    cp "$rpm/RPMS/$arch"/*.rpm "$target/$name.rpm"

    # Cleanup
    rm -rf "$tmpdir"
}

create_run
create_deb
create_rpm
