#!/usr/bin/env bash
set -euo pipefail

. ci/utils.sh

if [ "$#" -ne 1 ]; then
  echo_stderr "Usage: $0 <rust_version>"
  exit 1
fi
rust_version="$1"

begin_group "Install Rust"
./ci/install-rust.sh "$rust_version" --profile minimal
. ci/cargo-env.sh
end_group

export RUSTDOCFLAGS="-D warnings"

begin_group "Fetch dependencies"
cargo fetch --locked
end_group

begin_group "Build"
cargo build --workspace --all-targets --frozen
end_group

begin_group "Test"
cargo test --workspace --frozen
end_group

begin_group "Doc"
cargo doc --workspace --frozen
end_group
