#!/usr/bin/env bash
set -euo pipefail

if ! command -v ydotoold >/dev/null 2>&1; then
  echo "ydotoold not found. Install ydotool first." >&2
  exit 1
fi

if [ "$(id -u)" -ne 0 ]; then
  exec sudo "$0" "$@"
fi

install -d /etc/systemd/system/ydotool.service.d
cat >/etc/systemd/system/ydotool.service.d/kazamo.conf <<'EOF'
[Service]
ExecStart=
ExecStart=/usr/bin/ydotoold --socket-path=/tmp/.ydotool_socket --socket-perm=0666
EOF

systemctl daemon-reload
systemctl enable --now ydotool.service
systemctl restart ydotool.service

echo "ydotoold configured for Kazamo:"
ls -l /tmp/.ydotool_socket
