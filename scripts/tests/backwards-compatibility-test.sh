#!/usr/bin/env bash

set -euo pipefail

tmpdir=$(dirname "$(mktemp -u)")
export TEST_RESULTS_FILE="$tmpdir/test_results"
echo "fed_version,client_version,gateway_version,exit_code" > "$TEST_RESULTS_FILE"

# TODO: weird approach, but get it working then take a differnt
export HAS_ERROR_FILE="$tmpdir/has_error"
rm -rf "$HAS_ERROR_FILE"

# all versions to use for testing
versions=("v0.2.1")
>&2 echo "Running backwards-compatibility tests for versions: ${versions[*]}"

# signal to downstream test scripts
export FM_BACKWARDS_COMPATIBILITY_TEST=1

function nix_build_binary_for_version() {
  binary="$1"
  version="$2"
  echo "$(nix build 'github:fedimint/fedimint/'"$version"'#'"$binary" --no-link --print-out-paths)/bin/$binary"
}
export -f nix_build_binary_for_version

function use_fed_binaries_for_version() {
  version=$1
  if [[ "$version" == "current" ]]; then
    unset FM_FEDIMINTD_BASE_EXECUTABLE
  else
    >&2 echo "Compiling fed binaries for version $version..."
    FM_FEDIMINTD_BASE_EXECUTABLE="$(nix_build_binary_for_version 'fedimintd' "$version")"
    export FM_FEDIMINTD_BASE_EXECUTABLE
  fi
}
export -f use_fed_binaries_for_version

function use_client_binaries_for_version() {
  version=$1
  if [[ "$version" == "current" ]]; then
    unset FM_FEDIMINT_CLI_BASE_EXECUTABLE
    unset FM_GATEWAY_CLI_BASE_EXECUTABLE
  else
    >&2 echo "Compiling client binaries for version $version..."
    FM_FEDIMINT_CLI_BASE_EXECUTABLE="$(nix_build_binary_for_version 'fedimint-cli' "$version")"
    export FM_FEDIMINT_CLI_BASE_EXECUTABLE
    FM_GATEWAY_CLI_BASE_EXECUTABLE="$(nix_build_binary_for_version 'gateway-cli' "$version")"
    export FM_GATEWAY_CLI_BASE_EXECUTABLE
  fi
}
export -f use_client_binaries_for_version

function use_gateway_binaries_for_version() {
  version=$1
  if [[ "$version" == "current" ]]; then
    unset FM_GATEWAYD_BASE_EXECUTABLE
  else
    >&2 echo "Compiling gateway binaries for version $version..."
    FM_GATEWAYD_BASE_EXECUTABLE="$(nix_build_binary_for_version 'gatewayd' "$version")"
    export FM_GATEWAYD_BASE_EXECUTABLE
  fi
}
export -f use_gateway_binaries_for_version

versions+=("current")
version_matrix=()
for fed_version in "${versions[@]}"; do
  for client_version in "${versions[@]}"; do
    for gateway_version in "${versions[@]}"; do
      # test-ci-all already tests binaries running the same version, so no need to run again
      if [[ "$fed_version" == "$client_version" && "$fed_version" == "$gateway_version" ]]; then
        continue
      fi
      version_matrix+=("$fed_version $client_version $gateway_version")
    done
  done
done

function run_test_for_versions() {
  IFS=' ' read -r fed_version client_version gateway_version <<< "$1"
  # fed_version="$1"
  # client_version="$2"
  # gateway_version="$3"
  # echo "fed_version: $fed_version"
  # echo "client_version: $client_version"
  # echo "gateway_version: $gateway_version"

  use_fed_binaries_for_version "$fed_version"
  use_client_binaries_for_version "$client_version"
  use_gateway_binaries_for_version "$gateway_version"

  >&2 echo "========== Starting backwards-compatibility run ==========="
  >&2 echo "fed version: $fed_version"
  >&2 echo "client version: $client_version"
  >&2 echo "gateway version: $gateway_version"

  # continue running against other versions if there's a failure
  # set +e
  # (
  #   ./scripts/tests/test-ci-all.sh
  # ) 2>&1
  set +e
  ./scripts/tests/test-ci-all.sh
  exit_code=$?
  set -e
  echo "$fed_version,$client_version,$gateway_version,$exit_code" >> "$TEST_RESULTS_FILE"
  if [[ "$exit_code" -gt 0 ]]; then
    touch "$HAS_ERROR_FILE"
  fi

  # cleanup devimint dir
  # don't want to delete devimint dirs while running
  # tmpdir=$(dirname "$(mktemp -u)")
  # rm -rf "$tmpdir"/devimint-*

  >&2 echo "========== Finished backwards-compatibility run ==========="
  >&2 echo "fed version: $fed_version"
  >&2 echo "client version: $client_version"
  >&2 echo "gateway version: $gateway_version"
}
export -f run_test_for_versions

export parallel_jobs='+0'
# export parallel_jobs='2'
joblog="$tmpdir/backwards-compatibility-joblog"

start_time=$(date +%s.%N)
# parallel -j 3 run_test_for_versions ::: "${version_matrix[@]}"
  # --joblog "$joblog" \
  # --halt-on-error 1 \
  # --nice 15 \
  # --jobs "$parallel_jobs" \
  # --timeout 600 \
  # --load 80% \
  # --delay 30 \
  # --memfree 1G \
parallel \
  run_test_for_versions ::: "${version_matrix[@]}"

end_time=$(date +%s.%N)
elapsed_time=$(echo "$end_time - $start_time" | bc)

rm -rf "$tmpdir"/devimint-*


>&2 echo "Elapsed time: $elapsed_time seconds"

>&2 echo "Backwards-compatibility tests summary:"
>&2 column -t -s ',' "$TEST_RESULTS_FILE"

if [[ -f "$HAS_ERROR_FILE" ]]; then
  exit 1
fi
