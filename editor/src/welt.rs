//! Das Weltmodell des Editors: Laden, Prüfen und Speichern der Räume.
//!
//! Gespeichert wird mit `toml_edit`. Dadurch bleiben Kommentare, Reihenfolge,
//! Einrückung und alles, was der Editor nicht kennt (z.B. `details`), genau so
//! erhalten, wie du sie geschrieben hast. Nur geänderte Felder werden neu geschrieben.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::{Array, ArrayOfTables, DocumentMut, InlineTable, Item, Table, Value};

// ---------------------------------------------------------------------------
// Richtungen (wie in der Engine, src/daten.rs)
// ---------------------------------------------------------------------------

pub const RICHTUNGEN: [&str; 12] = [
    "nord", "nordost", "ost", "suedost", "sued", "suedwest", "west", "nordwest", "hoch", "runter", "rein", "raus",
];

pub fn gegenteil(r: &str) -> &'static str {
    match r {
        "nord" => "sued",
        "nordost" => "suedwest",
        "ost" => "west",
        "suedost" => "nordwest",
        "sued" => "nord",
        "suedwest" => "nordost",
        "west" => "ost",
        "nordwest" => "suedost",
        "hoch" => "runter",
        "runter" => "hoch",
        "rein" => "raus",
        _ => "rein",
    }
}

/// Versatz auf der Karte: x nach Osten, y nach Norden, z nach oben.
pub fn versatz(r: &str) -> Option<(i32, i32, i32)> {
    Some(match r {
        "nord" => (0, 1, 0),
        "nordost" => (1, 1, 0),
        "ost" => (1, 0, 0),
        "suedost" => (1, -1, 0),
        "sued" => (0, -1, 0),
        "suedwest" => (-1, -1, 0),
        "west" => (-1, 0, 0),
        "nordwest" => (-1, 1, 0),
        "hoch" => (0, 0, 1),
        "runter" => (0, 0, -1),
        _ => return None,
    })
}

/// Die Richtung, die zu einem Versatz passt (nur direkte Nachbarn).
pub fn richtung_fuer(dx: i32, dy: i32, dz: i32) -> Option<&'static str> {
    RICHTUNGEN.iter().copied().find(|r| versatz(r) == Some((dx, dy, dz)))
}

/// Die Richtung, die grob zu einem beliebigen Versatz auf derselben Ebene passt.
pub fn richtung_ungefaehr(dx: i32, dy: i32) -> Option<&'static str> {
    if dx == 0 && dy == 0 {
        return None;
    }
    richtung_fuer(dx.signum(), dy.signum(), 0)
}

