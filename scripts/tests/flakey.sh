#!/usr/bin/env bash
# Runs the all the Rust integration tests

#set -euo pipefail
export RUST_LOG="${RUST_LOG:-info,timing=debug}"

mkdir -p ~/fedimint/temp

## now loop through the above array
for i in $(seq 1 10)
do
  echo "test run $i" >> temp/test-ci-all-runs.txt
  nix develop -c bash -c 'just test-ci-all |& ansi2txt >> temp/test-ci-all-runs.txt'
done
