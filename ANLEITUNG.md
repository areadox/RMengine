# Anleitung für Weltenbauer

Mit diesem MUD-Treiber baust du eine Welt, **ohne zu programmieren**. Die ganze
Welt steht in Textdateien im Ordner `daten/`. Du änderst eine Datei, tippst im
Spiel `neuladen`, und die Änderung ist da. Ein Neustart ist dafür nicht nötig.

---

## 1. Starten

Einmalig Rust installieren: <https://rustup.rs>

```bash
cargo run --release
```

Danach verbindest du dich mit einem MUD-Programm, zum Beispiel
[Mudlet](https://www.mudlet.org) (Server `localhost`, Port `4000`), oder im
Terminal mit:

```bash
telnet localhost 4000
```

**Wichtig:** Lege als Erstes die Figur `Admin` an. Wer so heißt, ist Baumeister
(einstellbar in `welt.toml` unter `admins`).

Nur die Daten prüfen, ohne den Server zu starten:

```bash
cargo run --release -- --pruefen
```

---

## 2. Der Aufbau des Datenordners

```
daten/
  welt.toml            Grundeinstellungen, Startwerte, Geländearten
  raeume/*.toml        Räume
  gegenstaende/*.toml  Gegenstände
  monster/*.toml       Monster und Bewohner
  fertigkeiten/*.toml  Fertigkeiten
  zauber/*.toml        Zaubersprüche
  oberwelt/            die Landschaft (siehe Abschnitt 4)
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

## 3. Räume und die Karte

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
stadtkarte_stil    = "kasten"   # oder "linien": ein Zeichen pro Raum, ohne Rahmen
karten_hintergrund = "*"        # Füllzeichen, z.B. "*" oder " "
```

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

---

## 4. Die Oberwelt (Landschaft)

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

## 5. Gegenstände

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

## 6. Monster

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

## 7. Fertigkeiten

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

## 8. Zauber

```toml
[[zauber]]
id   = "kleine_heilung"
name = "Kleine Heilung"            # Spieler tippen: zaubere kleine heilung
schluesselwoerter = ["heilung"]    # oder: zaubere heilung
mana_kosten   = 10
ziel          = "spieler"          # selbst | spieler | gegner | raum
fertigkeit    = "heilkunde"        # nötige Fertigkeit (optional)
mindest_stufe = 1
abklingzeit   = 0                  # Sekunden bis zum nächsten Mal
wirkungen = [ { art = "heilen", wert = 20 } ]
text_selbst = "Du heilst {ziel}."
text_ziel   = "{name} heilt dich."
text_raum   = "{name} spricht eine Heilformel."
```

**Wirkungen:**

| art        | bedeutet                       | Angabe            |
|------------|--------------------------------|-------------------|
| `heilen`   | gibt Lebenspunkte              | `wert`            |
| `mana`     | gibt Mana                      | `wert`            |
| `teleport` | bringt das Ziel in einen Raum  | `raum = "id"`     |
| `schaden`  | wirkt erst mit dem Kampfsystem | `wert`            |

Ein Zauber darf mehrere Wirkungen haben. In Texten setzt der Treiber für
`{name}` den Zaubernden ein, für `{ziel}` das Ziel und für `{zauber}` den Namen
des Zaubers.

Mit dem Zaubern übt man die zugehörige Fertigkeit und kann dabei aufsteigen.

---

## 9. Baumeister-Befehle im Spiel

| Befehl                         | Wirkung                                      |
|--------------------------------|----------------------------------------------|
| `neuladen`                     | alle Dateien neu einlesen; bei Fehlern bleibt die alte Welt aktiv |
| `pruefen`                      | Dateien prüfen, ohne sie zu laden            |
| `wo`                           | ID, Koordinaten und Datei des aktuellen Raums |
| `springe <raum-id>`            | zu einem Raum springen                       |
| `erschaffe <gegenstand-id>`    | Gegenstand ins Inventar legen                |
| `beschwoere <monster-id>`      | Monster in den Raum holen                    |
| `lehre <spieler> <id> [stufe]` | Fertigkeit oder Zauber beibringen            |
| `liste raeume` (usw.)          | alles einer Art auflisten                    |
| `zuruecksetzen`                | Monster und Gegenstände sofort auffüllen     |

Baumeister sehen außerdem die IDs hinter Raumnamen, und `schau` zeigt ihnen
technische Werte zu Gegenständen und Monstern.

---

## 10. Wenn etwas nicht stimmt

Der Treiber prüft beim Laden alles und meldet Fehler mit Datei und Ort, oft mit
einem Vorschlag:

```
✗ [raeume/dorf.toml] Raum 'taverne': Monster 'wird' gibt es nicht. Meintest du 'wirt'?
```

**Fehler** verhindern das Laden. **Warnungen** (zum Beispiel ein Raum, den man
nicht erreichen kann) sind nur Hinweise.

Spielerfiguren werden im Ordner `spieler/` gespeichert, eine Datei pro Figur.
