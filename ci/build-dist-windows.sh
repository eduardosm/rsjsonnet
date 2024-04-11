#!/usr/bin/env bash
set -euo pipefail

echo "::group::Install dependencies"
apt-get update
apt-get install -y --no-install-recommends \
  curl \
  ca-certificates \
  gcc \
  libc6-dev \
  gcc-mingw-w64-x86-64 \
  gcc-mingw-w64-i686
echo "::endgroup::"

echo "::group::Install Rust"

rust_version="$(cat "ci/rust-versions/stable.txt")"

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
  sh -s -- -y --default-toolchain "$rust_version" --profile minimal \
    -t x86_64-pc-windows-gnu \
    -t i686-pc-windows-gnu

# shellcheck disable=SC1090
. "$HOME/.cargo/env"

echo "::endgroup::"

export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
export CARGO_PROFILE_RELEASE_CODEGEN_PANIC=abort
export CARGO_PROFILE_RELEASE_STRIP=debuginfo

echo "::group::Fetch Rust dependencies"
cargo fetch --locked
echo "::endgroup::"

mkdir output

echo "::group::Build x86_64"
cargo build -p rsjsonnet --target x86_64-pc-windows-gnu --release --frozen
mkdir output/rsjsonnet-windows-x86_64
cp -t output/rsjsonnet-windows-x86_64 target/x86_64-pc-windows-gnu/release/rsjsonnet.exe
echo "::endgroup::"

echo "::group::Build i686"
cargo build -p rsjsonnet --target i686-pc-windows-gnu --release --frozen
mkdir output/rsjsonnet-windows-i686
cp -t output/rsjsonnet-windows-i686 target/i686-pc-windows-gnu/release/rsjsonnet.exe
echo "::endgroup::"
