#!/bin/bash

set -e

# Find the entrypoint script dynamically
ENTRYPOINT_SCRIPT=$(find /nix/store -type f -name '*-fedimintd-container-entrypoint.sh' | head -n 1)

if [[ -z "$ENTRYPOINT_SCRIPT" ]]; then
    echo "Error: fedimintd-container-entrypoint.sh not found in /nix/store" >&2
    exit 1
fi

# Load Bitcoin RPC credentials
if [ -f /start-os/bitcoin-rpc.env ]; then
    source /start-os/bitcoin-rpc.env
    export FM_BITCOIN_RPC_URL="http://${BITCOIN_RPC_USER}:${BITCOIN_RPC_PASS}@bitcoind.embassy:8332"
else
    echo "ERROR: Bitcoin RPC credentials not found. Please configure the service."
    exit 1
fi

# Bitcoin Core connection
export FM_BITCOIN_NETWORK=bitcoin
export FM_BITCOIN_RPC_KIND=bitcoind

# Other Fedimint settings
export FM_ENABLE_IROH=true
export FM_BIND_UI=0.0.0.0:8175
export FM_DATA_DIR=/fedimintd

echo "Connecting to Bitcoin Core at bitcoind.embassy:8332"

exec bash "$ENTRYPOINT_SCRIPT" "$@"
