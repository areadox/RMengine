//! Die Oberwelt: eine große Landkarte, die man einfach als Textbild zeichnet.
//!
//! In `daten/oberwelt/karte.txt` steht jedes Zeichen für ein Feld, zum Beispiel
//!
//!     ~~~^^^^WWWW,,,,
//!     ~~^^^WWW...,,,,
//!
//! und in `daten/oberwelt/oberwelt.toml` steht, was die Zeichen bedeuten.
//! Jedes Feld ist ein eigener, automatisch erzeugter Raum. Städte und Dungeons
//! sind "Eingänge", die man mit `rein` betritt und mit `raus` wieder verlässt.
//!
//! Koordinaten: x = Spalte, y = Zeile in der Textdatei, beide ab 1 gezählt.
//! Norden ist oben.

use crate::daten::{RaumDef, Richtung};
use crate::karte::Zelle;
use crate::text::{self, aufzaehlen};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::Path;

pub const PRAEFIX: &str = "ow:";

pub fn id(x: i32, y: i32) -> String {
    format!("{PRAEFIX}{x}:{y}")
}

pub fn aus_id(id: &str) -> Option<(i32, i32)> {
    let (x, y) = id.strip_prefix(PRAEFIX)?.split_once(':')?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
}

pub fn ist_oberwelt(id: &str) -> bool {
    id.starts_with(PRAEFIX)
}

/// Das Feld in Richtung `r` (Norden = eine Zeile nach oben).
pub fn nachbar(x: i32, y: i32, r: Richtung) -> (i32, i32) {
    let (dx, dy, _) = r.versatz();
    (x + dx, y - dy)
}

const ACHT: [Richtung; 8] = [
    Richtung::Nord,
    Richtung::Nordost,
    Richtung::Ost,
    Richtung::Suedost,
    Richtung::Sued,
    Richtung::Suedwest,
    Richtung::West,
    Richtung::Nordwest,
];

// ---------------------------------------------------------------------------
// Datenformat
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct OberweltConfig {
    pub name: String,
    pub karte: String,
    pub sicht: i32,
    pub anzeige_breite: i32,
    pub anzeige_hoehe: i32,
    pub gebiet: String,
    /// "kompakt" = ein Zeichen pro Feld im Rahmen, "weit" = mit Abständen
    pub stil: String,
}

