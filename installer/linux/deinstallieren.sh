#!/usr/bin/env bash
# Entfernt RMengine. Weltdaten und Spielstände bleiben erhalten,
# außer man ruft das Skript mit --alles auf.
set -euo pipefail
ZIEL="${RMENGINE_ZIEL:-$HOME/.local/share/rmengine}"
BIN="${RMENGINE_BIN:-$HOME/.local/bin}"

rm -f "$BIN/rmengine" "$HOME/.local/share/applications/rmengine.desktop"
rm -f "$ZIEL/rmengine" "$ZIEL/rmengine.alt" "$ZIEL/rmengine.neu" "$ZIEL/ANLEITUNG.md" "$ZIEL/README.md"
if [ "${1:-}" = "--alles" ]; then
  rm -rf "$ZIEL"
  echo "RMengine wurde vollständig entfernt (inklusive Weltdaten und Spielständen)."
else
  rm -f "$ZIEL/deinstallieren.sh"
  echo "RMengine wurde entfernt. Deine Welt liegt weiterhin in $ZIEL."
  echo "Alles löschen: dieses Skript mit --alles aufrufen."
fi
