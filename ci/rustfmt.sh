#!/usr/bin/env bash
set -euo pipefail

. ci/utils.sh

begin_group "Install Rust"
./ci/install-rust.sh stable.txt --profile minimal -c rustfmt
. ci/cargo-env.sh
end_group

begin_group "Check formatting"
cargo fmt --all -- --check
end_group
