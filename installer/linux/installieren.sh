#!/usr/bin/env bash
# Installiert RMengine für den aktuellen Benutzer (ohne root).
#   Programm und Welt:  ~/.local/share/rmengine
#   Befehl:             ~/.local/bin/rmengine
# Vorhandene Weltdaten und Spielstände werden NIE überschrieben.
#
# Bei der ersten Installation wird gefragt, ob die Beispielwelt oder eine leere
# Welt angelegt werden soll. Ohne Frage:
#   installieren.sh --beispielwelt     bzw.   installieren.sh --leere-welt
set -euo pipefail

WELT=""
for a in "$@"; do
  case "$a" in
    --beispielwelt) WELT="beispiel" ;;
    --leere-welt)   WELT="leer" ;;
    *) echo "Unbekannte Angabe: $a (möglich: --beispielwelt, --leere-welt)" >&2; exit 1 ;;
  esac
done

ZIEL="${RMENGINE_ZIEL:-$HOME/.local/share/rmengine}"
BIN="${RMENGINE_BIN:-$HOME/.local/bin}"
QUELLE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Installiere RMengine nach $ZIEL ..."
mkdir -p "$ZIEL" "$BIN"
install -m 755 "$QUELLE/rmengine" "$ZIEL/rmengine"
if [ -f "$QUELLE/rmeditor" ]; then
  install -m 755 "$QUELLE/rmeditor" "$ZIEL/rmeditor"
fi
install -m 644 "$QUELLE/ANLEITUNG.md" "$QUELLE/README.md" "$ZIEL/"
install -m 755 "$QUELLE/deinstallieren.sh" "$ZIEL/"

if [ -e "$ZIEL/daten/welt.toml" ]; then
  echo "Deine Weltdaten in $ZIEL/daten bleiben unverändert."
else
  if [ -z "$WELT" ]; then
    WELT="beispiel"
    # Auch bei "curl ... | bash" vom Terminal fragen
    if { exec 3</dev/tty; } 2>/dev/null; then
      echo
      echo "Welche Welt soll angelegt werden?"
      echo "  1) Beispielwelt \"Eichenhain\" (Dorf, Wald, Händler, Gilden, Quests – zum Ausprobieren)"
      echo "  2) Leere Welt (nur ein Startraum – zum Selberbauen)"
      read -r -u 3 -p "Auswahl [1]: " antwort || antwort=""
      exec 3<&-
      case "$antwort" in 2*|l*|L*) WELT="leer" ;; esac
    fi
  fi
  if [ "$WELT" = "leer" ]; then
    "$ZIEL/rmengine" "$ZIEL/daten" --neue-welt
  else
    cp -r "$QUELLE/daten" "$ZIEL/daten"
    echo "Beispielwelt nach $ZIEL/daten kopiert."
  fi
fi
ln -sf "$ZIEL/rmengine" "$BIN/rmengine"
if [ -f "$ZIEL/rmeditor" ]; then
  ln -sf "$ZIEL/rmeditor" "$BIN/rmeditor"
fi

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
  if [ -f "$ZIEL/rmeditor" ]; then
    cat > "$HOME/.local/share/applications/rmeditor.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=RMengine Welteditor
Comment=Räume, Wege und Beschreibungen auf einer Karte bauen
Exec=$ZIEL/rmeditor $ZIEL/daten
Path=$ZIEL
Terminal=false
Categories=Game;Development;
DESKTOP
  fi
fi

echo
echo "Fertig! RMengine $("$ZIEL/rmengine" --version | awk '{print $2}') ist installiert."
echo
echo "  Starten:         rmengine"
echo "  Welteditor:      rmeditor  (braucht eine grafische Oberfläche)"
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
