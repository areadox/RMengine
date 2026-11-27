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
   Bei der ersten Installation wählst du die Welt:
   - **Beispielwelt „Eichenhain“**: Dorf, Wald, Händler, Gilden und Quests –
     gut zum Ausprobieren und Abschauen
   - **Leere Welt**: nur ein Startraum, zum Selberbauen

   Bei einem Update wird nicht gefragt, deine Welt bleibt, wie sie ist.
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

Bei der ersten Installation fragt das Skript, ob die Beispielwelt oder eine
leere Welt angelegt werden soll. Ohne Nachfrage geht es so:

```bash
curl -fsSL https://github.com/areadox/RMengine/releases/latest/download/install.sh | bash -s -- --leere-welt
```

(oder `--beispielwelt`). Das Programm landet in `~/.local/share/rmengine`, die
Welt in `~/.local/share/rmengine/daten`. Danach:

```bash
rmengine            # Server starten
rmengine --pruefen  # Weltdaten prüfen
```

### Eine leere Welt anlegen

Eine leere Welt kannst du auch später jederzeit anlegen, z.B. um neben der
Beispielwelt eine eigene zu bauen:

```bash
rmengine meinewelt --neue-welt     # legt den Ordner "meinewelt" an
rmengine meinewelt                 # und startet ihn
```

Die leere Welt hat eine vollständige `welt.toml` (mit Erklärungen,
Geländearten, Geld und Darstellung), einen Startraum und die üblichen Ordner.
Eine vorhandene Welt wird dabei nie überschrieben. Öffne den Ordner im
Welteditor und baue los.

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

Danach steht die Arbeitskopie in der Weltauswahl des Editors (🌐 mit der
Adresse). Mit *Schlüssel auf diesem PC merken* musst du Adresse und Schlüssel
nie wieder eingeben: Welt in der Auswahl öffnen, fertig.

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

Die wichtigsten Befehle zeigt `hilfe`.

**Auswahlen gehen auch mit Zahlen:** Listen im Spiel sind nummeriert, und statt
des Namens kann man die Nummer tippen: bei der Rassenwahl (`3` oder `info 2`),
bei `waren` (`kaufe 2`, `kaufe 3 5` = dreimal Nummer 5, `lerne 1`), bei
`zauber` (`zaubere 4`, `spezial 6 ratte`), bei Aufgaben (`annehmen 1`,
`abbrechen 1`) und bei `beitreten` und `austreten`.

Zwei Befehle gehören jeder Figur:

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

Wie der Spielstand aussieht, legt die Welt in `[darstellung]` fest (Kapitel 18):
als „Bogen“ mit Rahmen und Balken, „kompakt“ in wenigen Zeilen oder „beides“ –
dann zeigt `sp` die kompakte Form und `sp voll` den ganzen Bogen.

### Spielerbuch (`spielerbuch`, `buch` oder `lies spielerbuch`)

Hier stellt jeder Spieler selbst ein, wie das Spiel für ihn aussieht. Geändert
wird mit `spielerbuch <einstellung> <wert>`, zum Beispiel
`spielerbuch karte aus`. Die Einstellungen sind nummeriert: `spielerbuch 2`
schaltet die zweite um (an/aus) oder geht zum nächsten Wert weiter
(z.B. klein → normal → gross).

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
  eigene_befehle/      eigene Befehle mit Bedingungen und Wirkungen (Kapitel 12)
  quests/, rassen/, gilden/, berufe/   Aufgaben, Rassen, Gilden, Berufe
  koerper/             Körperbau: Hände, Maul, Pfoten ... (Kapitel 22)
  wetter/              Wetterarten (Kapitel 23)
  skripte/             Lua-Skripte: eigene Befehle und Ereignisse (Kapitel 24)
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

**Weltauswahl:** Beim Start zeigt der Editor die Welten, die du schon
bearbeitet hast: lokale Ordner (💻) und Arbeitskopien vom Server (🌐, mit
gemerktem Schlüssel). Von dort kannst du auch einen anderen Ordner öffnen, eine
neue leere Welt anlegen oder dich mit einem Server verbinden. Während der Arbeit
wechselt der Knopf **🌍 Welten** oben rechts die Welt. Soll der Editor gleich die
zuletzt benutzte Welt öffnen, nimm den Haken bei *Beim Start diese Auswahl
zeigen* weg. Die Liste steht in `%APPDATA%\RMengine\editor.json` (Windows) bzw.
`~/.config/rmengine/editor.json` (Linux).

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
- **Rechtsklick** in ein Textfeld: Ausschneiden, Kopieren, Einfügen, Alles
  markieren, Alles kopieren, Rückgängig. Rechtsklick auf einen Raum in der
  Liste oder auf der Karte und auf Einträge unter 📚 Inhalte: ID kopieren,
  Name kopieren, Kopie anlegen, Löschen und mehr.

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

