#!/usr/bin/env bash
set -euo pipefail

if [ $# -ne 2 ]; then
  echo "Usage: $0 <base_ref> <head_ref>"
  exit 1
fi

commits="$(git rev-list --reverse "^$1" "$2")"
mapfile -t commits <<< "$commits"

lf=$'\n'
cr=$'\r'
exclam='!'
types="build|change|chore|ci|doc|feat|fix|perf|refactor|revert|style|test"
regex="^($types)(\([^$lf$cr]+\))?$exclam?: .+\$"

echo "Checking ${#commits[@]} commit(s)"
for commit in "${commits[@]}"; do
  echo "Checking commit $commit"
  commit_msg="$(git log --format=%B -n 1 "$commit")"
  if [[ ! "$commit_msg" =~ $regex ]]; then
    echo "Commit $commit does not follow Conventional Commits format"
    exit 1
  fi
done
