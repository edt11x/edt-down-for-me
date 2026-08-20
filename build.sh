#!/usr/bin/env bash
# Build edt-down-for-me in release mode.
set -euo pipefail
cd "$(dirname "$0")"

if ! command -v cargo >/dev/null 2>&1; then
    echo "error: cargo is not installed. Install Rust from https://rustup.rs/" >&2
    exit 1
fi

echo "Building edt-down-for-me (release)..."
cargo build --release
echo "Built: $(pwd)/target/release/edt-down-for-me"
