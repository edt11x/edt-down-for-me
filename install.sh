#!/usr/bin/env bash
# Fedora (and other RPM-based) system-wide installer.
# Installs into PREFIX (default /usr/local). Requires root for that prefix.
set -euo pipefail
cd "$(dirname "$0")"

PREFIX="${PREFIX:-/usr/local}"
BIN_DIR="$PREFIX/bin"
APP_DIR="$PREFIX/share/applications"
ICON_SVG="$PREFIX/share/icons/hicolor/scalable/apps"
ICON_PNG="$PREFIX/share/icons/hicolor/128x128/apps"
DOC_DIR="$PREFIX/share/doc/edt-down-for-me"

need_root=0
if [[ ! -w "$PREFIX" ]] || [[ ! -d "$PREFIX" && ! -w "$(dirname "$PREFIX")" ]]; then
    need_root=1
fi
if [[ "$need_root" -eq 1 && "${EUID:-$(id -u)}" -ne 0 ]]; then
    echo "This installer writes to $PREFIX and needs root."
    echo "Re-run as: sudo $0"
    echo "Or install for your user only with: ./local_install.sh"
    exit 1
fi

if [[ ! -x ./target/release/edt-down-for-me ]]; then
    if [[ "${EUID:-$(id -u)}" -eq 0 && -n "${SUDO_USER:-}" ]]; then
        echo "Building as $SUDO_USER (avoid compiling as root)..."
        sudo -u "$SUDO_USER" ./build.sh
    else
        ./build.sh
    fi
fi

install_file() {
    install -d -m 0755 "$(dirname "$2")"
    install -m "$3" "$1" "$2"
}

install_file ./target/release/edt-down-for-me "$BIN_DIR/edt-down-for-me" 0755
install_file ./assets/edt-down-for-me.desktop "$APP_DIR/edt-down-for-me.desktop" 0644
install_file ./assets/edt-down-for-me.svg "$ICON_SVG/edt-down-for-me.svg" 0644
if [[ -f ./assets/edt-down-for-me.png ]]; then
    install_file ./assets/edt-down-for-me.png "$ICON_PNG/edt-down-for-me.png" 0644
fi
install_file ./README.md "$DOC_DIR/README.md" 0644
install_file ./LICENSE "$DOC_DIR/LICENSE" 0644

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APP_DIR" >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$PREFIX/share/icons/hicolor" >/dev/null 2>&1 || true
fi

echo "Installed (Fedora/system):"
echo "  $BIN_DIR/edt-down-for-me"
echo "Launch with: edt-down-for-me"
echo
echo "Optional runtime packages on Fedora:"
echo "  sudo dnf install fontconfig libxkbcommon libxcb libwayland-client"