- **typ:** `waffe ruestung trank nahrung getraenk schluessel werkzeug schmuck behaelter sonstiges`
- **platz:** `waffe schild kopf koerper haende beine fuesse hals ring umhang licht ruecken guertel`
- **eigenschaften:** Hier darfst du beliebige Zahlen eintragen. `heilung` und
  `mana` machen einen Gegenstand benutzbar (er wird dabei verbraucht),
  `schaden` und `ruestung` zählen im Kampf, Werte wie `staerke` erhöhen den Wert,
  solange man den Gegenstand angelegt hat. Mit Hunger und Durst (Kapitel 21)
  sättigt `essen` und `trinken` löscht den Durst.
- **haende:** So viele Hände braucht man, um ihn zu halten (Waffe, Schild,
  Fackel: Standard 1). Ein Zweihänder hat `haende = 2`. Siehe Kapitel 22.

### Behälter: Taschen, Beutel und Rucksäcke

Ein Gegenstand mit `fassungsvermoegen` ist ein Behälter. Was darin liegt, wiegt
um `gewicht_reduktion` Prozent weniger. In `vorlagen/gegenstaende.toml` gibt es
die Vorlagen `vorlage_behaelter` und `vorlage_rucksack`.

```toml
[[gegenstand]]
id   = "rucksack"
erbt = "vorlage_rucksack"
name = "ein Rucksack"
akkusativ = "einen Rucksack"
platz = "ruecken"           # kann man anlegen (muss man aber nicht)
fassungsvermoegen = 40      # so viel Gewicht passt hinein
max_dinge         = 0       # höchstens so viele Dinge (0 = beliebig)
gewicht_reduktion = 25      # der Inhalt wiegt 25 % weniger
nur_typen = []              # z.B. ["trank"] für einen Trankgürtel
dativ = ""                  # nur bei Adjektiven nötig: "einer bestickten Tasche"
```

Im Spiel: `stecke brot in rucksack`, `stecke alles in rucksack`,
`nimm brot aus rucksack`, `nimm alles aus rucksack`, `schau rucksack`.
`inventar` zeigt den Inhalt aller Behälter. Einen gefüllten Behälter kann
man nicht ablegen oder verkaufen. Behälter lassen sich nicht ineinander stecken.

### Tragkraft

Jede Figur kann nur so viel tragen: `tragkraft_basis + Stärke ×
tragkraft_pro_staerke` (in `welt.toml`, Standard 40 + 3 je Stärkepunkt).
Gezählt wird alles: Gepäck, angelegte Ausrüstung und der Inhalt von Behältern,
abzüglich ihrer Gewichtsreduktion. Wer zu viel trägt, kann nichts mehr aufheben
oder kaufen. Mit `tragkraft_basis = 0` und `tragkraft_pro_staerke = 0` gibt es
keine Grenze.

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

Hat ein Monster einen Körperbau (`koerper = "wolf"`, Kapitel 22), greift es mit
seinen Körperteilen an – z.B. „Ein grauer Wolf beißt dich!“ mit dem Maul oder
„… schlägt mit der Pfote nach dir“. Dann gelten die `angriff`- und
`schaden`-Angaben der Körperteile statt `angriffe`.

### Leichen

Besiegte Gegner hinterlassen eine Leiche. Sie liegt ein paar Minuten im Raum,
die Beute steckt darin. Spieler: `schau leiche` (zeigt auch, was darin ist),
`nimm alles aus leiche`, `nimm pelz aus leiche`, dazu `riech leiche`,
`taste leiche` und die Details. Danach verwest sie; was noch darin war, fällt
zu Boden.

```toml
[monster.leiche]
name      = "der Kadaver eines Wolfs"       # leer = "die Leiche von ..."
akkusativ = "den Kadaver eines Wolfs"
dativ     = "dem Kadaver eines Wolfs"       # für "nimm alles aus ..."
bodentext = "Der Kadaver eines grauen Wolfs liegt im Laub."
beschreibung = "Der Wolf liegt ausgestreckt da ..."
geruch    = "Nasses Fell und Blut."
gefuehl   = "Das Fell ist rau und noch ein wenig warm."
minuten   = 5                               # leer = [kampf] leichen_minuten
verwest   = "Vom Wolf bleiben nur ein paar Knochen."
# aktiv = false                             # dieses Monster hinterlässt keine Leiche

[monster.leiche.details.fell]                # Details wie bei Räumen, mit Unter-Details
text    = "Grau, struppig und an der Flanke aufgerissen."
woerter = ["pelz"]

[monster.leiche.details.fell.details.wunde]
text = "Eine tiefe Wunde – hier hat dein letzter Hieb getroffen."
```

Die Körperteile des Monsters (Maul, Pfoten …) kann man auch an der Leiche
ansehen. In `[kampf]` gibt es:

- `leichen_minuten = 5`: so lange bleibt eine Leiche liegen (0 = keine Leichen)
- `beute_in_leiche = true`: false = die Beute fällt wie früher zu Boden
- `pluendern_sekunden = 60`: so lange darf nur plündern, wer den Gegner besiegt
  hat (0 = jeder sofort). Baumeister und Admins dürfen immer.

Leichen überstehen einen Neustart des Servers: Sie werden mit den Spielern
gespeichert (`spieler/leichen.json`) und liegen danach wieder da.

