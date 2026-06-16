#!/usr/bin/env bash
set -euo pipefail

arch="$(uname -m)"
case "$arch" in
  x86_64) target="linux-x86_64" ;;
  aarch64|arm64) target="linux-aarch64" ;;
  *) echo "Unsupported architecture: $arch" >&2; exit 1 ;;
esac

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_dir="$repo_root/src-tauri/resources/python/$target/site-packages"
venv_dir="$repo_root/.tmp/sherpa-python-$target"

rm -rf "$target_dir"
mkdir -p "$target_dir"

rm -rf "$venv_dir"
if ! python3 -m venv "$venv_dir"; then
  cat >&2 <<'EOF'
Failed to create a temporary Python venv.

Install Python venv support for your distro, then retry.
Fedora/Asahi example:
  sudo dnf install python3
EOF
  exit 1
fi

"$venv_dir/bin/python" -m ensurepip --upgrade >/dev/null
"$venv_dir/bin/python" -m pip install \
  --upgrade \
  --target "$target_dir" \
  numpy \
  sherpa-onnx

rm -rf "$venv_dir"
find "$target_dir" -type d -name '__pycache__' -prune -exec rm -rf {} +
touch "$target_dir/.gitkeep"
echo "Prepared sherpa-onnx Python runtime in $target_dir"
