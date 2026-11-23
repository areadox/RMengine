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

**Richtungen:** `nord nordost ost suedost sued suedwest west nordwest hoch runter`

**Rückwege:** Wenn A nach Norden zu B führt, bekommt B automatisch einen Ausgang
nach Süden zu A (abschaltbar mit `ausgaenge_beidseitig = false` in `welt.toml`).

Passt ein Ausgang nicht zu den Koordinaten (Ausgang `ost`, aber der Zielraum liegt
nicht genau östlich), bekommst du eine Warnung. Der Ausgang funktioniert trotzdem,
nur die Linie fehlt auf der Karte.

### Geländearten (in welt.toml)

```toml
[gelaende.wald]
name   = "Wald"      # erscheint in der Legende
symbol = "♣"         # genau ein Zeichen
farbe  = "gruen"
```

Auf der Karte steht `@` für dich selbst und `*` für andere Spieler. Verbindungen
werden als `- | / \` gezeichnet.

---

## 4. Gegenstände

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

## 5. Monster

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

## 6. Fertigkeiten

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

## 7. Zauber

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

## 8. Baumeister-Befehle im Spiel

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

## 9. Wenn etwas nicht stimmt

Der Treiber prüft beim Laden alles und meldet Fehler mit Datei und Ort, oft mit
einem Vorschlag:

```
✗ [raeume/dorf.toml] Raum 'taverne': Monster 'wird' gibt es nicht. Meintest du 'wirt'?
```

**Fehler** verhindern das Laden. **Warnungen** (zum Beispiel ein Raum, den man
nicht erreichen kann) sind nur Hinweise.

Spielerfiguren werden im Ordner `spieler/` gespeichert, eine Datei pro Figur.
