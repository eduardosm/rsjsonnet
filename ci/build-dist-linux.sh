#!/usr/bin/env bash
set -euo pipefail

. ci/utils.sh

begin_group "Install dependencies"
yum install -y glibc-devel.i686 libgcc.i686
end_group

begin_group "Install Rust"
./ci/install-rust.sh stable --profile minimal -t x86_64-unknown-linux-gnu -t i686-unknown-linux-gnu
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
  cp -t "output/$full_name" "target/$target/release/rsjsonnet"
  (cd output; tar -czf "$full_name.tar.gz" "$full_name")
  end_group
}

build_and_compress linux-x86_64 x86_64-unknown-linux-gnu
build_and_compress linux-i686 i686-unknown-linux-gnu
