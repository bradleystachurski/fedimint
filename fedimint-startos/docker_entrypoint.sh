#!/bin/bash

set -e

echo "=== Container starting for debugging ==="
echo "Container will stay running. Use podman exec to inspect."
echo ""
echo "Commands to try:"
echo "  sudo podman exec -it fedimintd-mutinynet.embassy /bin/bash"
echo ""
echo "Then inside the container:"
echo "  find / -name '*.yaml' 2>/dev/null"
echo "  ls -la /data/"
echo "  ls -la /data/start9/"
echo "  cat /data/start9/config.yaml"
echo ""

# Keep container running
while true; do
    sleep 3600
done
