#!/usr/bin/env bash
# Install edt-down-for-me for the current user (no root required).
# Binary: ~/.local/bin/edt-down-for-me
# Desktop entry + icon: ~/.local/share/...
set -euo pipefail
cd "$(dirname "$0")"

PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"
APP_DIR="$PREFIX/share/applications"
ICON_DIR="$PREFIX/share/icons/hicolor/scalable/apps"
PNG_DIR="$PREFIX/share/icons/hicolor/128x128/apps"

if [[ ! -x ./target/release/edt-down-for-me ]]; then
    ./build.sh
fi

mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR" "$PNG_DIR"
install -m 0755 ./target/release/edt-down-for-me "$BIN_DIR/edt-down-for-me"
install -m 0644 ./assets/edt-down-for-me.desktop "$APP_DIR/edt-down-for-me.desktop"
install -m 0644 ./assets/edt-down-for-me.svg "$ICON_DIR/edt-down-for-me.svg"
if [[ -f ./assets/edt-down-for-me.png ]]; then
    install -m 0644 ./assets/edt-down-for-me.png "$PNG_DIR/edt-down-for-me.png"
fi

# Point the desktop file at this prefix if ~/.local/bin is not on PATH.
if ! command -v edt-down-for-me >/dev/null 2>&1; then
    abs_bin="$BIN_DIR/edt-down-for-me"
    sed -i "s|^Exec=edt-down-for-me|Exec=$abs_bin|" "$APP_DIR/edt-down-for-me.desktop"
fi

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APP_DIR" >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$PREFIX/share/icons/hicolor" >/dev/null 2>&1 || true
fi

echo "Installed for the current user:"
echo "  $BIN_DIR/edt-down-for-me"
echo "  $APP_DIR/edt-down-for-me.desktop"
echo
if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
    echo "Note: $BIN_DIR is not on PATH. Add it, or launch from the application menu."
fi
echo "Launch with: edt-down-for-me"
