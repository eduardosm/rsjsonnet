#!/usr/bin/env bash
set -euo pipefail

pkgs_dir="packages"
if [ -e "$pkgs_dir" ]; then
  echo "$pkgs_dir already exists"
  exit 1
fi

version="$(awk '/^version = ".+"$/ { sub("^version = \"", ""); sub("\"$", ""); print }' Cargo.toml)"

echo "::group::Fetch dependencies"
cargo fetch --locked
echo "::endgroup::"

echo "::group::Vendor dependencies"
mkdir .cargo
cargo vendor --frozen "$pkgs_dir" > .cargo/config.toml
echo "::endgroup::"

crates=(
  rsjsonnet-lang
  rsjsonnet-front
  rsjsonnet
)

for crate in "${crates[@]}"; do
  echo "::group::Package $crate"
  cargo package -p "$crate" --frozen
  tar -xf "target/package/$crate-$version.crate" -C "$pkgs_dir"
  pkg_checksum="$(sha256sum "target/package/$crate-$version.crate" | awk '{print $1}')"
  echo "{\"files\":{},\"package\":\"$pkg_checksum\"}" > "$pkgs_dir/$crate-$version/.cargo-checksum.json"
  echo "::endgroup::"
done

mkdir output
for crate in "${crates[@]}"; do
  cp -t output "target/package/$crate-$version.crate"
done
