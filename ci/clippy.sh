#!/usr/bin/env bash
set -euo pipefail

. ci/utils.sh

begin_group "Install Rust"
./ci/install-rust.sh stable.txt --profile minimal -c clippy
# shellcheck disable=SC1090
. "$HOME/.cargo/env"
end_group

begin_group "Fetch dependencies"
cargo fetch --locked
end_group

begin_group "Run clippy"
cargo clippy --workspace --all-targets --frozen -- -D warnings
end_group
