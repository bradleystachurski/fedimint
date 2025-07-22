#!/bin/bash

set -e

# Find the entrypoint script dynamically
ENTRYPOINT_SCRIPT=$(find /nix/store -type f -name '*-fedimintd-container-entrypoint.sh' | head -n 1)

if [[ -z "$ENTRYPOINT_SCRIPT" ]]; then
    echo "Error: fedimintd-container-entrypoint.sh not found in /nix/store" >&2
    exit 1
fi

# Set the arguments that fedimintd expects
export FM_DATA_DIR=/fedimintd
export FM_BITCOIN_NETWORK=bitcoin
export FM_BIND_UI=0.0.0.0:8175
export FM_ENABLE_IROH=true

# Bitcoin Core connection
# When Bitcoin Core is a dependency, it's available at bitcoind.embassy
export FM_BITCOIND_URL="http://bitcoin:password@bitcoind.embassy:8332"

echo "Starting Fedimint with Bitcoin Core at bitcoind.embassy:8332"

# The entrypoint script should handle converting these FM_ env vars to command line args
exec bash "$ENTRYPOINT_SCRIPT" "$@"
