#!/usr/bin/env bash
# Runs a CLI-based integration test

set -euo pipefail

export CARGO_BUILD_JOBS=2
export RUST_LOG=info,timing=debug
export RUST_BACKTRACE=1
source ./scripts/build.sh

devimint audit-benchmark