---

## 12. Befehle anpassen, Gesten und eigene Befehle

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

### Eigene Befehle (`daten/eigene_befehle/`)

Eigene Befehle funktionieren wie Gesten, können aber mehr: Sie prüfen
Bedingungen und haben Wirkungen. Im Welteditor stehen sie unter
📚 Inhalte → *Eigene Befehle*. Ein Beispiel:

```toml
[[befehl]]
id          = "beten"
woerter     = ["bete", "beten"]
hilfe       = "Im Tempel beten (heilt ein wenig)"
raeume      = ["tempel"]                 # nur hier
nicht_hier  = "Zum Beten solltest du in den Tempel gehen."
abklingzeit = 60                         # Sekunden bis zum nächsten Mal
selbst      = "Du kniest nieder und sprichst ein leises Gebet."
andere      = "{name} kniet nieder und betet leise."
lp          = 15                         # heilt 15 Lebenspunkte
```

**Bedingungen** (alle freiwillig):

| Feld | Bedeutung |
|------|-----------|
| `raeume`, `gebiete` | nur in diesen Räumen bzw. Gebieten; `nicht_hier` = Meldung sonst |
| `braucht` | diese Gegenstände muss man dabeihaben (bleiben erhalten) |
| `nimm` | diese Gegenstände werden verbraucht |
| `mindest_stufe`, `gilde`, `beruf` | Stufe, Gilde oder Beruf nötig |
| `fertigkeit`, `fertigkeit_stufe` | eine Fertigkeit ab einer Stufe |
| `mana_kosten`, `ausdauer_kosten`, `kosten` | kostet Mana, Ausdauer oder Geld (kleinste Münze) |
| `abklingzeit` | Sekunden, bis man ihn wieder benutzen kann |
| `nicht_im_kampf` | `true` = nicht mitten im Kampf |
| `nur_draussen` | nicht in Häusern und Höhlen |
| `tageszeiten`, `jahreszeiten`, `wetter` | nur dann, z.B. `["nacht"]`, `["winter"]`, `["klar"]`; `nicht_jetzt` = Meldung sonst (Kapitel 23) |
| `rolle`, `aktiv` | wie bei Gesten |

**Texte:** `selbst`, `andere`, `selbst_ziel`, `ziel`, `andere_ziel` wie bei
Gesten. Zusätzlich gibt es `{rest}`: alles, was hinter dem Befehl getippt wurde.

**Zufall:** `chance = 30` heißt: klappt in 30 von 100 Fällen. Dann zeigen
`misserfolg` und `misserfolg_andere` den Text. Kosten werden auch bei einem
Misserfolg fällig.

**Wirkungen** (nur bei Erfolg):

| Feld | Bedeutung |
|------|-----------|
| `lp`, `mp`, `ausdauer` | heilt (oder schadet, mit Minus) |
| `essen`, `trinken` | sättigt bzw. löscht den Durst (Kapitel 21), z.B. an einem Brunnen |
| `geld`, `erfahrung` | gibt Geld bzw. Erfahrungspunkte |
| `gib` | diese Gegenstände bekommt man |
| `teleport` | versetzt in diesen Raum |
| `zauber` | legt die Wirkungen dieses Zaubers auf einen selbst, ohne Kosten (z.B. einen Schild) |
| `befehle` | danach weitere Befehle ausführen, z.B. `["sage Hallo {rest}", "winke {rest}"]` |

**Mit einem Gegenstand:** Ein Befehl kann sich auf einen Gegenstand beziehen,
z.B. „fuehre schwert“. Mit `gegenstand_typen` geht er nur mit bestimmten Arten:

```toml
[[befehl]]
id                  = "fuehren"
woerter             = ["fuehre", "fuehren"]
syntax              = "fuehre <waffe>"
gegenstand_typen    = ["waffe"]          # waffe, ruestung, trank, nahrung, getraenk ...
gegenstand_ort      = "dabei"            # dabei (Standard), inventar, angelegt, raum
mit_gegenstand      = "anlegen"          # anlegen, ausziehen, benutzen, verbrauchen, fallenlassen, nehmen
ohne_gegenstand     = "Welche Waffe willst du führen?"
falscher_gegenstand = "{ding} ist keine Waffe."
selbst              = "Du nimmst {ding_akk} fest in die Hand."
andere              = "{name} nimmt {ding_akk} fest in die Hand."
```

In den Texten steht `{ding}` für den Gegenstand („ein Schwert“), `{ding_akk}`
für den Akkusativ und `{ding_dativ}` für den Dativ. Gibt es einen eigenen Text
`selbst`, entfällt die eingebaute Meldung („Du legst … an.“). Ob das Anlegen
klappt (freier Platz, freie Hand), wird vorher geprüft – sonst kostet der
Befehl nichts und zeigt keinen Text.

