#!/usr/bin/env bash
set -euo pipefail

. ci/utils.sh

begin_group "Install Rust"
./ci/install-rust.sh stable.txt --profile minimal -c clippy
# shellcheck disable=SC1091
. "$HOME/.cargo/env"
end_group

begin_group "Get release version"

if [[ "$GITHUB_REF" != "refs/tags/v"* ]]; then
  echo "Invalid ref: $GITHUB_REF"
  exit 1
fi

tag_version="${GITHUB_REF#refs/tags/v}"
echo "Tag version: $tag_version"

crate="rsjsonnet"
crate_version="$(crate_version "$crate")"
echo "Crate version: $crate_version"

if [ "$tag_version" != "$crate_version" ]; then
  echo "Tag version does not match crate version"
  exit 1
fi

echo "version=$tag_version" >> "$GITHUB_OUTPUT"
end_group
