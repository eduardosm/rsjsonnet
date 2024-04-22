#!/usr/bin/env bash
set -euo pipefail

. ci/utils.sh

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
