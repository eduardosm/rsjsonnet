#!/usr/bin/env bash
set -euo pipefail

rust_version="$(cat "ci/rust-versions/${1}.txt")"
shift

echo "Installing Rust $rust_version"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain "$rust_version" "$@"

if [ -n "${GITHUB_PATH+x}" ]; then
  echo "$HOME/.cargo/bin" >> "$GITHUB_PATH"
fi
