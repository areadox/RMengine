# MUD-Treiber

Ein datengetriebener MUD-Treiber (Engine) in Rust. Räume, Gegenstände, Monster,
Fertigkeiten und Zauber werden zentral in einfachen TOML-Dateien verwaltet.
Programmieren muss man dafür nicht.

**Wie man eine Welt baut, steht in der [ANLEITUNG.md](ANLEITUNG.md).**

```bash
cargo run --release              # Server starten (Port 4000)
cargo run --release -- --pruefen # nur die Weltdaten prüfen
```

```bash
telnet localhost 4000
```

## Funktionen

- Telnet-Server für mehrere Spieler gleichzeitig, mit Farben und verdeckter Passworteingabe
- ASCII-Karte, die automatisch aus den Raumkoordinaten entsteht, mit Stockwerken, Geländearten, Legende und Diagonalen
- zentrale Verwaltung von Items, Monstern, Fertigkeiten und Zaubern, alles in TOML
- gründliche Prüfung der Daten mit deutschen Fehlermeldungen und Tippfehler-Vorschlägen
- `neuladen` im laufenden Spiel
- Spieler werden gespeichert: Inventar, Ausrüstung, Fertigkeiten, Zauber, besuchte Räume
- Monster wandern in ihrem Gebiet umher und sprechen
- Zauber mit Heilung, Mana und Teleport, Abklingzeiten und Fertigkeitssteigerung
- Ausrüstungsplätze, benutzbare Tränke und Nahrung
- Baumeister-Befehle zum Testen

## Code-Überblick (`src/`)

| Datei        | Inhalt                                                       |
|--------------|--------------------------------------------------------------|
| `daten.rs`   | Datenformate, Laden und Prüfen der Weltdaten                 |
| `welt.rs`    | laufender Zustand: Spieler, Monster, Takt, Zurücksetzen      |
| `befehle.rs` | alle Spielerbefehle                                          |
| `karte.rs`   | Kartendarstellung                                            |
| `netz.rs`    | Telnet-Server und Login                                      |
| `spieler.rs` | Speichern, Laden, Passwörter                                 |
| `text.rs`    | Farben, Zeilenumbruch, Tippfehler-Vorschläge                 |

## Vorbereitet für das Kampfsystem

Diese Felder werden schon gelesen und geprüft, wirken aber noch nicht:

- Monster: `lebenspunkte`, `werte`, `aggressiv`, `beute`, `zauber`
- Gegenstände: `eigenschaften` wie `schaden` und `ruestung`
- Zauber: Wirkung `schaden` mit dem Ziel `gegner`

Ein Kampfsystem kann sich hier in den Takt (`Welt::takt`) einklinken.
