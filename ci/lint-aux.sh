#!/usr/bin/env bash
set -euo pipefail

. ci/utils.sh

begin_group "Check MSRV consistency"

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

crates=(
  rsjsonnet-lang
  rsjsonnet-front
  rsjsonnet
)

for crate in "${crates[@]}"; do
  if [ "$(crate_metadata "$crate" | jq -r '.rust_version')" != "$msrv" ]; then
    echo "Incorrect rust-version for $crate"
    exit 1
  fi
done

end_group

begin_group "Check shell scripts with shellcheck"
find . -type f -name "*.sh" -not -path "./.git/*" -print0 | xargs -0 shellcheck
end_group

begin_group "Check markdown documents with markdownlint"
find . -type f -name "*.md" -not -path "./.git/*" -print0 | xargs -0 markdownlint
end_group