Eigene Befehle stehen in der Hilfe unter *Besondere Befehle*. Ruft ein Befehl
über `befehle` sich selbst auf, wird das übersprungen. Eine Quest kann einen
Schritt `art = "befehl"` haben: `ziel` ist dann die ID des eigenen Befehls,
z.B. „Bete dreimal im Tempel“ (`anzahl = 3`).

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

### Nur für Gilden

Mit `gilde = "..."` gehört ein Zauber oder eine Technik einer Gilde: Nur ihre
Mitglieder können ihn lernen und benutzen. **Kampftechniken gibt es nur für
Gilden** (`techniken_nur_gilden = true` in `welt.toml`, das ist der Standard).
Jede Technik braucht dann eine `gilde`. Eine Technik ohne Gilde kann niemand
benutzen, und die Prüfung zeigt einen Hinweis.
In der Beispielwelt sind die Magier eine Gilde (Magister Orelius im
Magierturm, Feuerpfeil, Schutzschild, Eissplitter), die Techniken gehören dem
Bund der Krieger (Hauptmann Brandt am Nordtor).

### Bereitschaft: Zauber und Techniken im Autoangriff

Im Kampf greift man nach `toete <monster>` automatisch weiter an. Mit
`bereite <nr|name>` legt man fest, was der Autoangriff statt eines normalen
Schlags einsetzt:

- **Zauber** nur mit einem angelegten **Zauberstab**: einem Gegenstand mit
  `eigenschaften = { zauberstab = 1 }`.
- **Techniken** ohne besonderen Gegenstand.
- Es geht nur, was gegen Gegner wirkt (`ziel = "gegner"`) und zur eigenen
  Gilde passt.
- Fehlt Mana oder Ausdauer oder läuft die Abklingzeit noch, schlägt man
  normal zu. Sobald es wieder geht, kommt der Zauber bzw. die Technik.
- `bereite` zeigt die Liste, `bereite nichts` schaltet es ab. Der Prompt
  zeigt, was bereit ist, z.B. `[Feuerpfeil]`.

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

Jede Figur hat Geld. Geld gibt es:

- von Monstern: `geld = 5` oder `geld = [2, 8]` (zufällig von 2 bis 8). Haben
  mehrere zusammen gekämpft, wird geteilt.
- als Belohnung für Aufgaben (Kapitel 20)
- beim Verkaufen an Händler

`geld` und `sp` zeigen, wie viel man hat.

### Währung und Münzen (`[geld]` in welt.toml)

Eine Währung kann aus mehreren Münzen bestehen, z.B. Gold, Silber und Kupfer:

```toml
[geld]
anzeige = "lang"        # "lang" = 14 Gold 7 Silber 5 Kupfer, "kurz" = 14G 7S 5K

[[geld.einheiten]]
name     = "Gold"
mehrzahl = "Gold"       # leer = wie der Name
kurz     = "G"
wert     = 100          # 1 Gold = 100 Kupfer

[[geld.einheiten]]
name = "Silber"
kurz = "S"
wert = 10

[[geld.einheiten]]
name = "Kupfer"
kurz = "K"
wert = 1                # die kleinste Münze – genau eine muss den Wert 1 haben
```

**Wichtig:** Alle Geldbeträge in den Daten (`wert` von Gegenständen, `preis`
bei Lehrmeistern, Gilden und Berufen, `geld` bei Monstern, Belohnungen und
Startgeld) stehen in der **kleinsten Münze**. `wert = 1475` wird im Spiel als
„14 Gold 7 Silber 5 Kupfer“ gezeigt. Beliebig viele Stufen sind möglich
(z.B. Platin = 1000).

Fehlt `[geld]`, gibt es nur eine Münze mit dem Namen aus `waehrung = "Gold"`
in `[welt]` – ältere Welten laufen also unverändert weiter. Wer Münzstufen
einführt, muss seine Preise entsprechend umrechnen (z.B. alles mal 10 oder
mal 100).

Im Welteditor (📚 Inhalte → 💰 Geld und Währung) zeigt eine Vorschau, wie
Beträge aussehen, und neben jedem Preisfeld steht der Betrag in Münzen
(z.B. `250` = 2 Gold 5 Silber).

### Wie die Listen aussehen (`[darstellung]` in welt.toml)

Händlerlisten, Spielstand und Spielerbuch gibt es in mehreren Stilen:

```toml
[darstellung]
haendler    = "kategorien"  # einfach | tabelle | pergament | kategorien
spielstand  = "beides"      # einfach | bogen | kompakt | beides
spielerbuch = "buch"        # einfach | buch | schalter
rahmen      = "unicode"     # unicode = ╔═╗-Rahmen, ascii = +-+ (für alte Clients)
inventar    = "figur"       # einfach | figur
```

Beim Inventar als Figur (`i`) sieht man ungefähr so etwas:

```
                      ___                     Waffen
                     /   \ Lederhelm          Dolch
                     |   |
                     \___/                    Essen
                   ___/ \___                  Laib Brot
            Fackel/         \
                 / /|     |\ \                Sonstiges
     Kurzschwert/ / |     | \ \  Holzschild   Landkarte
                 /  |     |  \
                    |  _  |
                    / / \ \
                   / /   \ \
                 _/ /     \ \_
                |__/       \__|
Gewicht 12/70   Geld: 45 Gold 3 Silber
```

