#!/bin/bash
set -e

# Output the config spec
cat << 'EOF'
{
  "bitcoin": {
    "type": "pointer",
    "subtype": "package",
    "package-id": "bitcoind",
    "target": "config",
    "interface": "rpc",
    "selector": {
      "multi": false
    },
    "name": "Bitcoin Core",
    "description": "The Bitcoin Core node to connect to"
  }
}
EOF
