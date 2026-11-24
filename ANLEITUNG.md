# Anleitung für Weltenbauer

Mit RMengine baust du eine Welt, **ohne zu programmieren**. Die ganze
Welt steht in Textdateien im Ordner `daten/`. Du änderst eine Datei, tippst im
Spiel `neuladen`, und die Änderung ist da. Ein Neustart ist dafür nicht nötig.

---

## 1. Installieren und starten

### Windows

1. Lade auf der [Release-Seite](https://github.com/areadox/RMengine/releases/latest)
   die Datei `RMengine-Setup-….exe` herunter und führe sie aus. Administratorrechte
   sind nicht nötig. Installiert wird nach **Dokumente\RMengine**.
2. Im Startmenü unter **RMengine** findest du:
   - *RMengine starten*: startet den Server in einem Konsolenfenster (siehe
     „Die Server-Konsole“)
   - *Weltdaten bearbeiten*: öffnet den Ordner mit deiner Welt
   - *Weltdaten prüfen*: sucht Fehler in den Dateien
   - *Nach Updates suchen*: installiert eine neue Version
   - *Anleitung*: diese Datei

Beim ersten Start fragt die Windows-Firewall nach. Erlaube den Zugriff, damit
sich Spieler verbinden können.

Ohne Installation geht es auch: `RMengine-windows-x86_64-portable.zip` entpacken
und `rmengine.exe` starten.

### Linux

```bash
curl -fsSL https://github.com/areadox/RMengine/releases/latest/download/install.sh | bash
```

Das Programm landet in `~/.local/share/rmengine`, die Welt in
`~/.local/share/rmengine/daten`. Danach:

```bash
rmengine            # Server starten
rmengine --pruefen  # Weltdaten prüfen
```

### Verbinden

Verbinde dich mit einem MUD-Programm, zum Beispiel
[Mudlet](https://www.mudlet.org) (Server `localhost`, Port `4000`), oder im
Terminal mit:

```bash
telnet localhost 4000
```

**Wichtig:** Trag in `welt.toml` unter `admins` deinen eigenen Figurennamen ein
und lege diese Figur gleich an. Sie ist dann Admin und kann im Spiel weitere
Rollen vergeben (siehe „Rollen: Spieler, Baumeister, Admin“).

### Die Server-Konsole

Der Server läuft in einem Konsolenfenster (unter Windows das schwarze Fenster,
das „RMengine starten“ öffnet, unter Linux das Terminal). Dort siehst du, wer
sich an- und abmeldet, und du kannst Befehle eintippen:

| Befehl                 | Wirkung                                              |
|------------------------|------------------------------------------------------|
| `hilfe`                | alle Befehle der Konsole                             |
| `neuladen`             | Weltdaten neu einlesen; die Spieler bleiben im Spiel |
| `pruefen`              | Weltdaten prüfen, ohne sie zu übernehmen             |
| `wer`                  | wer gerade spielt, wo und mit welchem Programm       |
| `verkuende <text>`     | Nachricht an alle Spieler                            |
| `rolle <name> <rolle>` | Rolle vergeben (spieler, baumeister, admin), auch für Figuren, die gerade nicht im Spiel sind |
| `speichern`            | alle Spieler sofort speichern                        |
| `fernwartung`          | Fernwartung für den Welteditor: `fernwartung ein` schaltet sie sofort ein (beim ersten Mal mit neuem Zugangsschlüssel), `fernwartung neu` erzeugt einen neuen Schlüssel, `fernwartung aus` schaltet ab, ohne Zusatz zeigt es den Zustand |
| `ende`                 | alle speichern und den Server beenden                |

Hast du im Welteditor etwas geändert, tippst du hier einfach `neuladen`. Auch
Strg+C oder das Schließen des Fensters speichert vorher alle Spieler.
Läuft der Server als Dienst ohne Fenster (Linux, systemd), gibt es keine
Konsole. Dann benutzt du `neuladen` im Spiel oder die Fernwartung.

### Aktualisieren

Beim Start meldet RMengine, wenn es eine neue Version gibt. Engine und
Welteditor haben immer dieselbe Versionsnummer und werden **gemeinsam**
aktualisiert:

- **Im Welteditor:** oben rechts **⟳ Updates**. Dort wählst du aus, was
  aktualisiert wird:
  - der Welteditor selbst,
  - die Engine auf diesem PC (falls sie neben dem Editor liegt),
  - die Engine auf dem Server (wenn du über die Fernwartung arbeitest). Der
    Server speichert dabei alle Spieler und startet mit der neuen Version neu.

  Danach startet der Editor auf Knopfdruck neu.
- **Windows:** Startmenü → *RMengine* → *Nach Updates suchen*, oder den neuen
  Installer einfach über die alte Version installieren.
- **Linux:**

  ```bash
  rmengine --update
  ```

  Das aktualisiert Engine und Editor zusammen.

Dabei wird nur das Programm ausgetauscht. **Deine Welt im Ordner `daten` und die
Spielstände im Ordner `spieler` bleiben unverändert.**

### Für Entwickler

Mit installiertem Rust (<https://rustup.rs>) startest du den Server aus dem
Quellcode mit `cargo run --release` und prüfst die Daten mit
`cargo run --release -- --pruefen`.

---

## 2. Server unter Linux, Editor am PC (Fernwartung)

Ein häufiger Aufbau: Die Engine läuft dauerhaft auf einem Linux-Server, und du
baust die Welt bequem am Windows-PC. Der Welteditor holt dazu die Welt vom
Server, du bearbeitest sie und lädt die Änderungen wieder hoch.

### 1. Auf dem Server einrichten

```bash
curl -fsSL https://github.com/areadox/RMengine/releases/latest/download/install.sh | bash
```

```bash
rmengine --fernwartung
```

Läuft der Server schon in einem Fenster (zum Beispiel unter Windows), tippst du
stattdessen in der Server-Konsole `fernwartung ein`. Das wirkt sofort, ohne
Neustart.

Das zeigt drei Dinge an. Schreib sie dir auf:

- den **Port** (4001), den du in der Firewall freigeben musst, zusammen mit dem
  Spielport 4000,
- den **Zugangsschlüssel** (z.B. `WFRTF-GGZTZ-RK82T-UL2SY-UZD4W`). Er wird nur
  dieses eine Mal angezeigt,
- den **Fingerabdruck** des Servers.

Firewall freigeben, zum Beispiel mit `ufw`:

```bash
sudo ufw allow 4000/tcp
```

```bash
sudo ufw allow 4001/tcp
```

Danach den Server starten (oder neu starten). Im Protokoll steht dann
„Fernwartung aktiv auf Port 4001“.

### 2. Am PC verbinden

1. Den Welteditor öffnen. Auf der Startseite oder oben rechts
   **🌐 Server** anklicken.
2. Adresse des Servers (z.B. `meinserver.de` oder `203.0.113.5`) und den
   Zugangsschlüssel eintragen. Die Arbeitskopie landet standardmäßig unter
   *Dokumente\RMengine-Server\<servername>*.
3. **Verbinden und Welt holen.** Beim ersten Mal zeigt der Editor den
   Fingerabdruck des Servers. Vergleiche ihn mit dem aus Schritt 1, und nur
   wenn er übereinstimmt, klickst du auf *Stimmt überein – verbinden*.

### 3. Arbeiten

- Bearbeite die Welt wie gewohnt: Räume, Zauber, Befehle.
- **⬆ Hochladen** überträgt alle geänderten Dateien. Ungespeicherte Änderungen
  speichert der Editor vorher selbst. Der Server **prüft die Welt zuerst in
  einer Kopie**. Nur wenn sie fehlerfrei ist, übernimmt er sie und lädt sie (wenn
  angehakt) sofort im laufenden Spiel neu. Bei Fehlern bleibt auf dem Server
  alles, wie es war, und du siehst die Fehlerliste.
- Hat jemand auf dem Server dieselben Dateien inzwischen geändert, warnt der
  Editor, bevor er etwas überschreibt.
- **⬇ Neu vom Server holen** holt den aktuellen Stand. Der Editor warnt, wenn
  dabei lokale Änderungen verloren gingen.
- **Auf dem Server prüfen** und **Im Spiel neu laden** wirken direkt auf dem
  Server.

### Sicherheit

- Die Verbindung ist **verschlüsselt** (TLS). Der Editor prüft bei jeder
  Verbindung den Fingerabdruck. Meldet er einen *anderen* Fingerabdruck als
  beim ersten Mal, verbinde dich nicht. Ausnahme: Du hast auf dem Server das
  Zertifikat bewusst neu erzeugt.
- Ohne Zugangsschlüssel geht nichts, und falsche Versuche werden ausgebremst.
  Auf dem Server liegt nur ein Hash des Schlüssels.
- Über die Fernwartung sind **nur die Welt-Dateien** erreichbar. Spielstände
  und Programme sind ausgeschlossen.
- **Neuer Schlüssel:** in der Server-Konsole `fernwartung neu` oder noch einmal
  `rmengine --fernwartung` ausführen. Der alte gilt dann nicht mehr.
- **Abschalten:** in der Server-Konsole `fernwartung aus` (sofort) oder
  `rmengine --fernwartung-aus` (ab dem nächsten Start).
- **Zustand ansehen:** in der Server-Konsole `fernwartung`.
- Die Dateien `fernwartung.toml`, `fernwartung.crt` und `fernwartung.key`
  liegen neben dem Datenordner. `fernwartung.key` darf niemand sonst bekommen.

### Den Server dauerhaft laufen lassen (systemd)

Damit die Engine nach einem Neustart des Servers von selbst startet, legst du
einen Dienst an. Die Datei `/etc/systemd/system/rmengine.service` (als root;
`BENUTZER` durch deinen Benutzernamen ersetzen):

```ini
[Unit]
Description=RMengine MUD-Server
After=network-online.target

[Service]
User=BENUTZER
WorkingDirectory=/home/BENUTZER/.local/share/rmengine
ExecStart=/home/BENUTZER/.local/share/rmengine/rmengine --keine-updatepruefung
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable --now rmengine
```

Das Protokoll siehst du mit `journalctl -u rmengine -f`. Updates über den Editor
funktionieren auch hier: Die Engine tauscht sich aus und startet sich selbst neu.

---

## 3. Befehle für Spieler

Die wichtigsten Befehle zeigt `hilfe`. Zwei davon gehören jeder Figur:

### Spielstand (`spielstand` oder kurz `sp`)

```
Spielstand von Bert der Wanderer

  Lebenspunkte [####################]  100/100
  Mana         [##########----------]   25/50

Werte
  Stärke          11 (+1)
  ...
Ausrüstung
  Waffe         ein Kurzschwert
Können
  Fertigkeiten  3  (beste: Magie 4, Heilkunde 2, Schwertkampf 1)
  Zauber        3  (Kleine Heilung, Heimkehr, Feuerpfeil)
Unterwegs
  Aufenthalt    Der Dorfplatz
  Erkundet      12 von 25 Räumen (48%)
  Spielzeit     1 Std 23 Min, dabei seit 23.11.2026
```

Boni durch angelegte Ausrüstung erscheinen in Klammern hinter dem Wert.

### Spielerbuch (`spielerbuch`, `buch` oder `lies spielerbuch`)

Hier stellt jeder Spieler selbst ein, wie das Spiel für ihn aussieht. Geändert
wird mit `spielerbuch <einstellung> <wert>`, zum Beispiel
`spielerbuch karte aus`.

| Einstellung      | Werte                 | Wirkung                                      |
|------------------|-----------------------|----------------------------------------------|
| `farben`         | an / aus              | farbige Texte                                |
| `karte`          | an / aus              | Karte beim Umsehen                           |
| `kartengroesse`  | klein / normal / gross | Größe der Karte                             |
| `kartenlage`     | daneben / oben        | Raumtext neben der Karte oder darunter       |
| `kurz`           | an / aus              | beim Gehen keine Raumbeschreibung            |
| `breite`         | auto oder 40 bis 200  | Zeilenbreite; auto = so breit wie das Fenster |
| `prompt`         | an / aus              | Eingabezeile mit Lebenspunkten und Mana      |
| `ankuendigungen` | an / aus              | Meldungen, wenn Spieler kommen und gehen     |
| `npc`            | an / aus              | Sprüche von Bewohnern und Tieren             |
| `titel`          | Text oder `loeschen`  | z.B. „der Wanderer“, steht hinter dem Namen  |
| `beschreibung`   | Text oder `loeschen`  | was andere sehen, wenn sie dich ansehen      |
| `passwort`       | `<alt> <neu>`         | Passwort ändern                              |

Alle Einstellungen werden sofort gespeichert.

### Anzeigenamen

IDs dürfen keine Umlaute haben. Wie Werte und Ausrüstungsplätze im Spiel heißen,
legst du in `welt.toml` fest:

```toml
[namen]
staerke = "Stärke"
koerper = "Körper"
```

---

## 4. Der Aufbau des Datenordners

```
daten/
  welt.toml            Grundeinstellungen, Startwerte, Geländearten
  raeume/*.toml        Räume
  gegenstaende/*.toml  Gegenstände
  monster/*.toml       Monster und Tiere
  npcs/*.toml          NPCs mit Gesprächen, ein NPC pro Datei
  beschreibungen/      allgemeine Beschreibungen (Bäume, Himmel, Möbel ...)
  befehle/             Befehle anpassen und eigene Gesten
  vorlagen/            Vorlagen, von denen andere Einträge erben
  fertigkeiten/*.toml  Fertigkeiten
  zauber/*.toml        Zaubersprüche
  oberwelt/            die Landschaft (siehe Abschnitt 7)
```

Die Ordner sind nur eine Ordnungshilfe. Der Treiber liest **alle** `.toml`-Dateien
im Datenordner und in allen Unterordnern, und jede Datei darf jede Art von
Eintrag enthalten. Du kannst also auch ein ganzes Gebiet mit Räumen, Monstern und
Gegenständen in eine Datei `gebiete/sumpf.toml` schreiben.

### Grundregeln für TOML-Dateien

```toml
# Ein Kommentar (wird ignoriert)
text     = "In Anführungszeichen"
zahl     = 42
schalter = true                       # oder false
liste    = ["eins", "zwei"]
tabelle  = { nord = "raum_a", ost = "raum_b" }
langtext = """
Ein Text über
mehrere Zeilen."""
```

`[[raum]]` beginnt einen **neuen** Raum, `[[gegenstand]]` einen neuen Gegenstand
und so weiter.

### IDs

Jeder Eintrag hat eine `id`. Über die ID verweisen Einträge aufeinander. IDs
bestehen nur aus Kleinbuchstaben ohne Umlaute, Ziffern und `_`, zum Beispiel
`alte_eiche` oder `heiltrank2`.

### Farben

In allen Texten kannst du Farben setzen: `"Ein {rot}gefährlicher{/} Ort"`.
`{/}` schaltet die Farbe wieder aus.
Verfügbar sind: `rot gruen gelb blau magenta cyan weiss grau schwarz hellrot
hellgruen hellgelb hellblau hellmagenta hellcyan hellweiss fett`.

---

### Vorlagen und Vererbung

Viele Einträge sehen sich ähnlich: Alle Waffen liegen am Platz `waffe`, alle
Tränke sind vom Typ `trank`, alle Dorfbewohner sind friedlich. Statt das überall
zu wiederholen, legst du eine **Vorlage** an und lässt andere Einträge davon
**erben**:

```toml
# daten/vorlagen/gegenstaende.toml
[[gegenstand]]
id         = "vorlage_waffe"
vorlage    = true            # nur zum Erben da, erscheint nicht in der Welt
name       = "eine Waffe"
typ        = "waffe"
platz      = "waffe"
fertigkeit = "schwertkampf"
erholung   = 2.0

# daten/gegenstaende/ausruestung.toml
[[gegenstand]]
id       = "dolch"
erbt     = "vorlage_waffe"   # übernimmt typ, platz, fertigkeit, erholung ...
name     = "ein Dolch"
erholung = 1.3               # ... und überschreibt nur, was anders ist
eigenschaften = { schaden = 3 }
```

- Erben können **Gegenstände, Monster, NPCs, Zauber, Fertigkeiten und
  Beschreibungen**. NPCs dürfen auch von Monster-Vorlagen erben.
- **Tabellen** wie `eigenschaften` oder `[npc.gespraech]` werden zusammengeführt:
  Der Wirt erbt zum Beispiel die gemeinsamen Antworten aller Dorfbewohner und
  ergänzt seine eigenen. **Listen** wie `schluesselwoerter` werden ersetzt.
- Eine Vorlage kann selbst wieder erben, und sie kann in einer anderen Datei
  stehen als die Einträge, die von ihr erben.
- Räume erben nicht. Jeder Raum ist einzigartig.
- Die Prüfung meldet unbekannte Vorlagen (mit Vorschlag) und Vererbung im Kreis.

In der Beispielwelt stehen die Vorlagen in `daten/vorlagen/`: Waffen,
Rüstungen, Tränke, Essen, Raubtiere, Dorfbewohner, Heil- und Kampfzauber.

---

## 5. Räume und die Karte

```toml
[[raum]]
id       = "dorfplatz"        # Pflicht
name     = "Der Dorfplatz"    # Pflicht
x = 0                         # Pflicht: Position auf der Karte
y = 0
z = 0                         # Stockwerk (optional, Standard 0)
gebiet   = "dorf"             # optional: Monster wandern nicht aus ihrem Gebiet heraus
gelaende = "platz"            # Aussehen auf der Karte (aus welt.toml)
symbol   = "Y"                # optional: eigenes Kartenzeichen nur für diesen Raum
farbe    = "hellgruen"        # optional: eigene Farbe
beschreibung = """
Text, den der Spieler beim Umsehen liest. Zeilenumbrüche werden automatisch
gesetzt. Eine Leerzeile beginnt einen neuen Absatz."""
ausgaenge    = { nord = "hauptstrasse", ost = "taverne" }
gegenstaende = ["apfel", "apfel"]   # liegen hier am Boden
monster      = ["wirt"]             # leben hier

[raum.details]                # Dinge, die man mit "schau brunnen" ansehen kann
brunnen = "Ein alter Brunnen aus Stein."
```

**Koordinaten:** `x` wird nach **Osten** größer, `y` nach **Norden**, `z` nach
**oben**. Ein Raum nördlich von (0,0) liegt also auf (0,1). Die Karte ergibt sich
automatisch aus den Koordinaten.

**Richtungen:** `nord nordost ost suedost sued suedwest west nordwest hoch runter rein raus`

Ein Ausgang darf auch direkt auf ein Feld der Oberwelt zeigen, zum Beispiel
`raus = "ow:36:11"`.

**Rückwege:** Wenn A nach Norden zu B führt, bekommt B automatisch einen Ausgang
nach Süden zu A (abschaltbar mit `ausgaenge_beidseitig = false` in `welt.toml`).

Passt ein Ausgang nicht zu den Koordinaten (Ausgang `ost`, aber der Zielraum liegt
nicht genau östlich), bekommst du eine Warnung. Der Ausgang funktioniert trotzdem,
nur die Linie fehlt auf der Karte.

### Die Stadtkarte

Jeder Raum ist ein Kästchen. In der Mitte steht das `symbol` des Raums, `X` bist
du, `@` sind andere Spieler:

```
+-------------+
|******|******|
|*****( )*(M)-|
|******|*/****|
|*[S]-(X)-[K]*|
|******|******|
|*****[T]*****|
|*************|
+-------------+
```

In `welt.toml` stellst du ein:

```toml
karten_radius      = 1          # 1 = 3x3 Räume; der Befehl "karte" zeigt mehr
karte_daneben      = true       # Raumbeschreibung rechts neben der Karte (false = darunter)
stadtkarte_stil    = "kasten"   # oder "linien": ein Zeichen pro Raum, ohne Rahmen
karten_hintergrund = "*"        # Füllzeichen, z.B. "*" oder " "
```

**Größere Karte durch eine Fertigkeit:** Mit `karten_fertigkeit` wird die Karte
größer, je besser man eine Fertigkeit beherrscht:

```toml
karten_fertigkeit        = "orientierung"  # "" = aus
karten_fertigkeit_stufen = 10              # alle 10 Stufen ein Feld mehr
karten_fertigkeit_max    = 3               # höchstens 3 Felder mehr
```

Das gilt für die Stadtkarte und die Oberwelt (dort sieht man auch weiter).
Die Fertigkeit lernt man zum Beispiel bei einem Lehrmeister.

Mit `karte_daneben = true` steht die Raumbeschreibung rechts neben der Karte
und läuft darunter in voller Breite weiter:

```
[Dorf]            Der Dorfplatz
+-------------+   Du stehst auf dem kopfsteingepflasterten Dorfplatz von
|******|******|   Eichenhain. In der Mitte plätschert ein alter Brunnen. Im
|*[L]-( )*(M)-|   Osten hängt das Schild einer Taverne, im Westen hörst du
|******|*/****|   Hammerschläge. Eine Straße führt nach Norden, im Süden
|*[S]-(X)-[K]*|   erhebt sich ein kleiner Tempel.
|*o**/*|******|   Ausgänge: Norden, Nordosten, Osten, Süden, Südwesten und
|o(G)&[T]*****|   Westen
|~~~~~~~~~****|
+-------------+
```

Ist das Fenster des Spielers zu schmal (weniger als 25 Zeichen neben der
Karte), steht die Karte automatisch darüber. Jeder Spieler kann es im
Spielerbuch mit `spielerbuch kartenlage oben` oder `daneben` selbst wählen.

### Geländearten (in welt.toml)

```toml
[gelaende.haus]
name     = "Gebäude"   # erscheint in der Legende
klammern = "[]"        # Kästchen auf der Karte: "()" draußen, "[]" Gebäude, "{}" Wald ...
symbol   = "#"         # nur für den Stil "linien"
farbe    = "hellweiss"
```

Den Buchstaben im Kästchen bestimmt der Raum selbst, zum Beispiel
`symbol = "K"` für die Kneipe. Mit `klammern = "<>"` kann ein einzelner Raum auch
eigene Klammern bekommen.

### Besondere Orte, Häuser und betretbare Gebiete

Ein **Buchstabe** im Kästchen kennzeichnet einen besonderen Ort, etwa einen Laden,
eine Stelle mit einem NPC oder ein Gebäude, das man betreten kann:

```toml
[[raum]]
id     = "gartentor"
name   = "Vor dem Kräutergarten"
symbol = "G"
x = -1
y = -1
ausgaenge = { nordost = "dorfplatz", rein = "kraeutergarten" }
```

Mit `rein` geht man hinein. Der Rückweg `raus` entsteht im Zielraum automatisch.
Das Innere bekommt eine **eigene Karte**, damit es sich nicht mit dem Stadtplan
überschneidet:

```toml
[[raum]]
id    = "kraeutergarten"
name  = "Im Kräutergarten"
karte = "kraeutergarten"    # alle Räume mit gleicher karte bilden einen Plan
x = 0
y = 0
```

Räume ohne `karte` liegen alle zusammen auf dem gemeinsamen Plan.

### Bäume, Wasser und andere Verzierungen

Rund um die Kästchen kannst du Verzierungen setzen:

```
|*o**/*|******|
|o(G)&[T]*****|
|~~~~~~~~~****|
```

```toml
# bei einer Geländeart (gilt für alle Räume dieser Art)
[gelaende.garten]
deko        = "o&"     # diese Zeichen werden verstreut
deko_dichte = 45       # Prozent der freien Stellen

# oder bei einem einzelnen Raum
deko_richtung = { sued = "~", suedost = "~" }   # eine ganze Seite: ein Bach
```

Die Farben der Verzierungen legst du in `welt.toml` fest:

```toml
[deko]
"o" = "gruen"
"&" = "hellgruen"
"~" = "hellblau"
```

Verzierungen kommen nur an Stellen, an denen kein Weg verläuft, und sie stehen
immer an derselben Stelle. Die Karte sieht deshalb bei jedem Umsehen gleich aus.

### Der grafische Welteditor

Statt Räume von Hand in die TOML-Dateien zu schreiben, kannst du sie auch mit dem
**Welteditor** (`rmeditor`) auf einer Karte bauen. Er liest und schreibt genau
dieselben Dateien wie der Server.

- **Windows:** Startmenü → *RMengine* → *Welteditor*
- **Linux:** im Anwendungsmenü *RMengine Welteditor* oder im Terminal `rmeditor`
- **Aus dem Quellcode:** `cargo run --release -p rmeditor` (öffnet `./daten`)

Der Editor braucht eine grafische Oberfläche. Er wird mit `rmengine --update`
automatisch mit aktualisiert.

- **Links:** Karte und Ebene (`z`) wählen, Räume suchen und anspringen.
- **Mitte:** die Karte. Ein Klick wählt einen Raum aus. Ziehen verschiebt ihn.
  Ein Doppelklick auf ein leeres Feld legt einen neuen Raum an. Liegt das Feld
  neben dem ausgewählten Raum, werden beide gleich verbunden.
- **Verbinden:** Wähle links das Werkzeug *Verbinden* oder halte Shift. Dann ziehst
  du von einem Raum zu einem anderen. Lässt du auf einem leeren Feld los, entsteht
  dort ein neuer, verbundener Raum.
- **Rechts:** alle Eigenschaften des Raums: Name, Beschreibung, Gelände, Ausgänge,
  Sinne, Gegenstände, Monster, NPCs und `nutzt`. Grau angezeigte Ausgänge sind die
  Rückwege, die der Server automatisch anlegt.
- **Rechtsklick:** Startraum (★) oder Wiederbelebungsort (✚) festlegen,
  Verbindung trennen, Raum löschen.
- **Prüfung:** Oben steht, wie viele Fehler und Hinweise es gibt. Ein Klick darauf
  zeigt die Liste, und ein Klick auf einen Eintrag springt zum Raum. *Mit Engine
  prüfen* startet `rmengine --pruefen` für den gespeicherten Stand.
- **Oberwelt:** Unter *Karte* gibt es den Eintrag *🌍 Oberwelt*. Er zeigt
  `karte.txt` mit allen Feldern, Eingängen und Orten. Die Koordinate unter der Maus
  steht im Lineal und in der Statuszeile, genau wie in `oberwelt.toml` gezählt
  (`x` = Spalte, `y` = Zeile, ab 1). Ein Klick auf ein Feld zeigt rechts die Details.
  Dort kannst du `ow:x:y` oder `x = … / y = …` kopieren oder das Feld direkt als
  Ausgang `raus` des zuletzt gewählten Raums eintragen.
- **Strg+S** speichert, **Strg+Z** macht rückgängig, **Strg+Y** stellt wieder her.

Oben schaltest du zwischen drei Bereichen um:

- **🗺 Räume:** die Karte, wie oben beschrieben.
- **📚 Inhalte:** alles außer Räumen und Befehlen. Oben links wählst du die Art:
  Gegenstände, Monster, NPCs (mit Gespräch, Händler, Lehrmeister, Gilde und
  Beruf), Fertigkeiten, Zauber und Techniken, Aufgaben und Quests, Rassen,
  Gilden, Berufe, allgemeine Beschreibungen, dazu die Welteinstellungen, der
  Start neuer Figuren und die Kampfregeln.
  - Darunter stehen die Einträge, nach Dateien sortiert. **+ Neu**, **Kopieren**
    und **Löschen** legen Einträge an oder entfernen sie. Ein neuer NPC bekommt
    automatisch seine eigene Datei.
  - In der Mitte steht das Formular. Felder mit * braucht die Engine unbedingt.
  - Verweise wählst du aus Listen aus: Gegenstände, Monster, Räume, Fertigkeiten
    und so weiter. Ein Verweis ins Leere wird rot angezeigt.
  - Erbt ein Eintrag von einer Vorlage, stehen die geerbten Werte grau im Feld.
    Leere Felder werden nicht gespeichert, dann gilt der Standardwert oder die
    Vorlage.
  - Benennst du eine ID um, passe die Verweise darauf selbst an. Die Prüfung
    oben zeigt dir, wo etwas fehlt.
- **⌨ Befehle:** alle eingebauten Befehle (gelb = angepasst, durchgestrichen =
  abgeschaltet) und alle Gesten. Bei Befehlen änderst du Wörter, Rolle,
  Hilfetext und Aufruf; *Auf Standard zurücksetzen* nimmt alles zurück. Bei
  Gesten siehst du unten eine Vorschau, wie die Texte im Spiel aussehen.
  Gespeichert wird in `daten/befehle/`, und zwar nur, was vom Standard abweicht.

Die Prüfung oben zählt Fehler in allen drei Bereichen, zum Beispiel ein Wort,
das zwei Befehlen gehört, einen Zauber mit unbekannter Fertigkeit oder eine
Aufgabe ohne Geber.

Beim Speichern ändert der Editor nur, was du wirklich geändert hast. Kommentare,
Leerzeilen, `details` und alles andere, was der Editor nicht kennt, bleiben in den
Dateien erhalten. Benennst du einen Raum um, passt er alle Verweise an: Ausgänge,
`startraum`, `wiederbelebung_raum`, Oberwelt-Eingänge und Teleport-Zauber.

---

## 6. Ausführlich beschreiben

### Details mit Unterdetails und allen Sinnen

Ein Detail kann ein einfacher Text sein:

```toml
[raum.details]
schild = "Auf dem Schild steht: 'Zum Goldenen Eber'."
```

Oder sehr ausführlich, mit Suchwörtern, Sinnen und Unterdetails in beliebiger
Tiefe:

```toml
[raum.details.brunnen]
text      = """
Der Brunnen von Eichenhain ist älter als jedes Haus im Dorf. In den
Rand hat jemand etwas eingeritzt."""
woerter   = ["dorfbrunnen", "schacht"]     # weitere Suchwörter
geruch    = "..."                          # riech brunnen
geraeusch = "Tief unten plätschert Wasser." # hoer brunnen
gefuehl   = "Die Steine sind kühl."         # taste brunnen

[raum.details.brunnen.details.rand]
text    = "In den Stein ist geritzt: 'Erbaut im Jahre 312'."
woerter = ["inschrift"]
gefuehl = "Mit dem Finger kannst du die Buchstaben nachfahren."
```

Spieler können dann `schau brunnen`, `schau inschrift`, `taste rand` oder
`schau rand des brunnens` eingeben.

**Räume, Geländearten, Gegenstände, Monster und NPCs** haben alle
`geruch`, `geraeusch` und `gefuehl`. Ohne Ziel (`riech`, `hoer`, `taste`)
beschreiben sie den ganzen Raum. Hat ein Raum keinen eigenen Text, gilt der
seiner Geländeart. Auch Gegenstände und Monster können `details` haben, zum
Beispiel den Griff eines Schwerts oder die Zähne eines Wolfs.

### Die Bibliothek: allgemeine Beschreibungen

In `daten/beschreibungen/allgemein.toml` stehen Dinge, die überall gleich
aussehen: Himmel, Wolken, Wind, Erde, Steine, Gras, Blumen, Moos, Bäume (Eiche,
Buche, Birke, Tanne, Weide, Linde …) samt Rinde, Ästen, Laub und Wurzeln,
Wasser, Bach, See, Meer, Vögel, Insekten, Wege, Pflaster, Mauern, Wände, Türen,
Fenster, Feuer, Kerzen, Möbel und vieles mehr.

Um sie zu benutzen, schreibst du nur `nutzt`:

```toml
[[raum]]
id    = "lichtung"
nutzt = ["gras", "blumen", "eiche", "himmel"]

[gelaende.wald]                       # gilt für ALLE Waldräume
nutzt = ["baum", "eiche", "moos", "farn", "laub"]

[feld."*"]                            # gilt für alle Waldfelder der Oberwelt
nutzt = ["baum", "eiche", "moos"]
```

Ein Bibliothekseintrag kann selbst andere nutzen (die Eiche nutzt „baum“, der
wiederum Rinde, Äste, Laub und Wurzeln). Ein Spieler an der Lichtung kann also
auch `taste rinde` oder `schau wurzeln` eingeben. Auf der Oberwelt sieht man
zusätzlich, was auf den Nachbarfeldern liegt.

Eigene Einträge legst du genauso an, gern in weiteren Dateien in
`daten/beschreibungen/`:

```toml
[[beschreibung]]
id      = "eiche"
woerter = ["eichen", "eichbaum"]
text    = "Eine mächtige Eiche ..."
geraeusch = "Die Eiche knarrt leise im Wind."
nutzt   = ["baum"]

[beschreibung.details]
eicheln = "Glatte, braune Eicheln ..."
```

**Vorrang:** Eigene Details eines Raums gehen immer vor. Ein Raum mit einem
eigenen Detail `brunnen` zeigt seinen Text und nicht den allgemeinen Brunnen
der Bibliothek.

---

## 7. Die Oberwelt (Landschaft)

Für Städte, Häuser und Höhlen baust du einzelne Räume wie oben. Für das **weite
Land dazwischen** zeichnest du stattdessen eine Landkarte. Jedes Zeichen darin ist
ein Feld, auf dem man laufen kann:

```
daten/oberwelt/
  karte.txt        die gezeichnete Landschaft
  oberwelt.toml    was die Zeichen bedeuten, Eingänge, besondere Orte
```

`karte.txt` ist ein ASCII-Bild der Landschaft, und so erscheint sie auch im Spiel
(Norden ist oben):

```
~~~~AAAA/^\/^.....*******....~~~
~~~AAA/^\=\/......***=***.....~~
~~~~~/\..====.......=........~~
~~~~~...........=======...%%%.~~
```

Ein Leerzeichen bedeutet: Hier ist nichts. Berge kannst du als kleine Bilder
zeichnen (`/^\`, `/\`): Du erklärst einfach `/`, `^` und `\` alle als
„Gebirge“.

Im Spiel sieht man einen Ausschnitt im Rahmen. `X` bist du, Felder außer Sicht
bleiben leer:

```
+-----------------+
|        ~~~~~~~~~|
|    ^^^^/^\/^....|
|  ^/^\/+\/.....**|
|^\/^\/=\X.....** |
|/^\..=..........*|
+-----------------+
```

```toml
[oberwelt]
anzeige_breite = 8          # Felder links und rechts (8 und 4 ergeben 17x9)
anzeige_hoehe  = 4          # Felder oben und unten
stil           = "kompakt"  # oder "weit": Leerzeichen zwischen den Feldern, ohne Rahmen
```

### Zeichen erklären

```toml
[feld.W]
name   = "Dunkelwald"        # Titel des Feldes und Name in der Legende
fern   = "dichten Wald"      # "Im Norden siehst du dichten Wald."
symbol = "♣"                 # Anzeige im Spiel (ohne Angabe: das Zeichen selbst)
farbe  = "gruen"
begehbar = true              # false: Meer, Felswand ...
sicht  = -1                  # wer hier steht, sieht 1 Feld weniger weit
sichtbehinderung = 2         # jedes Waldfeld dazwischen kostet 2 Felder Sicht
blockiert_sicht = false      # true: dahinter sieht man nichts (Berge)
hoehe  = 0                   # siehe unten
beschreibungen = ["Alte Eichen ...", "Farne wachsen ..."]  # Abwechslung
```

Zeichen wie `~`, `.` oder `%` müssen in Anführungszeichen stehen: `[feld."~"]`.
Den Rückstrich schreibt man doppelt: `[feld."\\"]`.

### Sicht

Die Karte im Spiel zeigt nur, was die Figur sehen kann:

- **Grundsichtweite:** `sicht` unter `[oberwelt]`. Das Feld, auf dem man steht,
  verändert sie: Berge und Hügel erhöhen sie, Wald verringert sie.
- **Wald und Nebel** (`sichtbehinderung`) verkürzen den Blick durch sie hindurch.
  Auf einer Straße durch den Wald sieht man deshalb entlang der Straße weit, zur
  Seite aber kaum.
- **Berge** (`blockiert_sicht = true`) verdecken alles dahinter. Wer selbst auf
  einem Berg steht, sieht über Berge gleicher oder geringerer `hoehe` hinweg.
  Höhere Gipfel verdecken auch dann noch die Sicht.

### Eingänge zu Städten und Dungeons

```toml
[[eingang]]
x      = 36              # Spalte in karte.txt (ab 1)
y      = 11              # Zeile in karte.txt (ab 1)
name   = "Eichenhain"
raum   = "osttor"        # dieser Raum wird mit "rein" betreten
symbol = "O"
farbe  = "hellgelb"
beschreibung = "Vor dir liegt das Osttor von Eichenhain."
```

Im Raum `osttor` entsteht automatisch der Ausgang `raus`. Er führt zurück auf
dieses Feld. Eingänge sieht man auf der Karte schon von Weitem, und die
Beschreibung nennt sie: „In der Ferne im Osten liegt Eichenhain.“

### Besondere Orte

```toml
[[ort]]
x = 15
y = 4
name   = "Der Steinkreis"
symbol = "*"             # optional: auf der Karte zu sehen und von Weitem erkennbar
beschreibung = "Zwölf uralte Steine ..."
gegenstaende = ["manatrank"]
monster      = ["wolf"]

[ort.details]
steine = "In jeden Stein ist ein Sternbild gemeißelt."
```

### Koordinaten finden

- Texteditoren zeigen Zeile und Spalte meist unten in der Statusleiste an.
- Im Spiel nennt der Baumeister-Befehl `wo` die Koordinaten des Feldes.
- Jedes Feld hat die Raum-ID `ow:x:y`. Damit kannst du springen
  (`springe ow:15:4`) oder einen Zauber dorthin teleportieren lassen.

### Weltkarte

`weltkarte` zeigt die ganze Oberwelt. Spieler brauchen dafür einen Gegenstand mit
der Eigenschaft `landkarte = 1`, Baumeister nicht.

---

## 8. Gegenstände

```toml
[[gegenstand]]
id        = "heiltrank"
name      = "ein roter Heiltrank"         # so steht er im Satz
akkusativ = "einen roten Heiltrank"       # für "Du nimmst ..." (optional)
schluesselwoerter = ["trank", "heiltrank"] # damit findet ihn "nimm trank"
typ       = "trank"
platz     = ""             # wo man ihn anlegt (leer = nicht anlegbar)
gewicht   = 1
wert      = 25
tragbar   = true           # false = kann nicht aufgehoben werden
bodentext = "Eine Flasche liegt im Staub."   # optional: Text am Boden
beschreibung = "Eine kleine Flasche mit roter Flüssigkeit."
eigenschaften = { heilung = 40 }
benutzen_text = "Du trinkst den Trank."
```

- **typ:** `waffe ruestung trank nahrung schluessel werkzeug schmuck behaelter sonstiges`
- **platz:** `waffe schild kopf koerper haende beine fuesse hals ring umhang licht`
- **eigenschaften:** Hier darfst du beliebige Zahlen eintragen. Schon jetzt
  wirken `heilung` und `mana`: Gegenstände mit diesen Eigenschaften kann man
  benutzen, und sie werden dabei verbraucht. Werte wie `schaden` oder `ruestung`
  wertet später das Kampfsystem aus.

---

## 9. Monster

```toml
[[monster]]
id        = "wolf"
name      = "ein grauer Wolf"
akkusativ = "einen grauen Wolf"
schluesselwoerter = ["wolf"]
raumtext  = "Ein grauer Wolf schleicht knurrend umher."   # optional
beschreibung = "Ein magerer Wolf mit hungrigen Augen."
stufe        = 3
lebenspunkte = 30
mana         = 0
werte        = { staerke = 12, geschick = 14 }
fertigkeiten = { schwertkampf = 5 }
zauber       = ["feuerpfeil"]
aggressiv    = true          # für das spätere Kampfsystem
wandert      = true          # läuft herum
wander_chance = 8            # Prozent pro Takt
sprueche = ["*heult.", "Grrr!"]   # * am Anfang = Handlung statt Rede
sprech_chance = 5
beute = [ { gegenstand = "wolfspelz", chance = 75 } ]
```

Monster werden in den Räumen erschaffen, die sie unter `monster = [...]` nennen.
Alle `zuruecksetzen_minuten` werden fehlende Monster und Gegenstände wieder
aufgefüllt.

---

## 10. NPCs und Gespräche

Jeder NPC hat seine **eigene Datei** in `daten/npcs/`, zum Beispiel
`daten/npcs/wirt.toml`, mit genau einem `[npc]`. Er hat alle Felder eines
Monsters, dazu Gespräche:

```toml
[npc]
id        = "wirt"
name      = "der Wirt"
akkusativ = "den Wirt"
schluesselwoerter = ["wirt", "bodo"]
beschreibung = "Bodo, der Wirt, ist ein freundlicher, rundlicher Mann ..."

[npc.gespraech]
begruessung = "Willkommen im Goldenen Eber, {spieler}!"
beruf       = "Ich bin der Wirt hier, seit zwanzig Jahren schon."
aufgabe     = "Im Keller treiben sich Ratten herum ..."
name        = "Bodo heiße ich."
neuigkeiten = ["Man munkelt ...", "Die Krämerin hat neue Seile."]   # Liste = zufällig
ort         = "Du bist im Goldenen Eber."
abschied    = "Komm bald wieder, {spieler}!"
unbekannt   = ["Davon weiß ich nichts.", "*zuckt mit den Schultern."]

[[npc.thema]]
woerter = ["ratten", "keller"]
antwort = "Die Viecher fressen mir den Käse weg!"
```

In einen Raum setzt du ihn mit `npcs = ["wirt"]`.

**Die Standardfragen erkennt das Spiel selbst**, egal wie der Spieler sie formuliert:

| Feld          | wird ausgelöst durch zum Beispiel                              |
|---------------|-----------------------------------------------------------------|
| `begruessung` | hallo, hi, moin, servus, guten Tag                              |
| `beruf`       | was ist dein Beruf, was machst du, wer bist du                  |
| `aufgabe`     | hast du eine Aufgabe für mich, Auftrag, kann ich helfen, Arbeit für mich |
| `name`        | wie heißt du, dein Name                                         |
| `neuigkeiten` | was gibt es Neues, Gerüchte, Klatsch                            |
| `ort`         | wo bin ich, was ist das hier                                    |
| `abschied`    | tschüss, auf Wiedersehen, bis bald                              |

Eigene Themen (`[[npc.thema]]`) haben Vorrang. In Antworten ersetzt das Spiel
`{spieler}` durch den Namen des Fragenden und `{name}` durch den des NPCs. Eine
Antwort, die mit `*` beginnt, ist eine Handlung („*nickt.“).

**So reden Spieler mit NPCs:**

- `sage hallo`: Ist nur ein NPC im Raum, antwortet er, wenn er etwas dazu weiß.
- `sage wirt, wie spät ist es?`: Wer mit Namen angesprochen wird, antwortet
  immer, notfalls mit `unbekannt`.
- `frage wirt nach ratten`
- `rede mit wirt`: Begrüßung und eine Liste der Themen, nach denen man fragen kann.

---

## 11. Kampf, Stufen und Tod

Gekämpft wird **in Echtzeit, ohne Runden**. Jeder Angriff hat eine
**Erholungszeit**: Mit einem Dolch schlägt man alle 1,3 Sekunden zu, mit einer
schweren Axt nur alle 4 Sekunden, dafür viel härter. Monster handeln nach ihrem
eigenen Takt. Bewegen, Tränke trinken, Reden und Zaubern geht jederzeit. Nur
aus dem Raum gehen kann man nicht einfach, dafür muss man **fliehen**.
Gegen andere Spieler wird nicht gekämpft.

### Für Spieler

| Befehl                  | Wirkung                                                         |
|-------------------------|------------------------------------------------------------------|
| `toete <monster>`       | angreifen; danach geht es automatisch weiter (abschaltbar im Spielerbuch) |
| `schaetze <monster>`    | Wie gefährlich ist es im Vergleich zu dir?                       |
| `ausweichen`            | 2,5 Sekunden lang treffen Angriffe viel seltener (kostet Ausdauer) |
| `haltung <art>`         | `angriffslustig` (mehr Schaden, mehr einstecken), `ausgewogen`, `defensiv` |
| `flieh`                 | in eine zufällige Richtung fliehen; klappt nicht immer           |
| `stopp`                 | nicht mehr angreifen                                             |
| `steigere <wert>`       | Punkte aus dem Stufenaufstieg verteilen, z.B. `steigere staerke` |
| `wiederbeleben <name>`  | einen toten Mitspieler zurückholen                               |
| `aufgeben`              | als Toter sofort im Tempel erwachen                              |

Der Prompt zeigt Lebenspunkte (LP), Mana (MP), Ausdauer (AP) und den Zustand
des Gegners: `LP 80/100 MP 50/50 AP 85/100 [Eine fette Ratte: schwer verletzt]`.

**So wird gerechnet:**
- **Treffer:** Geschick, die Fertigkeit der Waffe und der Stufenunterschied zählen.
- **Schaden:** Waffenschaden plus Stärke und Fertigkeit, minus Rüstung des Gegners.
- **Erholungszeit:** Hohes Geschick verkürzt sie.
- **Ausdauer:** Jeder Angriff kostet etwas. Wer erschöpft ist, trifft schwächer und braucht länger.
- **Kampfzauber:** Die Wirkung `schaden` trifft Monster. Intelligenz verstärkt sie.
- **Übung:** Durch Benutzen steigt die Fertigkeit der Waffe.

### Stufen und Erfahrung

Für besiegte Monster gibt es Erfahrungspunkte, und zwar für jeden, der
mitgekämpft hat. Mit jeder neuen Stufe bekommt man mehr Lebenspunkte, Mana und
Ausdauer, wird ganz geheilt und erhält Punkte, die man mit `steigere` auf die
Werte verteilt. Die Fertigkeiten steigen zusätzlich durch Benutzen.

### Tod und Wiederbelebung

Wer stirbt, bleibt leblos liegen. Alle Spieler erfahren, wo jemand Hilfe
braucht. Innerhalb von 5 Minuten kann ein anderer Spieler am selben Ort
`wiederbeleben <name>` eingeben, dann geht es dort weiter. Sonst, oder sofort
mit `aufgeben`, erwacht man im Tempel. Es geht nichts verloren. Solange man
tot ist, kann man nur reden, schauen und aufgeben.

### Einstellungen in welt.toml

```toml
[kampf]
auto_angriff         = true
waffenlos_schaden    = 2
waffenlos_erholung   = 2.5      # Sekunden
zauber_erholung      = 2.0
ausdauer             = 100
ausdauer_pro_angriff = 5
ausdauer_erholung    = 3        # pro Takt
ep_basis             = 100      # Erfahrung für Stufe 2: 100, dann 283, 520 ...
ep_faktor            = 1.5
lp_pro_stufe         = 10
mp_pro_stufe         = 5
ausdauer_pro_stufe   = 5
punkte_pro_stufe     = 2
wiederbelebung_raum  = "tempel"
tod_wartezeit        = 300      # Sekunden, in denen andere helfen können
wiederbelebung_lp_prozent = 50
tod_ep_verlust_prozent    = 0   # später z.B. 10
tod_gegenstaende     = "behalten"   # oder "fallen_lassen"
monster_erholung_prozent  = 5
```

### Waffen und Rüstungen

```toml
[[gegenstand]]
id    = "kurzschwert"
platz = "waffe"
eigenschaften = { schaden = 6, staerke = 1 }   # staerke = Bonus, solange angelegt
erholung   = 2.0             # Sekunden zwischen zwei Angriffen
fertigkeit = "schwertkampf"  # wird benutzt und geübt

[[gegenstand]]
id    = "holzschild"
platz = "schild"
eigenschaften = { ruestung = 3 }   # zieht 3 von jedem Treffer ab
```

### Monster im Kampf

```toml
aggressiv  = true                   # greift von selbst an
schaden    = 8                      # ohne Angabe: 2 + 2 * stufe
ruestung   = 1
erholung   = 2.8                    # Sekunden zwischen zwei Angriffen
angriffe   = ["beißt", "springt"]  # "Der Wolf beißt dich."
erfahrung  = 120                    # ohne Angabe: aus Stufe und Lebenspunkten
todestext  = "*jault auf und bricht zusammen."
zauber     = ["feuerpfeil"]         # Kampfzauber, wenn das Mana reicht
angreifbar = false                  # z.B. für friedliche Tiere (NPCs: Standard)
```

Admins können mit `erfahrung <spieler> <menge>` zum Testen Erfahrung vergeben.

---

## 12. Befehle anpassen und eigene Gesten

Alle Befehle lassen sich anpassen, ohne zu programmieren. Die Dateien liegen im
Ordner `daten/befehle/` (alternativ eine einzelne Datei `daten/befehle.toml`).
Am bequemsten geht es im Welteditor im Bereich *⌨ Befehle*.

### Eingebaute Befehle ändern (`befehle/anpassungen.toml`)

```toml
[befehl.toete]
woerter = ["toete", "attacke", "kill", "k"]  # ersetzt alle Wörter, das erste ist der Hauptname
rolle   = "spieler"                          # spieler, baumeister oder admin
aktiv   = true                               # false = abgeschaltet
hilfe   = "Angreifen"                        # Text in der Hilfe
syntax  = "toete <monster>"
```

Die Namen aller Befehle (`toete`, `schau`, `nimm` …) zeigt der Welteditor im
Bereich *Befehle*, im Spiel `hilfe <befehl>`. Die Hilfe im Spiel entsteht
automatisch aus diesen Angaben: Umbenannte, abgeschaltete und neue Befehle
erscheinen dort sofort richtig. Die Prüfung meldet, wenn ein Wort doppelt
vergeben ist oder mit einer Richtung (`n`, `o` …) zusammenstößt.

### Gesten (`befehle/gesten.toml`)

```toml
[[geste]]
woerter     = ["winke", "winken"]
hilfe       = "Winken oder jemandem zuwinken"
selbst      = "Du winkst fröhlich."              # winke
andere      = "{name} winkt fröhlich."
selbst_ziel = "Du winkst {ziel_dativ} zu."       # winke bert / winke wirt
ziel        = "{name} winkt dir zu."
andere_ziel = "{name} winkt {ziel_dativ} zu."
```

`{name}` ist, wer die Geste macht. `{ziel}` ist das Ziel im Akkusativ („den
Wirt“), `{ziel_dativ}` im Dativ („dem Wirt“). Den Dativ trägt man bei NPCs und
Monstern mit `dativ = "dem Wirt"` ein. Mit `rolle` kann eine Geste auch nur für
Baumeister oder Admins gelten, mit `aktiv = false` wird sie abgeschaltet.

---

## 13. Fertigkeiten

```toml
[[fertigkeit]]
id        = "heilkunde"
name      = "Heilkunde"
kategorie = "zauberei"         # zum Gruppieren
attribut  = "weisheit"         # passender Grundwert aus [start.werte]
max_stufe = 100
lernchance = 20                # Prozent: Chance auf +1 bei jeder Benutzung
voraussetzungen = []           # andere Fertigkeiten, die vorher nötig sind
beschreibung = "Wunden heilen."
```

---

## 14. Zauber und Kampftechniken

Zauber spricht man mit `zaubere <name> [ziel]`. **Kampftechniken** (`typ = "technik"`)
setzt man mit `spezial <name> [ziel]` ein. Beide stehen in `daten/zauber/` und
funktionieren gleich. Der Unterschied: Eine Technik kann einen Schlag mit der
Waffe enthalten (`waffen_faktor`), und sie kostet meist Ausdauer statt Mana.
`zauber` zeigt beides an. Ohne Ziel trifft ein Angriff den aktuellen Gegner.

```toml
[[zauber]]
id   = "kleine_heilung"
name = "Kleine Heilung"            # Spieler tippen: zaubere kleine heilung
schluesselwoerter = ["heilung"]    # oder: zaubere heilung
typ           = "zauber"           # zauber | technik
mana_kosten   = 10
ausdauer_kosten = 0
ziel          = "spieler"          # selbst | spieler | gegner | raum
fertigkeit    = "heilkunde"        # nötige Fertigkeit (optional, wird geübt)
mindest_stufe = 1
abklingzeit   = 0                  # Sekunden bis zum nächsten Mal
gilde         = ""                 # nur für Mitglieder dieser Gilde (optional)
wirkungen = [ { art = "heilen", wert = 20 } ]
text_selbst = "Du heilst {ziel}."
text_ziel   = "{name} heilt dich."
text_raum   = "{name} spricht eine Heilformel."
text_ende   = ""                   # wenn eine Wirkung mit Dauer endet
```

**Wirkungen, die sofort wirken:**

| art         | bedeutet                                  | Angabe                 |
|-------------|-------------------------------------------|------------------------|
| `heilen`    | gibt Lebenspunkte                         | `wert`                 |
| `mana`      | gibt Mana                                 | `wert`                 |
| `ausdauer`  | gibt Ausdauer                             | `wert`                 |
| `schaden`   | trifft einen Gegner                       | `wert`                 |
| `betaeuben` | der Gegner kann so lange nicht angreifen  | `wert` = Sekunden      |
| `teleport`  | bringt das Ziel in einen Raum             | `raum = "id"`          |

**Wirkungen mit Dauer** (Schilde und Stärkungen, `dauer` in Sekunden):

| art        | bedeutet                                         |
|------------|--------------------------------------------------|
| `schild`   | fängt `wert` Schaden ab, dann zerbricht er       |
| `ruestung` | `wert` zusätzliche Rüstung                       |
| `bonus`    | `wert` mehr auf `attribut`, z.B. `attribut = "staerke"` |
| `kraft`    | `wert` Prozent mehr Schaden                      |
| `tempo`    | `wert` Prozent schnellere Angriffe               |
| `heilen`, `mana`, `ausdauer` mit `dauer` | wirken alle `intervall` Sekunden (Standard 3) |
| `schaden` mit `dauer` (gegen Gegner) | Gift: alle `intervall` Sekunden Schaden |

```toml
[[zauber]]
id   = "schutzschild"
name = "Schutzschild"
mana_kosten = 15
wirkungen = [ { art = "schild", wert = 30, dauer = 90 } ]
text_ende = "Der schimmernde Schild um dich verblasst."

[[zauber]]
id   = "wuchtschlag"
typ  = "technik"                   # spezial wuchtschlag
name = "Wuchtschlag"
ziel = "gegner"
ausdauer_kosten = 15
abklingzeit     = 8
waffen_faktor   = 1.8              # Waffenschaden mal 1,8
```

Wer einen Zauber noch einmal spricht, frischt die Wirkung auf. Zwei gleiche
Wirkungen addieren sich nicht. Mit `wirkungen` sieht man im Spiel, was gerade
wirkt und wie lange noch.

Bei Zaubern auf einen Gegner wirken `schaden` und `betaeuben` auf den Gegner,
alle anderen Wirkungen auf den Zaubernden selbst. So geht zum Beispiel
„Lebensraub“: Schaden am Gegner und Heilung für sich.

Ein Zauber darf mehrere Wirkungen haben. In Texten setzt der Treiber für
`{name}` den Zaubernden ein, für `{ziel}` das Ziel und für `{zauber}` den Namen
des Zaubers. Mit dem Zaubern übt man die zugehörige Fertigkeit.

Spieler lernen Zauber und Techniken bei Lehrmeistern (Kapitel 18), durch
Belohnungen für Quests (Kapitel 20), über ihre Rasse oder Gilde (Kapitel 19)
oder von Anfang an (`zauber` in `[start]`).

------------|--------------------------------|-------------------|
| `heilen`   | gibt Lebenspunkte              | `wert`            |
| `mana`     | gibt Mana                      | `wert`            |
| `teleport` | bringt das Ziel in einen Raum  | `raum = "id"`     |
| `schaden`  | wirkt erst mit dem Kampfsystem | `wert`            |

Ein Zauber darf mehrere Wirkungen haben. In Texten setzt der Treiber für
`{name}` den Zaubernden ein, für `{ziel}` das Ziel und für `{zauber}` den Namen
des Zaubers.

Mit dem Zaubern übt man die zugehörige Fertigkeit und kann dabei aufsteigen.

---

## 15. Rollen: Spieler, Baumeister, Admin

Jede Figur hat eine von drei Rollen:

| Rolle          | darf                                                            |
|----------------|-----------------------------------------------------------------|
| **Spieler**    | spielen                                                          |
| **Baumeister** | zusätzlich die Welt bauen und testen, sieht IDs und technische Werte |
| **Admin**      | zusätzlich Rollen vergeben, Spieler hinauswerfen, Verkündungen  |

Die Rolle wird in der Spielerdatei gespeichert. Ein Admin vergibt sie im Spiel:

```
rolle bert baumeister
```

Das funktioniert auch, wenn Bert gerade nicht online ist. In `welt.toml`
legst du fest, wer **mindestens** Admin oder Baumeister ist. So gibt es immer
einen ersten Admin:

```toml
admins     = ["Areadox"]
baumeister = ["Lina"]
```

In `wer` und im Spielstand sind Baumeister und Admins markiert.

### Baumeister-Befehle

| Befehl                         | Wirkung                                      |
|--------------------------------|----------------------------------------------|
| `neuladen`                     | alle Dateien neu einlesen; bei Fehlern bleibt die alte Welt aktiv |
| `pruefen`                      | Dateien prüfen, ohne sie zu laden            |
| `wo`                           | ID, Koordinaten und Datei des aktuellen Raums |
| `springe <raum-id>`            | zu einem Raum springen                       |
| `erschaffe <gegenstand-id>`    | Gegenstand ins Inventar legen                |
| `beschwoere <monster-id>`      | Monster oder NPC in den Raum holen           |
| `liste raeume` (usw.)          | alles einer Art auflisten                    |
| `zuruecksetzen`                | Monster und Gegenstände sofort auffüllen     |

Baumeister sehen außerdem die IDs hinter Raumnamen. `schau` zeigt ihnen
technische Werte sowie Unterdetails und Sinne eines Details.

### Admin-Befehle

| Befehl                            | Wirkung                                 |
|-----------------------------------|-----------------------------------------|
| `rolle <spieler> spieler\|baumeister\|admin` | Rolle vergeben (auch offline) |
| `rauswerfen <spieler> [grund]`    | Spieler vom Server werfen               |
| `verkuende <text>`                | Nachricht an alle                       |
| `lehre <spieler> <id> [stufe]`    | Fertigkeit oder Zauber beibringen       |
| `erfahrung <spieler> <menge>`     | Erfahrung geben (zum Testen)            |
| `gebegeld <spieler> <menge>`      | Geld geben (zum Testen)                 |
| `speichernalle`                   | alle Spieler sofort speichern           |

---

## 16. Wenn etwas nicht stimmt

Der Treiber prüft beim Laden alles und meldet Fehler mit Datei und Ort, oft mit
einem Vorschlag:

```
✗ [raeume/dorf.toml] Raum 'taverne': Monster 'wird' gibt es nicht. Meintest du 'wirt'?
```

**Fehler** verhindern das Laden. **Warnungen** (zum Beispiel ein Raum, den man
nicht erreichen kann) sind nur Hinweise.

Spielerfiguren werden im Ordner `spieler/` gespeichert, eine Datei pro Figur.

---

## 17. MUD-Programme und Protokolle

Man kann mit jedem Telnet-Programm spielen. Besser geht es mit einem
MUD-Programm (Client) wie **Mudlet**, **TinTin++**, **MUSHclient** oder
**Blightmud**. RMengine erkennt beim Verbinden selbst, was das Programm kann.
Einstellen musst du nichts.

| Protokoll | Was es bringt                                                         |
|-----------|-----------------------------------------------------------------------|
| NAWS      | Das Programm meldet seine Fensterbreite, und Texte werden passend umbrochen. |
| TTYPE/MTTS| Das Programm nennt seinen Namen (Admins sehen ihn bei `wer`).          |
| EOR / GA  | Markiert das Ende des Prompts, damit das Programm ihn sicher erkennt.  |
| CHARSET   | UTF-8 für Umlaute. Alte Programme bekommen auf Wunsch ISO-8859-1.     |
| MCCP2     | Kompression: deutlich weniger Datenverkehr                            |
| GMCP      | Daten im Hintergrund für Lebensbalken, Automapper und Chatfenster     |
| MSDP      | dasselbe für Programme, die MSDP statt GMCP benutzen                  |
| MSSP      | Infos für MUD-Verzeichnisse (Name, Spielerzahl, Laufzeit …)           |

Im **Spielerbuch** steht unter „Dein Programm“, welches Programm erkannt wurde
und welche Protokolle laufen. Mit `spielerbuch breite auto` (Standard) passt
sich die Zeilenbreite dem Fenster an, mit `spielerbuch breite 80` ist sie fest.

### GMCP-Pakete (für Skripte in Mudlet & Co.)

| Paket               | Wann                       | Inhalt                                                    |
|---------------------|----------------------------|-----------------------------------------------------------|
| `Char.Name`         | beim Einloggen             | `name`, `fullname`                                        |
| `Char.Vitals`       | bei Änderung, vor dem Prompt | `hp`, `maxhp`, `mp`, `maxmp`, `ep`, `maxep` (LP, MP, AP) |
| `Char.Status`       | bei Änderung, vor dem Prompt | `name`, `level`, `xp`, `xp_next`, `free_points`, `gold`, `race`, `guild`, `role`, `dead`, `enemy`, `enemy_health` (Prozent) |
| `Room.Info`         | bei jedem Raum             | `num`, `id`, `name`, `area`, `environment`, `x`, `y`, `z`, `coords`, `map`, `exits` (`{"n": 12345, ...}`) |
| `Comm.Channel.Text` | bei `sage` und `rufe`      | `channel` (`say`/`shout`), `talker`, `text`               |
| `Core.Ping`         | auf Anfrage                | Antwort auf den Ping des Programms                        |

`num` ist eine feste Nummer, die aus der Raum-ID berechnet wird. Sie bleibt
gleich, solange du die ID nicht änderst, und der Automapper erkennt den Raum so
wieder. `area` ist das `gebiet` des Raums, `environment` die Geländeart.
Richtungen in `exits`: `n ne e se s sw w nw u d in out`.

### MSDP-Variablen

`CHARACTER_NAME`, `SERVER_ID`, `SERVER_TIME`, `HEALTH`, `HEALTH_MAX`, `MANA`,
`MANA_MAX`, `MOVEMENT`, `MOVEMENT_MAX`, `LEVEL`, `MONEY`, `EXPERIENCE`, `EXPERIENCE_MAX`,
`EXPERIENCE_TNL`, `OPPONENT_NAME`, `OPPONENT_HEALTH`, `OPPONENT_HEALTH_MAX`,
`ROOM`, `ROOM_NAME`, `ROOM_VNUM`, `ROOM_AREA`, `ROOM_TERRAIN` und `ROOM_EXITS`.
Die Befehle `LIST`, `REPORT`, `UNREPORT`, `SEND` und `RESET` werden verstanden.
Mit `REPORT` bestellte Werte kommen automatisch, sobald sie sich ändern.

### Eintrag in MUD-Verzeichnissen (MSSP)

Verzeichnisse wie mudverse.com oder Grapevine fragen den Server per MSSP ab.
Name, Spielerzahl, Laufzeit, Anzahl der Räume und die Protokolle schickt
RMengine selbst. Weitere Angaben trägst du in `welt.toml` ein:

```toml
[mssp]
GENRE    = "Fantasy"
GAMEPLAY = "Adventure"
STATUS   = "Alpha"
HOSTNAME = "mud.example.de"
WEBSITE  = "https://..."
```

---

## 18. Geld, Händler und Lehrmeister

Jede Figur hat Geld. Wie es heißt, steht in `welt.toml` (`waehrung = "Gold"`),
das Startgeld in `[start]` (`geld = 20`). Geld gibt es:

- von Monstern: `geld = 5` oder `geld = [2, 8]` (zufällig von 2 bis 8). Haben
  mehrere zusammen gekämpft, wird geteilt.
- als Belohnung für Aufgaben (Kapitel 20)
- beim Verkaufen an Händler

`geld` und `sp` zeigen, wie viel man hat.

### Händler

Ein NPC wird zum Händler mit `[npc.haendler]`. Die Preise richten sich nach dem
`wert` der Gegenstände.

```toml
[npc.haendler]
waren           = ["brot", "heiltrank", "fackel"]   # verkauft er (unbegrenzt)
ankauf          = true          # kauft er an? (Standard: ja)
ankauf_typen    = ["waffe"]     # nur diese Typen (leer = alles)
verkauf_prozent = 100           # Verkaufspreis in % vom Wert
ankauf_prozent  = 50            # er zahlt die Hälfte des Werts
```

Spieler: `waren` (Angebot zeigen), `kaufe brot`, `kaufe 3 brot`,
`verkaufe pelz`.

### Lehrmeister

Ein NPC lehrt Fertigkeiten und Zauber mit `[[npc.lehrt]]`:

```toml
[[npc.lehrt]]
id    = "schwertkampf"   # Fertigkeit: jede Lektion +1 Stufe
preis = 5                # pro Lektion
bis   = 20               # höchstens bis Stufe 20

[[npc.lehrt]]
id    = "wuchtschlag"    # Zauber oder Technik: einmal lernen
preis = 40
stufe = 2                # ab Spielerstufe 2
gilde = ""               # nur für Mitglieder dieser Gilde (optional)
```

Spieler: `waren` zeigt auch, was gelehrt wird. Gelernt wird mit
`lerne schwertkampf`. Voraussetzungen von Fertigkeiten werden geprüft.

**Achtung TOML:** Felder des NPCs (wie `gilde = "..."`) müssen **vor** dem
ersten `[[npc.lehrt]]` oder `[npc.haendler]` stehen. Sonst gehören sie zur
Lehre bzw. zum Händler. Der Welteditor achtet von selbst darauf.

---

## 19. Rassen, Gilden und Berufe

### Rassen (`daten/rassen/`)

Gibt es Rassen, wählt jede neue Figur bei der Erschaffung eine aus
(`info elf` zeigt Einzelheiten). Ältere Figuren können mit `rasse <name>` einmal
nachwählen.

```toml
[[rasse]]
id   = "zwerg"
name = "Zwerge"
beschreibung = "Klein, zäh und stur."
werte        = { staerke = 2, geschick = -1 }  # Zuschläge auf die Grundwerte
lebenspunkte = 20                              # mehr (oder weniger) LP, MP, AP
mana         = -10
ausdauer     = 0
fertigkeiten = { schwertkampf = 3 }            # von Anfang an
zauber       = []
waehlbar     = true                            # false = nicht wählbar
```

### Gilden (`daten/gilden/`) und Berufe (`daten/berufe/`)

Man ist in höchstens **einer Gilde**, kann aber mehrere **Berufe** haben
(`max_berufe` in `welt.toml`). Beigetreten wird bei einem NPC, der dafür
zuständig ist:

```toml
[npc]
id    = "priesterin"
gilde = "lichtorden"     # nimmt in diese Gilde auf
beruf = ""               # oder in einen Beruf
```

```toml
[[gilde]]
id    = "lichtorden"
name  = "Orden des Lichts"
beschreibung   = "Heiler und Hüter."
aufnahme_stufe = 2               # ab Stufe
preis          = 10              # Aufnahmegebühr
rassen         = []              # nur diese Rassen (leer = alle)
titel          = "vom Orden des Lichts"   # wird als Titel gesetzt
mana           = 15              # Zuschläge wie bei Rassen
fertigkeiten   = { heilkunde = 5 }
zauber         = ["lebensquell"]
aufnahme_text  = "Möge das Licht dich leiten."
```

Berufe haben dieselben Felder (`[[beruf]]`).

Spieler: `beitreten` (beim NPC), `austreten gilde` oder `austreten <beruf>`,
`gilde` (eigene Gilde und wer von ihr gerade spielt), `beruf`, `rasse`.

Das ist die **Grundlage**, die du ausbauen kannst:

- Lehrmeister nur für Mitglieder: `gilde = "..."` bei `[[npc.lehrt]]`
- Zauber und Techniken nur für Mitglieder: `gilde = "..."` beim Zauber
- Aufgaben nur für Mitglieder: `gilde = "..."` bei der Quest

Das Herstellen von Dingen in Berufen (Handwerk) ist noch nicht eingebaut.
Die Berufe geben aber schon ihre Fertigkeiten.

---

## 20. Aufgaben und Quests

Aufgaben (klein, oft wiederholbar) und Quests (größere Geschichten) stehen in
`daten/quests/`. Ein NPC bietet sie an:

```
> sage hast du eine aufgabe für mich?
Der Wirt sagt: „Im Keller wimmelt es von Ratten! ...“
(Quest: Die Rattenplage – annehmen mit annehmen die rattenplage)
> annehmen
> aufgaben                 (zeigt alle Aufgaben und den Fortschritt)
...
> rede mit wirt            (oder: abgeben)
*** Quest erledigt: Die Rattenplage ***
```

```toml
[[quest]]
id     = "rattenplage"
name   = "Die Rattenplage"
art    = "quest"             # "aufgabe" oder "quest"
geber  = "wirt"              # NPC, der sie anbietet und annimmt
beschreibung = "Im Keller treiben sich Ratten herum."
angebot   = "Wenn du mir drei Ratten vom Hals schaffst, lohnt es sich."
annahme   = "Wunderbar! Die Falltür ist hinter der Theke."
abschluss = "Endlich Ruhe im Keller!"
mindest_stufe   = 1
voraussetzungen = []         # andere Quests, die vorher erledigt sein müssen
gilde           = ""         # nur für Mitglieder
wiederholbar    = false
abgabe          = true       # false = fertig, sobald alle Schritte erledigt sind

[[quest.schritte]]           # werden der Reihe nach erledigt
art    = "besuchen"
ziel   = "keller"

[[quest.schritte]]
art    = "toeten"
ziel   = "ratte"
anzahl = 3
text   = "Besiege 3 Ratten"  # steht im Aufgabenbuch (sonst automatisch)

[quest.belohnung]
erfahrung    = 60
geld         = 25
gegenstaende = ["heiltrank"]
fertigkeiten = { orientierung = 2 }   # steigen um so viele Stufen
zauber       = []
titel        = ""
```

**Arten von Schritten:**

| art        | erledigt, wenn ...                                           |
|------------|--------------------------------------------------------------|
| `toeten`   | man `anzahl` Monster mit der ID `ziel` besiegt hat            |
| `bringen`  | man `anzahl` Gegenstände `ziel` beim NPC `an` abgibt (leer = Geber) |
| `besuchen` | man den Raum `ziel` betritt                                   |
| `sprechen` | man mit dem NPC `ziel` redet                                  |

Spieler: `aufgaben`, `aufgaben erledigt`, `annehmen [name]`, `abgeben`,
`abbrechen <name>`. Zum Abgeben reicht es auch, mit dem Geber zu reden.

