//! Die zentrale Verwaltung aller Weltdaten.
//!
//! Hier wird festgelegt, wie Räume, Gegenstände, Monster, Fertigkeiten und Zauber
//! in den TOML-Dateien aussehen. Beim Laden wird alles gründlich geprüft, damit
//! Tippfehler sofort mit einer verständlichen Meldung auffallen.

use crate::oberwelt::{self, Oberwelt};
use crate::text;
use std::borrow::Cow;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Himmelsrichtungen
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Richtung {
    Nord,
    Nordost,
    Ost,
    Suedost,
    Sued,
    Suedwest,
    West,
    Nordwest,
    Hoch,
    Runter,
    /// Eine Stadt oder einen Dungeon betreten (von der Oberwelt aus)
    Rein,
    /// Zurück auf die Oberwelt
    Raus,
}

impl Richtung {
    pub const ALLE: [Richtung; 12] = [
        Richtung::Nord,
        Richtung::Nordost,
        Richtung::Ost,
        Richtung::Suedost,
        Richtung::Sued,
        Richtung::Suedwest,
        Richtung::West,
        Richtung::Nordwest,
        Richtung::Hoch,
        Richtung::Runter,
        Richtung::Rein,
        Richtung::Raus,
    ];

    /// Der Name, der in den Dateien verwendet wird.
    pub fn schluessel(self) -> &'static str {
        match self {
            Richtung::Nord => "nord",
            Richtung::Nordost => "nordost",
            Richtung::Ost => "ost",
            Richtung::Suedost => "suedost",
            Richtung::Sued => "sued",
            Richtung::Suedwest => "suedwest",
            Richtung::West => "west",
            Richtung::Nordwest => "nordwest",
            Richtung::Hoch => "hoch",
            Richtung::Runter => "runter",
            Richtung::Rein => "rein",
            Richtung::Raus => "raus",
        }
    }

    /// Der Name, wie ihn Spieler lesen.
    pub fn anzeige(self) -> &'static str {
        match self {
            Richtung::Nord => "Norden",
            Richtung::Nordost => "Nordosten",
            Richtung::Ost => "Osten",
            Richtung::Suedost => "Südosten",
            Richtung::Sued => "Süden",
            Richtung::Suedwest => "Südwesten",
            Richtung::West => "Westen",
            Richtung::Nordwest => "Nordwesten",
            Richtung::Hoch => "oben",
            Richtung::Runter => "unten",
            Richtung::Rein => "hinein",
            Richtung::Raus => "hinaus",
        }
    }

    pub fn aus_text(t: &str) -> Option<Richtung> {
        Some(match text::normalisieren(t).as_str() {
            "n" | "nord" | "norden" | "north" => Richtung::Nord,
            "no" | "ne" | "nordost" | "nordosten" | "northeast" => Richtung::Nordost,
            "o" | "e" | "ost" | "osten" | "east" => Richtung::Ost,
            "so" | "se" | "suedost" | "suedosten" | "southeast" => Richtung::Suedost,
            "s" | "sued" | "sueden" | "south" => Richtung::Sued,
            "sw" | "suedwest" | "suedwesten" | "southwest" => Richtung::Suedwest,
            "w" | "west" | "westen" => Richtung::West,
            "nw" | "nordwest" | "nordwesten" | "northwest" => Richtung::Nordwest,
            "h" | "hoch" | "oben" | "rauf" | "u" | "up" => Richtung::Hoch,
            "r" | "runter" | "unten" | "d" | "down" => Richtung::Runter,
            "rein" | "hinein" | "betreten" | "betrete" | "enter" | "in" => Richtung::Rein,
            "raus" | "hinaus" | "verlassen" | "verlasse" | "leave" | "out" => Richtung::Raus,
            _ => return None,
        })
    }

    pub fn gegenteil(self) -> Richtung {
        match self {
            Richtung::Nord => Richtung::Sued,
            Richtung::Nordost => Richtung::Suedwest,
            Richtung::Ost => Richtung::West,
            Richtung::Suedost => Richtung::Nordwest,
            Richtung::Sued => Richtung::Nord,
            Richtung::Suedwest => Richtung::Nordost,
            Richtung::West => Richtung::Ost,
            Richtung::Nordwest => Richtung::Suedost,
            Richtung::Hoch => Richtung::Runter,
            Richtung::Runter => Richtung::Hoch,
            Richtung::Rein => Richtung::Raus,
            Richtung::Raus => Richtung::Rein,
        }
    }

    /// Verschiebung auf der Karte: x wächst nach Osten, y nach Norden, z nach oben.
    pub fn versatz(self) -> (i32, i32, i32) {
        match self {
            Richtung::Nord => (0, 1, 0),
            Richtung::Nordost => (1, 1, 0),
            Richtung::Ost => (1, 0, 0),
            Richtung::Suedost => (1, -1, 0),
            Richtung::Sued => (0, -1, 0),
            Richtung::Suedwest => (-1, -1, 0),
            Richtung::West => (-1, 0, 0),
            Richtung::Nordwest => (-1, 1, 0),
            Richtung::Hoch => (0, 0, 1),
            Richtung::Runter => (0, 0, -1),
            Richtung::Rein | Richtung::Raus => (0, 0, 0),
        }
    }

    /// "geht nach Norden" / "geht hinein"
    pub fn geht(self) -> String {
        match self {
            Richtung::Rein => "geht hinein".into(),
            Richtung::Raus => "geht hinaus".into(),
            _ => format!("geht nach {}", self.anzeige()),
        }
    }

    /// Meldung im Zielraum, wenn jemand in diese Richtung gegangen ist.
    pub fn kommt(self) -> String {
        match self {
            Richtung::Rein => "kommt von draußen herein".into(),
            Richtung::Raus => "kommt heraus".into(),
            _ => format!("kommt von {} herein", self.gegenteil().anzeige()),
        }
    }
}

