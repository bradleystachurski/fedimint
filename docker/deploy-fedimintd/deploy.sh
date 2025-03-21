#!/usr/bin/env bash

set -euo pipefail

##
## Args handling
##

# domain to use for a host
domain="$1"
# ssh host name to use for ssh commands (defaults to the domain above)
ssh_host="${2:-${domain}}"

# host dir we'll keep the files on
host_dir=/root/fedimint-docker

echo >&2 '### Wiping previous setup'

# shellcheck disable=SC2087
ssh -q "root@$ssh_host" << 'EOF'
  systemctl stop fedimint-docker-compose
  cd /root
  rm -rf /root/fedimint-docker
  docker run --rm \
    -v fedimint-docker_bitcoind_data:/data \
    -v $(pwd):/backup \
    busybox tar czvf /backup/volume_backup.tar.gz -C /data .

  systemctl stop docker.socket
  systemctl disable docker.socket
  systemctl stop docker.service
  systemctl disable docker.service

  rm -rf /etc/systemd/system/fedimint-docker-compose.service
  systemctl daemon-reload

  apt-get purge -y $(dpkg -l | grep -Ei 'docker|containerd' | awk '{print $2}')
  apt-get autoremove -y
  rm -rf /var/lib/docker
  rm -rf /var/lib/containerd

  curl -fsSL https://get.docker.com -o get-docker.sh
  sh ./get-docker.sh
  rm get-docker.sh

  docker run --rm \
    -v fedimint-docker_bitcoind_data:/data \
    -v $(pwd):/backup \
    busybox sh -c "cd /data && tar xzvf /backup/volume_backup.tar.gz"
EOF


##
## Setup new machine
##

echo >&2 '### Setting up the server'

echo >&2 '### Copying files...'
cat << EOF | ssh "root@$ssh_host" "sudo cat > /etc/systemd/system/fedimint-docker-compose.service"
[Unit]
Description=Fedimint Docker Compose Service
Requires=docker.service
After=docker.service

[Service]
WorkingDirectory=${host_dir}
ExecStart=/usr/bin/docker compose up
ExecStop=/usr/bin/docker compose down
Restart=always

[Install]
WantedBy=multi-user.target
EOF

scp -r .env docker-compose.yaml "root@$ssh_host:/root/fedimint-docker"

echo >&2 '### Setting up...'

# shellcheck disable=SC2087
ssh -q "root@$ssh_host" << EOF
  touch ~/.hushlogin

  sed -i 's/my-super-host.com/$domain/g' $host_dir/.env

  systemctl daemon-reload
  systemctl enable fedimint-docker-compose
  systemctl start fedimint-docker-compose
EOF
