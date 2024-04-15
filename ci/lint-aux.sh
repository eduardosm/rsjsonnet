#!/usr/bin/env bash
set -euo pipefail

echo "Checking MSRV consistency"

msrv="$(cat ci/rust-versions/msrv.txt)"
msrv="${msrv%.*}"

msrv_readmes=(
  README.md
  rsjsonnet/README.md
  rsjsonnet-front/README.md
  rsjsonnet-lang/README.md
)

for readme in "${msrv_readmes[@]}"; do
  if [[ "$(grep img.shields.io/badge/rustc "$readme")" != *"rustc-$msrv+-lightgray.svg"* ]]; then
    echo "Incorrect MSRV in $readme"
    exit 1
  fi
done

if [ "$(grep rust-version Cargo.toml)" != "rust-version = \"$msrv\"" ]; then
  echo "Incorrect rust-version in Cargo.toml"
  exit 1
fi

echo "Checking shell scripts with shellcheck"
find . -type f -name "*.sh" -not -path "./.git/*" -print0 | xargs -0 shellcheck

echo "Checking markdown documents with markdownlint"
find . -type f -name "*.md" -not -path "./.git/*" -print0 | xargs -0 markdownlint