pub fn id_gueltig(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

pub const FARBEN: [&str; 16] = [
    "schwarz", "rot", "gruen", "gelb", "blau", "magenta", "cyan", "weiss", "grau", "hellrot", "hellgruen",
    "hellgelb", "hellblau", "hellmagenta", "hellcyan", "hellweiss",
];

// ---------------------------------------------------------------------------
// Modell
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Raum {
    pub id: String,
    pub name: String,
    pub beschreibung: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub gebiet: String,
    pub gelaende: String,
    pub symbol: String,
    pub farbe: String,
    pub klammern: String,
    pub karte: String,
    pub geruch: String,
    pub geraeusch: String,
    pub gefuehl: String,
    pub nutzt: Vec<String>,
    pub gegenstaende: Vec<String>,
    pub monster: Vec<String>,
    pub npcs: Vec<String>,
    pub ausgaenge: BTreeMap<String, String>,
    /// Datei relativ zum Datenordner, z.B. "raeume/dorf.toml"
    pub datei: String,
    /// Die ID beim Laden (None = neu im Editor angelegt)
    pub herkunft: Option<String>,
    /// Anzahl der Details (werden nicht im Editor bearbeitet, bleiben aber erhalten)
    pub details: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Gelaende {
    pub name: String,
    pub symbol: String,
    pub farbe: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Problem {
    pub fehler: bool,
    pub raum: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct OwFeld {
    pub name: String,
    pub symbol: char,
    pub farbe: String,
    pub begehbar: bool,
}

#[derive(Debug, Clone)]
pub struct OwPunkt {
    pub x: i32,
    pub y: i32,
    pub name: String,
    pub symbol: String,
    pub farbe: String,
    /// Nur bei Eingängen: der Raum, in den man kommt
    pub raum: String,
}

/// Die Oberwelt aus oberwelt/oberwelt.toml und dem Kartenbild (karte.txt).
/// Koordinaten wie in der Engine: x = Spalte, y = Zeile, beide ab 1.
#[derive(Debug, Clone, Default)]
pub struct Oberwelt {
    pub name: String,
    pub gitter: Vec<Vec<char>>,
    pub felder: BTreeMap<char, OwFeld>,
    pub eingaenge: Vec<OwPunkt>,
    pub orte: Vec<OwPunkt>,
}

impl Oberwelt {
    pub fn zeichen(&self, x: i32, y: i32) -> Option<char> {
        if x < 1 || y < 1 {
            return None;
        }
        self.gitter.get(y as usize - 1)?.get(x as usize - 1).copied().filter(|c| *c != ' ')
    }
    pub fn begehbar(&self, x: i32, y: i32) -> bool {
        self.zeichen(x, y).is_some_and(|c| self.felder.get(&c).is_none_or(|f| f.begehbar))
    }
    pub fn breite(&self) -> i32 {
        self.gitter.iter().map(|z| z.len()).max().unwrap_or(0) as i32
    }
    pub fn hoehe(&self) -> i32 {
        self.gitter.len() as i32
    }
}

pub struct Welt {
    pub ordner: PathBuf,
    pub raeume: Vec<Raum>,
    pub startraum: String,
    /// [kampf] wiederbelebung_raum: hier erwacht man nach dem Tod (leer = Startraum)
    pub wiederbelebung: String,
    wiederbelebung_geladen: String,
    pub beidseitig: bool,
    pub gelaende: BTreeMap<String, Gelaende>,
    pub gegenstaende: BTreeSet<String>,
    pub monster: BTreeSet<String>,
    pub npcs: BTreeSet<String>,
    pub bibliothek: BTreeSet<String>,
    /// Räume, die über einen Oberwelt-Eingang den Ausgang "raus" bekommen: Raum -> Name des Eingangs
    pub eingaenge: BTreeMap<String, String>,
    /// Alle Inhaltsdateien (relativ), in denen Räume stehen dürfen
    pub dateien: Vec<String>,
    docs: BTreeMap<String, DocumentMut>,
    welt_doc: DocumentMut,
    oberwelt_doc: Option<DocumentMut>,
    pub oberwelt: Option<Oberwelt>,
    startraum_geladen: String,
    /// Original-Tabellen der Räume: herkunft -> (Datei, Tabelle)
    tabellen: HashMap<String, (String, Table)>,
}

pub(crate) fn s(t: &Table, k: &str) -> String {
    t.get(k).and_then(|i| i.as_str()).unwrap_or_default().to_string()
}
fn n(t: &Table, k: &str) -> i32 {
    t.get(k).and_then(|i| i.as_integer()).unwrap_or(0) as i32
}
pub(crate) fn liste(t: &Table, k: &str) -> Vec<String> {
    t.get(k)
        .and_then(|i| i.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default()
}
fn tabelle_als_map(i: Option<&Item>) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    if let Some(t) = i.and_then(|i| i.as_table_like()) {
        for (k, v) in t.iter() {
            if let Some(v) = v.as_str() {
                m.insert(k.to_string(), v.to_string());
            }
        }
    }
    m
}

pub(crate) fn dateien_sammeln(ordner: &Path, basis: &Path, liste: &mut Vec<String>) {
    let Ok(eintraege) = fs::read_dir(ordner) else { return };
    for e in eintraege.flatten() {
        let p = e.path();
        if p.is_dir() {
            dateien_sammeln(&p, basis, liste);
        } else if p.extension().is_some_and(|x| x == "toml") {
            let rel = p.strip_prefix(basis).unwrap_or(&p).to_string_lossy().replace('\\', "/");
            if rel != "welt.toml" && rel != "oberwelt/oberwelt.toml" && rel != "befehle.toml" && !rel.starts_with("befehle/") {
                liste.push(rel);
            }
        }
    }
}

fn ids_aus(doc: &DocumentMut, art: &str) -> Vec<String> {
    let echt = |t: &&Table| t.get("vorlage").and_then(|v| v.as_bool()) != Some(true);
    match doc.get(art) {
        Some(Item::ArrayOfTables(a)) => a.iter().filter(echt).map(|t| s(t, "id")).filter(|i| !i.is_empty()).collect(),
        Some(Item::Table(t)) => Some(t).filter(echt).map(|t| s(t, "id")).into_iter().filter(|i| !i.is_empty()).collect(),
        _ => Vec::new(),
    }
}

impl Welt {
    pub fn laden(ordner: &Path) -> Result<Welt, String> {
        let welt_pfad = ordner.join("welt.toml");
        let text = fs::read_to_string(&welt_pfad)
            .map_err(|e| format!("Kann '{}' nicht lesen: {e}", welt_pfad.display()))?;
        let welt_doc: DocumentMut = text.parse().map_err(|e| format!("Fehler in welt.toml:\n{e}"))?;

        let mut startraum = String::new();
        let mut beidseitig = true;
        if let Some(w) = welt_doc.get("welt").and_then(|i| i.as_table()) {
            startraum = s(w, "startraum");
            beidseitig = w.get("ausgaenge_beidseitig").and_then(|i| i.as_bool()).unwrap_or(true);
        }
        let wiederbelebung = welt_doc
            .get("kampf")
            .and_then(|i| i.as_table())
            .map(|k| s(k, "wiederbelebung_raum"))
            .unwrap_or_default();
        let mut gelaende = BTreeMap::new();
        if let Some(g) = welt_doc.get("gelaende").and_then(|i| i.as_table()) {
            for (id, item) in g.iter() {
                if let Some(t) = item.as_table() {
                    gelaende.insert(
                        id.to_string(),
                        Gelaende { name: s(t, "name"), symbol: s(t, "symbol"), farbe: s(t, "farbe") },
                    );
                }
            }
        }

        let mut dateien = Vec::new();
        dateien_sammeln(ordner, ordner, &mut dateien);
        dateien.sort();

        let mut w = Welt {
            ordner: ordner.to_path_buf(),
            raeume: Vec::new(),
            startraum: startraum.clone(),
            wiederbelebung: wiederbelebung.clone(),
            wiederbelebung_geladen: wiederbelebung,
            beidseitig,
            gelaende,
            gegenstaende: BTreeSet::new(),
            monster: BTreeSet::new(),
            npcs: BTreeSet::new(),
            bibliothek: BTreeSet::new(),
            eingaenge: BTreeMap::new(),
            dateien: Vec::new(),
            docs: BTreeMap::new(),
            welt_doc,
            oberwelt_doc: None,
            oberwelt: None,
            startraum_geladen: startraum,
            tabellen: HashMap::new(),
        };

        let mut fehler = Vec::new();
        for rel in dateien {
            let text = match fs::read_to_string(ordner.join(&rel)) {
                Ok(t) => t,
                Err(e) => {
                    fehler.push(format!("Kann '{rel}' nicht lesen: {e}"));
                    continue;
                }
            };
            let doc: DocumentMut = match text.parse() {
                Ok(d) => d,
                Err(e) => {
                    fehler.push(format!("Fehler in '{rel}':\n{e}"));
                    continue;
                }
            };
            w.gegenstaende.extend(ids_aus(&doc, "gegenstand"));
            w.monster.extend(ids_aus(&doc, "monster"));
            w.npcs.extend(ids_aus(&doc, "npc"));
            w.bibliothek.extend(ids_aus(&doc, "beschreibung"));
            if let Some(Item::ArrayOfTables(a)) = doc.get("raum") {
                for t in a.iter() {
                    let id = s(t, "id");
                    let r = Raum {
                        id: id.clone(),
                        name: s(t, "name"),
                        beschreibung: s(t, "beschreibung"),
                        x: n(t, "x"),
                        y: n(t, "y"),
                        z: n(t, "z"),
                        gebiet: s(t, "gebiet"),
                        gelaende: s(t, "gelaende"),
                        symbol: s(t, "symbol"),
                        farbe: s(t, "farbe"),
                        klammern: s(t, "klammern"),
                        karte: s(t, "karte"),
                        geruch: s(t, "geruch"),
                        geraeusch: s(t, "geraeusch"),
                        gefuehl: s(t, "gefuehl"),
                        nutzt: liste(t, "nutzt"),
                        gegenstaende: liste(t, "gegenstaende"),
                        monster: liste(t, "monster"),
                        npcs: liste(t, "npcs"),
                        ausgaenge: tabelle_als_map(t.get("ausgaenge")),
                        datei: rel.clone(),
                        herkunft: Some(id.clone()),
                        details: t.get("details").and_then(|d| d.as_table_like()).map_or(0, |d| d.len()),
                    };
                    if w.tabellen.contains_key(&id) {
                        fehler.push(format!("Raum '{id}' gibt es doppelt ({rel}). Bitte erst von Hand beheben."));
                    }
                    w.tabellen.insert(id, (rel.clone(), t.clone()));
                    w.raeume.push(r);
                }
            }
            w.dateien.push(rel.clone());
            w.docs.insert(rel, doc);
        }

        let ow = ordner.join("oberwelt/oberwelt.toml");
        if ow.exists() {
            match fs::read_to_string(&ow).map_err(|e| e.to_string()).and_then(|t| {
                t.parse::<DocumentMut>().map_err(|e| e.to_string())
            }) {
                Ok(doc) => {
                    if let Some(Item::ArrayOfTables(a)) = doc.get("eingang") {
                        for t in a.iter() {
                            w.eingaenge.insert(s(t, "raum"), s(t, "name"));
                        }
                    }
                    match oberwelt_lesen(&doc, &ordner.join("oberwelt")) {
                        Ok(o) => w.oberwelt = Some(o),
                        Err(e) => fehler.push(e),
                    }
                    w.oberwelt_doc = Some(doc);
                }
                Err(e) => fehler.push(format!("Fehler in oberwelt/oberwelt.toml:\n{e}")),
            }
        }

        if fehler.is_empty() {
            Ok(w)
        } else {
            Err(fehler.join("\n\n"))
        }
    }

    pub fn index(&self, id: &str) -> Option<usize> {
        self.raeume.iter().position(|r| r.id == id)
    }

    pub fn raum(&self, id: &str) -> Option<&Raum> {
        self.raeume.iter().find(|r| r.id == id)
    }

    /// Ist das ein gültiges Ziel für einen Ausgang? Ein Raum oder ein begehbares Oberwelt-Feld wie "ow:36:11".
    pub fn ziel_gueltig(&self, ziel: &str) -> bool {
        self.index(ziel).is_some()
            || match (oberwelt_feld(ziel), &self.oberwelt) {
                (Some((x, y)), Some(o)) => o.begehbar(x, y),
                _ => false,
            }
    }

    pub fn raum_bei(&self, karte: &str, x: i32, y: i32, z: i32) -> Option<usize> {
        self.raeume.iter().position(|r| r.karte == karte && r.x == x && r.y == y && r.z == z)
    }

    pub fn karten(&self) -> Vec<String> {
        let mut k: BTreeSet<String> = self.raeume.iter().map(|r| r.karte.clone()).collect();
        k.insert(String::new());
        k.into_iter().collect()
    }

    pub fn ebenen(&self, karte: &str) -> Vec<i32> {
        let mut e: BTreeSet<i32> = self.raeume.iter().filter(|r| r.karte == karte).map(|r| r.z).collect();
        e.insert(0);
        e.into_iter().rev().collect()
    }

    pub fn gebiete(&self) -> Vec<String> {
        let e: BTreeSet<String> = self.raeume.iter().map(|r| r.gebiet.clone()).filter(|g| !g.is_empty()).collect();
        e.into_iter().collect()
    }

    /// Eine noch freie ID, z.B. "dorf_3".
    pub fn neue_id(&self, basis: &str) -> String {
        let basis = if id_gueltig(basis) { basis } else { "raum" };
        (1..).map(|i| format!("{basis}_{i}")).find(|id| self.index(id).is_none()).unwrap()
    }

    /// Alle Ausgänge eines Raums, wie die Engine sie sieht: eigene, Rückwege und "raus" zur Oberwelt.
    /// Rückgabe: Richtung -> (Ziel, automatisch?)
    pub fn alle_ausgaenge(&self, id: &str) -> BTreeMap<String, (String, bool)> {
        let mut m: BTreeMap<String, (String, bool)> = BTreeMap::new();
        if let Some(r) = self.raum(id) {
            for (ri, z) in &r.ausgaenge {
                m.insert(ri.clone(), (z.clone(), false));
            }
        }
        if let Some(name) = self.eingaenge.get(id) {
            m.entry("raus".into()).or_insert((format!("Oberwelt: {name}"), true));
        }
        if self.beidseitig {
            for r in &self.raeume {
                for (ri, z) in &r.ausgaenge {
                    if z == id {
                        m.entry(gegenteil(ri).to_string()).or_insert((r.id.clone(), true));
                    }
                }
            }
        }
        m
    }

    /// Einen Ausgang anlegen. Bei `beidseitig = false` auch den Rückweg eintragen.
    pub fn verbinden(&mut self, von: &str, richtung: &str, nach: &str) {
        if let Some(i) = self.index(von) {
            self.raeume[i].ausgaenge.insert(richtung.into(), nach.into());
        }
        if !self.beidseitig {
            if let Some(i) = self.index(nach) {
                self.raeume[i].ausgaenge.entry(gegenteil(richtung).into()).or_insert(von.into());
            }
        }
    }

    /// Verbindung zwischen zwei Räumen in beide Richtungen entfernen.
    pub fn trennen(&mut self, a: &str, b: &str) {
        for r in &mut self.raeume {
            if r.id == a {
                r.ausgaenge.retain(|_, z| z != b);
            } else if r.id == b {
                r.ausgaenge.retain(|_, z| z != a);
            }
        }
    }

    /// Raum umbenennen und alle Verweise mitziehen.
    pub fn umbenennen(&mut self, alt: &str, neu: &str) -> Result<(), String> {
        if alt == neu {
            return Ok(());
        }
        if !id_gueltig(neu) {
            return Err("Eine ID darf nur aus Kleinbuchstaben (ohne Umlaute), Ziffern und _ bestehen.".into());
        }
        if self.index(neu).is_some() {
            return Err(format!("Die ID '{neu}' gibt es schon."));
        }
        for r in &mut self.raeume {
            if r.id == alt {
                r.id = neu.into();
            }
            for z in r.ausgaenge.values_mut() {
                if z == alt {
                    *z = neu.into();
                }
            }
        }
        if self.startraum == alt {
            self.startraum = neu.into();
        }
        if self.wiederbelebung == alt {
            self.wiederbelebung = neu.into();
        }
        if let Some(n) = self.eingaenge.remove(alt) {
            self.eingaenge.insert(neu.into(), n);
        }
        Ok(())
    }

    pub fn loeschen(&mut self, id: &str) {
        self.raeume.retain(|r| r.id != id);
        for r in &mut self.raeume {
            r.ausgaenge.retain(|_, z| z != id);
        }
    }

    // -----------------------------------------------------------------------
    // Prüfen
    // -----------------------------------------------------------------------

    pub fn pruefen(&self) -> Vec<Problem> {
        let mut p = Vec::new();
        let mut fehler = |raum: &str, text: String| {
            p.push(Problem { fehler: true, raum: Some(raum.to_string()), text })
        };
        let mut warnungen: Vec<Problem> = Vec::new();
        let mut gesehen = BTreeSet::new();
        let mut orte: HashMap<(&str, i32, i32, i32), &str> = HashMap::new();
        for r in &self.raeume {
            if !id_gueltig(&r.id) {
                fehler(&r.id, format!("'{}': ungültige ID (nur a-z, 0-9 und _).", r.id));
            }
            if !gesehen.insert(r.id.as_str()) {
                fehler(&r.id, format!("Raum '{}' gibt es doppelt.", r.id));
            }
            if r.name.trim().is_empty() {
                fehler(&r.id, format!("Raum '{}' hat keinen Namen.", r.id));
            }
            if let Some(anderer) = orte.insert((r.karte.as_str(), r.x, r.y, r.z), &r.id) {
                warnungen.push(Problem {
                    fehler: false,
                    raum: Some(r.id.clone()),
                    text: format!(
                        "'{}' und '{anderer}' liegen auf demselben Feld ({}, {}, {}). Auf der Karte ist nur einer zu sehen.",
                        r.id, r.x, r.y, r.z
                    ),
                });
            }
            for (ri, z) in &r.ausgaenge {
                if !RICHTUNGEN.contains(&ri.as_str()) {
                    fehler(&r.id, format!("'{}': '{ri}' ist keine Richtung.", r.id));
                }
                if !self.ziel_gueltig(z) {
                    fehler(&r.id, if z.starts_with("ow:") {
                            format!("'{}': Ausgang {ri}: '{z}' ist kein begehbares Feld der Oberwelt.", r.id)
                        } else {
                            format!("'{}': Ausgang {ri} führt zu unbekanntem Raum '{z}'.", r.id)
                        });
                }
            }
            if !r.gelaende.is_empty() && !self.gelaende.contains_key(&r.gelaende) {
                fehler(&r.id, format!("'{}': unbekanntes Gelände '{}'.", r.id, r.gelaende));
            }
            if r.symbol.chars().count() > 1 {
                fehler(&r.id, format!("'{}': 'symbol' muss genau ein Zeichen sein.", r.id));
            }
            if !r.klammern.is_empty() && r.klammern.chars().count() != 2 {
                fehler(&r.id, format!("'{}': 'klammern' müssen genau zwei Zeichen sein.", r.id));
            }
            if !r.farbe.is_empty() && !FARBEN.contains(&r.farbe.as_str()) {
                fehler(&r.id, format!("'{}': unbekannte Farbe '{}'.", r.id, r.farbe));
            }
            for g in &r.gegenstaende {
                if !self.gegenstaende.contains(g) {
                    fehler(&r.id, format!("'{}': unbekannter Gegenstand '{g}'.", r.id));
                }
            }
            for m in &r.monster {
                if !self.monster.contains(m) {
                    fehler(&r.id, format!("'{}': unbekanntes Monster '{m}'.", r.id));
                }
            }
            for m in &r.npcs {
                if !self.npcs.contains(m) && !self.monster.contains(m) {
                    fehler(&r.id, format!("'{}': unbekannter NPC '{m}'.", r.id));
                }
            }
        }
        p.extend(warnungen);
        if self.startraum.is_empty() || !self.ziel_gueltig(&self.startraum) {
            p.push(Problem {
                fehler: true,
                raum: None,
                text: format!("Der Startraum '{}' existiert nicht.", self.startraum),
            });
        }
        if !self.wiederbelebung.is_empty() && !self.ziel_gueltig(&self.wiederbelebung) {
            p.push(Problem {
                fehler: true,
                raum: None,
                text: format!(
                    "Der Wiederbelebungsort '{}' ([kampf] wiederbelebung_raum) existiert nicht.",
                    self.wiederbelebung
                ),
            });
        }
        for (raum, name) in &self.eingaenge {
            if self.index(raum).is_none() {
                p.push(Problem {
                    fehler: true,
                    raum: None,
                    text: format!("Der Oberwelt-Eingang '{name}' führt zu '{raum}', den es nicht mehr gibt."),
                });
            }
        }
        for r in &self.raeume {
            if r.beschreibung.trim().is_empty() {
                p.push(Problem { fehler: false, raum: Some(r.id.clone()), text: format!("'{}' hat keine Beschreibung.", r.id) });
            }
            // Wie die Engine: Ausgänge müssen ins Nachbarfeld führen (gilt auch für Rückwege)
            for (ri, (ziel, auto)) in self.alle_ausgaenge(&r.id) {
                if !passt_zur_lage(self, r, &ri, &ziel) {
                    let z = self.raum(&ziel).unwrap();
                    let (dx, dy, dz) = versatz(&ri).unwrap();
                    p.push(Problem {
                        fehler: false,
                        raum: Some(r.id.clone()),
                        text: format!(
                            "'{}': {}Ausgang {ri} führt zu '{ziel}' ({}, {}, {}), erwartet wird ({}, {}, {}). Die Verbindung fehlt auf der Karte im Spiel.",
                            r.id,
                            if auto { "automatischer " } else { "" },
                            z.x, z.y, z.z, r.x + dx, r.y + dy, r.z + dz
                        ),
                    });
                }
            }
            if r.ausgaenge.is_empty() && self.alle_ausgaenge(&r.id).is_empty() {
                p.push(Problem { fehler: false, raum: Some(r.id.clone()), text: format!("'{}' hat keine Ausgänge.", r.id) });
            }
        }
        // Erreichbarkeit vom Startraum (Oberwelt-Eingänge zählen als erreichbar)
        if self.index(&self.startraum).is_some() {
            let mut erreicht: BTreeSet<String> = BTreeSet::new();
            let mut offen: Vec<String> = vec![self.startraum.clone()];
            offen.extend(self.eingaenge.keys().cloned());
            while let Some(id) = offen.pop() {
                if !erreicht.insert(id.clone()) {
                    continue;
                }
                for (_, (z, _)) in self.alle_ausgaenge(&id) {
                    if self.index(&z).is_some() && !erreicht.contains(&z) {
                        offen.push(z);
                    }
                }
            }
            for r in &self.raeume {
                if !erreicht.contains(&r.id) {
                    p.push(Problem {
                        fehler: false,
                        raum: Some(r.id.clone()),
                        text: format!("'{}' ist vom Startraum aus nicht erreichbar.", r.id),
                    });
                }
            }
        }
        p
    }

    // -----------------------------------------------------------------------
    // Speichern
    // -----------------------------------------------------------------------

    /// Schreibt alle geänderten Dateien. Gibt die Liste der geschriebenen Dateien zurück.
    pub fn speichern(&mut self) -> Result<Vec<String>, String> {
        let probleme: Vec<_> = self.raeume.iter().filter(|r| !id_gueltig(&r.id)).map(|r| r.id.clone()).collect();
        if !probleme.is_empty() {
            return Err(format!("Ungültige IDs: {}", probleme.join(", ")));
        }
        let umbenannt: HashMap<String, String> = self
            .raeume
            .iter()
            .filter_map(|r| r.herkunft.as_ref().filter(|h| **h != r.id).map(|h| (h.clone(), r.id.clone())))
            .collect();

        let mut geschrieben = Vec::new();
        let mut neue_docs: BTreeMap<String, DocumentMut> = BTreeMap::new();

        // Räume nach Datei sortiert (Reihenfolge innerhalb der Datei bleibt erhalten)
        let mut nach_datei: BTreeMap<String, Vec<&Raum>> = BTreeMap::new();
        for r in &self.raeume {
            nach_datei.entry(r.datei.clone()).or_default().push(r);
        }
        let mut alle: BTreeSet<String> = self.docs.keys().cloned().collect();
        alle.extend(nach_datei.keys().cloned());

        for datei in alle {
            let alt = self.docs.get(&datei);
            let mut doc = alt.cloned().unwrap_or_default();
            let raeume = nach_datei.get(&datei).cloned().unwrap_or_default();
            // Kommentare vor gelöschten oder verschobenen Räumen retten
            let alte: Vec<Table> = match doc.get("raum") {
                Some(Item::ArrayOfTables(a)) => a.iter().cloned().collect(),
                _ => Vec::new(),
            };
            let bleibt: Vec<bool> = alte
                .iter()
                .map(|t| raeume.iter().any(|r| r.herkunft.as_deref() == Some(s(t, "id").as_str())))
                .collect();
            let (extra, rest) = kommentare_umverteilen(&alte, &bleibt);
            let mut aot = ArrayOfTables::new();
            for r in &raeume {
                let mut t = match r.herkunft.as_ref().and_then(|h| self.tabellen.get(h)) {
                    Some((quelle, t)) => {
                        let mut t = t.clone();
                        if *quelle != datei {
                            // In eine andere Datei verschoben: hinten anhängen
                            positionen_loeschen(&mut t);
                        }
                        t
                    }
                    None => neue_tabelle(),
                };
                raum_schreiben(&mut t, r);
                if let Some(p) = alte.iter().position(|a| r.herkunft.as_deref() == Some(s(a, "id").as_str())).and_then(|i| extra.get(&i)) {
                    praefix_voranstellen(&mut t, p);
                }
                aot.push(t);
            }
            kommentar_ans_ende(&mut doc, &rest);
            let hatte = matches!(doc.get("raum"), Some(Item::ArrayOfTables(_)));
            if aot.is_empty() {
                if hatte {
                    doc.remove("raum");
                }
            } else {
                doc.insert("raum", Item::ArrayOfTables(aot));
            }
            if datei_umbenennen_teleport(&mut doc, &umbenannt) || alt.map(|a| a.to_string()) != Some(doc.to_string()) {
                neue_docs.insert(datei, doc);
            }
        }

        // welt.toml: Startraum und Wiederbelebungsort
        let mut welt_neu = None;
        if self.startraum != self.startraum_geladen || self.wiederbelebung != self.wiederbelebung_geladen {
            let mut d = self.welt_doc.clone();
            if let Some(w) = d.get_mut("welt").and_then(|i| i.as_table_mut()) {
                wert_setzen(w, "startraum", Value::from(self.startraum.as_str()));
            }
            if self.wiederbelebung != self.wiederbelebung_geladen {
                if d.get("kampf").is_none() && !self.wiederbelebung.is_empty() {
                    d.insert("kampf", Item::Table(neue_tabelle()));
                }
                if let Some(k) = d.get_mut("kampf").and_then(|i| i.as_table_mut()) {
                    if self.wiederbelebung.is_empty() {
                        k.remove("wiederbelebung_raum");
                    } else {
                        wert_setzen(k, "wiederbelebung_raum", Value::from(self.wiederbelebung.as_str()));
                    }
                }
            }
            welt_neu = Some(d);
        }
        // oberwelt.toml: Eingänge zu umbenannten Räumen
        let mut ow_neu = None;
        if let Some(d) = &self.oberwelt_doc {
            let mut d = d.clone();
            let mut geaendert = false;
            if let Some(Item::ArrayOfTables(a)) = d.get_mut("eingang") {
                for t in a.iter_mut() {
                    if let Some(neu) = umbenannt.get(&s(t, "raum")) {
                        wert_setzen(t, "raum", Value::from(neu.as_str()));
                        geaendert = true;
                    }
                }
            }
            if geaendert {
                ow_neu = Some(d);
            }
        }

        // Schreiben
        let schreibe = |rel: &str, text: String| -> Result<(), String> {
            let pfad = self.ordner.join(rel);
            if let Some(o) = pfad.parent() {
                fs::create_dir_all(o).map_err(|e| format!("{rel}: {e}"))?;
            }
            fs::write(&pfad, text).map_err(|e| format!("{rel}: {e}"))
        };
        for (rel, doc) in &neue_docs {
            let mut text = doc.to_string();
            if !text.ends_with('\n') {
                text.push('\n');
            }
            schreibe(rel, text)?;
            geschrieben.push(rel.clone());
        }
        if let Some(d) = &welt_neu {
            schreibe("welt.toml", d.to_string())?;
            geschrieben.push("welt.toml".into());
        }
        if let Some(d) = &ow_neu {
            schreibe("oberwelt/oberwelt.toml", d.to_string())?;
            geschrieben.push("oberwelt/oberwelt.toml".into());
        }
        Ok(geschrieben)
    }

    /// Gibt es Änderungen gegenüber den Dateien?
    pub fn geaendert(&self, geladen: &[Raum], start: &str, wiederbelebung: &str) -> bool {
        self.raeume != geladen || self.startraum != start || self.wiederbelebung != wiederbelebung
    }
}

fn oberwelt_lesen(doc: &DocumentMut, ordner: &Path) -> Result<Oberwelt, String> {
    let kopf = doc.get("oberwelt").and_then(|i| i.as_table());
    let datei = kopf.map(|t| s(t, "karte")).filter(|k| !k.is_empty()).unwrap_or_else(|| "karte.txt".into());
    let pfad = ordner.join(&datei);
    let bild = fs::read_to_string(&pfad).map_err(|e| format!("Kann 'oberwelt/{datei}' nicht lesen: {e}"))?;
    let gitter = bild.lines().map(|z| z.trim_end_matches('\r').chars().collect()).collect();
    let mut felder = BTreeMap::new();
    if let Some(f) = doc.get("feld").and_then(|i| i.as_table()) {
        for (k, item) in f.iter() {
            let (Some(c), Some(t)) = (k.chars().next(), item.as_table()) else { continue };
            felder.insert(
                c,
                OwFeld {
                    name: s(t, "name"),
                    symbol: s(t, "symbol").chars().next().unwrap_or(c),
                    farbe: s(t, "farbe"),
                    begehbar: t.get("begehbar").and_then(|i| i.as_bool()).unwrap_or(true),
                },
            );
        }
    }
    let punkte = |art: &str| -> Vec<OwPunkt> {
        match doc.get(art) {
            Some(Item::ArrayOfTables(a)) => a
                .iter()
                .map(|t| OwPunkt {
                    x: n(t, "x"),
                    y: n(t, "y"),
                    name: s(t, "name"),
                    symbol: s(t, "symbol"),
                    farbe: s(t, "farbe"),
                    raum: s(t, "raum"),
                })
                .collect(),
            _ => Vec::new(),
        }
    };
    Ok(Oberwelt {
        name: kopf.map(|t| s(t, "name")).unwrap_or_default(),
        gitter,
        felder,
        eingaenge: punkte("eingang"),
        orte: punkte("ort"),
    })
}

/// "ow:36:11" -> (36, 11)
pub fn oberwelt_feld(id: &str) -> Option<(i32, i32)> {
    let rest = id.strip_prefix("ow:")?;
    let (x, y) = rest.split_once(':')?;
    Some((x.parse().ok()?, y.parse().ok()?))
}

/// Liegt das Ziel eines Ausgangs dort, wo die Richtung hinzeigt? (rein/raus und andere Karten zählen nicht)
pub fn passt_zur_lage(w: &Welt, r: &Raum, richtung: &str, ziel: &str) -> bool {
    let (Some((dx, dy, dz)), Some(z)) = (versatz(richtung), w.raum(ziel)) else { return true };
    z.karte != r.karte || (z.x, z.y, z.z) == (r.x + dx, r.y + dy, r.z + dz)
}

/// Teleport-Zauber, die auf umbenannte Räume zeigen, mitziehen.
fn datei_umbenennen_teleport(doc: &mut DocumentMut, umbenannt: &HashMap<String, String>) -> bool {
    fn durchsuchen(t: &mut dyn toml_edit::TableLike, umbenannt: &HashMap<String, String>) -> bool {
        let mut geaendert = false;
        let ist_teleport = t.get("art").and_then(|i| i.as_str()) == Some("teleport");
        if ist_teleport {
            let raum = t.get("raum").and_then(|i| i.as_str()).map(String::from);
            if let Some(neu) = raum.and_then(|r| umbenannt.get(&r)) {
                if let Some(Item::Value(v)) = t.get_mut("raum") {
                    let decor = v.decor().clone();
                    *v = Value::from(neu.as_str());
                    *v.decor_mut() = decor;
                    geaendert = true;
                }
            }
        }
        let keys: Vec<String> = t.iter().map(|(k, _)| k.to_string()).collect();
        for k in keys {
            match t.get_mut(&k) {
                Some(Item::Table(st)) => geaendert |= durchsuchen(st, umbenannt),
                Some(Item::ArrayOfTables(a)) => {
                    for st in a.iter_mut() {
                        geaendert |= durchsuchen(st, umbenannt);
                    }
                }
                Some(Item::Value(Value::InlineTable(it))) => geaendert |= durchsuchen(it, umbenannt),
                Some(Item::Value(Value::Array(arr))) => {
                    for v in arr.iter_mut() {
                        if let Value::InlineTable(it) = v {
                            geaendert |= durchsuchen(it, umbenannt);
                        }
                    }
                }
                _ => {}
            }
        }
        geaendert
    }
    if umbenannt.is_empty() {
        return false;
    }
    let Some(Item::ArrayOfTables(a)) = doc.get_mut("zauber") else { return false };
    let mut g = false;
    for t in a.iter_mut() {
        g |= durchsuchen(t, umbenannt);
    }
    g
}

fn positionen_loeschen(t: &mut Table) {
    t.set_position(None);
    for (_, i) in t.iter_mut() {
        match i {
            Item::Table(st) => positionen_loeschen(st),
            Item::ArrayOfTables(a) => a.iter_mut().for_each(positionen_loeschen),
            _ => {}
        }
    }
}

/// Kommentare stehen in toml_edit VOR einer Tabelle als deren "Präfix".
/// Wird eine Tabelle gelöscht, geben wir ihren Kommentar an die nächste weiter.
pub(crate) fn praefix(t: &Table) -> String {
    t.decor().prefix().and_then(|p| p.as_str()).unwrap_or("").to_string()
}

pub(crate) fn praefix_voranstellen(t: &mut Table, p: &str) {
    if p.contains('#') {
        let alt = praefix(t);
        t.decor_mut().set_prefix(format!("{}{}", p.trim_end_matches('\n'), if alt.starts_with('\n') { alt } else { format!("\n{alt}") }));
    }
}

/// Nicht mehr verwendete Kommentare ans Dateiende hängen, statt sie zu verlieren.
pub(crate) fn kommentar_ans_ende(doc: &mut DocumentMut, p: &str) {
    if p.contains('#') {
        let alt = doc.trailing().as_str().unwrap_or("").to_string();
        doc.set_trailing(format!("{alt}{}\n", p.trim_end()));
    }
}

/// Neue Liste aus alten Tabellen: `nehmen[i]` sagt, welche bleiben (und in welcher
/// Reihenfolge sie erscheinen). Kommentare gelöschter Tabellen wandern zur nächsten
/// bleibenden Tabelle. Rückgabe: (alte Position -> zusätzlicher Präfix, Rest).
pub(crate) fn kommentare_umverteilen(alte: &[Table], bleibt: &[bool]) -> (HashMap<usize, String>, String) {
    let mut extra: HashMap<usize, String> = HashMap::new();
    let mut offen = String::new();
    for (i, t) in alte.iter().enumerate() {
        if bleibt.get(i).copied().unwrap_or(false) {
            if !offen.is_empty() {
                extra.insert(i, std::mem::take(&mut offen));
            }
        } else {
            let p = praefix(t);
            if p.contains('#') {
                offen.push_str(&p);
            }
        }
    }
    (extra, offen)
}

pub(crate) fn neue_tabelle() -> Table {
    let mut t = Table::new();
    t.decor_mut().set_prefix("\n");
    t
}

/// Setzt einen Wert und behält dabei Kommentare/Abstände des alten Werts.
pub(crate) fn wert_setzen(t: &mut Table, k: &str, mut neu: Value) {
    // Vorhandenen Wert an Ort und Stelle ersetzen: So bleiben auch die
    // Ausrichtung des Schlüssels ("startraum  = ...") und Kommentare erhalten.
    if let Some(Item::Value(alt)) = t.get_mut(k) {
        let decor = alt.decor().clone();
        *neu.decor_mut() = decor;
        *alt = neu;
        return;
    }
    t.insert(k, Item::Value(neu));
}

pub(crate) fn text_wert(v: &str) -> Value {
    if v.contains('\n') {
        // Mehrzeilig wie in den Beispieldateien: """\n...\"""
        let roh = format!("\"\"\"\n{}\"\"\"", v.replace('\\', "\\\\").replace("\"\"\"", "\\\"\\\"\\\""));
        if let Ok(val) = roh.parse::<Value>() {
            if val.as_str() == Some(v) {
                return val;
            }
        }
    }
    Value::from(v)
}

pub(crate) fn setze_text(t: &mut Table, k: &str, v: &str, pflicht: bool) {
    let alt = t.get(k).and_then(|i| i.as_str());
    if alt == Some(v) {
        return;
    }
    if v.is_empty() && !pflicht {
        t.remove(k);
        return;
    }
    wert_setzen(t, k, text_wert(v));
}

fn setze_zahl(t: &mut Table, k: &str, v: i32, pflicht: bool) {
    let alt = t.get(k).and_then(|i| i.as_integer());
    if alt == Some(v as i64) || (alt.is_none() && v == 0 && !pflicht) {
        return;
    }
    if v == 0 && !pflicht {
        t.remove(k);
        return;
    }
    wert_setzen(t, k, Value::from(v as i64));
}

pub(crate) fn setze_liste(t: &mut Table, k: &str, v: &[String]) {
    let alt: Option<Vec<String>> = t.get(k).and_then(|i| i.as_array()).map(|a| {
        a.iter().filter_map(|x| x.as_str().map(String::from)).collect()
    });
    if alt.as_deref() == Some(v) || (alt.is_none() && v.is_empty()) {
        return;
    }
    if v.is_empty() {
        t.remove(k);
        return;
    }
    let mut a = Array::new();
    for x in v {
        a.push(x.as_str());
    }
    wert_setzen(t, k, Value::Array(a));
}

fn setze_ausgaenge(t: &mut Table, v: &BTreeMap<String, String>) {
    let alt = tabelle_als_map(t.get("ausgaenge"));
    if &alt == v {
        return;
    }
    if v.is_empty() {
        t.remove("ausgaenge");
        return;
    }
    // Reihenfolge der vorhandenen Einträge beibehalten, neue in Richtungs-Reihenfolge anhängen
    let mut reihenfolge: Vec<String> = Vec::new();
    if let Some(tl) = t.get("ausgaenge").and_then(|i| i.as_table_like()) {
        reihenfolge.extend(tl.iter().map(|(k, _)| k.to_string()).filter(|k| v.contains_key(k)));
    }
    for r in RICHTUNGEN {
        if v.contains_key(r) && !reihenfolge.iter().any(|x| x == r) {
            reihenfolge.push(r.to_string());
        }
    }
    for k in v.keys() {
        if !reihenfolge.contains(k) {
            reihenfolge.push(k.clone());
        }
    }
    let mut it = InlineTable::new();
    for k in reihenfolge {
        it.insert(&k, Value::from(v[&k].as_str()));
    }
    it.fmt();
    wert_setzen(t, "ausgaenge", Value::InlineTable(it));
}

fn raum_schreiben(t: &mut Table, r: &Raum) {
    let neu = t.is_empty();
    setze_text(t, "id", &r.id, true);
    setze_text(t, "name", &r.name, true);
    if neu {
        // Gleiche Reihenfolge wie in den Beispieldateien
        setze_text(t, "symbol", &r.symbol, false);
        setze_text(t, "karte", &r.karte, false);
    }
    setze_zahl(t, "x", r.x, true);
    setze_zahl(t, "y", r.y, true);
    setze_zahl(t, "z", r.z, false);
    setze_text(t, "gebiet", &r.gebiet, false);
    setze_text(t, "gelaende", &r.gelaende, false);
    setze_text(t, "symbol", &r.symbol, false);
    setze_text(t, "farbe", &r.farbe, false);
    setze_text(t, "klammern", &r.klammern, false);
    setze_text(t, "karte", &r.karte, false);
    setze_text(t, "beschreibung", &r.beschreibung, false);
    setze_ausgaenge(t, &r.ausgaenge);
    setze_liste(t, "gegenstaende", &r.gegenstaende);
    setze_liste(t, "monster", &r.monster);
    setze_liste(t, "npcs", &r.npcs);
    setze_liste(t, "nutzt", &r.nutzt);
    setze_text(t, "geruch", &r.geruch, false);
    setze_text(t, "geraeusch", &r.geraeusch, false);
    setze_text(t, "gefuehl", &r.gefuehl, false);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn beispiel() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../daten")
    }

    fn kopie(name: &str) -> PathBuf {
        let ziel = std::env::temp_dir().join(format!("rmeditor_test_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&ziel);
        kopieren(&beispiel(), &ziel);
        ziel
    }

    fn kopieren(von: &Path, nach: &Path) {
        fs::create_dir_all(nach).unwrap();
        for e in fs::read_dir(von).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                kopieren(&p, &nach.join(e.file_name()));
            } else {
                fs::copy(&p, nach.join(e.file_name())).unwrap();
            }
        }
    }

    #[test]
    fn beispielwelt_laden_und_pruefen() {
        let w = Welt::laden(&beispiel()).unwrap();
        assert!(w.raum("dorfplatz").is_some());
        // Die Beispielwelt ist laut Engine fehlerfrei und ohne Warnungen
        let probleme = w.pruefen();
        assert!(probleme.is_empty(), "{probleme:?}");
    }

    #[test]
    fn ausgang_ins_falsche_feld_wird_gemeldet() {
        let mut w = Welt::laden(&beispiel()).unwrap();
        let i = w.index("schmiede").unwrap();
        w.raeume[i].x = -2;
        let p = w.pruefen();
        assert!(p.iter().any(|p| !p.fehler && p.text.contains("erwartet wird (-1, 0, 0)")), "{p:?}");
    }

    #[test]
    fn oberwelt_ziele_sind_erlaubt() {
        let mut w = Welt::laden(&beispiel()).unwrap();
        let i = w.index("nordtor").unwrap();
        w.raeume[i].ausgaenge.insert("raus".into(), "ow:36:11".into());
        assert!(w.pruefen().iter().all(|p| !p.fehler));
        let o = w.oberwelt.as_ref().unwrap();
        assert_eq!(o.zeichen(36, 11), Some('='));
        assert_eq!(o.eingaenge[0].name, "Eichenhain");
        assert!(!o.begehbar(1, 1)); // Meer
        w.raeume[i].ausgaenge.insert("raus".into(), "ow:1:1".into());
        assert!(w.pruefen().iter().any(|p| p.fehler));
        w.raeume[i].ausgaenge.insert("raus".into(), "ow:x".into());
        assert!(w.pruefen().iter().any(|p| p.fehler));
    }

    #[test]
    fn unveraendert_speichern_aendert_nichts() {
        let ordner = kopie("unveraendert");
        let mut w = Welt::laden(&ordner).unwrap();
        let geschrieben = w.speichern().unwrap();
        assert!(geschrieben.is_empty(), "{geschrieben:?}");
        let _ = fs::remove_dir_all(ordner);
    }

    #[test]
    fn aendern_umbenennen_und_neu() {
        let ordner = kopie("aendern");
        let mut w = Welt::laden(&ordner).unwrap();
        let i = w.index("hauptstrasse").unwrap();
        w.raeume[i].beschreibung = "Zeile eins\nZeile zwei".into();
        w.umbenennen("dorfplatz", "marktplatz").unwrap();
        let mut neu = w.raeume[i].clone();
        neu.id = "nebengasse".into();
        neu.herkunft = None;
        neu.x = 5;
        neu.ausgaenge.clear();
        w.raeume.push(neu);
        w.verbinden("nebengasse", "west", "hauptstrasse");
        let geschrieben = w.speichern().unwrap();
        assert!(geschrieben.contains(&"welt.toml".to_string()));

        let text = fs::read_to_string(ordner.join("raeume/dorf.toml")).unwrap();
        // Kommentare und Details bleiben erhalten
        assert!(text.contains("#  Das Dorf Eichenhain"));
        assert!(text.contains("[raum.details.brunnen.details.rand]"));
        assert!(text.contains("beschreibung = \"\"\"\nZeile eins\nZeile zwei\"\"\""));

        let w2 = Welt::laden(&ordner).unwrap();
        assert_eq!(w2.startraum, "marktplatz");
        // Der Tempel ist Wiederbelebungsort und hat sich nicht geändert
        assert_eq!(w2.wiederbelebung, "tempel");
        assert!(w2.raum("marktplatz").unwrap().details > 0);
        assert_eq!(w2.raum("nebengasse").unwrap().ausgaenge["west"], "hauptstrasse");
        assert_eq!(w2.raum("hauptstrasse").unwrap().beschreibung, "Zeile eins\nZeile zwei");
        let fehler: Vec<_> = w2.pruefen().into_iter().filter(|p| p.fehler).collect();
        assert!(fehler.is_empty(), "{fehler:?}");
        let _ = fs::remove_dir_all(ordner);
    }

    #[test]
    fn wiederbelebungsort_wird_mit_umbenannt() {
        let ordner = kopie("wiederbelebung");
        let mut w = Welt::laden(&ordner).unwrap();
        assert_eq!(w.wiederbelebung, "tempel");
        w.umbenennen("tempel", "kapelle").unwrap();
        assert!(w.pruefen().iter().all(|p| !p.fehler));
        w.speichern().unwrap();
        let text = fs::read_to_string(ordner.join("welt.toml")).unwrap();
        assert!(text.contains("wiederbelebung_raum       = \"kapelle\""), "{text}");
        // Kommentar hinter dem Wert bleibt erhalten
        assert!(text.contains("tod_wartezeit"));
        let w2 = Welt::laden(&ordner).unwrap();
        assert_eq!(w2.wiederbelebung, "kapelle");

        // Gelöschter Wiederbelebungsort ist ein Fehler
        let mut w3 = Welt::laden(&ordner).unwrap();
        w3.loeschen("kapelle");
        assert!(w3.pruefen().iter().any(|p| p.fehler && p.text.contains("Wiederbelebungsort")));
        let _ = fs::remove_dir_all(ordner);
    }

    #[test]
    fn ersten_raum_loeschen_behaelt_dateikommentar() {
        let ordner = kopie("loeschen");
        let mut w = Welt::laden(&ordner).unwrap();
        w.loeschen("dorfplatz");
        w.startraum = "hauptstrasse".into();
        w.speichern().unwrap();
        let text = fs::read_to_string(ordner.join("raeume/dorf.toml")).unwrap();
        assert!(text.contains("#  Das Dorf Eichenhain"), "{text}");
        assert!(!text.contains("id       = \"dorfplatz\""));
        let _ = fs::remove_dir_all(ordner);
    }
}