// ---------------------------------------------------------------------------
// welt.toml
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct WeltConfig {
    pub name: String,
    pub port: u16,
    pub startraum: String,
    pub willkommen: String,
    pub admins: Vec<String>,
    pub karte_beim_umsehen: bool,
    pub karten_radius: i32,
    pub nur_besuchte_raeume_auf_karte: bool,
    pub ausgaenge_beidseitig: bool,
    pub takt_sekunden: u64,
    pub zuruecksetzen_minuten: u64,
    pub speichern_minuten: u64,
    pub regeneration_lp: i32,
    pub regeneration_mp: i32,
    pub spieler_verzeichnis: String,
    /// "kasten" = Räume als ( ) und [ ] in einem Rahmen, "linien" = ein Zeichen pro Raum
    pub stadtkarte_stil: String,
    /// Füllzeichen für leere Stellen der Stadtkarte
    pub karten_hintergrund: String,
}

impl Default for WeltConfig {
    fn default() -> Self {
        WeltConfig {
            name: "Namenlose Welt".into(),
            port: 4000,
            startraum: String::new(),
            willkommen: String::new(),
            admins: Vec::new(),
            karte_beim_umsehen: true,
            karten_radius: 3,
            nur_besuchte_raeume_auf_karte: false,
            ausgaenge_beidseitig: true,
            takt_sekunden: 3,
            zuruecksetzen_minuten: 10,
            speichern_minuten: 5,
            regeneration_lp: 1,
            regeneration_mp: 1,
            spieler_verzeichnis: "spieler".into(),
            stadtkarte_stil: "kasten".into(),
            karten_hintergrund: "*".into(),
        }
    }
}

/// Womit neue Spieler beginnen.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct StartWerte {
    pub lebenspunkte: i32,
    pub mana: i32,
    pub gegenstaende: Vec<String>,
    pub fertigkeiten: BTreeMap<String, u32>,
    pub zauber: Vec<String>,
    pub werte: BTreeMap<String, i32>,
}

impl Default for StartWerte {
    fn default() -> Self {
        StartWerte {
            lebenspunkte: 100,
            mana: 50,
            gegenstaende: Vec::new(),
            fertigkeiten: BTreeMap::new(),
            zauber: Vec::new(),
            werte: BTreeMap::new(),
        }
    }
}

/// Geländeart: legt fest, wie ein Raum auf der Karte aussieht.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct GelaendeDef {
    pub name: String,
    pub symbol: String,
    pub farbe: String,
    /// Klammern um den Raum auf der Stadtkarte, z.B. "()" draußen, "[]" Gebäude
    pub klammern: String,
    /// Verzierung rund um die Kästchen, z.B. "o&" für Bäume
    pub deko: String,
    /// Wie dicht die Verzierung ist (Prozent)
    pub deko_dichte: u32,
}

