# shellcheck shell=bash

echo_stderr() {
  echo "$@" >&2
}

begin_group() {
  if [ $# -ne 1 ]; then
    echo_stderr "Invalid use of $0"
    exit 1
  fi
  echo "::group::$1"
}

end_group() {
  if [ $# -ne 0 ]; then
    echo_stderr "Invalid use of $0"
    exit 1
  fi
  echo "::endgroup::"
}

crate_metadata() {
  if [ $# -ne 1 ]; then
    echo_stderr "Invalid use of $0"
    exit 1
  fi
  crate="$1"
  cargo metadata --format-version 1 --locked --no-deps | jq -r "
    [ .packages[] | select(.name == \"$crate\") ] |
      if length == 1 then
        first
      else
        error(\"expected exactly one package named $crate\")
      end
  "
}
