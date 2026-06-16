#!/usr/bin/env bash
set -euo pipefail

arch="$(uname -m)"
case "$arch" in
  x86_64) target="linux-x86_64" ;;
  aarch64|arm64) target="linux-aarch64" ;;
  *) exit 0 ;;
esac

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
site_packages="$repo_root/src-tauri/resources/python/$target/site-packages"

if PYTHONPATH="$site_packages" python3 - <<'PY' >/dev/null 2>&1
import numpy
import sherpa_onnx
PY
then
  exit 0
fi

"$repo_root/scripts/prepare-sherpa-python.sh"
