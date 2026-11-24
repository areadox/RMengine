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

1. In `Cargo.toml` und `editor/Cargo.toml` die `version` erhöhen (wir zählen hinten weiter: 0.2.1, 0.2.2, …) und committen.
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
- **MUD-Protokolle** für Programme wie Mudlet, TinTin++ oder MUSHclient: GMCP (Lebensbalken, Automapper, Chat), MSDP, MCCP2-Kompression, NAWS (Zeilenbreite passt sich dem Fenster an), TTYPE/MTTS, EOR/GA, CHARSET (UTF-8 oder ISO-8859-1) und MSSP für MUD-Verzeichnisse
- **Oberwelt:** eine Landschaft, die man als Textbild zeichnet, mit geländeabhängiger Sicht (Wald dämpft, Berge verdecken, Gipfel geben Weitblick), Städten und Dungeons als Eingängen, besonderen Orten, automatischen Beschreibungen und `weltkarte`
- ASCII-Stadtkarte, die automatisch aus den Raumkoordinaten entsteht, mit Stockwerken, Geländearten, Legende und Diagonalen
- zentrale Verwaltung von Items, Monstern, Fertigkeiten und Zaubern, alles in TOML
- gründliche Prüfung der Daten mit deutschen Fehlermeldungen und Tippfehler-Vorschlägen
- `neuladen` im laufenden Spiel und eine **Server-Konsole** im Fenster der Engine (neuladen, wer, verkuende, rolle, fernwartung, ende …)
- Spieler werden gespeichert: Inventar, Ausrüstung, Fertigkeiten, Zauber, besuchte Räume, Spielzeit
- **Spielstand** (`sp`) mit Balken, Ausrüstungsboni und Fortschritt; **Spielerbuch** für persönliche Einstellungen (Karte, Kurzmodus, Zeilenbreite, Titel, Passwort …)
- Monster wandern in ihrem Gebiet umher und sprechen
- Zauber mit Heilung, Mana und Teleport, Abklingzeiten und Fertigkeitssteigerung; **Schilde und Stärkungen**, die eine Zeit lang wirken (Schild, Rüstung, Werte, Kraft, Tempo, Heilung über Zeit, Gift)
- **Kampftechniken** (`spezial`) mit Waffenschlag, Mana- und Ausdauerkosten, Betäuben
- **Geld**, **Händler** (`waren`, `kaufe`, `verkaufe`) und **Lehrmeister** (`lerne`)
- **Rassen** (Wahl bei der Erschaffung), **Gilden** und **Berufe** als Grundlage zum Ausbauen
- **Aufgaben und Quests** mit Schritten (töten, bringen, besuchen, sprechen) und Belohnungen
- Karte, die durch eine **Fertigkeit** größer wird
- Ausrüstungsplätze, benutzbare Tränke und Nahrung; **Behälter** (Taschen, Rucksäcke, Trankgürtel) mit Gewichtsreduktion und **Tragkraft**
- **Ausführliche Beschreibungen:** Details mit beliebig tiefen Unterdetails, Gerüche, Geräusche und Gefühl (`riech`, `hoer`, `taste`)
- **Bibliothek allgemeiner Beschreibungen** (Bäume, Himmel, Wasser, Möbel …), die Räume, Geländearten und Oberwelt-Felder mit `nutzt = [...]` verwenden
- **NPCs in eigenen Dateien** mit Gesprächen: Standardfragen (Gruß, Beruf, Aufgabe, Neuigkeiten …) und eigene Themen
- **Rollen:** Spieler, Baumeister und Admin, mit eigenen Befehlen
- **Kampf in Echtzeit, ohne Runden:** Erholungszeit je nach Waffe, Ausdauer, Ausweichen, Haltungen, Flucht, Kampfzauber, aggressive und zaubernde Monster, Beute
- **Stufen und Erfahrung**, Punkte zum Verteilen, dazu steigende Fertigkeiten
- **Tod und Wiederbelebung:** Mitspieler können 5 Minuten lang helfen, sonst (oder mit `aufgeben`) erwacht man im Tempel
- **Befehle anpassen:** Wörter, Rollen, Hilfetexte und Abschalten über `daten/befehle/`, dazu eigene **Gesten** (`winke`, `verbeuge` …)
- **Vorlagen und Vererbung** (`erbt = "..."`, `vorlage = true`) für Gegenstände, Monster, NPCs, Zauber, Fertigkeiten und Beschreibungen
- **Fernwartung:** Server unter Linux, Welteditor am PC – Welt holen, bearbeiten und hochladen, verschlüsselt und mit Prüfung vor dem Übernehmen
- **Grafischer Welteditor** (`rmeditor`): Räume auf einer Karte anlegen, verschieben, verbinden und beschreiben; alle Inhalte (Gegenstände, Monster, NPCs mit Händler/Lehrer, Zauber, Quests, Rassen, Gilden, Berufe, Welteinstellungen) und Befehle/Gesten bearbeiten. Kommentare in den TOML-Dateien bleiben beim Speichern erhalten.

