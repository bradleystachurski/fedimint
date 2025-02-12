#!/usr/bin/env bash

function version_lt() {
  if [ "$1" = "current" ]; then
    return 1
  elif [ "$2" = "current" ]; then
    return 0
  fi

  # replace `-` with `~` so `sort -V` correctly sorts pre-releases
  v1="${1//-/\~}"
  v2="${2//-/\~}"

  # use sort -V to compare the normalized versions.
  # if v1 is not the highest, then it is less than v2.
  [ "$v1" != "$(echo -e "$v1\n$v2" | sort -V | tail -n 1)" ]
}

# Example unit tests.
function run_test() {
  local ver1="$1"
  local ver2="$2"
  local expected="$3"

  version_lt "$ver1" "$ver2"
  local result=$?

  if [ "$result" -eq "$expected" ]; then
    echo "PASS: version_lt '$ver1' '$ver2' returned $result as expected."
  else
    echo "FAIL: version_lt '$ver1' '$ver2' returned $result, expected $expected."
  fi
}

echo "Running tests for version_lt function..."

# "current" handling
run_test "current" "current" 1
run_test "current" "v0.6.0" 1
run_test "v0.6.0" "current" 0

# Basic comparisons
run_test "v0.4.4" "v0.6.0" 0
run_test "v1.2.3" "v1.2.3" 1

# Pre-release comparisons
run_test "v0.3.4-rc.1" "v0.4.4" 0
run_test "v1.0.0-rc.1" "v1.0.0" 0
run_test "v1.0.0" "v1.0.0-rc.1" 1
run_test "v1.0.0-beta.2" "v1.0.0-beta.10" 0
run_test "v1.0.0-rc.10" "v1.0.0" 0
run_test "v1.2.3" "v1.2.3-alpha" 1
run_test "v1.2.3-alpha" "v1.2.3" 0

# More comparisons
run_test "v0.9.9" "v1.0.0" 0
run_test "v1.0.0" "v0.9.9" 1
run_test "v2.0.0" "v10.0.0" 0
run_test "v10.0.0" "v2.0.0" 1
run_test "v1.2.3" "v1.2.4" 0
run_test "v1.2.4" "v1.2.3" 1

run_test "v0.6-rc.1" "v0.6.0" 0

