#!/usr/bin/env bash
set -euo pipefail

. ci/utils.sh

begin_group "Install lint utilities"
sudo apt-get update
sudo apt-get install -y --no-install-recommends shellcheck
sudo npm install -g markdownlint-cli
end_group

begin_group "Install Rust"
./ci/install-rust.sh stable.txt --profile minimal
# shellcheck disable=SC1091
. "$HOME/.cargo/env"
end_group

crates=(
  rsjsonnet-lang
  rsjsonnet-front
  rsjsonnet
)

begin_group "Check crate versions"

expected_version="$(crate_version rsjsonnet)"
for crate in "${crates[@]}"; do
  version="$(crate_version "$crate")"

  if [[ ! "$version" =~ ^[0-9]\.[0-9]\.[0-9](-pre)?$ ]]; then
    echo "Invalid version for $crate"
    exit 1
  fi

  if [[ "$version" = *-pre ]]; then
    publish_ok="$(crate_metadata "$crate" | jq '.publish == []')"
  else
    publish_ok="$(crate_metadata "$crate" | jq '.publish == null')"
  fi
  if [ "$publish_ok" != "true" ]; then
    echo "Invalid publish for $crate"
    exit 1
  fi

  if [ "$version" != "$expected_version" ]; then
    echo "Incorrect version for $crate"
    exit 1
  fi
done

version="$(crate_version "rsjsonnet")"
changelog_date="$(awk -v ver="${version%-pre}" '/^## / { if ($2 == ver) print $3 }' CHANGELOG.md)"
if [[ "$version" = *-pre ]]; then
  if [ "$changelog_date" != "(unreleased)" ]; then
    echo "Invalid date in changelog for version $version"
    exit 1
  fi
else
  if [[ ! "$changelog_date" =~ \([0-9]{4}-[0-9]{2}-[0-9]{2}\) ]]; then
    echo "Invalid date in changelog for version $version"
    exit 1
  fi
fi

end_group

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