## Code-Überblick (`src/`)

| Datei        | Inhalt                                                       |
|--------------|--------------------------------------------------------------|
| `daten.rs`   | Datenformate, Laden und Prüfen der Weltdaten                 |
| `welt.rs`    | laufender Zustand: Spieler, Monster, Takt, Zurücksetzen      |
| `befehle.rs` | alle Spielerbefehle                                          |
| `karte.rs`   | Kartendarstellung für Städte und Dungeons                    |
| `oberwelt.rs`| Oberwelt: Kartenbild laden, Sicht, Felder als Räume, Karten  |
| `netz.rs`    | Telnet-Server und Login                                      |
| `konsole.rs` | Befehle im Konsolenfenster des Servers                       |
| `telnet.rs`  | Telnet-Aushandlung und MUD-Protokolle (GMCP, MSDP, MCCP2 …)  |
| `clientdaten.rs` | Spieldaten für Clients: Werte, Raum, Gespräche, MSSP     |
| `spieler.rs` | Speichern, Laden, Passwörter                                 |
| `text.rs`    | Farben, Zeilenumbruch, Tippfehler-Vorschläge                 |
| `update.rs`  | Update-Funktion über GitHub Releases                         |
| `beschreibung.rs` | Details, Sinne und Bibliothek                          |
| `gespraech.rs` | NPC-Gespräche                                              |
| `kampf.rs`   | Kampf, Stufen, Tod und Wiederbelebung                        |
| `wirkung.rs` | Wirkungen mit Dauer: Schilde, Stärkungen, Gift               |
| `handel.rs`  | Geld, Händler und Lehrmeister                                |
| `gemeinschaft.rs` | Rassen, Gilden und Berufe                               |
| `quests.rs`  | Aufgaben und Quests                                          |
| `behaelter.rs` | Taschen und Rucksäcke, Tragkraft                           |
| `fernwartung.rs` | Fernwartung: verschlüsselter Zugang für den Welteditor   |
| `fernwartung_protokoll.rs` | Nachrichten der Fernwartung (auch vom Editor benutzt) |
| `befehlsliste.rs` | alle eingebauten Befehle (auch vom Editor benutzt)      |
| `befehlskonfig.rs` | befehle.toml: Anpassungen und Gesten                   |

Der Welteditor (`editor/src/`):

| Datei     | Inhalt                                                                |
|-----------|-----------------------------------------------------------------------|
| `main.rs` | Programmstart, Suche nach dem Datenordner                             |
| `welt.rs` | Räume laden, prüfen und kommentarerhaltend speichern (`toml_edit`)    |
| `app.rs`  | Oberfläche: Karte, Raumliste, Eigenschaften, Rückgängig (`egui`)      |
| `inhalte.rs` | Bereich Inhalte: Formulare für alle Arten, direkt auf den TOML-Dateien |
| `schema.rs` | Welche Felder jede Art hat (daraus entstehen die Formulare)           |
| `befehle.rs` | Befehle und Gesten laden, prüfen und speichern                     |
| `bereiche.rs` | Oberfläche des Bereichs Befehle                                   |
| `fernwartung.rs` | Verbindung zum Server, Arbeitskopie holen und hochladen       |
| `fenster.rs` | Fenster „Server“ und „Updates“                                      |
