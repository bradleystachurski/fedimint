#!/usr/bin/env bash
# run-full-backcompat.sh

set -euo pipefail

# Match CI environment
export FM_USE_UNKNOWN_MODULE=0
export FM_ENABLE_IROH=false
export RUST_LOG=${RUST_LOG:-h2=off,fm=debug,info}
export TMPDIR=/tmp

# Run the same script CI uses with the same version list
VERSIONS_TO_TEST="v0.7.0 v0.7.1 v0.7.2 v0.8.0 v0.8.1 v0.8.2 v0.9.0 v0.9.1 v0.10.0-beta.2"

scripts/tests/test-ci-all-backcompat.sh $VERSIONS_TO_TEST
