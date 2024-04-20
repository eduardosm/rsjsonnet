#!/usr/bin/env bash
set -euo pipefail

rust_version="$1"
shift

if [[ "$rust_version" = *.txt ]]; then
  rust_version="$(cat "ci/rust-versions/$rust_version")"
fi

echo "Installing Rust $rust_version"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain "$rust_version" "$@"
