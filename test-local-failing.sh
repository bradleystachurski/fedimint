#!/usr/bin/env bash
# test-local-failing.sh - Run only the failing backwards compat combinations

set -euo pipefail

source scripts/_common.sh

export FM_USE_UNKNOWN_MODULE=0
export FM_ENABLE_IROH=false
export RUST_LOG=${RUST_LOG:-h2=off,fm=debug,info}

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
add_target_dir_to_path

echo "=== Test 1: FM v0.10.0-beta.2 + CLI v0.10.0-beta.2 + GW v0.8.0 ==="
use_fed_binaries_for_version "v0.10.0-beta.2"
use_client_binaries_for_version "v0.10.0-beta.2"
use_gateway_binaries_for_version "v0.8.0"
export FM_BACKWARDS_COMPATIBILITY_TEST=1
export FM_OFFLINE_NODES=0
export FM_ENABLE_MODULE_LNV2=0
devimint cli-tests

echo ""
echo "=== Test 2: FM v0.10.0-beta.2 + CLI v0.10.0-beta.2 + GW v0.8.1 ==="
use_fed_binaries_for_version "v0.10.0-beta.2"
use_client_binaries_for_version "v0.10.0-beta.2"
use_gateway_binaries_for_version "v0.8.1"
export FM_BACKWARDS_COMPATIBILITY_TEST=1
export FM_OFFLINE_NODES=0
export FM_ENABLE_MODULE_LNV2=1
devimint cli-tests

echo ""
echo "=== Test run completed ==="