Die Figur kannst du selbst zeichnen: `figur = '''...'''` (mit drei einfachen
Anführungszeichen, dann bleiben Backslashes so, wie sie sind). Wo die Namen
stehen, sagt `[[darstellung.figur_plaetze]]`:

```toml
[[darstellung.figur_plaetze]]
platz       = "waffe"     # waffe, schild, kopf, koerper, haende, beine, fuesse, hals, ring, umhang, licht, ruecken, guertel
zeile       = 7           # ab 0 = oberste Zeile der Figur
spalte      = 15          # ab 0
ausrichtung = "rechts"    # links: Name beginnt an der Spalte, rechts: Name endet dort
```

Im Welteditor (🎨 Darstellung) holt *Eingebaute Figur übernehmen* die
eingebaute Figur samt Plätzen in die Felder – dann kannst du sie ändern und
siehst die Wirkung sofort in der Vorschau. Angelegtes ohne Platz an der Figur
steht in der Liste unter „Angelegt“.

| Stil          | Aussehen                                                        |
|---------------|-----------------------------------------------------------------|
| `tabelle`     | Waren als Tabelle mit Nummer, Name und Preis                    |
| `pergament`   | Laden mit Schild und verzierten Kopf- und Fußzeilen             |
| `kategorien`  | wie Tabelle, aber nach Waffen, Rüstungen, Tränken … geordnet, mit kurzen Angaben (+LP, Schaden …) |
| `bogen`       | Spielstand mit Rahmen, Balken und allen Abschnitten             |
| `kompakt`     | Spielstand in wenigen Zeilen                                    |
| `beides`      | `sp` kompakt, `sp voll` ganzer Bogen                            |
| `buch`        | Spielerbuch als Buch mit Abschnitten                            |
| `schalter`    | Spielerbuch als Liste mit [x]-Schaltern                         |
| `figur` (Inventar) | angelegte Dinge stehen an einer Figur, alles andere nach Art geordnet daneben |

Ist das Fenster des Spielers zu schmal, fällt die Anzeige von selbst auf
`einfach` zurück. Clients ohne UTF-8 bekommen automatisch ASCII-Rahmen.
Mit `schau <nr>` sieht man sich eine Ware aus der Händlerliste genauer an.

Im Welteditor (📚 Inhalte → 🎨 Darstellung) kann man die Stile auswählen
und sieht sofort eine farbige Vorschau, wie im Spiel; ein Schieberegler
stellt die Fensterbreite ein.

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
laden           = "Gretas Krämerladen"  # Name über der Liste (optional)
```

Spieler: `waren` (Angebot zeigen, nummeriert), `kaufe brot` oder `kaufe 2`
(Nummer aus der Liste), `kaufe 3 brot` bzw. `kaufe 3 2` (dreimal),
`verkaufe pelz`.

### Lehrmeister

Ein NPC lehrt Fertigkeiten und Zauber mit `[[npc.lehrt]]`:

```toml
[[npc.lehrt]]
id    = "schwertkampf"   # Fertigkeit: jede Lektion +1 Stufe
preis = 50               # pro Lektion (kleinste Münze)
bis   = 20               # höchstens bis Stufe 20

[[npc.lehrt]]
id    = "wuchtschlag"    # Zauber oder Technik: einmal lernen
preis = 400
stufe = 2                # ab Spielerstufe 2
gilde = ""               # nur für Mitglieder dieser Gilde (optional)
```

Spieler: `waren` zeigt auch, was gelehrt wird. Gelernt wird mit
`lerne schwertkampf` oder mit der Nummer aus der Liste (`lerne 1`).
Voraussetzungen von Fertigkeiten werden geprüft.

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
| `befehl`   | man den eigenen Befehl `ziel` `anzahl`-mal benutzt (Kapitel 12) |
| `skript`   | ein Skript `s:fortschritt(ziel)` aufruft (Kapitel 24)           |

Spieler: `aufgaben`, `aufgaben erledigt`, `annehmen [nr|name]`, `abgeben`,
`abbrechen <nr|name>`. Zum Abgeben reicht es auch, mit dem Geber zu reden.

---

## 21. Hunger und Durst

Eingeschaltet wird es in `welt.toml` (Standard: aus):

```toml
[ernaehrung]
aktiv              = true
max_essen          = 100
max_trinken        = 100
essen_pro_minute   = 1.0     # von satt bis leer: 100 Minuten
trinken_pro_minute = 1.5
hungrig_ab         = 30      # darunter: "hungrig"
durstig_ab         = 30
schaden_pro_minute = 2       # LP-Verlust pro Minute bei 0 (sterben kann man daran nicht)
ohne_erholung      = true    # bei 0 keine Erholung von LP und Mana
nahrung_standard   = 25      # Essen ohne Angabe
getraenk_standard  = 30      # Getränke ohne Angabe
nur_spieler        = true    # Baumeister und Admins werden nicht hungrig
```

- **Essen** (`typ = "nahrung"`) sättigt, **Getränke** (`typ = "getraenk"`)
  löschen den Durst. Wie viel, steht bei den Gegenständen:
  `eigenschaften = { essen = 40 }` bzw. `{ trinken = 50 }`. Auch Tränke können
  `trinken` haben.
- Spieler essen und trinken mit `iss brot` und `trinke wasser` (oder `benutze`).
  Wer satt ist, bekommt nichts mehr hinunter.
- Wird man hungrig oder durstig, kommt eine Meldung, und der Prompt zeigt
  `[hungrig]` bzw. `[durstig]`. `sp` zeigt den Zustand immer.
- Eigene Befehle können mit `essen` und `trinken` sättigen – z.B. Wasser an
  einem Brunnen schöpfen (Beispiel: `schoepfe` auf dem Dorfplatz).
- Für Skripte: GMCP `Char.Vitals` enthält dann `food`, `maxfood`, `drink`, `maxdrink`.

---

## 22. Körperbau und Körperteile

Ein Körperbau legt fest, welche Körperteile ein Wesen hat (`daten/koerper/`).
Rassen bekommen ihn mit `koerper = "..."`, alle anderen Spieler den aus
`[welt] koerper`. Monster ebenfalls mit `koerper = "..."`. Im Welteditor
steht er unter 📚 Inhalte → ✋ Körperbau.

```toml
[[koerper]]
id   = "wolf"
name = "Wolf"

