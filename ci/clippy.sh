#!/usr/bin/env bash
set -euo pipefail

cargo fetch --locked
cargo clippy --workspace --all-targets --frozen -- -D warnings
