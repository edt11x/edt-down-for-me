#!/usr/bin/env bash
# Run edt-down-for-me. Builds a release binary if one is not already present.
set -euo pipefail
cd "$(dirname "$0")"

bin="./target/release/edt-down-for-me"
if [[ ! -x "$bin" ]]; then
    echo "Release binary not found; building..."
    ./build.sh
fi
exec "$bin" "$@"