[[koerper.teile]]
id      = "maul"
name    = "Maul"
woerter = ["schnauze", "zaehne"]         # weitere Wörter zum Ansehen
text    = "Ein langes Maul voller gelblicher Reißzähne."
geruch  = "Der Atem riecht nach rohem Fleisch."
angriff = "beißt"                        # "Ein grauer Wolf beißt dich!"
schaden = 7                              # 0 = normaler Schaden des Monsters

[[koerper.teile]]
id      = "pfoten"
name    = "Vorderpfoten"
angriff = "schlägt mit der Pfote nach {ziel_dativ}"   # {ziel} = dich, {ziel_dativ} = dir
schaden = 4
```

Jedes Körperteil kann wie ein Detail beschrieben werden: `text`, `geruch`,
`geraeusch`, `gefuehl`, `nutzt` und `details` mit Unter-Details. Dazu:

| Feld | Bedeutung |
|------|-----------|
| `text_andere` | so sehen andere das Teil („schau bert hand“), `{name}` = der Besitzer |
| `plaetze` | diese Ausrüstungsplätze trägt das Teil, z.B. `["kopf", "hals"]` |
| `halten` | eine Hand: hält Waffe, Schild oder Fackel |
| `angriff` | Angriff von Monstern (und was andere sehen, wenn ein Spieler damit angreift) |
| `angriff_du` | Angriff eines Spielers ohne Waffe, z.B. `"schlägst {ziel} mit der Faust"` |
| `schaden` | Schaden dieses Angriffs (0 = normal) |

Ein Körperbau kann auch eine **eigene Inventar-Figur** haben (`figur` und
`[[koerper.figur_plaetze]]`, wie in `[darstellung]`). Die Zwerge der Beispielwelt
haben so eine eigene, gedrungene Figur. Mit `erbt = "humanoid"` übernimmt ein
Körperbau alle Teile eines anderen.

- **Ausrüstung:** Gibt irgendein Teil `plaetze` an, kann man nur noch anlegen,
  wofür es ein passendes Teil gibt – ein Wesen ohne Kopf trägt keinen Helm.
- **Ohne Waffe** kämpft ein Spieler mit seinen Körperteilen („Du schlägst eine
  fette Ratte mit der Faust.“).
- **Halten:** Teile mit `halten = true` sind Hände. Man kann nur so viele Dinge
  halten (Platz `waffe`, `schild`, `licht`), wie man Hände hat. Ein Mensch mit
  zwei Händen hält also z.B. Schwert und Schild – für eine Fackel ist dann
  keine Hand mehr frei. Ein Zweihänder (`haende = 2`) braucht beide.
- **Angreifen:** Monster greifen mit den Teilen an, die einen `angriff` haben.
- **Ansehen:** `schau maul` (am Monster im Raum oder an seiner Leiche),
  `schau hand` (der eigene Körper), `schau bert hand` und `schau wolf maul`.
  Beim Ansehen eines Monsters oder Spielers steht dabei, welche Körperteile man
  genauer ansehen kann.

Ohne Körperbau gibt es keine Begrenzung der Hände, und Monster greifen wie
bisher mit `angriffe` an.

---

## 23. Tag und Nacht, Jahreszeiten und Wetter

```toml
[zeit]
aktiv               = true
minuten_pro_stunde  = 2.0    # echte Minuten pro Spielstunde: ein Tag dauert 48 Minuten
tage_pro_jahreszeit = 7      # ein Jahr hat 28 Spieltage
morgen = 6                   # ab diesen Stunden beginnen die Tageszeiten
tag    = 9
abend  = 18
nacht  = 21
text_morgen = "Im Osten färbt sich der Himmel rot – die Sonne geht auf."   # Meldung draußen
text_nacht  = "Es wird dunkel. Die ersten Sterne erscheinen."
zusatz_nacht = "Es ist Nacht."       # steht draußen unter der Raumbeschreibung
text_winter  = "Der Winter hält Einzug."   # Meldung an alle
wetter_minuten = 15                  # ungefähr so oft wechselt das Wetter
```

(Dazu gibt es `text_tag`, `text_abend`, `zusatz_morgen`, `zusatz_tag`,
`zusatz_abend`, `text_fruehling`, `text_sommer`, `text_herbst`.)

- Die Spielzeit wird aus der echten Uhr berechnet: Sie ist für alle gleich und
  läuft auch weiter, wenn der Server aus ist.
- `zeit` (oder `uhr`, `datum`, `wetter`) zeigt Uhrzeit, Tag, Jahreszeit und Wetter.
- Draußen steht unter der Raumbeschreibung die Tageszeit und das Wetter, z.B.
  „Es ist Nacht. Dichter Nebel liegt über allem.“ Beim Wechsel bekommen alle
  draußen eine Meldung.
- **Drinnen:** Gelände mit `drinnen = true` (in `welt.toml`, z.B. Haus, Tempel,
  Höhle) haben kein Wetter. Ein einzelner Raum kann das mit `drinnen = true`
  oder `false` ändern (im Welteditor: *Drinnen oder draußen*).
- **Nachts anders:** Räume können `beschreibung_nacht` haben – nachts steht
  dann diese Beschreibung da (im Welteditor unter der Beschreibung: *Bei Nacht*).
- **Eigene Befehle** können an Tageszeit, Jahreszeit und Wetter gebunden werden
  (Beispiel: `sterne` geht nur nachts bei klarem Himmel und nur draußen).
- Für Skripte: GMCP `Char.Status` enthält `time`, `daytime`, `season`, `weather`.

**Wetterarten** stehen in `daten/wetter/` (im Welteditor: ☁ Wetter):

```toml
[[wetter]]
id      = "schnee"
name    = "Schnee"                         # für "zeit"
text    = "Schneeflocken tanzen durch die Luft."   # draußen unter der Raumbeschreibung
beginn  = "Es fängt an zu schneien."       # Meldung, wenn es anfängt
gewicht = 4                                # wie oft (höher = häufiger)
jahreszeiten = ["winter"]                  # leer = immer
```

---

## 24. Skripte (Lua)

Mit Skripten baust du alles, was es nicht fertig gibt: einen Befehl, den es nur
in einem Raum gibt („kriech unter den tisch“), Rätsel, Fallen, Schalter,
Belohnungen, Ereignisse beim Betreten eines Raums und vieles mehr. Skripte
werden in **Lua** geschrieben – einer kleinen, leicht lernbaren
Programmiersprache, die auch viele andere Spiele benutzen.

Skripte liegen in `daten/skripte/*.lua`. Im Welteditor gibt es dafür den
Bereich **📜 Skripte** mit Code-Editor, *Prüfen* (findet Tippfehler) und einem
Spickzettel. Über die Fernwartung werden sie wie alle anderen Weltdateien
übertragen; `neuladen` lädt sie neu.

### Ein Beispiel

```lua
-- "kriech unter den tisch" – nur in der Taverne
befehl{
  woerter = {"kriech", "krieche", "kriechen"},
  raum    = "taverne",                 -- weglassen = überall; gebiet = "dorf" geht auch
  syntax  = "kriech unter den tisch",  -- steht in der Hilfe
  hilfe   = "Unter einen der Tische kriechen",
  funktion = function(s, rest, raum)
    if not rest:find("tisch") then
      return false        -- nicht gemeint: dann gelten die normalen Befehle
    end
    s:sende("Du kriechst unter einen der schweren Eichentische.")
    s:andere(s.name .. " kriecht unter einen der Tische.")
    if s.merker.taverne_muenze then
      s:sende("Außer Krümeln ist hier nichts.")
    else
      s:sende("Zwischen den Dielen glänzt eine Silbermünze! Du steckst sie ein.")
      s:geld_aendern(10)
      s:merke("taverne_muenze", true)   -- bleibt gespeichert: jeder findet sie nur einmal
    end
  end,
}
```

- `s` ist der Spieler, `rest` das, was hinter dem Befehlswort getippt wurde
  („unter den tisch“), `raum` der Raum.
- Skript-Befehle kommen **vor** den eingebauten Befehlen. So kannst du auch
  eingebaute Befehle in einem Raum anders machen. Gibt die Funktion `false`
  zurück, war der Befehl nicht gemeint und es geht normal weiter.
- Mehr Beispiele stehen in `daten/skripte/taverne.lua` und `wald.lua`.

### Anmelden: Befehle und Ereignisse

| Aufruf | Wann |
|--------|------|
| `befehl{ woerter = {...}, raum = "id", gebiet = "id", hilfe = "...", syntax = "...", funktion = function(s, rest, raum) ... end }` | Ein Befehl (raum/gebiet auch als Liste, z.B. `raum = {"a", "b"}`) |
| `bei_betreten("raum", function(s, raum) ... end)` | Jemand betritt den Raum (`"*"` = jeden Raum) |
| `bei_verlassen("raum", function(s, raum) ... end)` | Jemand verlässt den Raum |
| `bei_sagen("raum", function(s, text, raum) ... end)` | Jemand sagt dort etwas |
| `bei_sieg("monster", function(s, monster, raum) ... end)` | Jemand besiegt dieses Monster (`"*"` = jedes) |
| `bei_takt(function() ... end)` | Alle paar Sekunden (Takt der Welt) |
| `spaeter(sekunden, function() ... end)` | Etwas später ausführen, z.B. eine Tür wieder schließen |

### Der Spieler `s`

| Wert | Bedeutung |
|------|-----------|
| `s.name`, `s.raum`, `s.stufe`, `s.rolle` | Name, Raum-ID, Stufe, spieler/baumeister/admin |
| `s.lp`, `s.max_lp`, `s.mp`, `s.max_mp`, `s.ausdauer`, `s.geld` | Werte (Geld in der kleinsten Münze) |
| `s.rasse`, `s.gilde`, `s.berufe` | Gemeinschaften |
| `s.inventar`, `s.angelegt` | Liste der Gegenstand-IDs; Platz → ID |
| `s.fertigkeiten` | Fertigkeit → Stufe |
| `s.merker` | Was sich Skripte für ihn gemerkt haben |
| `s.tot`, `s.im_kampf` | wahr/falsch |

| Tun | Wirkung |
|-----|---------|
| `s:sende(text)` | Text an den Spieler (Farben wie überall: `{gelb}...{/}`) |
| `s:andere(text)` | Text an alle anderen im Raum |
| `s:hat(id)` | Hat er den Gegenstand dabei oder angelegt? |
| `s:gib(id, anzahl)` / `s:nimm(id)` | Gegenstand geben / wegnehmen (`nimm` gibt false, wenn er ihn nicht hat) |
| `s:geld_aendern(n)` / `s:zahle(n)` | Geld geben oder abziehen / bezahlen (false, wenn es nicht reicht) |
| `s:heile(n)`, `s:schade(n)`, `s:mana(n)`, `s:erfahrung(n)` | Werte ändern (Schaden kann töten) |
| `s:gehe(raum, text_weg, text_an)` | In einen anderen Raum versetzen |
| `s:merke(name, wert)` | Dauerhaft merken (wahr/falsch, Zahl, Text; `nil` löscht) |
| `s:befehl("schau")` | Einen normalen Befehl für ihn ausführen |
| `s:zauber(id)` | Die Wirkungen eines Zaubers auf ihn legen (ohne Kosten) |
| `s:fortschritt(ziel)` | Quest-Schritt `art = "skript"` mit diesem `ziel` zählt eins weiter |

### Der Raum und die Welt

| Wert / Aufruf | Bedeutung |
|---------------|-----------|
| `raum.id`, `raum.name`, `raum.gebiet`, `raum.drinnen` | Der Raum |
| `raum.gegenstaende`, `raum.monster`, `raum.spieler` | Was dort ist (IDs bzw. Namen) |
| `welt.sende_raum(raum, text)`, `welt.sende_alle(text)` | Nachrichten |
| `welt.lege(raum, id)`, `welt.entferne(raum, id)` | Gegenstand in einen Raum legen / daraus entfernen |
| `welt.erschaffe(raum, monster)` | Ein Monster oder einen NPC erscheinen lassen |
| `welt.merke(name, wert)`, `welt.merker.name` | Für die ganze Welt merken (bleibt gespeichert) |
| `welt.zeit.stunde`, `.minute`, `.tageszeit`, `.jahreszeit`, `.wetter` | Tag und Nacht (nur wenn eingeschaltet) |
| `math.random(1, 6)`, `string.find`, `print(...)` | Lua selbst; `print` schreibt in die Server-Konsole |

### Gut zu wissen

- **Sicher:** Skripte haben keinen Zugriff auf Dateien, Programme oder das
  Netz. Ein Skript, das zu lange läuft (Endlosschleife), wird abgebrochen.
- **Fehler** zeigt `rmengine --pruefen` (und *Prüfen* im Editor) mit Datei und
  Zeile. Fehler während des Spiels stehen in der Server-Konsole und werden
  Baumeistern und Admins im Spiel angezeigt; Spieler lesen nur „Da ist etwas
  schiefgegangen.“
- **Reihenfolge:** Was ein Skript tut, führt die Engine aus, sobald das Skript
  fertig ist. `s.geld` usw. zeigen innerhalb des Skripts schon den neuen Stand.
- **Merker** der Spieler stehen in ihrem Spielstand, die der Welt in
  `spieler/welt_merker.json`.
- **Quests:** Ein Schritt mit `art = "skript"` und `ziel = "hebel"` wird mit
  `s:fortschritt("hebel")` erledigt – so werden Rätsel zu Quest-Schritten.

