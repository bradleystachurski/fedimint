#!/bin/bash

set -e

# Find the entrypoint script dynamically
ENTRYPOINT_SCRIPT=$(find /nix/store -type f -name '*-fedimintd-container-entrypoint.sh' | head -n 1)

if [[ -z "$ENTRYPOINT_SCRIPT" ]]; then
    echo "Error: fedimintd-container-entrypoint.sh not found in /nix/store" >&2
    exit 1
fi

# Wait for Start9 to create the config file
# Based on FM_DATA_DIR=/data, the config should be at /data/start9/config.yaml
echo "Waiting for Start9 config..."
while [ ! -f /data/start9/config.yaml ]; do
    sleep 1
done

echo "Config file found at /data/start9/config.yaml, parsing Bitcoin RPC credentials..."

# Parse the YAML manually
BITCOIN_USER=$(grep -A10 "bitcoin:" /data/start9/config.yaml | grep "user:" | sed 's/.*user: *//; s/"//g' | tr -d ' ')
BITCOIN_PASS=$(grep -A10 "bitcoin:" /data/start9/config.yaml | grep "password:" | sed 's/.*password: *//; s/"//g' | tr -d ' ')

if [ -z "$BITCOIN_USER" ] || [ -z "$BITCOIN_PASS" ]; then
    echo "ERROR: Could not parse Bitcoin RPC credentials from config"
    exit 1
fi

echo "Got Bitcoin RPC credentials: user=$BITCOIN_USER"

# Set environment variables that fedimintd expects
export FM_BITCOIN_NETWORK=bitcoin
export FM_BIND_UI=0.0.0.0:8175
export FM_ENABLE_IROH=true

# Bitcoin Core connection with actual credentials
export FM_BITCOIND_URL="http://${BITCOIN_USER}:${BITCOIN_PASS}@bitcoind.embassy:8332"

echo "Starting Fedimint with Bitcoin Core at bitcoind.embassy:8332"

exec bash "$ENTRYPOINT_SCRIPT" "$@"
