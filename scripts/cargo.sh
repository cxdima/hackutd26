#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if command -v cargo >/dev/null 2>&1; then exec cargo "$@"; fi
export CARGO_HOME="$PWD/.tools/cargo"
export RUSTUP_HOME="$PWD/.tools/rustup"
export PATH="$CARGO_HOME/bin:$PATH"
exec "$CARGO_HOME/bin/cargo" "$@"
