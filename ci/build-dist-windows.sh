#!/usr/bin/env bash
set -euo pipefail

. ci/utils.sh

begin_group "Install dependencies"
apt-get update
apt-get install -y --no-install-recommends \
  curl \
  ca-certificates \
  zip \
  gcc \
  libc6-dev \
  gcc-mingw-w64-x86-64 \
  gcc-mingw-w64-i686
end_group

begin_group "Install Rust"
./ci/install-rust.sh stable --profile minimal -t x86_64-pc-windows-gnu -t i686-pc-windows-gnu
. ci/cargo-env.sh
end_group

begin_group "Fetch Rust dependencies"
cargo fetch --locked
end_group

mkdir output

build_and_compress() {
  name="$1"
  target="$2"
  full_name="rsjsonnet-$name"

  begin_group "Build $name"
  cargo build -p rsjsonnet --target "$target" --release --frozen
  mkdir "output/$full_name"
  cp -t "output/$full_name" "target/$target/release/rsjsonnet.exe"
  (cd output; zip -r "$full_name.zip" "$full_name")
  end_group
}

build_and_compress windows-x86_64 x86_64-pc-windows-gnu
build_and_compress windows-i686 i686-pc-windows-gnu
