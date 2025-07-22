#!/bin/bash

set -e

# Find the entrypoint script dynamically
ENTRYPOINT_SCRIPT=$(find /nix/store -type f -name '*-fedimintd-container-entrypoint.sh' | head -n 1)

if [[ -z "$ENTRYPOINT_SCRIPT" ]]; then
    echo "Error: fedimintd-container-entrypoint.sh not found in /nix/store" >&2
    exit 1
fi

echo "=== Debugging Start9 environment ==="
echo "Environment variables:"
env | sort

echo -e "\n=== Checking for config files ==="
find /start-os -type f 2>/dev/null || echo "Nothing in /start-os"

echo -e "\n=== Network check ==="
# Check if bitcoind.embassy is resolvable
ping -c 1 bitcoind.embassy 2>/dev/null && echo "bitcoind.embassy is reachable" || echo "bitcoind.embassy not found"

# Keep container running for inspection
sleep 3600