impl Default for GelaendeDef {
    fn default() -> Self {
        GelaendeDef {
            name: String::new(),
            symbol: "#".into(),
            farbe: String::new(),
            klammern: "[]".into(),
            deko: String::new(),
            deko_dichte: 30,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WeltDatei {
    welt: WeltConfig,
    #[serde(default)]
    start: StartWerte,
    #[serde(default)]
    gelaende: BTreeMap<String, GelaendeDef>,
    #[serde(default)]
    deko: BTreeMap<String, String>,
}

// ---------------------------------------------------------------------------
// Inhalte: Räume, Gegenstände, Monster, Fertigkeiten, Zauber
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RaumDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub beschreibung: String,
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub z: i32,
    #[serde(default)]
    pub gebiet: String,
    #[serde(default)]
    pub gelaende: String,
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub farbe: String,
    #[serde(default)]
    pub klammern: String,
    /// Eigene Karte, z.B. für das Innere eines Hauses (leer = gemeinsame Karte)
    #[serde(default)]
    pub karte: String,
    #[serde(default)]
    pub deko: String,
    #[serde(default)]
    pub deko_dichte: Option<u32>,
    /// Verzierung an bestimmten Seiten, z.B. { sued = "~" } für einen Bach
    #[serde(default)]
    pub deko_richtung: BTreeMap<String, String>,
    #[serde(default, rename = "ausgaenge")]
    pub ausgaenge_roh: BTreeMap<String, String>,
    #[serde(default)]
    pub details: BTreeMap<String, String>,
    #[serde(default)]
    pub gegenstaende: Vec<String>,
    #[serde(default)]
    pub monster: Vec<String>,

    #[serde(skip)]
    pub ausgaenge: BTreeMap<Richtung, String>,
    #[serde(skip)]
    pub quelle: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GegenstandDef {
    pub id: String,
    pub name: String,
    /// Wie der Name nach "Du nimmst ..." heißt, z.B. "einen roten Apfel".
    #[serde(default)]
    pub akkusativ: String,
    #[serde(default)]
    pub beschreibung: String,
    #[serde(default)]
    pub bodentext: String,
    #[serde(default)]
    pub schluesselwoerter: Vec<String>,
    #[serde(default = "standard_typ")]
    pub typ: String,
    #[serde(default)]
    pub platz: String,
    #[serde(default = "eins")]
    pub gewicht: i32,
    #[serde(default)]
    pub wert: i32,
    #[serde(default = "wahr")]
    pub tragbar: bool,
    #[serde(default)]
    pub eigenschaften: BTreeMap<String, i32>,
    #[serde(default)]
    pub benutzen_text: String,

    #[serde(skip)]
    pub quelle: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BeuteDef {
    pub gegenstand: String,
    #[serde(default = "hundert")]
    pub chance: u32,
}

/// `werte` und `aggressiv` werden erst vom Kampfsystem benutzt.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MonsterDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub akkusativ: String,
    #[serde(default)]
    pub beschreibung: String,
    #[serde(default)]
    pub raumtext: String,
    #[serde(default)]
    pub schluesselwoerter: Vec<String>,
    #[serde(default = "eins_u")]
    pub stufe: u32,
    #[serde(default = "zehn")]
    pub lebenspunkte: i32,
    #[serde(default)]
    pub mana: i32,
    #[serde(default)]
    pub werte: BTreeMap<String, i32>,
    #[serde(default)]
    pub fertigkeiten: BTreeMap<String, u32>,
    #[serde(default)]
    pub zauber: Vec<String>,
    #[serde(default)]
    pub beute: Vec<BeuteDef>,
    #[serde(default)]
    pub aggressiv: bool,
    #[serde(default)]
    pub wandert: bool,
    #[serde(default = "zehn_u")]
    pub wander_chance: u32,
    #[serde(default)]
    pub sprueche: Vec<String>,
    #[serde(default = "fuenf_u")]
    pub sprech_chance: u32,

    #[serde(skip)]
    pub quelle: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FertigkeitDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub beschreibung: String,
    #[serde(default = "standard_kategorie")]
    pub kategorie: String,
    #[serde(default = "hundert")]
    pub max_stufe: u32,
    #[serde(default)]
    pub attribut: String,
    #[serde(default)]
    pub voraussetzungen: Vec<String>,
    #[serde(default = "zehn_u")]
    pub lernchance: u32,

    #[serde(skip)]
    pub quelle: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WirkungDef {
    pub art: String,
    #[serde(default)]
    pub wert: i32,
    #[serde(default)]
    pub raum: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZauberDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub beschreibung: String,
    #[serde(default)]
    pub schluesselwoerter: Vec<String>,
    #[serde(default)]
    pub mana_kosten: i32,
    #[serde(default = "standard_ziel")]
    pub ziel: String,
    #[serde(default)]
    pub fertigkeit: String,
    #[serde(default)]
    pub mindest_stufe: u32,
    #[serde(default)]
    pub abklingzeit: u64,
    #[serde(default)]
    pub wirkungen: Vec<WirkungDef>,
    #[serde(default)]
    pub text_selbst: String,
    #[serde(default)]
    pub text_ziel: String,
    #[serde(default)]
    pub text_raum: String,

    #[serde(skip)]
    pub quelle: String,
}

fn standard_typ() -> String {
    "sonstiges".into()
}
fn standard_kategorie() -> String {
    "allgemein".into()
}
fn standard_ziel() -> String {
    "selbst".into()
}
fn eins() -> i32 {
    1
}
fn eins_u() -> u32 {
    1
}
fn zehn() -> i32 {
    10
}
fn zehn_u() -> u32 {
    10
}
fn fuenf_u() -> u32 {
    5
}
fn hundert() -> u32 {
    100
}
fn wahr() -> bool {
    true
}

/// Eine Inhaltsdatei darf beliebig viele Einträge jeder Art enthalten.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
struct InhaltsDatei {
    raum: Vec<RaumDef>,
    gegenstand: Vec<GegenstandDef>,
    monster: Vec<MonsterDef>,
    fertigkeit: Vec<FertigkeitDef>,
    zauber: Vec<ZauberDef>,
}

// ---------------------------------------------------------------------------
// Die geladenen Daten
// ---------------------------------------------------------------------------

pub const GEGENSTAND_TYPEN: &[&str] =
    &["waffe", "ruestung", "trank", "nahrung", "schluessel", "werkzeug", "schmuck", "behaelter", "sonstiges"];
pub const PLAETZE: &[&str] =
    &["waffe", "schild", "kopf", "koerper", "haende", "beine", "fuesse", "hals", "ring", "umhang", "licht"];
pub const ZAUBER_ZIELE: &[&str] = &["selbst", "spieler", "gegner", "raum"];
pub const WIRKUNGEN: &[&str] = &["heilen", "mana", "schaden", "teleport"];

#[derive(Debug, Clone)]
pub struct Daten {
    pub welt: WeltConfig,
    pub start: StartWerte,
    pub gelaende: BTreeMap<String, GelaendeDef>,
    pub raeume: BTreeMap<String, RaumDef>,
    pub gegenstaende: BTreeMap<String, GegenstandDef>,
    pub monster: BTreeMap<String, MonsterDef>,
    pub fertigkeiten: BTreeMap<String, FertigkeitDef>,
    pub zauber: BTreeMap<String, ZauberDef>,
    /// Welcher Raum liegt an welcher Koordinate?
    /// (karte, x, y, z) -> Raum
    pub koordinaten: HashMap<(String, i32, i32, i32), String>,
    /// Farben der Verzierungen: Zeichen -> Farbe
    pub deko_farben: BTreeMap<String, String>,
    pub oberwelt: Option<Oberwelt>,
    pub warnungen: Vec<String>,
}

impl Daten {
    /// Holt einen Raum – einen gebauten oder ein Feld der Oberwelt ("ow:x:y").
    pub fn raum(&self, id: &str) -> Option<Cow<'_, RaumDef>> {
        if let Some(r) = self.raeume.get(id) {
            return Some(Cow::Borrowed(r));
        }
        let (x, y) = oberwelt::aus_id(id)?;
        self.oberwelt.as_ref()?.raum_bauen(x, y).map(Cow::Owned)
    }

    pub fn raum_existiert(&self, id: &str) -> bool {
        if self.raeume.contains_key(id) {
            return true;
        }
        match (oberwelt::aus_id(id), &self.oberwelt) {
            (Some((x, y)), Some(ow)) => ow.begehbar(x, y),
            _ => false,
        }
    }

    /// Die Ausgänge eines Raums (schnell, auch für Oberweltfelder).
    pub fn ausgaenge(&self, id: &str) -> BTreeMap<Richtung, String> {
        if let Some(r) = self.raeume.get(id) {
            return r.ausgaenge.clone();
        }
        match (oberwelt::aus_id(id), &self.oberwelt) {
            (Some((x, y)), Some(ow)) => ow.ausgaenge(x, y),
            _ => BTreeMap::new(),
        }
    }

    pub fn gebiet(&self, id: &str) -> String {
        if let Some(r) = self.raeume.get(id) {
            return r.gebiet.clone();
        }
        match &self.oberwelt {
            Some(ow) if oberwelt::ist_oberwelt(id) => ow.cfg.gebiet.clone(),
            _ => String::new(),
        }
    }

    /// Alle Orte, an denen von Anfang an Gegenstände oder Monster sind.
    pub fn bevoelkerung(&self) -> Vec<(String, Vec<String>, Vec<String>)> {
        let mut aus: Vec<_> = self
            .raeume
            .values()
            .map(|r| (r.id.clone(), r.gegenstaende.clone(), r.monster.clone()))
            .collect();
        if let Some(ow) = &self.oberwelt {
            for o in &ow.orte {
                aus.push((oberwelt::id(o.x, o.y), o.gegenstaende.clone(), o.monster.clone()));
            }
        }
        aus
    }

    /// Kartensymbol und Farbe eines Raums.
    pub fn raum_symbol(&self, r: &RaumDef) -> (char, String) {
        let g = self.gelaende.get(&r.gelaende);
        let symbol = r
            .symbol
            .chars()
            .next()
            .or_else(|| g.and_then(|g| g.symbol.chars().next()))
            .unwrap_or('#');
        let farbe = if !r.farbe.is_empty() {
            r.farbe.clone()
        } else {
            g.map(|g| g.farbe.clone()).unwrap_or_default()
        };
        (symbol, farbe)
    }

    /// Name im Akkusativ ("Du nimmst einen Apfel"). Ohne Angabe: der normale Name.
    pub fn gegenstand_akk(&self, id: &str) -> String {
        match self.gegenstaende.get(id) {
            Some(g) if !g.akkusativ.is_empty() => g.akkusativ.clone(),
            _ => self.gegenstand_name(id),
        }
    }

    pub fn gegenstand_name(&self, id: &str) -> String {
        self.gegenstaende.get(id).map(|g| g.name.clone()).unwrap_or_else(|| format!("<{id}>"))
    }
}

/// Lädt die komplette Welt aus einem Ordner.
/// Gibt bei Fehlern eine Liste verständlicher Meldungen zurück.
pub fn laden(ordner: &Path) -> Result<Daten, Vec<String>> {
    let welt_pfad = ordner.join("welt.toml");
    let inhalt = fs::read_to_string(&welt_pfad)
        .map_err(|e| vec![format!("Kann '{}' nicht lesen: {}", welt_pfad.display(), e)])?;
    let wd: WeltDatei = toml::from_str(&inhalt)
        .map_err(|e| vec![format!("Fehler in 'welt.toml':\n{}", toml_fehler_uebersetzen(&e))])?;

    let mut d = Daten {
        welt: wd.welt,
        start: wd.start,
        gelaende: wd.gelaende,
        raeume: BTreeMap::new(),
        gegenstaende: BTreeMap::new(),
        monster: BTreeMap::new(),
        fertigkeiten: BTreeMap::new(),
        zauber: BTreeMap::new(),
        koordinaten: HashMap::new(),
        deko_farben: wd.deko,
        oberwelt: None,
        warnungen: Vec::new(),
    };
    let mut fehler = Vec::new();
    let mut warnungen = Vec::new();

    let mut dateien = Vec::new();
    toml_dateien_sammeln(ordner, &mut dateien);
    dateien.sort();

    macro_rules! einsortieren {
        ($liste:expr, $ziel:expr, $art:expr, $datei:expr, $fehler:expr) => {
            for mut e in $liste {
                e.quelle = $datei.clone();
                id_pruefen(&e.id, $art, &$datei, $fehler);
                if $ziel.contains_key(&e.id) {
                    $fehler.push(format!("[{}] {} '{}' gibt es doppelt.", $datei, $art, e.id));
                } else {
                    $ziel.insert(e.id.clone(), e);
                }
            }
        };
    }

    for pfad in dateien {
        if pfad == welt_pfad {
            continue;
        }
        let datei = pfad.strip_prefix(ordner).unwrap_or(&pfad).display().to_string();
        let inhalt = match fs::read_to_string(&pfad) {
            Ok(i) => i,
            Err(e) => {
                fehler.push(format!("Kann '{datei}' nicht lesen: {e}"));
                continue;
            }
        };
        let inh: InhaltsDatei = match toml::from_str(&inhalt) {
            Ok(i) => i,
            Err(e) => {
                fehler.push(format!("Fehler in '{datei}':\n{}", toml_fehler_uebersetzen(&e)));
                continue;
            }
        };
        einsortieren!(inh.raum, d.raeume, "Raum", datei, &mut fehler);
        einsortieren!(inh.gegenstand, d.gegenstaende, "Gegenstand", datei, &mut fehler);
        einsortieren!(inh.monster, d.monster, "Monster", datei, &mut fehler);
        einsortieren!(inh.fertigkeit, d.fertigkeiten, "Fertigkeit", datei, &mut fehler);
        einsortieren!(inh.zauber, d.zauber, "Zauber", datei, &mut fehler);
    }

    d.oberwelt = oberwelt::laden(ordner, &mut fehler);
    pruefen(&mut d, &mut fehler, &mut warnungen);

    if fehler.is_empty() {
        d.warnungen = warnungen;
        Ok(d)
    } else {
        Err(fehler)
    }
}

/// Macht die englischen Meldungen des TOML-Lesers verständlicher.
pub fn toml_fehler_uebersetzen(e: &toml::de::Error) -> String {
    let mut t = e.to_string();
    for (en, de) in [
        ("TOML parse error at line", "Fehler in Zeile"),
        (", column", ", Spalte"),
        ("unknown field", "unbekanntes Feld"),
        ("expected one of", "erlaubt sind"),
        ("missing field", "es fehlt das Pflichtfeld"),
        ("duplicate key", "doppelter Eintrag"),
        ("invalid type: string", "falscher Typ: Text"),
        ("invalid type: integer", "falscher Typ: Zahl"),
        ("invalid type: boolean", "falscher Typ: true/false"),
        ("invalid type", "falscher Typ"),
        ("expected a boolean", "erwartet: true oder false"),
        ("expected a string", "erwartet: Text in \"Anführungszeichen\""),
        ("expected a sequence", "erwartet: eine Liste [ ... ]"),
        ("expected i32", "erwartet: eine ganze Zahl"),
        ("expected u32", "erwartet: eine ganze Zahl ab 0"),
        ("expected u64", "erwartet: eine ganze Zahl ab 0"),
        ("expected u16", "erwartet: eine ganze Zahl ab 0"),
        ("invalid string", "ungültiger Text (fehlt ein Anführungszeichen?)"),
        ("expected newline", "erwartet: Zeilenende"),
        ("invalid basic string", "ungültiger Text (fehlt ein Anführungszeichen?)"),
    ] {
        t = t.replace(en, de);
    }
    t
}

fn toml_dateien_sammeln(ordner: &Path, liste: &mut Vec<PathBuf>) {
    let Ok(eintraege) = fs::read_dir(ordner) else { return };
    for e in eintraege.flatten() {
        let p = e.path();
        if p.is_dir() {
            toml_dateien_sammeln(&p, liste);
        } else if p.extension().is_some_and(|x| x == "toml") && !p.ends_with("oberwelt/oberwelt.toml") {
            liste.push(p);
        }
    }
}

fn id_pruefen(id: &str, art: &str, datei: &str, fehler: &mut Vec<String>) {
    let ok = !id.is_empty() && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if !ok {
        fehler.push(format!(
            "[{datei}] {art} '{id}': Eine ID darf nur aus Kleinbuchstaben (ohne Umlaute), Ziffern und _ bestehen."
        ));
    }
}

/// Prüft, ob eine ID existiert, und schlägt bei Tippfehlern die richtige vor.
fn verweis<T>(map: &BTreeMap<String, T>, id: &str, wo: &str, art: &str, fehler: &mut Vec<String>) {
    if map.contains_key(id) {
        return;
    }
    let mut m = format!("{wo}: {art} '{id}' gibt es nicht.");
    if let Some(v) = text::vorschlag(id, map.keys()) {
        m += &format!(" Meintest du '{v}'?");
    }
    fehler.push(m);
}

/// Wie `verweis`, aber Räume dürfen auch Oberweltfelder ("ow:x:y") sein.
fn raum_verweis(d: &Daten, id: &str, wo: &str, fehler: &mut Vec<String>) {
    if oberwelt::ist_oberwelt(id) {
        if !d.raum_existiert(id) {
            fehler.push(format!("{wo}: '{id}' ist kein begehbares Feld der Oberwelt."));
        }
    } else {
        verweis(&d.raeume, id, wo, "Raum", fehler);
    }
}

fn farbe_pruefen(farbe: &str, wo: &str, fehler: &mut Vec<String>) {
    if !farbe.is_empty() && !text::ist_farbe(farbe) {
        let namen: Vec<String> = text::FARBEN.iter().skip(2).map(|(n, _)| n.to_string()).collect();
        fehler.push(format!("{wo}: Unbekannte Farbe '{farbe}'. Erlaubt: {}", namen.join(", ")));
    }
}

fn pruefen(d: &mut Daten, fehler: &mut Vec<String>, warn: &mut Vec<String>) {
    // --- welt.toml ---
    if d.welt.startraum.is_empty() {
        fehler.push("welt.toml: 'startraum' fehlt.".into());
    } else {
        raum_verweis(d, &d.welt.startraum, "welt.toml (startraum)", fehler);
    }
    if !["kasten", "linien"].contains(&d.welt.stadtkarte_stil.as_str()) {
        fehler.push("welt.toml: 'stadtkarte_stil' muss \"kasten\" oder \"linien\" sein.".into());
    }
    if d.welt.karten_hintergrund.chars().count() != 1 {
        fehler.push("welt.toml: 'karten_hintergrund' muss genau ein Zeichen sein (z.B. \"*\" oder \" \").".into());
    }
    for (zeichen, farbe) in &d.deko_farben {
        if zeichen.chars().count() != 1 {
            fehler.push(format!("welt.toml [deko]: '{zeichen}' muss genau ein Zeichen sein."));
        }
        farbe_pruefen(farbe, &format!("welt.toml [deko] '{zeichen}'"), fehler);
    }
    if d.welt.takt_sekunden == 0 {
        fehler.push("welt.toml: 'takt_sekunden' muss mindestens 1 sein.".into());
    }
    for g in &d.start.gegenstaende {
        verweis(&d.gegenstaende, g, "welt.toml [start]", "Gegenstand", fehler);
    }
    for f in d.start.fertigkeiten.keys() {
        verweis(&d.fertigkeiten, f, "welt.toml [start]", "Fertigkeit", fehler);
    }
    for z in &d.start.zauber {
        verweis(&d.zauber, z, "welt.toml [start]", "Zauber", fehler);
    }
    for (id, g) in &d.gelaende {
        let wo = format!("welt.toml [gelaende.{id}]");
        if g.symbol.chars().count() != 1 {
            fehler.push(format!("{wo}: 'symbol' muss genau ein Zeichen sein."));
        }
        if g.klammern.chars().count() != 2 {
            fehler.push(format!("{wo}: 'klammern' müssen genau zwei Zeichen sein, z.B. \"()\" oder \"[]\"."));
        }
        farbe_pruefen(&g.farbe, &wo, fehler);
    }

    // --- Räume ---
    let mut ausgaenge_neu = Vec::new();
    for r in d.raeume.values() {
        let wo = format!("[{}] Raum '{}'", r.quelle, r.id);
        let mut aus = BTreeMap::new();
        for (richt, ziel) in &r.ausgaenge_roh {
            match Richtung::aus_text(richt) {
                Some(ri) => {
                    raum_verweis(d, ziel, &format!("{wo}, Ausgang '{richt}'"), fehler);
                    aus.insert(ri, ziel.clone());
                }
                None => {
                    let namen: Vec<String> = Richtung::ALLE.iter().map(|r| r.schluessel().to_string()).collect();
                    let tipp = text::vorschlag(richt, namen.iter())
                        .map(|v| format!(" Meintest du '{v}'?"))
                        .unwrap_or_default();
                    fehler.push(format!(
                        "{wo}: '{richt}' ist keine Richtung.{tipp} Erlaubt: {}",
                        namen.join(", ")
                    ));
                }
            }
        }
        ausgaenge_neu.push((r.id.clone(), aus));
        if !r.gelaende.is_empty() {
            verweis(&d.gelaende, &r.gelaende, &wo, "Gelände", fehler);
        }
        if r.symbol.chars().count() > 1 {
            fehler.push(format!("{wo}: 'symbol' muss genau ein Zeichen sein."));
        }
        for richt in r.deko_richtung.keys() {
            match Richtung::aus_text(richt) {
                Some(ri) if ri.versatz().2 == 0 && ri.versatz() != (0, 0, 0) => {}
                _ => fehler.push(format!("{wo}: deko_richtung '{richt}' – erlaubt sind nur Himmelsrichtungen (nord, nordost, ...).")),
            }
        }
        if r.deko_dichte.is_some_and(|x| x > 100) {
            fehler.push(format!("{wo}: 'deko_dichte' ist eine Prozentzahl (0 bis 100)."));
        }
        if !r.klammern.is_empty() && r.klammern.chars().count() != 2 {
            fehler.push(format!("{wo}: 'klammern' müssen genau zwei Zeichen sein, z.B. \"()\"."));
        }
        farbe_pruefen(&r.farbe, &wo, fehler);
        for g in &r.gegenstaende {
            verweis(&d.gegenstaende, g, &wo, "Gegenstand", fehler);
        }
        for m in &r.monster {
            verweis(&d.monster, m, &wo, "Monster", fehler);
        }
        if r.beschreibung.trim().is_empty() {
            warn.push(format!("{wo}: hat keine Beschreibung."));
        }
    }
    for (id, aus) in ausgaenge_neu {
        if let Some(r) = d.raeume.get_mut(&id) {
            r.ausgaenge = aus;
            r.details = std::mem::take(&mut r.details)
                .into_iter()
                .map(|(k, v)| (text::normalisieren(&k), v))
                .collect();
        }
    }

    // Eingänge der Oberwelt: Die Stadt bekommt automatisch den Ausgang "raus".
    if let Some(ow) = &d.oberwelt {
        let mut neu = Vec::new();
        for e in &ow.eingaenge {
            verweis(&d.raeume, &e.raum, &format!("oberwelt.toml, Eingang '{}'", e.name), "Raum", fehler);
            neu.push((e.raum.clone(), oberwelt::id(e.x, e.y)));
        }
        for o in &ow.orte {
            let wo = format!("oberwelt.toml, Ort '{}'", o.name);
            for g in &o.gegenstaende {
                verweis(&d.gegenstaende, g, &wo, "Gegenstand", fehler);
            }
            for m in &o.monster {
                verweis(&d.monster, m, &wo, "Monster", fehler);
            }
        }
        for (raum, feld) in neu {
            if let Some(r) = d.raeume.get_mut(&raum) {
                r.ausgaenge.entry(Richtung::Raus).or_insert(feld);
            }
        }
    }

    // Rückwege automatisch anlegen
    if d.welt.ausgaenge_beidseitig {
        let mut neu = Vec::new();
        for r in d.raeume.values() {
            for (richt, ziel) in &r.ausgaenge {
                if let Some(z) = d.raeume.get(ziel) {
                    if !z.ausgaenge.contains_key(&richt.gegenteil()) {
                        neu.push((ziel.clone(), richt.gegenteil(), r.id.clone()));
                    }
                }
            }
        }
        for (raum, richt, ziel) in neu {
            if let Some(r) = d.raeume.get_mut(&raum) {
                r.ausgaenge.entry(richt).or_insert(ziel);
            }
        }
    }

    // Koordinaten
    for r in d.raeume.values() {
        let k = (r.karte.clone(), r.x, r.y, r.z);
        if let Some(anderer) = d.koordinaten.get(&k) {
            warn.push(format!(
                "[{}] Raum '{}' liegt auf denselben Koordinaten ({}, {}, {}) wie '{}'. Auf der Karte ist nur einer zu sehen.",
                r.quelle, r.id, r.x, r.y, r.z, anderer
            ));
        } else {
            d.koordinaten.insert(k, r.id.clone());
        }
        for (richt, ziel) in &r.ausgaenge {
            if let Some(z) = d.raeume.get(ziel) {
                let (dx, dy, dz) = richt.versatz();
                if (dx, dy, dz) == (0, 0, 0) || z.karte != r.karte {
                    continue;
                }
                if (z.x, z.y, z.z) != (r.x + dx, r.y + dy, r.z + dz) {
                    warn.push(format!(
                        "[{}] Raum '{}': Ausgang {} führt zu '{}' ({}, {}, {}), der dort nicht liegt. Erwartet: ({}, {}, {}). Die Verbindung fehlt deshalb auf der Karte.",
                        r.quelle, r.id, richt.schluessel(), z.id, z.x, z.y, z.z, r.x + dx, r.y + dy, r.z + dz
                    ));
                }
            }
        }
    }

    // Erreichbarkeit vom Startraum
    if d.raum_existiert(&d.welt.startraum) {
        let mut gesehen = HashSet::new();
        let mut schlange = VecDeque::from([d.welt.startraum.clone()]);
        while let Some(id) = schlange.pop_front() {
            if !gesehen.insert(id.clone()) {
                continue;
            }
            schlange.extend(d.ausgaenge(&id).into_values());
        }
        for r in d.raeume.values() {
            if !gesehen.contains(&r.id) {
                warn.push(format!("[{}] Raum '{}' ist vom Startraum aus nicht erreichbar.", r.quelle, r.id));
            }
        }
    }

    // --- Gegenstände ---
    for g in d.gegenstaende.values() {
        let wo = format!("[{}] Gegenstand '{}'", g.quelle, g.id);
        if !GEGENSTAND_TYPEN.contains(&g.typ.as_str()) {
            warn.push(format!("{wo}: unbekannter Typ '{}'. Bekannt: {}", g.typ, GEGENSTAND_TYPEN.join(", ")));
        }
        if !g.platz.is_empty() && !PLAETZE.contains(&g.platz.as_str()) {
            warn.push(format!("{wo}: unbekannter Platz '{}'. Bekannt: {}", g.platz, PLAETZE.join(", ")));
        }
        if (g.typ == "waffe" || g.typ == "ruestung") && g.platz.is_empty() {
            warn.push(format!("{wo}: ist eine {} ohne 'platz' und kann darum nicht angelegt werden.", g.typ));
        }
    }

    // --- Monster ---
    for m in d.monster.values() {
        let wo = format!("[{}] Monster '{}'", m.quelle, m.id);
        for f in m.fertigkeiten.keys() {
            verweis(&d.fertigkeiten, f, &wo, "Fertigkeit", fehler);
        }
        for z in &m.zauber {
            verweis(&d.zauber, z, &wo, "Zauber", fehler);
        }
        for b in &m.beute {
            verweis(&d.gegenstaende, &b.gegenstand, &format!("{wo}, Beute"), "Gegenstand", fehler);
            if b.chance > 100 {
                fehler.push(format!("{wo}: Beute-Chance {} ist größer als 100.", b.chance));
            }
        }
        if m.wander_chance > 100 || m.sprech_chance > 100 {
            fehler.push(format!("{wo}: Chancen werden in Prozent angegeben (0 bis 100)."));
        }
        if m.lebenspunkte <= 0 {
            fehler.push(format!("{wo}: 'lebenspunkte' muss größer als 0 sein."));
        }
    }

    // --- Fertigkeiten ---
    for f in d.fertigkeiten.values() {
        let wo = format!("[{}] Fertigkeit '{}'", f.quelle, f.id);
        for v in &f.voraussetzungen {
            verweis(&d.fertigkeiten, v, &wo, "Fertigkeit", fehler);
        }
        if !f.attribut.is_empty() && !d.start.werte.contains_key(&f.attribut) {
            warn.push(format!(
                "{wo}: Attribut '{}' gibt es in [start.werte] nicht.",
                f.attribut
            ));
        }
        if f.lernchance > 100 {
            fehler.push(format!("{wo}: 'lernchance' ist eine Prozentzahl (0 bis 100)."));
        }
    }

    // --- Zauber ---
    for z in d.zauber.values() {
        let wo = format!("[{}] Zauber '{}'", z.quelle, z.id);
        if !ZAUBER_ZIELE.contains(&z.ziel.as_str()) {
            fehler.push(format!("{wo}: Ziel '{}' gibt es nicht. Erlaubt: {}", z.ziel, ZAUBER_ZIELE.join(", ")));
        }
        if !z.fertigkeit.is_empty() {
            verweis(&d.fertigkeiten, &z.fertigkeit, &wo, "Fertigkeit", fehler);
        }
        if z.wirkungen.is_empty() {
            warn.push(format!("{wo}: hat keine Wirkungen."));
        }
        for w in &z.wirkungen {
            if !WIRKUNGEN.contains(&w.art.as_str()) {
                fehler.push(format!("{wo}: Wirkung '{}' gibt es nicht. Erlaubt: {}", w.art, WIRKUNGEN.join(", ")));
            }
            if w.art == "teleport" {
                raum_verweis(d, &w.raum, &format!("{wo}, Wirkung teleport"), fehler);
            }
        }
    }
}
