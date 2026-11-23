#!/usr/bin/env bash
# Lädt die neueste RMengine-Version herunter und installiert sie:
#   curl -fsSL https://github.com/areadox/RMengine/releases/latest/download/install.sh | bash
set -euo pipefail
URL="https://github.com/areadox/RMengine/releases/latest/download/rmengine-linux-x86_64.tar.gz"

if [ "$(uname -m)" != "x86_64" ]; then
  echo "Leider gibt es RMengine bisher nur für x86_64 (64-Bit-PCs)." >&2
  exit 1
fi
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
echo "Lade RMengine herunter ..."
if command -v curl >/dev/null; then
  curl -fsSL "$URL" -o "$TMP/rmengine.tar.gz"
else
  wget -qO "$TMP/rmengine.tar.gz" "$URL"
fi
tar -xzf "$TMP/rmengine.tar.gz" -C "$TMP"
bash "$TMP/rmengine/installieren.sh"
