#!/bin/bash

set -e

# Find the entrypoint script dynamically
ENTRYPOINT_SCRIPT=$(find /nix/store -type f -name '*-fedimintd-container-entrypoint.sh' | head -n 1)

if [[ -z "$ENTRYPOINT_SCRIPT" ]]; then
    echo "Error: fedimintd-container-entrypoint.sh not found in /nix/store" >&2
    exit 1
fi

# Debug: Check what's available
echo "=== Checking for Bitcoin RPC info ==="
echo "Environment variables with BITCOIN:"
env | grep -i bitcoin || echo "No BITCOIN env vars found"

echo "Checking mounted volumes:"
ls -la /mnt/ 2>/dev/null || echo "No /mnt directory"
ls -la /start-os/ 2>/dev/null || echo "No /start-os directory"

# For now, let's see if Start9 sets any standard env vars
# Otherwise we'll need to implement proper config

# Set environment variables that fedimintd expects
export FM_DATA_DIR=/fedimintd
export FM_BITCOIN_NETWORK=bitcoin
export FM_BIND_UI=0.0.0.0:8175
export FM_ENABLE_IROH=true

# Bitcoin Core connection - we need the real credentials
# This is a placeholder that won't work
export FM_BITCOIND_URL="http://bitcoin:password@bitcoind.embassy:8332"

echo "Starting Fedimint with Bitcoin Core at bitcoind.embassy:8332"
echo "WARNING: Using placeholder credentials - this will fail!"

exec bash "$ENTRYPOINT_SCRIPT" "$@"
