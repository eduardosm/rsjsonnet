#!/usr/bin/env bash
set -euo pipefail

. ci/utils.sh

begin_group "Install Rust"
./ci/install-rust.sh stable.txt --profile minimal -c clippy
. ci/cargo-env.sh
end_group

begin_group "Fetch dependencies"
cargo fetch --locked
end_group

export CARGO_REGISTRY_TOKEN="$CRATES_IO_TOKEN"

crates=(
  rsjsonnet-lang
  rsjsonnet-front
  rsjsonnet
)

for crate in "${crates[@]}"; do
  begin_group "Publish $crate"
  cargo publish -p "$crate" --no-verify --locked
  end_group
done
