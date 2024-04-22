#!/usr/bin/env bash
set -euo pipefail

. ci/utils.sh

begin_group "Install Rust"
./ci/install-rust.sh stable.txt --profile minimal
. ci/cargo-env.sh
end_group

pkgs_dir="packages"
if [ -e "$pkgs_dir" ]; then
  echo "$pkgs_dir already exists"
  exit 1
fi

begin_group "Fetch dependencies"
cargo fetch --locked
end_group

begin_group "Vendor dependencies"
mkdir .cargo
cargo vendor --frozen "$pkgs_dir" > .cargo/config.toml
end_group

crates=(
  rsjsonnet-lang
  rsjsonnet-front
  rsjsonnet
)

mkdir output

for crate in "${crates[@]}"; do
  begin_group "Package $crate"
  version="$(crate_metadata "$crate" | jq -r ".version")"
  cargo package -p "$crate" --frozen
  tar -xf "target/package/$crate-$version.crate" -C "$pkgs_dir"
  pkg_checksum="$(sha256sum "target/package/$crate-$version.crate" | awk '{print $1}')"
  echo "{\"files\":{},\"package\":\"$pkg_checksum\"}" > "$pkgs_dir/$crate-$version/.cargo-checksum.json"
  cp -t output "target/package/$crate-$version.crate"
  end_group
done
