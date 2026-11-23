# RMengine

Ein datengetriebener MUD-Treiber (Engine) in Rust. Räume, Gegenstände, Monster,
Fertigkeiten und Zauber werden zentral in einfachen TOML-Dateien verwaltet,
und die Landschaft zeichnet man als Textbild. Programmieren muss man dafür nicht.

**Wie man eine Welt baut, steht in der [ANLEITUNG.md](ANLEITUNG.md).**

## Installation

**Windows:** Installer von der [Release-Seite](https://github.com/areadox/RMengine/releases/latest) herunterladen und ausführen.

**Linux:**

```bash
curl -fsSL https://github.com/areadox/RMengine/releases/latest/download/install.sh | bash
```

**Aktualisieren:**

```bash
rmengine --update
```

Unter Windows geht das auch über das Startmenü. Die Weltdaten bleiben dabei unverändert.

Danach verbinden:

```bash
telnet localhost 4000
```

Mudlet oder ein anderes MUD-Programm funktioniert genauso.

## Aus dem Quellcode

```bash
cargo run --release              # Server starten (Port 4000)
cargo run --release -- --pruefen # nur die Weltdaten prüfen
cargo run --release -p rmeditor  # grafischer Welteditor (öffnet ./daten)
```

Der Welteditor liegt in `editor/` als eigenes Programm im selben Cargo-Workspace.
Ein einfaches `cargo build` baut weiterhin nur den Server.
Wie man den Editor bedient, steht in der [ANLEITUNG.md](ANLEITUNG.md#der-grafische-welteditor).

## Eine neue Version veröffentlichen

1. In `Cargo.toml` die `version` erhöhen, z.B. auf `0.2.0`, und committen.
2. Auf GitHub unter **Actions → Release → Run workflow** starten. Ins Feld *„Was ist neu?“* die Neuerungen schreiben: Sie erscheinen im Release und beim Update.

   Alternativ ein Tag hochladen:

   ```bash
   git tag v0.2.0
   ```

   ```bash
   git push origin v0.2.0
   ```

GitHub Actions baut dann Windows-Installer, Linux-Paket und die Programmdateien für
das Update und veröffentlicht alles als Release.

## Funktionen

- Telnet-Server für mehrere Spieler gleichzeitig, mit Farben und verdeckter Passworteingabe
- **Oberwelt:** eine Landschaft, die man als Textbild zeichnet, mit geländeabhängiger Sicht (Wald dämpft, Berge verdecken, Gipfel geben Weitblick), Städten und Dungeons als Eingängen, besonderen Orten, automatischen Beschreibungen und `weltkarte`
- ASCII-Stadtkarte, die automatisch aus den Raumkoordinaten entsteht, mit Stockwerken, Geländearten, Legende und Diagonalen
- zentrale Verwaltung von Items, Monstern, Fertigkeiten und Zaubern, alles in TOML
- gründliche Prüfung der Daten mit deutschen Fehlermeldungen und Tippfehler-Vorschlägen
- `neuladen` im laufenden Spiel
- Spieler werden gespeichert: Inventar, Ausrüstung, Fertigkeiten, Zauber, besuchte Räume, Spielzeit
- **Spielstand** (`sp`) mit Balken, Ausrüstungsboni und Fortschritt; **Spielerbuch** für persönliche Einstellungen (Karte, Kurzmodus, Zeilenbreite, Titel, Passwort …)
- Monster wandern in ihrem Gebiet umher und sprechen
- Zauber mit Heilung, Mana und Teleport, Abklingzeiten und Fertigkeitssteigerung
- Ausrüstungsplätze, benutzbare Tränke und Nahrung
- **Ausführliche Beschreibungen:** Details mit beliebig tiefen Unterdetails, Gerüche, Geräusche und Gefühl (`riech`, `hoer`, `taste`)
- **Bibliothek allgemeiner Beschreibungen** (Bäume, Himmel, Wasser, Möbel …), die Räume, Geländearten und Oberwelt-Felder mit `nutzt = [...]` verwenden
- **NPCs in eigenen Dateien** mit Gesprächen: Standardfragen (Gruß, Beruf, Aufgabe, Neuigkeiten …) und eigene Themen
- **Rollen:** Spieler, Baumeister und Admin, mit eigenen Befehlen
- **Grafischer Welteditor** (`rmeditor`): Räume auf einer Karte anlegen, verschieben, verbinden und beschreiben. Kommentare in den TOML-Dateien bleiben beim Speichern erhalten.

## Code-Überblick (`src/`)

| Datei        | Inhalt                                                       |
|--------------|--------------------------------------------------------------|
| `daten.rs`   | Datenformate, Laden und Prüfen der Weltdaten                 |
| `welt.rs`    | laufender Zustand: Spieler, Monster, Takt, Zurücksetzen      |
| `befehle.rs` | alle Spielerbefehle                                          |
| `karte.rs`   | Kartendarstellung für Städte und Dungeons                    |
| `oberwelt.rs`| Oberwelt: Kartenbild laden, Sicht, Felder als Räume, Karten  |
| `netz.rs`    | Telnet-Server und Login                                      |
| `spieler.rs` | Speichern, Laden, Passwörter                                 |
| `text.rs`    | Farben, Zeilenumbruch, Tippfehler-Vorschläge                 |
| `update.rs`  | Update-Funktion über GitHub Releases                         |
| `beschreibung.rs` | Details, Sinne und Bibliothek                          |
| `gespraech.rs` | NPC-Gespräche                                              |

Der Welteditor (`editor/src/`):

| Datei     | Inhalt                                                                |
|-----------|-----------------------------------------------------------------------|
| `main.rs` | Programmstart, Suche nach dem Datenordner                             |
| `welt.rs` | Räume laden, prüfen und kommentarerhaltend speichern (`toml_edit`)    |
| `app.rs`  | Oberfläche: Karte, Raumliste, Eigenschaften, Rückgängig (`egui`)      |

## Vorbereitet für das Kampfsystem

Diese Felder werden schon gelesen und geprüft, wirken aber noch nicht:

- Monster: `lebenspunkte`, `werte`, `aggressiv`, `beute`, `zauber`
- Gegenstände: `eigenschaften` wie `schaden` und `ruestung`
- Zauber: Wirkung `schaden` mit dem Ziel `gegner`

Ein Kampfsystem kann sich hier in den Takt (`Welt::takt`) einklinken.