impl Default for OberweltConfig {
    fn default() -> Self {
        OberweltConfig {
            name: "Oberwelt".into(),
            karte: "karte.txt".into(),
            sicht: 6,
            anzeige_breite: 8,
            anzeige_hoehe: 4,
            gebiet: "oberwelt".into(),
            stil: "kompakt".into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct FeldDef {
    pub name: String,
    /// Wie es aus der Ferne heißt: "Im Norden siehst du {fern}."
    pub fern: String,
    pub symbol: String,
    pub farbe: String,
    pub begehbar: bool,
    pub sicht: i32,
    /// Jedes Feld dieser Art, durch das man hindurchschaut, kostet so viel Sichtweite.
    pub sichtbehinderung: i32,
    pub blockiert_sicht: bool,
    /// Wer auf einem sichtblockierenden Feld steht (z.B. einem Berg),
    /// sieht über gleich hohe oder niedrigere Blocker hinweg.
    pub hoehe: i32,
    pub beschreibungen: Vec<String>,
}

impl Default for FeldDef {
    fn default() -> Self {
        FeldDef {
            name: String::new(),
            fern: String::new(),
            symbol: String::new(),
            farbe: String::new(),
            begehbar: true,
            sicht: 0,
            sichtbehinderung: 0,
            blockiert_sicht: false,
            hoehe: 0,
            beschreibungen: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EingangDef {
    pub x: i32,
    pub y: i32,
    pub raum: String,
    pub name: String,
    #[serde(default = "standard_eingang_symbol")]
    pub symbol: String,
    #[serde(default = "standard_eingang_farbe")]
    pub farbe: String,
    #[serde(default)]
    pub beschreibung: String,
}

fn standard_eingang_symbol() -> String {
    "O".into()
}
fn standard_eingang_farbe() -> String {
    "hellgelb".into()
}

/// Ein besonderes Feld mit eigenem Namen, Text, Gegenständen oder Monstern.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrtDef {
    pub x: i32,
    pub y: i32,
    pub name: String,
    #[serde(default)]
    pub beschreibung: String,
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub farbe: String,
    #[serde(default)]
    pub gegenstaende: Vec<String>,
    #[serde(default)]
    pub monster: Vec<String>,
    #[serde(default)]
    pub details: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OberweltDatei {
    #[serde(default)]
    oberwelt: OberweltConfig,
    #[serde(default)]
    feld: BTreeMap<String, FeldDef>,
    #[serde(default)]
    eingang: Vec<EingangDef>,
    #[serde(default)]
    ort: Vec<OrtDef>,
}

#[derive(Debug, Clone)]
pub struct Oberwelt {
    pub cfg: OberweltConfig,
    pub felder: HashMap<char, FeldDef>,
    pub gitter: Vec<Vec<char>>,
    pub eingaenge: Vec<EingangDef>,
    pub orte: Vec<OrtDef>,
    pub breite: i32,
    pub hoehe: i32,
}

// ---------------------------------------------------------------------------
// Laden und prüfen
// ---------------------------------------------------------------------------

/// Lädt `oberwelt/oberwelt.toml`. Gibt es die Datei nicht, hat die Welt keine Oberwelt.
pub fn laden(ordner: &Path, fehler: &mut Vec<String>) -> Option<Oberwelt> {
    let unterordner = ordner.join("oberwelt");
    let pfad = unterordner.join("oberwelt.toml");
    if !pfad.exists() {
        return None;
    }
    let inhalt = match fs::read_to_string(&pfad) {
        Ok(i) => i,
        Err(e) => {
            fehler.push(format!("Kann 'oberwelt/oberwelt.toml' nicht lesen: {e}"));
            return None;
        }
    };
    let datei: OberweltDatei = match toml::from_str(&inhalt) {
        Ok(d) => d,
        Err(e) => {
            fehler.push(format!("Fehler in 'oberwelt/oberwelt.toml':\n{}", crate::daten::toml_fehler_uebersetzen(&e)));
            return None;
        }
    };

    let karten_pfad = unterordner.join(&datei.oberwelt.karte);
    let bild = match fs::read_to_string(&karten_pfad) {
        Ok(b) => b,
        Err(e) => {
            fehler.push(format!("Kann die Karte 'oberwelt/{}' nicht lesen: {e}", datei.oberwelt.karte));
            return None;
        }
    };
    let gitter: Vec<Vec<char>> = bild.lines().map(|z| z.trim_end_matches('\r').chars().collect()).collect();
    let breite = gitter.iter().map(|z| z.len()).max().unwrap_or(0) as i32;
    let hoehe = gitter.len() as i32;
    let kartenname = format!("oberwelt/{}", datei.oberwelt.karte);

    // Zeichen -> Feld
    let mut felder = HashMap::new();
    for (zeichen, f) in datei.feld {
        let mut c = zeichen.chars();
        match (c.next(), c.next()) {
            (Some(z), None) => {
                let wo = format!("oberwelt.toml [feld.\"{zeichen}\"]");
                if f.name.is_empty() {
                    fehler.push(format!("{wo}: 'name' fehlt."));
                }
                if f.symbol.chars().count() > 1 {
                    fehler.push(format!("{wo}: 'symbol' muss genau ein Zeichen sein."));
                }
                if !f.farbe.is_empty() && !text::ist_farbe(&f.farbe) {
                    fehler.push(format!("{wo}: unbekannte Farbe '{}'.", f.farbe));
                }
                felder.insert(z, f);
            }
            _ => fehler.push(format!(
                "oberwelt.toml: [feld.\"{zeichen}\"] – ein Feld muss genau ein Zeichen sein."
            )),
        }
    }

    // Jedes Zeichen der Karte muss erklärt sein
    let mut gemeldet = BTreeSet::new();
    for (zeile, z) in gitter.iter().enumerate() {
        for (spalte, c) in z.iter().enumerate() {
            if *c != ' ' && !felder.contains_key(c) && gemeldet.insert(*c) {
                fehler.push(format!(
                    "{kartenname}, Zeile {}, Spalte {}: Das Zeichen '{c}' ist in oberwelt.toml nicht als [feld.\"{c}\"] erklärt.",
                    zeile + 1,
                    spalte + 1
                ));
            }
        }
    }

    let ow = Oberwelt {
        cfg: datei.oberwelt,
        felder,
        gitter,
        eingaenge: datei.eingang,
        orte: datei.ort,
        breite,
        hoehe,
    };

    if !["kompakt", "weit"].contains(&ow.cfg.stil.as_str()) {
        fehler.push("oberwelt.toml: 'stil' muss \"kompakt\" oder \"weit\" sein.".into());
    }
    let mut belegt = BTreeSet::new();
    for e in &ow.eingaenge {
        let wo = format!("oberwelt.toml, Eingang '{}' ({}, {})", e.name, e.x, e.y);
        if !ow.begehbar(e.x, e.y) {
            fehler.push(format!(
                "{wo}: An dieser Stelle der Karte ist kein begehbares Feld (Karte: {} Spalten x {} Zeilen).",
                ow.breite, ow.hoehe
            ));
        }
        if !belegt.insert((e.x, e.y)) {
            fehler.push(format!("{wo}: An dieser Stelle gibt es schon einen Eingang."));
        }
        if e.symbol.chars().count() != 1 {
            fehler.push(format!("{wo}: 'symbol' muss genau ein Zeichen sein."));
        }
    }
    for o in &ow.orte {
        if ow.feld(o.x, o.y).is_none() {
            fehler.push(format!(
                "oberwelt.toml, Ort '{}' ({}, {}): liegt außerhalb der Karte oder auf einem Leerzeichen.",
                o.name, o.x, o.y
            ));
        }
    }
    Some(ow)
}

// ---------------------------------------------------------------------------
// Abfragen
// ---------------------------------------------------------------------------

impl Oberwelt {
    pub fn zeichen(&self, x: i32, y: i32) -> Option<char> {
        if x < 1 || y < 1 {
            return None;
        }
        let c = *self.gitter.get((y - 1) as usize)?.get((x - 1) as usize)?;
        (c != ' ').then_some(c)
    }

    pub fn feld(&self, x: i32, y: i32) -> Option<&FeldDef> {
        self.felder.get(&self.zeichen(x, y)?)
    }

    pub fn begehbar(&self, x: i32, y: i32) -> bool {
        self.feld(x, y).is_some_and(|f| f.begehbar)
    }

    pub fn eingang_bei(&self, x: i32, y: i32) -> Option<&EingangDef> {
        self.eingaenge.iter().find(|e| e.x == x && e.y == y)
    }

    pub fn ort_bei(&self, x: i32, y: i32) -> Option<&OrtDef> {
        self.orte.iter().find(|o| o.x == x && o.y == y)
    }

    fn symbol(&self, x: i32, y: i32) -> Option<(char, String)> {
        if let Some(e) = self.eingang_bei(x, y) {
            return Some((e.symbol.chars().next().unwrap_or('O'), e.farbe.clone()));
        }
        let c = self.zeichen(x, y)?;
        let f = self.felder.get(&c)?;
        if let Some(o) = self.ort_bei(x, y) {
            if let Some(s) = o.symbol.chars().next() {
                let farbe = if o.farbe.is_empty() { f.farbe.clone() } else { o.farbe.clone() };
                return Some((s, farbe));
            }
        }
        Some((f.symbol.chars().next().unwrap_or(c), f.farbe.clone()))
    }

    /// Wie weit man von hier aus sehen kann.
    pub fn sichtweite(&self, x: i32, y: i32) -> i32 {
        (self.cfg.sicht + self.feld(x, y).map(|f| f.sicht).unwrap_or(0)).max(1)
    }

    /// Kann man von (x0,y0) aus das Feld (x1,y1) sehen?
    /// Felder mit `blockiert_sicht` (z.B. Berge) verdecken alles dahinter,
    /// Felder mit `sichtbehinderung` (z.B. Wald) verkürzen den Blick.
    pub fn sieht(&self, x0: i32, y0: i32, x1: i32, y1: i32, weite: i32) -> bool {
        let (dx, dy) = (x1 - x0, y1 - y0);
        let abstand = ((dx * dx + dy * dy) as f64).sqrt();
        if abstand > weite as f64 + 0.5 {
            return false;
        }
        let mut behinderung = 0;
        let hier = self.feld(x0, y0);
        let erhoeht = hier.is_some_and(|f| f.blockiert_sicht);
        let hoehe_hier = hier.map(|f| f.hoehe).unwrap_or(0);
        // Bresenham-Linie vom Betrachter zum Ziel
        let (ax, ay) = (dx.abs(), -dy.abs());
        let (sx, sy) = (dx.signum(), dy.signum());
        let (mut x, mut y, mut err) = (x0, y0, ax + ay);
        loop {
            if (x, y) == (x1, y1) {
                return true;
            }
            let e2 = 2 * err;
            if e2 >= ay {
                err += ay;
                x += sx;
            }
            if e2 <= ax {
                err += ax;
                y += sy;
            }
            if (x, y) == (x1, y1) {
                return true;
            }
            if let Some(f) = self.feld(x, y) {
                behinderung += f.sichtbehinderung;
                let verdeckt = f.blockiert_sicht && !(erhoeht && f.hoehe <= hoehe_hier);
                if verdeckt || abstand + behinderung as f64 > weite as f64 + 0.5 {
                    return false;
                }
            }
        }
    }

    /// Nur die Ausgänge eines Feldes, ohne den ganzen Raum zu bauen.
    pub fn ausgaenge(&self, x: i32, y: i32) -> BTreeMap<Richtung, String> {
        let mut aus = BTreeMap::new();
        if self.feld(x, y).is_none() {
            return aus;
        }
        for r in ACHT {
            let (nx, ny) = nachbar(x, y, r);
            if self.begehbar(nx, ny) {
                aus.insert(r, id(nx, ny));
            }
        }
        if let Some(e) = self.eingang_bei(x, y) {
            aus.insert(Richtung::Rein, e.raum.clone());
        }
        aus
    }

    /// Baut aus einem Kartenfeld einen vollständigen Raum.
    pub fn raum_bauen(&self, x: i32, y: i32) -> Option<RaumDef> {
        let f = self.feld(x, y)?;
        let ort = self.ort_bei(x, y);
        let eingang = self.eingang_bei(x, y);

        let name = match (ort, eingang) {
            (Some(o), _) => o.name.clone(),
            (None, Some(e)) => e.name.clone(),
            _ => f.name.clone(),
        };

        let mut absaetze = Vec::new();
        let grund = match (ort, eingang) {
            (Some(o), _) if !o.beschreibung.is_empty() => o.beschreibung.clone(),
            (_, Some(e)) if !e.beschreibung.is_empty() => e.beschreibung.clone(),
            _ if !f.beschreibungen.is_empty() => {
                // Immer derselbe Text für dasselbe Feld, aber Abwechslung zwischen den Feldern
                let n = ((x as i64 * 7919 + y as i64 * 104_729).unsigned_abs() as usize) % f.beschreibungen.len();
                f.beschreibungen[n].clone()
            }
            _ => String::new(),
        };
        if !grund.is_empty() {
            absaetze.push(grund);
        }
        let umgebung = self.umgebung_text(x, y);
        if !umgebung.is_empty() {
            absaetze.push(umgebung);
        }
        let fern = self.fernsicht_text(x, y);
        if !fern.is_empty() {
            absaetze.push(fern);
        }
        if let Some(e) = eingang {
            absaetze.push(format!("{{hellgelb}}Hier geht es hinein: {} ({{fett}}rein{{/}}{{hellgelb}}){{/}}", e.name));
        }

        let (symbol, farbe) = self.symbol(x, y).unwrap_or(('?', String::new()));
        Some(RaumDef {
            id: id(x, y),
            name,
            beschreibung: absaetze.join("\n\n"),
            x,
            y,
            z: 0,
            gebiet: self.cfg.gebiet.clone(),
            gelaende: String::new(),
            symbol: symbol.to_string(),
            farbe,
            klammern: String::new(),
            ausgaenge_roh: BTreeMap::new(),
            details: ort
                .map(|o| o.details.iter().map(|(k, v)| (text::normalisieren(k), v.clone())).collect())
                .unwrap_or_default(),
            gegenstaende: ort.map(|o| o.gegenstaende.clone()).unwrap_or_default(),
            monster: ort.map(|o| o.monster.clone()).unwrap_or_default(),
            ausgaenge: self.ausgaenge(x, y),
            quelle: "oberwelt/oberwelt.toml".into(),
        })
    }

    /// "Im Norden und Nordosten siehst du hohe Berge."
    fn umgebung_text(&self, x: i32, y: i32) -> String {
        // Nach Geländename gruppieren, damit z.B. / ^ \ zusammen "Gebirge" sind.
        let hier = self.feld(x, y).map(|f| f.name.clone());
        let mut gruppen: Vec<(String, Vec<String>)> = Vec::new();
        for r in ACHT {
            let (nx, ny) = nachbar(x, y, r);
            let Some(f) = self.feld(nx, ny) else { continue };
            if Some(&f.name) == hier.as_ref() {
                continue;
            }
            let was = if f.fern.is_empty() { f.name.clone() } else { f.fern.clone() };
            match gruppen.iter_mut().find(|(g, _)| *g == was) {
                Some((_, liste)) => liste.push(r.anzeige().to_string()),
                None => gruppen.push((was, vec![r.anzeige().to_string()])),
            }
        }
        let saetze: Vec<String> = gruppen
            .into_iter()
            .map(|(was, richtungen)| format!("Im {} siehst du {}.", aufzaehlen(&richtungen), was))
            .collect();
        saetze.join(" ")
    }

    /// Städte und besondere Orte, die man in der Ferne erkennen kann.
    fn fernsicht_text(&self, x: i32, y: i32) -> String {
        let weite = self.sichtweite(x, y);
        let mut ziele: Vec<(i32, i32, String)> =
            self.eingaenge.iter().map(|e| (e.x, e.y, e.name.clone())).collect();
        ziele.extend(self.orte.iter().filter(|o| !o.symbol.is_empty()).map(|o| (o.x, o.y, o.name.clone())));

        let mut saetze = Vec::new();
        for (zx, zy, name) in ziele {
            let (dx, dy) = (zx - x, zy - y);
            if (dx, dy) == (0, 0) || !self.sieht(x, y, zx, zy, weite) {
                continue;
            }
            let r = richtung_zu(dx, -dy);
            let abstand = ((dx * dx + dy * dy) as f64).sqrt();
            let wie_weit = if abstand < 2.5 { "Ganz in der Nähe" } else if abstand < 5.0 { "Nicht weit entfernt" } else { "In der Ferne" };
            saetze.push(format!("{wie_weit} im {} liegt {{hellgelb}}{name}{{/}}.", r.anzeige()));
        }
        saetze.join(" ")
    }

    // -----------------------------------------------------------------------
    // Zeichnen
    // -----------------------------------------------------------------------

    /// Der Ausschnitt rund um den Spieler, so weit er sehen kann.
    /// Stil "kompakt": ein Zeichen pro Feld im Rahmen, X = du.
    /// Stil "weit": ein Zeichen und ein Leerzeichen pro Feld, ohne Rahmen.
    pub fn karte_zeichnen(&self, x: i32, y: i32, andere: &[(i32, i32)], mit_legende: bool) -> String {
        let (rx, ry) = (self.cfg.anzeige_breite.max(1), self.cfg.anzeige_hoehe.max(1));
        let kompakt = self.cfg.stil != "weit";
        let weite = self.sichtweite(x, y);
        let mut gesehen: BTreeSet<char> = BTreeSet::new();
        let mut orte_gesehen: Vec<(char, String, String)> = Vec::new();
        let ich = if kompakt { 'X' } else { '@' };

        let mut gitter: Vec<Vec<Zelle>> = Vec::new();
        for ty in y - ry..=y + ry {
            let mut z = Vec::new();
            for tx in x - rx..=x + rx {
                let zelle = if (tx, ty) == (x, y) {
                    Zelle { zeichen: ich, farbe: "hellgelb".into() }
                } else if !self.sieht(x, y, tx, ty, weite) {
                    Zelle { zeichen: ' ', farbe: String::new() }
                } else if andere.contains(&(tx, ty)) {
                    Zelle { zeichen: '@', farbe: "hellmagenta".into() }
                } else if let Some((sym, farbe)) = self.symbol(tx, ty) {
                    if let Some(e) = self.eingang_bei(tx, ty) {
                        orte_gesehen.push((sym, e.farbe.clone(), e.name.clone()));
                    } else if let Some(o) = self.ort_bei(tx, ty).filter(|o| !o.symbol.is_empty()) {
                        orte_gesehen.push((sym, farbe.clone(), o.name.clone()));
                    } else if let Some(c) = self.zeichen(tx, ty) {
                        gesehen.insert(c);
                    }
                    Zelle { zeichen: sym, farbe }
                } else {
                    Zelle { zeichen: ' ', farbe: String::new() }
                };
                z.push(zelle);
                if !kompakt {
                    z.push(Zelle { zeichen: ' ', farbe: String::new() });
                }
            }
            gitter.push(z);
        }

        let feldname = self.feld(x, y).map(|f| f.name.clone()).unwrap_or_default();
        let mut aus = format!("{{grau}}[{} – {feldname} – Sicht {weite}]{{/}}\n", self.cfg.name);
        if kompakt {
            aus.push_str(&crate::karte::rahmen(&gitter));
        } else {
            for z in &gitter {
                let mut t = String::from("  ");
                for c in z {
                    t.push_str(&farbig(c.zeichen, &c.farbe));
                }
                aus.push_str(t.trim_end());
                aus.push('\n');
            }
        }
        if mit_legende {
            let mut teile = vec![format!("{{hellgelb}}{ich}{{/}} du"), "{hellmagenta}@{/} andere".to_string()];
            for (sym, farbe, name) in orte_gesehen {
                let eintrag = format!("{} {name}", farbig(sym, &farbe));
                if !teile.contains(&eintrag) {
                    teile.push(eintrag);
                }
            }
            // Gleichnamige Felder (z.B. / ^ \ = Gebirge) zusammenfassen
            let mut nach_name: Vec<(String, String)> = Vec::new();
            for c in gesehen {
                if let Some(f) = self.felder.get(&c) {
                    let sym = farbig(f.symbol.chars().next().unwrap_or(c), &f.farbe);
                    match nach_name.iter_mut().find(|(n, _)| *n == f.name) {
                        Some((_, s)) => s.push_str(&sym),
                        None => nach_name.push((f.name.clone(), sym)),
                    }
                }
            }
            teile.extend(nach_name.into_iter().map(|(n, s)| format!("{s} {n}")));
            for teil in teile.chunks(5) {
                aus.push_str(&format!("{}\n", teil.join("   ")));
            }
        }
        aus
    }

    /// Die ganze Oberwelt auf einen Blick (ein Zeichen pro Feld).
    pub fn weltkarte(&self, x: i32, y: i32) -> String {
        let mut aus = format!("{{fett}}{}{{/}}  {{grau}}(Norden ist oben, X = du){{/}}\n", self.cfg.name);
        for ty in 1..=self.hoehe {
            let mut z = String::new();
            for tx in 1..=self.breite {
                if (tx, ty) == (x, y) {
                    z.push_str("{hellgelb}{fett}X{/}");
                } else if let Some((sym, farbe)) = self.symbol(tx, ty) {
                    z.push_str(&farbig(sym, &farbe));
                } else {
                    z.push(' ');
                }
            }
            aus.push_str(z.trim_end());
            aus.push('\n');
        }
        let namen: Vec<String> = self
            .eingaenge
            .iter()
            .map(|e| format!("{} {}", farbig(e.symbol.chars().next().unwrap_or('O'), &e.farbe), e.name))
            .collect();
        if !namen.is_empty() {
            aus.push_str(&format!("{}\n", namen.join("   ")));
        }
        aus
    }
}

fn farbig(c: char, farbe: &str) -> String {
    if farbe.is_empty() {
        c.to_string()
    } else {
        format!("{{{farbe}}}{c}{{/}}")
    }
}

/// Grobe Himmelsrichtung zu einem Punkt (dx nach Osten, dy nach Norden).
pub fn richtung_zu(dx: i32, dy_nord: i32) -> Richtung {
    let winkel = (dy_nord as f64).atan2(dx as f64).to_degrees();
    let sektor = (((winkel + 360.0 + 22.5) % 360.0) / 45.0) as i32;
    match sektor {
        0 => Richtung::Ost,
        1 => Richtung::Nordost,
        2 => Richtung::Nord,
        3 => Richtung::Nordwest,
        4 => Richtung::West,
        5 => Richtung::Suedwest,
        6 => Richtung::Sued,
        _ => Richtung::Suedost,
    }
}
