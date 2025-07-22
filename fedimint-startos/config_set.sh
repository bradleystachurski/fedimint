#!/bin/bash
set -e

# Read config from stdin
CONFIG=$(cat)

# Extract Bitcoin RPC details from the config
RPC_USER=$(echo "$CONFIG" | jq -r '.bitcoin.username // empty')
RPC_PASS=$(echo "$CONFIG" | jq -r '.bitcoin.password // empty')

# Save to a file that docker_entrypoint.sh can read
cat > /start-os/bitcoin-rpc.env << EOF
BITCOIN_RPC_USER=${RPC_USER}
BITCOIN_RPC_PASS=${RPC_PASS}
EOF

echo "Config saved"
