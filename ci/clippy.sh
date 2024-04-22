#!/usr/bin/env bash
set -euo pipefail

. ci/utils.sh

begin_group "Fetch dependencies"
cargo fetch --locked
end_group

begin_group "Run clippy"
cargo clippy --workspace --all-targets --frozen -- -D warnings
end_group
