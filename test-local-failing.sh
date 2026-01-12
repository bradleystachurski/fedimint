#!/usr/bin/env bash
# test-local-failing.sh - Run only the failing backwards compat combinations

set -euo pipefail

source scripts/_common.sh

echo "=== Building binaries for v0.10.0-beta.2, v0.8.0, v0.8.1 ==="
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

echo "=== Running failing test combinations ==="
echo "These should fail with 'Gateway balance changed by 2007296'"
echo ""

# Run the two most basic failing scenarios
run_test_for_versions devimint_cli_test_single FM: v0.10.0-beta.2 CLI: v0.10.0-beta.2 GW: v0.8.0 LNv2: 0
run_test_for_versions devimint_cli_test_single FM: v0.10.0-beta.2 CLI: v0.10.0-beta.2 GW: v0.8.1 LNv2: 1

echo ""
echo "=== Test run completed ==="
