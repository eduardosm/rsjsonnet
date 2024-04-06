#!/usr/bin/env bash
set -euo pipefail

export RUSTDOCFLAGS="-D warnings"

cargo fetch --locked

cargo build --workspace --all-targets --frozen
cargo test --workspace --frozen
cargo doc --workspace --frozen
