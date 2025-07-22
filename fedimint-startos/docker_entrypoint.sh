#!/bin/bash

set -e

# Find the entrypoint script dynamically
ENTRYPOINT_SCRIPT=$(find /nix/store -type f -name '*-fedimintd-container-entrypoint.sh' | head -n 1)

if [[ -z "$ENTRYPOINT_SCRIPT" ]]; then
    echo "Error: fedimintd-container-entrypoint.sh not found in /nix/store" >&2
    exit 1
fi

# Bitcoin Core connection - Start9 convention
# When Bitcoin Core is a dependency, it's available at bitcoind.embassy
export FM_BITCOIN_NETWORK=bitcoin
export FM_BITCOIN_RPC_KIND=bitcoind

# Default RPC credentials for Start9 Bitcoin Core
# These are typically 'bitcoin' and a generated password
# For now, we'll use the standard defaults that most Start9 services use
export FM_BITCOIN_RPC_URL="http://bitcoin:password@bitcoind.embassy:8332"

# Other Fedimint settings
export FM_ENABLE_IROH=true
export FM_BIND_UI=0.0.0.0:8175
export FM_DATA_DIR=/fedimintd

echo "Starting Fedimint with Bitcoin Core at bitcoind.embassy:8332"

exec bash "$ENTRYPOINT_SCRIPT" "$@"
