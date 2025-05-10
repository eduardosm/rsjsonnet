#!/usr/bin/env bash
set -euo pipefail

. ci/utils.sh

begin_group "Fetch dependencies"
cargo fetch --locked
end_group

begin_group "Build"
cargo build -p rsjsonnet --frozen
end_group

ext_jsonnet_ver="0.21.0"
begin_group "Download jsonnet $ext_jsonnet_ver"
curl -fL "https://github.com/google/jsonnet/archive/refs/tags/v${ext_jsonnet_ver}.tar.gz" | tar -xz
end_group

begin_group "Test"

ext_src_dir="$(readlink -f -- "jsonnet-$ext_jsonnet_ver")"
test_bin="$(readlink -f -- "target/debug/rsjsonnet")"

failures=0

cd "$ext_src_dir/test_suite"
for test in *.jsonnet; do
  if [[ "$test" = "safe_integer_conversion.jsonnet" ]]; then
    # Skip this test, since C++ Jsonnet considers 2^53 as a safe.
    continue
  fi

  extra_args=()

  if [[ "$test" =~ ^tla[.] ]]; then
    extra_args+=("--tla-str" "var1=test" "--tla-code" "var2={x:1,y:2}")
  else
    extra_args+=("--ext-str" "var1=test" "--ext-code" "var2={x:1,y:2}")
  fi

  echo -n "test $test ..."
  set +e
  (
    NO_COLOR=1 "$test_bin" "${extra_args[@]}" "$test" > test_result.stdout 2> test_result.stderr
    exit_code=$?

    if [[ "$test" = error.* ]] || [ "$test" = "invariant_manifest.jsonnet" ]; then
      if [[ $exit_code -ne 1 ]]; then
        echo "Finished with exit code $exit_code"
        exit 1
      fi
    else
      if [[ $exit_code -ne 0 ]]; then
        echo "Finished with exit code $exit_code"
        echo "stderr:"
        cat test_result.stderr
        exit 1
      fi

      if [ "$test" = "trace.jsonnet" ]; then
        # Skip checking output for trace test
        exit 0
      fi

      if [ -s test_result.stderr ]; then
        echo "stderr is not empty:"
        cat test_result.stderr
        exit 1
      fi

      if [ "$test" = "unparse.jsonnet" ]; then
        # Skip checking stdout for unparse.jsonnet due to rounding differences
        exit 0
      fi

      if [ ! -e "$test.golden" ]; then
        echo "true" > "$test.golden"
      fi

      diff "$test.golden" test_result.stdout > stdout.diff
      if [ -s stdout.diff ]; then
        echo "unexpected stdout"
        cat stdout.diff
        exit 1
      fi
    fi
  ) > test_report.txt 2>&1
  exit_code=$?
  set -e

  if [[ $exit_code -ne 0 ]]; then
    echo " fail"
    failures=$((failures + 1))
    cat test_report.txt
  else
    echo " ok"
  fi
done

end_group

if [[ $failures -ne 0 ]]; then
  echo "Failed $failures test(s)"
  exit 1
fi
