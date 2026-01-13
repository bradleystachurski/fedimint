#!/usr/bin/env bash
# run-failing-backcompat.sh - Test only the v0.8.0-v0.8.1 scenarios that failed

set -euo pipefail

source scripts/_common.sh

export FM_USE_UNKNOWN_MODULE=0
export FM_ENABLE_IROH=false
export RUST_LOG=${RUST_LOG:-h2=off,fm=debug,info}

echo "=== Building binaries ==="
export fm_bin_fedimintd_v0_10_0_beta_2=$(nix_build_binary_for_version "fedimintd" "v0.10.0-beta.2")
export fm_bin_gatewayd_v0_10_0_beta_2=$(nix_build_binary_for_version "gatewayd" "v0.10.0-beta.2")
export fm_bin_gateway_cli_v0_10_0_beta_2=$(nix_build_binary_for_version "gateway-cli" "v0.10.0-beta.2")
export fm_bin_fedimint_cli_v0_10_0_beta_2=$(nix_build_binary_for_version "fedimint-cli" "v0.10.0-beta.2")

export fm_bin_gatewayd_v0_8_0=$(nix_build_binary_for_version "gatewayd" "v0.8.0")
export fm_bin_gateway_cli_v0_8_0=$(nix_build_binary_for_version "gateway-cli" "v0.8.0")

export fm_bin_gatewayd_v0_8_1=$(nix_build_binary_for_version "gatewayd" "v0.8.1")
export fm_bin_gateway_cli_v0_8_1=$(nix_build_binary_for_version "gateway-cli" "v0.8.1")

echo "=== Pre-building workspace ==="
build_workspace
build_workspace_tests

# Define test functions
function devimint_cli_test_single() {
  ./scripts/tests/devimint-cli-test.sh
}
export -f devimint_cli_test_single

function devimint_cli_test() {
  ./scripts/tests/devimint-cli-tests.sh
}
export -f devimint_cli_test

function gw_config_test_lnd() {
  ./scripts/tests/gateway-module-test.sh config-test lnd
}
export -f gw_config_test_lnd

echo "=== Running failing test combinations ==="

tests=(
  "devimint_cli_test_single FM: v0.10.0-beta.2 CLI: v0.10.0-beta.2 GW: v0.8.0 LNv2: 0"
  "devimint_cli_test_single FM: v0.10.0-beta.2 CLI: v0.10.0-beta.2 GW: v0.8.0 LNv2: 1"
  "devimint_cli_test_single FM: v0.10.0-beta.2 CLI: v0.10.0-beta.2 GW: v0.8.1 LNv2: 0"
  "devimint_cli_test_single FM: v0.10.0-beta.2 CLI: v0.10.0-beta.2 GW: v0.8.1 LNv2: 1"
  
  "devimint_cli_test FM: v0.10.0-beta.2 CLI: v0.10.0-beta.2 GW: v0.8.0 LNv2: 0"
  "devimint_cli_test FM: v0.10.0-beta.2 CLI: v0.10.0-beta.2 GW: v0.8.0 LNv2: 1"
  "devimint_cli_test FM: v0.10.0-beta.2 CLI: v0.10.0-beta.2 GW: v0.8.1 LNv2: 0"
  "devimint_cli_test FM: v0.10.0-beta.2 CLI: v0.10.0-beta.2 GW: v0.8.1 LNv2: 1"
  
  "gw_config_test_lnd FM: v0.10.0-beta.2 CLI: v0.10.0-beta.2 GW: v0.8.0 LNv2: 0"
  "gw_config_test_lnd FM: v0.10.0-beta.2 CLI: v0.10.0-beta.2 GW: v0.8.0 LNv2: 1"
  "gw_config_test_lnd FM: v0.10.0-beta.2 CLI: v0.10.0-beta.2 GW: v0.8.1 LNv2: 0"
  "gw_config_test_lnd FM: v0.10.0-beta.2 CLI: v0.10.0-beta.2 GW: v0.8.1 LNv2: 1"
)

for test in "${tests[@]}"; do
  echo ""
  echo "### Running: $test"
  run_test_for_versions $test || {
    echo "FAILED: $test"
    exit 1
  }
done

echo ""
echo "=== All 12 previously-failing tests passed! ==="
