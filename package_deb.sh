#!/usr/bin/env bash
# Build a .deb package for Debian and Ubuntu.
set -euo pipefail
cd "$(dirname "$0")"

NAME="edt-down-for-me"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)"
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64) DEB_ARCH="amd64" ;;
    aarch64|arm64) DEB_ARCH="arm64" ;;
    armv7l) DEB_ARCH="armhf" ;;
    i686|i386) DEB_ARCH="i386" ;;
    *) DEB_ARCH="$ARCH" ;;
esac

if [[ ! -x ./target/release/"$NAME" ]]; then
    ./build.sh
fi

DIST="$(pwd)/dist"
ROOT="$DIST/${NAME}_${VERSION}_${DEB_ARCH}"
rm -rf "$ROOT"
mkdir -p \
    "$ROOT/DEBIAN" \
    "$ROOT/usr/bin" \
    "$ROOT/usr/share/applications" \
    "$ROOT/usr/share/icons/hicolor/scalable/apps" \
    "$ROOT/usr/share/icons/hicolor/128x128/apps" \
    "$ROOT/usr/share/doc/$NAME"

install -m 0755 "./target/release/$NAME" "$ROOT/usr/bin/$NAME"
install -m 0644 ./assets/edt-down-for-me.desktop "$ROOT/usr/share/applications/edt-down-for-me.desktop"
install -m 0644 ./assets/edt-down-for-me.svg "$ROOT/usr/share/icons/hicolor/scalable/apps/edt-down-for-me.svg"
if [[ -f ./assets/edt-down-for-me.png ]]; then
    install -m 0644 ./assets/edt-down-for-me.png "$ROOT/usr/share/icons/hicolor/128x128/apps/edt-down-for-me.png"
fi
install -m 0644 ./README.md "$ROOT/usr/share/doc/$NAME/README.md"
install -m 0644 ./LICENSE "$ROOT/usr/share/doc/$NAME/copyright"

SIZE_KB="$(du -sk "$ROOT/usr" | awk '{print $1}')"

cat > "$ROOT/DEBIAN/control" <<EOF
Package: $NAME
Version: $VERSION
Section: net
Priority: optional
Architecture: $DEB_ARCH
Installed-Size: $SIZE_KB
Maintainer: edt-down-for-me contributors <edt-down-for-me@localhost>
Depends: libc6, libgcc-s1 | libgcc1, libfontconfig1
Recommends: libxkbcommon0, libwayland-client0, libxcb1
Description: Compact monitor for web property accessibility
 A small Rust + Slint desktop app that pings configured hostnames and
 IP addresses at a low rate, with TCP/HTTP fallback when ICMP is blocked,
 and shows green/yellow/red/grey status dots.
EOF

DEB="$DIST/${NAME}_${VERSION}_${DEB_ARCH}.deb"

build_with_ar() {
    local pkg_dir="$1"
    local out="$2"
    local work
    work="$(mktemp -d)"
    (
        cd "$pkg_dir"
        tar --owner=0 --group=0 --mtime='UTC 2026-01-01' -czf "$work/data.tar.gz" usr
        tar --owner=0 --group=0 --mtime='UTC 2026-01-01' -czf "$work/control.tar.gz" -C DEBIAN .
    )
    echo "2.0" > "$work/debian-binary"
    rm -f "$out"
    ar r "$out" "$work/debian-binary" "$work/control.tar.gz" "$work/data.tar.gz" >/dev/null
    rm -rf "$work"
}

if command -v dpkg-deb >/dev/null 2>&1; then
    dpkg-deb --root-owner-group --build "$ROOT" "$DEB"
else
    echo "dpkg-deb not found; assembling .deb with ar/tar..."
    build_with_ar "$ROOT" "$DEB"
fi

echo "Created $DEB"
ls -lh "$DEB"
