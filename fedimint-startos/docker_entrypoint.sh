#!/bin/bash

set -e

echo "=== Debugging: Let's see what config files exist ==="
find /data -name "*.yaml" -o -name "*.json" 2>/dev/null | head -20

echo -e "\n=== Checking if config file exists ==="
if [ -f /data/start9/config.yaml ]; then
    echo "Config file found at /data/start9/config.yaml"
    echo "First 50 lines:"
    head -50 /data/start9/config.yaml
else
    echo "No config file at /data/start9/config.yaml"
fi

echo -e "\n=== Let's also check Bitcoin Core's exported config ==="
# Sometimes Start9 exports config to environment or files
find /mnt -name "*bitcoin*" -type f 2>/dev/null | head -10

# Keep container running for inspection
echo -e "\n=== Container will stay running for debugging ==="
sleep 3600
