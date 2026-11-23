#!/usr/bin/env bash
# Installiert RMengine für den aktuellen Benutzer (ohne root).
#   Programm und Welt:  ~/.local/share/rmengine
#   Befehl:             ~/.local/bin/rmengine
# Vorhandene Weltdaten und Spielstände werden NIE überschrieben.
set -euo pipefail

ZIEL="${RMENGINE_ZIEL:-$HOME/.local/share/rmengine}"
BIN="${RMENGINE_BIN:-$HOME/.local/bin}"
QUELLE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Installiere RMengine nach $ZIEL ..."
mkdir -p "$ZIEL" "$BIN"
install -m 755 "$QUELLE/rmengine" "$ZIEL/rmengine"
install -m 644 "$QUELLE/ANLEITUNG.md" "$QUELLE/README.md" "$ZIEL/"
install -m 755 "$QUELLE/deinstallieren.sh" "$ZIEL/"

if [ -e "$ZIEL/daten/welt.toml" ]; then
  echo "Deine Weltdaten in $ZIEL/daten bleiben unverändert."
else
  cp -r "$QUELLE/daten" "$ZIEL/daten"
  echo "Beispielwelt nach $ZIEL/daten kopiert."
fi
ln -sf "$ZIEL/rmengine" "$BIN/rmengine"

# Eintrag im Anwendungsmenü (falls es eins gibt)
if [ -d "$HOME/.local/share/applications" ] || mkdir -p "$HOME/.local/share/applications" 2>/dev/null; then
  cat > "$HOME/.local/share/applications/rmengine.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=RMengine
Comment=MUD-Server starten
Exec=$ZIEL/rmengine
Path=$ZIEL
Terminal=true
Categories=Game;
DESKTOP
fi

echo
echo "Fertig! RMengine $("$ZIEL/rmengine" --version | awk '{print $2}') ist installiert."
echo
echo "  Starten:         rmengine"
echo "  Welt prüfen:     rmengine --pruefen"
echo "  Aktualisieren:   rmengine --update"
echo "  Weltdaten:       $ZIEL/daten"
echo "  Anleitung:       $ZIEL/ANLEITUNG.md"
case ":$PATH:" in
  *":$BIN:"*) ;;
  *) echo
     echo "Hinweis: $BIN ist nicht in deinem PATH. Füge diese Zeile zu ~/.bashrc hinzu:"
     echo "  export PATH=\"\$HOME/.local/bin:\$PATH\"" ;;
esac
