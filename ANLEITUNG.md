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
   - *RMengine starten*: startet den Server
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

### Aktualisieren

Beim Start meldet RMengine, wenn es eine neue Version gibt. Aktualisieren kannst
du so:

- **Windows:** Startmenü → *RMengine* → *Nach Updates suchen*, oder den neuen
  Installer einfach über die alte Version installieren.
- **Linux:**

  ```bash
  rmengine --update
  ```

Dabei wird nur das Programm ausgetauscht. **Deine Welt im Ordner `daten` und die
Spielstände im Ordner `spieler` bleiben unverändert.** Beende den Server vor dem
Update.

### Für Entwickler

Mit installiertem Rust (<https://rustup.rs>) startest du den Server aus dem
Quellcode mit `cargo run --release` und prüfst die Daten mit
`cargo run --release -- --pruefen`.

---

## 2. Befehle für Spieler

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
| `kurz`           | an / aus              | beim Gehen keine Raumbeschreibung            |
| `breite`         | 40 bis 200            | Zeilenbreite für Texte                       |
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

## 3. Der Aufbau des Datenordners

```
daten/
  welt.toml            Grundeinstellungen, Startwerte, Geländearten
  raeume/*.toml        Räume
  gegenstaende/*.toml  Gegenstände
  monster/*.toml       Monster und Tiere
  npcs/*.toml          NPCs mit Gesprächen, ein NPC pro Datei
  beschreibungen/      allgemeine Beschreibungen (Bäume, Himmel, Möbel ...)
  fertigkeiten/*.toml  Fertigkeiten
  zauber/*.toml        Zaubersprüche
  oberwelt/            die Landschaft (siehe Abschnitt 6)
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

## 4. Räume und die Karte

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

---

## 5. Ausführlich beschreiben

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

## 6. Die Oberwelt (Landschaft)

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

## 7. Gegenstände

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

## 8. Monster

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

## 9. NPCs und Gespräche

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

## 10. Kampf, Stufen und Tod

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

## 11. Fertigkeiten

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

## 12. Zauber

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

## 13. Rollen: Spieler, Baumeister, Admin

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
| `speichernalle`                   | alle Spieler sofort speichern           |

---

## 14. Wenn etwas nicht stimmt

Der Treiber prüft beim Laden alles und meldet Fehler mit Datei und Ort, oft mit
einem Vorschlag:

```
✗ [raeume/dorf.toml] Raum 'taverne': Monster 'wird' gibt es nicht. Meintest du 'wirt'?
```

**Fehler** verhindern das Laden. **Warnungen** (zum Beispiel ein Raum, den man
nicht erreichen kann) sind nur Hinweise.

Spielerfiguren werden im Ordner `spieler/` gespeichert, eine Datei pro Figur.
