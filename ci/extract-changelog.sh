#!/usr/bin/env bash
set -euo pipefail

. ci/utils.sh

trim_empty_lines() {
  awk 'NF { x = 1 } x' | tac | awk 'NF { x = 1 } x' | tac
}

version="$(crate_version rsjsonnet)"
version="${version%-pre}"

input_file="CHANGELOG.md"
output_file="version-changelog"

# shellcheck disable=SC2016
awk_script='/^##[^#]/ { if (x) { exit }; if ($2 == ver) { x = 1; next } } x'
awk -v ver="$version" "$awk_script" "$input_file" | trim_empty_lines > "$output_file"

if [ ! -s "$output_file" ]; then
  echo "Changelog for version $version is empty"
  exit 1
fi

echo "Extracted changelog for version $version"
