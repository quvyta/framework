#!/usr/bin/env bash
# Starts the Quvyta showcase from the folder this script sits in.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"
command -v cargo >/dev/null || { echo "cargo is not installed; install Rust from https://rustup.rs" >&2; exit 1; }
exec cargo run -p showcase "$@"
