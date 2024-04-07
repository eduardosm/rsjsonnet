#!/usr/bin/env bash
set -euo pipefail

echo "::group::Install dependencies"
yum install -y glibc-devel.i686 libgcc.i686
echo "::endgroup::"

echo "::group::Install Rust"

rust_version="$(cat "ci/rust-versions/stable.txt")"

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
  sh -s -- -y --default-toolchain "$rust_version" --profile minimal \
    -t x86_64-unknown-linux-gnu \
    -t i686-unknown-linux-gnu

# shellcheck disable=SC1090
. "$HOME/.cargo/env"

echo "::endgroup::"

export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
export CARGO_PROFILE_RELEASE_CODEGEN_PANIC=abort
export CARGO_PROFILE_RELEASE_STRIP=debuginfo

echo "::group::Fetch Rust dependencies"
cargo fetch --locked
mkdir output
echo "::endgroup::"

echo "::group::Build x86_64"
cargo build -p rsjsonnet --target x86_64-unknown-linux-gnu --release --frozen
mkdir output/rsjsonnet-linux-x86_64
cp target/x86_64-unknown-linux-gnu/release/rsjsonnet output/rsjsonnet-linux-x86_64/rsjsonnet
echo "::endgroup::"

echo "::group::Build i686"
cargo build -p rsjsonnet --target i686-unknown-linux-gnu --release --frozen
mkdir output/rsjsonnet-linux-i686
cp target/i686-unknown-linux-gnu/release/rsjsonnet output/rsjsonnet-linux-i686/rsjsonnet
echo "::endgroup::"
