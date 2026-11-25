//! Die zentrale Verwaltung aller Weltdaten.
//!
//! Hier wird festgelegt, wie Räume, Gegenstände, Monster, Fertigkeiten und Zauber
//! in den TOML-Dateien aussehen. Beim Laden wird alles gründlich geprüft, damit
//! Tippfehler sofort mit einer verständlichen Meldung auffallen.

use crate::beschreibung::{self, BeschreibungDef, Detail};
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
    pub baumeister: Vec<String>,
    pub karte_beim_umsehen: bool,
    pub karten_radius: i32,
    /// Raumbeschreibung rechts neben der Karte (sonst darunter)
    pub karte_daneben: bool,
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
    /// Name des Geldes, z.B. "Gold" oder "Taler"
    pub waehrung: String,
    /// Fertigkeit, mit der die Karte größer wird (leer = aus), z.B. "orientierung"
    pub karten_fertigkeit: String,
    /// Alle wie viele Stufen der Fertigkeit die Karte ein Feld größer wird
    pub karten_fertigkeit_stufen: u32,
    /// So groß kann die Karte durch die Fertigkeit höchstens werden (zusätzliche Felder)
    pub karten_fertigkeit_max: i32,
    /// Wie viele Berufe man gleichzeitig haben darf
    pub max_berufe: u32,
    /// Kampftechniken gibt es nur für Mitglieder einer Gilde (gilde = "..." bei jeder Technik)
    pub techniken_nur_gilden: bool,
    /// Tragkraft: so viel Gewicht trägt jeder ...
    pub tragkraft_basis: i32,
    /// ... und so viel mehr pro Punkt Stärke (beides 0 = unbegrenzt)
    pub tragkraft_pro_staerke: i32,
}

impl Default for WeltConfig {
    fn default() -> Self {
        WeltConfig {
            name: "Namenlose Welt".into(),
            port: 4000,
            startraum: String::new(),
            willkommen: String::new(),
            admins: Vec::new(),
            baumeister: Vec::new(),
            karte_beim_umsehen: true,
            karten_radius: 3,
            karte_daneben: true,
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
            waehrung: "Gold".into(),
            karten_fertigkeit: String::new(),
            karten_fertigkeit_stufen: 20,
            karten_fertigkeit_max: 3,
            max_berufe: 2,
            techniken_nur_gilden: true,
            tragkraft_basis: 40,
            tragkraft_pro_staerke: 3,
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
    pub geld: u64,
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
            geld: 0,
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
    /// Gilt für alle Räume dieser Art: Bibliothek, Gerüche, Geräusche, Gefühl
    pub nutzt: Vec<String>,
    pub geruch: String,
    pub geraeusch: String,
    pub gefuehl: String,
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
            nutzt: Vec::new(),
            geruch: String::new(),
            geraeusch: String::new(),
            gefuehl: String::new(),
        }
    }
}

/// Einstellungen für das Kampfsystem ([kampf] in welt.toml).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct KampfConfig {
    /// Spieler greifen automatisch weiter an, sobald sie bereit sind
    pub auto_angriff: bool,
    pub waffenlos_schaden: i32,
    /// Sekunden zwischen zwei Angriffen ohne Waffe
    pub waffenlos_erholung: f64,
    /// Fertigkeit, die ohne Waffe benutzt und geübt wird (leer = keine)
    pub waffenlos_fertigkeit: String,
    /// Sekunden, die man nach einem Kampfzauber nicht angreifen kann
    pub zauber_erholung: f64,
    pub ausdauer: i32,
    pub ausdauer_pro_angriff: i32,
    /// Erholung der Ausdauer pro Takt
    pub ausdauer_erholung: i32,
    /// Erfahrung für die nächste Stufe: ep_basis * stufe ^ ep_faktor
    pub ep_basis: f64,
    pub ep_faktor: f64,
    pub lp_pro_stufe: i32,
    pub mp_pro_stufe: i32,
    pub ausdauer_pro_stufe: i32,
    /// Punkte, die man pro Stufe auf Werte verteilen kann (Befehl: steigere)
    pub punkte_pro_stufe: u32,
    /// Hier wacht man nach dem Tod auf (leer = Startraum)
    pub wiederbelebung_raum: String,
    pub wiederbelebung_lp_prozent: i32,
    /// So lange (Sekunden) können andere Spieler einen Toten wiederbeleben,
    /// danach erwacht er von selbst im Wiederbelebungsraum
    pub tod_wartezeit: u64,
    /// Wie viel Prozent der Erfahrung der aktuellen Stufe man beim Tod verliert
    pub tod_ep_verlust_prozent: u32,
    /// "behalten" oder "fallen_lassen" (Gepäck bleibt am Ort des Todes liegen)
    pub tod_gegenstaende: String,
    /// Monster erholen sich außerhalb des Kampfes um so viel Prozent pro Takt
    pub monster_erholung_prozent: i32,
}

impl Default for KampfConfig {
    fn default() -> Self {
        KampfConfig {
            auto_angriff: true,
            waffenlos_schaden: 2,
            waffenlos_erholung: 2.5,
            waffenlos_fertigkeit: String::new(),
            zauber_erholung: 2.0,
            ausdauer: 100,
            ausdauer_pro_angriff: 5,
            ausdauer_erholung: 3,
            ep_basis: 100.0,
            ep_faktor: 1.5,
            lp_pro_stufe: 10,
            mp_pro_stufe: 5,
            ausdauer_pro_stufe: 5,
            punkte_pro_stufe: 2,
            wiederbelebung_raum: String::new(),
            wiederbelebung_lp_prozent: 50,
            tod_wartezeit: 300,
            tod_ep_verlust_prozent: 0,
            tod_gegenstaende: "behalten".into(),
            monster_erholung_prozent: 5,
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
    #[serde(default)]
    namen: BTreeMap<String, String>,
    #[serde(default)]
    kampf: KampfConfig,
    /// Zusätzliche Angaben für MUD-Verzeichnisse (MSSP), z.B. GENRE = "Fantasy"
    #[serde(default)]
    mssp: BTreeMap<String, String>,
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
    pub details: BTreeMap<String, Detail>,
    /// Was man riecht, hört und fühlt (Befehle riech, hoer, taste)
    #[serde(default)]
    pub geruch: String,
    #[serde(default)]
    pub geraeusch: String,
    #[serde(default)]
    pub gefuehl: String,
    /// Allgemeine Beschreibungen aus der Bibliothek (daten/beschreibungen)
    #[serde(default)]
    pub nutzt: Vec<String>,
    #[serde(default)]
    pub gegenstaende: Vec<String>,
    #[serde(default)]
    pub monster: Vec<String>,
    /// NPCs, die hier wohnen (aus daten/npcs)
    #[serde(default)]
    pub npcs: Vec<String>,

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
    /// "aus einem roten Beutel" – für Behälter (leer = aus dem Namen geraten)
    #[serde(default)]
    pub dativ: String,
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
    /// Waffe: Sekunden zwischen zwei Angriffen (schnelle Waffen < 2, schwere > 3)
    #[serde(default)]
    pub erholung: Option<f64>,
    /// Waffe: Fertigkeit, die benutzt und geübt wird, z.B. "schwertkampf"
    #[serde(default)]
    pub fertigkeit: String,
    #[serde(default)]
    pub eigenschaften: BTreeMap<String, i32>,
    #[serde(default)]
    pub benutzen_text: String,
    /// Behälter: so viel Gewicht passt hinein (0 = kein Behälter)
    #[serde(default)]
    pub fassungsvermoegen: i32,
    /// Behälter: höchstens so viele Dinge (0 = beliebig viele)
    #[serde(default)]
    pub max_dinge: u32,
    /// Behälter: um so viel Prozent wird der Inhalt leichter (z.B. 50)
    #[serde(default)]
    pub gewicht_reduktion: u32,
    /// Behälter: nur diese Typen passen hinein, z.B. ["trank"] (leer = alles)
    #[serde(default)]
    pub nur_typen: Vec<String>,
    #[serde(default)]
    pub details: BTreeMap<String, Detail>,
    /// Was man riecht, hört und fühlt (Befehle riech, hoer, taste)
    #[serde(default)]
    pub geruch: String,
    #[serde(default)]
    pub geraeusch: String,
    #[serde(default)]
    pub gefuehl: String,
    /// Allgemeine Beschreibungen aus der Bibliothek (daten/beschreibungen)
    #[serde(default)]
    pub nutzt: Vec<String>,

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
    /// "dem Wirt" – für Gesten wie "Du winkst {ziel_dativ} zu."
    #[serde(default)]
    pub dativ: String,
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
    /// Grundschaden eines Treffers (ohne Angabe: 2 + 2 * stufe)
    #[serde(default)]
    pub schaden: Option<i32>,
    #[serde(default)]
    pub ruestung: i32,
    /// Sekunden zwischen zwei Angriffen
    #[serde(default = "standard_monster_erholung")]
    pub erholung: f64,
    /// Wie das Monster angreift: "beißt", "kratzt" ... (wird zufällig gewählt)
    #[serde(default)]
    pub angriffe: Vec<String>,
    /// Erfahrung für den Sieg (ohne Angabe: aus Stufe und Lebenspunkten berechnet)
    #[serde(default)]
    pub erfahrung: Option<u64>,
    /// Kann man es angreifen? (ohne Angabe: Monster ja, NPCs nein)
    #[serde(default)]
    pub angreifbar: Option<bool>,
    /// Was beim Tod gesagt wird, z.B. "*bricht röchelnd zusammen."
    #[serde(default)]
    pub todestext: String,
    #[serde(default)]
    pub wandert: bool,
    #[serde(default = "zehn_u")]
    pub wander_chance: u32,
    #[serde(default)]
    pub sprueche: Vec<String>,
    #[serde(default = "fuenf_u")]
    pub sprech_chance: u32,
    /// Antworten auf Standardfragen (hallo, beruf, aufgabe ...)
    #[serde(default)]
    pub gespraech: crate::gespraech::GespraechDef,
    /// Eigene Gesprächsthemen
    #[serde(default)]
    pub thema: Vec<crate::gespraech::ThemaDef>,
    /// Geld, das man beim Sieg bekommt: eine Zahl oder [von, bis]
    #[serde(default)]
    pub geld: GeldDef,
    /// Der NPC verkauft und kauft Gegenstände
    #[serde(default)]
    pub haendler: Option<HaendlerDef>,
    /// Der NPC lehrt Fertigkeiten und Zauber gegen Geld
    #[serde(default)]
    pub lehrt: Vec<LehrDef>,
    /// Der NPC nimmt in diese Gilde auf (Befehl: beitreten)
    #[serde(default)]
    pub gilde: String,
    /// Der NPC nimmt in diesen Beruf / diese Zunft auf (Befehl: beitreten)
    #[serde(default)]
    pub beruf: String,
    /// Steht in einer eigenen NPC-Datei ([npc])
    #[serde(skip)]
    pub ist_npc: bool,
    #[serde(default)]
    pub details: BTreeMap<String, Detail>,
    /// Was man riecht, hört und fühlt (Befehle riech, hoer, taste)
    #[serde(default)]
    pub geruch: String,
    #[serde(default)]
    pub geraeusch: String,
    #[serde(default)]
    pub gefuehl: String,
    /// Allgemeine Beschreibungen aus der Bibliothek (daten/beschreibungen)
    #[serde(default)]
    pub nutzt: Vec<String>,

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
    /// Sekunden, die die Wirkung anhält (0 = sofort und einmalig)
    #[serde(default)]
    pub dauer: f64,
    /// Bei heilen/mana/ausdauer/schaden mit Dauer: alle wie viele Sekunden (Standard 3)
    #[serde(default)]
    pub intervall: Option<f64>,
    /// Bei "bonus": welcher Wert, z.B. "staerke"
    #[serde(default)]
    pub attribut: String,
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
    /// Wenn eine Wirkung mit Dauer endet, z.B. "Dein Schild verblasst."
    #[serde(default)]
    pub text_ende: String,
    /// "zauber" oder "technik" (Kampftechnik, Befehl: spezial)
    #[serde(default = "standard_zaubertyp")]
    pub typ: String,
    #[serde(default)]
    pub ausdauer_kosten: i32,
    /// Technik: Schaden der Waffe mal diesen Faktor (z.B. 1.5); 0 = kein Waffenschlag
    #[serde(default)]
    pub waffen_faktor: f64,
    /// Nur für Mitglieder dieser Gilde (leer = für alle)
    #[serde(default)]
    pub gilde: String,

    #[serde(skip)]
    pub quelle: String,
}

impl ZauberDef {
    pub fn ist_technik(&self) -> bool {
        self.typ == "technik"
    }
}

fn standard_zaubertyp() -> String {
    "zauber".into()
}

/// Geld als feste Zahl (geld = 5) oder Bereich (geld = [2, 8]).
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(untagged)]
pub enum GeldDef {
    #[default]
    Keins,
    Fest(u64),
    Bereich(Vec<u64>),
}

impl GeldDef {
    pub fn wuerfeln(&self) -> u64 {
        use rand::Rng;
        match self {
            GeldDef::Keins => 0,
            GeldDef::Fest(n) => *n,
            GeldDef::Bereich(v) => match v.as_slice() {
                [] => 0,
                [n] => *n,
                [a, b, ..] => rand::thread_rng().gen_range((*a).min(*b)..=(*a).max(*b)),
            },
        }
    }
}

/// [npc.haendler]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HaendlerDef {
    /// Was der Händler verkauft (Gegenstand-IDs, unbegrenzter Vorrat)
    #[serde(default)]
    pub waren: Vec<String>,
    /// Kauft er Gegenstände an?
    #[serde(default = "wahr")]
    pub ankauf: bool,
    /// Nur diese Typen ankaufen, z.B. ["waffe", "ruestung"] (leer = alles)
    #[serde(default)]
    pub ankauf_typen: Vec<String>,
    /// Verkaufspreis in Prozent vom Wert (100 = genau der Wert)
    #[serde(default = "hundert")]
    pub verkauf_prozent: u32,
    /// Ankaufspreis in Prozent vom Wert
    #[serde(default = "fuenfzig")]
    pub ankauf_prozent: u32,
}

/// [[npc.lehrt]]: eine Fertigkeit oder ein Zauber, den der NPC lehrt.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LehrDef {
    /// ID einer Fertigkeit oder eines Zaubers
    pub id: String,
    /// Preis pro Lektion (Fertigkeit: +1 Stufe) bzw. für den Zauber
    #[serde(default)]
    pub preis: u64,
    /// Fertigkeiten: bis zu dieser Stufe kann man hier lernen
    #[serde(default)]
    pub bis: Option<u32>,
    /// Ab dieser Spielerstufe
    #[serde(default)]
    pub stufe: u32,
    /// Nur für Mitglieder dieser Gilde (leer = für alle)
    #[serde(default)]
    pub gilde: String,
}

/// Rasse, Gilde oder Beruf ([[rasse]], [[gilde]], [[beruf]]).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GemeinschaftDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub beschreibung: String,
    /// Zuschläge auf die Werte, z.B. { staerke = 2, intelligenz = -1 }
    #[serde(default)]
    pub werte: BTreeMap<String, i32>,
    #[serde(default)]
    pub lebenspunkte: i32,
    #[serde(default)]
    pub mana: i32,
    #[serde(default)]
    pub ausdauer: i32,
    /// Fertigkeiten, die man bekommt: { schwertkampf = 5 }
    #[serde(default)]
    pub fertigkeiten: BTreeMap<String, u32>,
    #[serde(default)]
    pub zauber: Vec<String>,
    /// Gilde/Beruf: ab dieser Spielerstufe
    #[serde(default)]
    pub aufnahme_stufe: u32,
    /// Gilde/Beruf: nur für diese Rassen (leer = alle)
    #[serde(default)]
    pub rassen: Vec<String>,
    /// Gilde/Beruf: Aufnahmegebühr
    #[serde(default)]
    pub preis: u64,
    /// Gilde: Titel für Mitglieder, z.B. "der Krieger"
    #[serde(default)]
    pub titel: String,
    /// Text bei der Aufnahme
    #[serde(default)]
    pub aufnahme_text: String,
    /// Rasse: bei der Erschaffung wählbar?
    #[serde(default = "wahr")]
    pub waehlbar: bool,

    #[serde(skip)]
    pub quelle: String,
}

/// [[quest]]: eine Aufgabe (klein) oder ein Quest (groß).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestDef {
    pub id: String,
    pub name: String,
    /// "aufgabe" (klein) oder "quest"
    #[serde(default = "standard_questart")]
    pub art: String,
    #[serde(default)]
    pub beschreibung: String,
    /// NPC, der die Aufgabe vergibt und bei dem man sie abgibt
    pub geber: String,
    #[serde(default)]
    pub mindest_stufe: u32,
    /// Diese Quests muss man vorher erledigt haben
    #[serde(default)]
    pub voraussetzungen: Vec<String>,
    /// Nur für Mitglieder dieser Gilde
    #[serde(default)]
    pub gilde: String,
    #[serde(default)]
    pub wiederholbar: bool,
    /// Was der NPC sagt, wenn er die Aufgabe anbietet
    #[serde(default)]
    pub angebot: String,
    /// ... wenn man sie annimmt
    #[serde(default)]
    pub annahme: String,
    /// ... wenn man sie abgibt
    #[serde(default)]
    pub abschluss: String,
    /// Muss man zum Geber zurück? (false = fertig, sobald alle Schritte erledigt sind)
    #[serde(default = "wahr")]
    pub abgabe: bool,
    #[serde(default)]
    pub schritte: Vec<SchrittDef>,
    #[serde(default)]
    pub belohnung: BelohnungDef,

    #[serde(skip)]
    pub quelle: String,
}

fn standard_questart() -> String {
    "quest".into()
}

pub const QUEST_ARTEN: &[&str] = &["aufgabe", "quest"];
pub const SCHRITT_ARTEN: &[&str] = &["toeten", "bringen", "besuchen", "sprechen"];

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchrittDef {
    /// toeten, bringen, besuchen, sprechen
    pub art: String,
    /// Monster-, Gegenstand-, Raum- oder NPC-ID
    pub ziel: String,
    #[serde(default = "eins_u")]
    pub anzahl: u32,
    /// bringen: an welchen NPC (leer = der Geber)
    #[serde(default)]
    pub an: String,
    /// Was im Aufgabenbuch steht, z.B. "Töte 3 Ratten im Keller"
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct BelohnungDef {
    pub erfahrung: u64,
    pub geld: u64,
    pub gegenstaende: Vec<String>,
    /// Fertigkeiten steigen um so viele Stufen: { kraeuterkunde = 2 }
    pub fertigkeiten: BTreeMap<String, u32>,
    pub zauber: Vec<String>,
    pub titel: String,
}

fn fuenfzig() -> u32 {
    50
}

fn standard_monster_erholung() -> f64 {
    3.0
}

impl MonsterDef {
    pub fn kampf_schaden(&self) -> i32 {
        self.schaden.unwrap_or(2 + 2 * self.stufe as i32)
    }
    pub fn kampf_erfahrung(&self) -> u64 {
        self.erfahrung.unwrap_or(10 * (self.stufe as u64).pow(2) + self.lebenspunkte.max(0) as u64 / 2)
    }
    pub fn ist_angreifbar(&self) -> bool {
        self.angreifbar.unwrap_or(!self.ist_npc)
    }
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


// ---------------------------------------------------------------------------
// Die geladenen Daten
// ---------------------------------------------------------------------------

pub const GEGENSTAND_TYPEN: &[&str] =
    &["waffe", "ruestung", "trank", "nahrung", "schluessel", "werkzeug", "schmuck", "behaelter", "sonstiges"];
pub const PLAETZE: &[&str] = &[
    "waffe", "schild", "kopf", "koerper", "haende", "beine", "fuesse", "hals", "ring", "umhang", "licht", "ruecken", "guertel",
];
pub const ZAUBER_ZIELE: &[&str] = &["selbst", "spieler", "gegner", "raum"];
pub const WIRKUNGEN: &[&str] = &[
    "heilen", "mana", "ausdauer", "schaden", "teleport", "schild", "ruestung", "bonus", "kraft", "tempo", "betaeuben",
];
/// Wirkungen, die nur mit einer Dauer Sinn ergeben
pub const WIRKUNGEN_MIT_DAUER: &[&str] = &["schild", "ruestung", "bonus", "kraft", "tempo"];
pub const ZAUBER_TYPEN: &[&str] = &["zauber", "technik"];

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
    pub rassen: BTreeMap<String, GemeinschaftDef>,
    pub gilden: BTreeMap<String, GemeinschaftDef>,
    pub berufe: BTreeMap<String, GemeinschaftDef>,
    pub quests: BTreeMap<String, QuestDef>,
    /// Welcher Raum liegt an welcher Koordinate?
    /// (karte, x, y, z) -> Raum
    pub koordinaten: HashMap<(String, i32, i32, i32), String>,
    /// Farben der Verzierungen: Zeichen -> Farbe
    pub deko_farben: BTreeMap<String, String>,
    /// Anzeigenamen für IDs, z.B. staerke -> "Stärke"
    pub namen: BTreeMap<String, String>,
    pub kampf: KampfConfig,
    /// Zusätzliche Serverinfos für MUD-Verzeichnisse ([mssp] in welt.toml)
    pub mssp: BTreeMap<String, String>,
    /// Befehle und Gesten aus befehle.toml / befehle/*.toml
    pub befehle: crate::befehlskonfig::Befehle,
    pub oberwelt: Option<Oberwelt>,
    /// Allgemeine Beschreibungen (Bäume, Himmel, Steine ...), auf die man mit `nutzt` verweist
    pub bibliothek: BTreeMap<String, BeschreibungDef>,
    pub warnungen: Vec<String>,
}

impl Daten {
    /// Alle Bibliotheks-Einträge, die über `nutzt` erreichbar sind (auch über mehrere Stufen),
    /// als Details zum Durchsuchen.
    pub fn bibliothek_fuer(&self, ids: &[String]) -> BTreeMap<String, Detail> {
        let mut aus = BTreeMap::new();
        let mut offen: Vec<String> = ids.to_vec();
        while let Some(id) = offen.pop() {
            if aus.contains_key(&id) {
                continue;
            }
            if let Some(b) = self.bibliothek.get(&id) {
                offen.extend(b.nutzt.iter().cloned());
                let mut innen = Vec::new();
                beschreibung::alle_verweise(&b.details, &mut innen);
                offen.extend(innen);
                aus.insert(id, b.als_detail());
            }
        }
        aus
    }

    /// Wonach man in einem Raum suchen kann: eigene Details + Bibliothek
    /// (vom Raum, seinem Gelände und seinen Details).
    pub fn raum_kontext(&self, r: &RaumDef) -> BTreeMap<String, Detail> {
        let mut ids = r.nutzt.clone();
        if let Some(g) = self.gelaende.get(&r.gelaende) {
            ids.extend(g.nutzt.iter().cloned());
        }
        beschreibung::alle_verweise(&r.details, &mut ids);
        self.bibliothek_fuer(&ids)
    }

    /// Geruch/Geräusch/Gefühl eines Raums – sonst das seines Geländes.
    pub fn raum_sinn(&self, r: &RaumDef, s: beschreibung::Sinn) -> String {
        use beschreibung::Sinn::*;
        let eigen = match s {
            Riechen => &r.geruch,
            Hoeren => &r.geraeusch,
            Tasten => &r.gefuehl,
        };
        if !eigen.is_empty() {
            return eigen.clone();
        }
        self.gelaende
            .get(&r.gelaende)
            .map(|g| match s {
                Riechen => g.geruch.clone(),
                Hoeren => g.geraeusch.clone(),
                Tasten => g.gefuehl.clone(),
            })
            .unwrap_or_default()
    }

    /// Schöner Name für eine ID (aus [namen] in welt.toml), sonst die ID mit großem Anfangsbuchstaben.
    pub fn anzeigename(&self, id: &str) -> String {
        self.namen.get(id).cloned().unwrap_or_else(|| text::gross(id))
    }

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
        rassen: BTreeMap::new(),
        gilden: BTreeMap::new(),
        berufe: BTreeMap::new(),
        quests: BTreeMap::new(),
        koordinaten: HashMap::new(),
        deko_farben: wd.deko,
        namen: wd.namen,
        kampf: wd.kampf,
        mssp: wd.mssp,
        befehle: Default::default(),
        oberwelt: None,
        bibliothek: BTreeMap::new(),
        warnungen: Vec::new(),
    };
    let mut fehler = Vec::new();
    let mut warnungen = Vec::new();

    let mut dateien = Vec::new();
    toml_dateien_sammeln(ordner, &mut dateien);
    dateien.sort();


    // 1. Alle Dateien als rohe Tabellen einlesen
    let mut roh: Vec<Roh> = Vec::new();
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
        let tabelle: toml::Table = match toml::from_str(&inhalt) {
            Ok(t) => t,
            Err(e) => {
                fehler.push(format!("Fehler in '{datei}':\n{}", toml_fehler_uebersetzen(&e)));
                continue;
            }
        };
        roh_einlesen(tabelle, &datei, &mut roh, &mut fehler);
    }

    // 2. Vererbung auflösen (erbt = "..."), Vorlagen aussortieren
    let fertig = vererbung_aufloesen(&roh, &mut fehler);

    // 3. In die richtigen Typen umwandeln und prüfen
    let mut raeume: Vec<(String, RaumDef)> = Vec::new();
    let mut gegenstaende: Vec<(String, GegenstandDef)> = Vec::new();
    let mut monster: Vec<(String, MonsterDef)> = Vec::new();
    let mut npcs: Vec<(String, MonsterDef)> = Vec::new();
    let mut fertigkeiten: Vec<(String, FertigkeitDef)> = Vec::new();
    let mut zauber: Vec<(String, ZauberDef)> = Vec::new();
    let mut bibliothek: Vec<(String, BeschreibungDef)> = Vec::new();
    let mut rassen: Vec<(String, GemeinschaftDef)> = Vec::new();
    let mut gilden: Vec<(String, GemeinschaftDef)> = Vec::new();
    let mut berufe: Vec<(String, GemeinschaftDef)> = Vec::new();
    let mut quests: Vec<(String, QuestDef)> = Vec::new();
    for (i, t) in fertig {
        let r = &roh[i];
        let id = t.get("id").and_then(|v| v.as_str()).unwrap_or("?").to_string();
        let wo = format!("[{}] {} '{}'", r.datei, art_anzeige(r.art), id);
        let wert = toml::Value::Table(t);
        macro_rules! umwandeln {
            ($liste:expr) => {
                match wert.try_into() {
                    Ok(e) => $liste.push((r.datei.clone(), e)),
                    Err(e) => fehler.push(format!("{wo}: {}", toml_fehler_uebersetzen(&e))),
                }
            };
        }
        match r.art {
            "raum" => umwandeln!(raeume),
            "gegenstand" => umwandeln!(gegenstaende),
            "monster" => umwandeln!(monster),
            "npc" => umwandeln!(npcs),
            "fertigkeit" => umwandeln!(fertigkeiten),
            "zauber" => umwandeln!(zauber),
            "rasse" => umwandeln!(rassen),
            "gilde" => umwandeln!(gilden),
            "beruf" => umwandeln!(berufe),
            "quest" => umwandeln!(quests),
            _ => umwandeln!(bibliothek),
        }
    }
    macro_rules! einsortieren {
        ($liste:expr, $ziel:expr, $art:expr) => {
            for (datei, mut e) in $liste {
                e.quelle = datei.clone();
                id_pruefen(&e.id, $art, &datei, &mut fehler);
                if $ziel.contains_key(&e.id) {
                    fehler.push(format!("[{}] {} '{}' gibt es doppelt.", datei, $art, e.id));
                } else {
                    $ziel.insert(e.id.clone(), e);
                }
            }
        };
    }
    let npcs: Vec<(String, MonsterDef)> = npcs
        .into_iter()
        .map(|(d, mut n)| {
            n.ist_npc = true;
            (d, n)
        })
        .collect();
    einsortieren!(raeume, d.raeume, "Raum");
    einsortieren!(gegenstaende, d.gegenstaende, "Gegenstand");
    einsortieren!(monster, d.monster, "Monster");
    einsortieren!(npcs, d.monster, "NPC");
    einsortieren!(fertigkeiten, d.fertigkeiten, "Fertigkeit");
    einsortieren!(zauber, d.zauber, "Zauber");
    einsortieren!(bibliothek, d.bibliothek, "Beschreibung");
    einsortieren!(rassen, d.rassen, "Rasse");
    einsortieren!(gilden, d.gilden, "Gilde");
    einsortieren!(berufe, d.berufe, "Beruf");
    einsortieren!(quests, d.quests, "Quest");

    d.oberwelt = oberwelt::laden(ordner, &mut fehler);
    d.befehle = crate::befehlskonfig::laden(ordner, &mut fehler, &mut warnungen);
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
    // Bei einem vertippten Feldnamen das passende Feld vorschlagen
    if let Some(rest) = t.split("unknown field `").nth(1) {
        if let Some((falsch, erlaubt)) = rest.split_once("`, expected one of ") {
            let felder: Vec<String> = erlaubt
                .split(',')
                .map(|f| f.trim().trim_matches('`').to_string())
                .filter(|f| !f.is_empty())
                .collect();
            if let Some(v) = text::vorschlag(falsch, felder.iter()) {
                t = format!("Meintest du '{v}'? {t}");
            }
        }
    }
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

// ---------------------------------------------------------------------------
// Rohe Einträge und Vererbung
// ---------------------------------------------------------------------------

/// Ein Eintrag, wie er in der Datei steht – noch ohne Vererbung.
struct Roh {
    art: &'static str,
    datei: String,
    t: toml::Table,
}

const ARTEN: &[&str] =
    &["raum", "gegenstand", "monster", "npc", "fertigkeit", "zauber", "beschreibung", "rasse", "gilde", "beruf", "quest"];

fn art_anzeige(art: &str) -> &'static str {
    match art {
        "raum" => "Raum",
        "gegenstand" => "Gegenstand",
        "monster" => "Monster",
        "npc" => "NPC",
        "fertigkeit" => "Fertigkeit",
        "zauber" => "Zauber",
        "rasse" => "Rasse",
        "gilde" => "Gilde",
        "beruf" => "Beruf",
        "quest" => "Quest",
        _ => "Beschreibung",
    }
}

/// NPCs dürfen von Monstern erben (und umgekehrt): Sie teilen sich einen Namensraum.
fn art_gruppe(art: &str) -> &str {
    if art == "npc" {
        "monster"
    } else {
        art
    }
}

fn roh_einlesen(tabelle: toml::Table, datei: &str, roh: &mut Vec<Roh>, fehler: &mut Vec<String>) {
    for (schluessel, wert) in tabelle {
        let Some(art) = ARTEN.iter().copied().find(|a| *a == schluessel) else {
            let namen: Vec<String> = ARTEN.iter().map(|a| a.to_string()).collect();
            let tipp = text::vorschlag(&schluessel, namen.iter()).map(|v| format!(" Meintest du [[{v}]]?")).unwrap_or_default();
            fehler.push(format!("[{datei}] Unbekannter Abschnitt '{schluessel}'.{tipp}"));
            continue;
        };
        match (art, wert) {
            ("npc", toml::Value::Table(t)) => roh.push(Roh { art, datei: datei.to_string(), t }),
            ("npc", _) => fehler.push(format!("[{datei}] Ein NPC steht als [npc] in seiner eigenen Datei.")),
            (_, toml::Value::Array(liste)) => {
                for e in liste {
                    match e {
                        toml::Value::Table(t) => roh.push(Roh { art, datei: datei.to_string(), t }),
                        _ => fehler.push(format!("[{datei}] Einträge müssen als [[{art}]] geschrieben werden.")),
                    }
                }
            }
            _ => fehler.push(format!("[{datei}] Bitte [[{art}]] (mit doppelten Klammern) schreiben.")),
        }
    }
}

/// Mischt eine Tabelle in eine andere: Werte des Kindes gewinnen,
/// Untertabellen (z.B. eigenschaften, gespraech) werden zusammengeführt.
fn mischen(mut eltern: toml::Table, kind: toml::Table) -> toml::Table {
    for (k, v) in kind {
        match (eltern.remove(&k), v) {
            (Some(toml::Value::Table(a)), toml::Value::Table(b)) => {
                eltern.insert(k, toml::Value::Table(mischen(a, b)));
            }
            (_, v) => {
                eltern.insert(k, v);
            }
        }
    }
    eltern
}

/// Löst `erbt = "..."` auf. Gibt (Index, fertige Tabelle) aller echten Einträge zurück;
/// Vorlagen (`vorlage = true`) werden nur zum Erben benutzt.
fn vererbung_aufloesen(roh: &[Roh], fehler: &mut Vec<String>) -> Vec<(usize, toml::Table)> {
    let id_von = |r: &Roh| r.t.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let mut index: HashMap<(String, String), usize> = HashMap::new();
    for (i, r) in roh.iter().enumerate() {
        index.entry((art_gruppe(r.art).to_string(), id_von(r))).or_insert(i);
    }

    fn aufloesen(
        i: usize,
        roh: &[Roh],
        index: &HashMap<(String, String), usize>,
        fertig: &mut Vec<Option<toml::Table>>,
        kette: &mut Vec<usize>,
        fehler: &mut Vec<String>,
    ) -> Option<toml::Table> {
        if let Some(t) = &fertig[i] {
            return Some(t.clone());
        }
        let r = &roh[i];
        let id = r.t.get("id").and_then(|v| v.as_str()).unwrap_or("?").to_string();
        let wo = format!("[{}] {} '{}'", r.datei, art_anzeige(r.art), id);
        let mut t = r.t.clone();
        let erbt = t.remove("erbt");
        let ergebnis = match erbt {
            None => t,
            Some(toml::Value::String(eltern_id)) => {
                if r.art == "raum" {
                    fehler.push(format!("{wo}: Räume können nicht erben."));
                    return None;
                }
                let Some(&j) = index.get(&(art_gruppe(r.art).to_string(), eltern_id.clone())) else {
                    let kandidaten: Vec<String> = index
                        .keys()
                        .filter(|(g, _)| g == art_gruppe(r.art))
                        .map(|(_, id)| id.clone())
                        .collect();
                    let tipp = text::vorschlag(&eltern_id, kandidaten.iter())
                        .map(|v| format!(" Meintest du '{v}'?"))
                        .unwrap_or_default();
                    fehler.push(format!("{wo}: erbt von '{eltern_id}', den es nicht gibt.{tipp}"));
                    return None;
                };
                if kette.contains(&j) || j == i {
                    fehler.push(format!("{wo}: Die Vererbung dreht sich im Kreis (über '{eltern_id}')."));
                    return None;
                }
                kette.push(i);
                let eltern = aufloesen(j, roh, index, fertig, kette, fehler);
                kette.pop();
                let mut eltern = eltern?;
                eltern.remove("vorlage");
                mischen(eltern, t)
            }
            Some(_) => {
                fehler.push(format!("{wo}: 'erbt' muss eine ID in Anführungszeichen sein."));
                return None;
            }
        };
        fertig[i] = Some(ergebnis.clone());
        Some(ergebnis)
    }

    let mut fertig: Vec<Option<toml::Table>> = vec![None; roh.len()];
    let mut aus = Vec::new();
    for i in 0..roh.len() {
        let Some(mut t) = aufloesen(i, roh, &index, &mut fertig, &mut Vec::new(), fehler) else { continue };
        match t.remove("vorlage") {
            Some(toml::Value::Boolean(true)) => continue, // nur zum Erben da
            Some(toml::Value::Boolean(false)) | None => {}
            Some(_) => {
                fehler.push(format!("[{}] 'vorlage' muss true oder false sein.", roh[i].datei));
                continue;
            }
        }
        aus.push((i, t));
    }
    aus
}

fn toml_dateien_sammeln(ordner: &Path, liste: &mut Vec<PathBuf>) {
    let Ok(eintraege) = fs::read_dir(ordner) else { return };
    for e in eintraege.flatten() {
        let p = e.path();
        if p.is_dir() {
            toml_dateien_sammeln(&p, liste);
        } else if p.extension().is_some_and(|x| x == "toml")
            && !p.ends_with("oberwelt/oberwelt.toml")
            && !p.ends_with("befehle.toml")
            && !p.parent().is_some_and(|o| o.ends_with("befehle"))
        {
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
    {
        let k = &d.kampf;
        if !["behalten", "fallen_lassen"].contains(&k.tod_gegenstaende.as_str()) {
            fehler.push("welt.toml [kampf]: 'tod_gegenstaende' muss \"behalten\" oder \"fallen_lassen\" sein.".into());
        }
        if !k.wiederbelebung_raum.is_empty() {
            raum_verweis(d, &k.wiederbelebung_raum, "welt.toml [kampf] (wiederbelebung_raum)", fehler);
        }
        if !k.waffenlos_fertigkeit.is_empty() {
            verweis(&d.fertigkeiten, &k.waffenlos_fertigkeit, "welt.toml [kampf]", "Fertigkeit", fehler);
        }
        if k.ep_basis <= 0.0 || k.waffenlos_erholung <= 0.0 {
            fehler.push("welt.toml [kampf]: 'ep_basis' und 'waffenlos_erholung' müssen größer als 0 sein.".into());
        }
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
        for n in &r.npcs {
            verweis(&d.monster, n, &wo, "NPC", fehler);
        }
        if r.beschreibung.trim().is_empty() {
            warn.push(format!("{wo}: hat keine Beschreibung."));
        }
    }
    // NPCs leben wie Monster in den Räumen
    for r in d.raeume.values_mut() {
        let npcs = std::mem::take(&mut r.npcs);
        r.monster.extend(npcs);
    }
    for (id, aus) in ausgaenge_neu {
        if let Some(r) = d.raeume.get_mut(&id) {
            r.ausgaenge = aus;
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

    // --- Bibliothek: Stimmen alle Verweise (nutzt)? ---
    {
        let mut pruefe = |ids: Vec<String>, wo: String| {
            for id in ids {
                verweis(&d.bibliothek, &id, &wo, "Beschreibung", fehler);
            }
        };
        let mit_details = |nutzt: &[String], details: &BTreeMap<String, Detail>| {
            let mut ids = nutzt.to_vec();
            beschreibung::alle_verweise(details, &mut ids);
            ids
        };
        for r in d.raeume.values() {
            pruefe(mit_details(&r.nutzt, &r.details), format!("[{}] Raum '{}', nutzt", r.quelle, r.id));
        }
        for (id, g) in &d.gelaende {
            pruefe(g.nutzt.clone(), format!("welt.toml [gelaende.{id}], nutzt"));
        }
        for g in d.gegenstaende.values() {
            pruefe(mit_details(&g.nutzt, &g.details), format!("[{}] Gegenstand '{}', nutzt", g.quelle, g.id));
        }
        for m in d.monster.values() {
            pruefe(mit_details(&m.nutzt, &m.details), format!("[{}] Monster '{}', nutzt", m.quelle, m.id));
        }
        for b in d.bibliothek.values() {
            pruefe(mit_details(&b.nutzt, &b.details), format!("[{}] Beschreibung '{}', nutzt", b.quelle, b.id));
        }
        if let Some(ow) = &d.oberwelt {
            for (z, f) in &ow.felder {
                pruefe(f.nutzt.clone(), format!("oberwelt.toml [feld.\"{z}\"], nutzt"));
            }
            for o in &ow.orte {
                pruefe(mit_details(&o.nutzt, &o.details), format!("oberwelt.toml, Ort '{}', nutzt", o.name));
            }
        }
    }

    // --- Gegenstände ---
    for g in d.gegenstaende.values() {
        let wo = format!("[{}] Gegenstand '{}'", g.quelle, g.id);
        if !g.fertigkeit.is_empty() {
            verweis(&d.fertigkeiten, &g.fertigkeit, &wo, "Fertigkeit", fehler);
        }
        if g.erholung.is_some_and(|e| e <= 0.0) {
            fehler.push(format!("{wo}: 'erholung' muss größer als 0 sein."));
        }
        if !GEGENSTAND_TYPEN.contains(&g.typ.as_str()) {
            warn.push(format!("{wo}: unbekannter Typ '{}'. Bekannt: {}", g.typ, GEGENSTAND_TYPEN.join(", ")));
        }
        if !g.platz.is_empty() && !PLAETZE.contains(&g.platz.as_str()) {
            warn.push(format!("{wo}: unbekannter Platz '{}'. Bekannt: {}", g.platz, PLAETZE.join(", ")));
        }
        if g.fassungsvermoegen < 0 || g.gewicht_reduktion > 100 {
            fehler.push(format!("{wo}: 'fassungsvermoegen' darf nicht negativ sein, 'gewicht_reduktion' ist 0 bis 100 (Prozent)."));
        }
        if g.fassungsvermoegen == 0 && (g.max_dinge > 0 || g.gewicht_reduktion > 0 || !g.nur_typen.is_empty()) {
            warn.push(format!("{wo}: hat Behälter-Angaben, aber kein 'fassungsvermoegen' – es passt nichts hinein."));
        }
        for t in &g.nur_typen {
            if !GEGENSTAND_TYPEN.contains(&t.as_str()) {
                warn.push(format!("{wo}: unbekannter Typ '{t}' in nur_typen."));
            }
        }
        if (g.typ == "waffe" || g.typ == "ruestung") && g.platz.is_empty() {
            warn.push(format!("{wo}: ist eine {} ohne 'platz' und kann darum nicht angelegt werden.", g.typ));
        }
    }

    // --- Monster ---
    for m in d.monster.values() {
        let wo = format!("[{}] Monster '{}'", m.quelle, m.id);
        if m.erholung <= 0.0 {
            fehler.push(format!("{wo}: 'erholung' muss größer als 0 sein."));
        }
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
        if m.ist_npc && m.gespraech.begruessung.ist_leer() && m.thema.is_empty() {
            warn.push(format!("{wo}: Der NPC hat keine [npc.gespraech]-Antworten und kann nicht reden."));
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
        if z.wirkungen.is_empty() && z.waffen_faktor <= 0.0 {
            warn.push(format!("{wo}: hat keine Wirkungen."));
        }
        if !ZAUBER_TYPEN.contains(&z.typ.as_str()) {
            fehler.push(format!("{wo}: Typ '{}' gibt es nicht. Erlaubt: zauber, technik", z.typ));
        }
        if !z.gilde.is_empty() {
            verweis(&d.gilden, &z.gilde, &wo, "Gilde", fehler);
        } else if z.ist_technik() && d.welt.techniken_nur_gilden {
            fehler.push(format!(
                "{wo}: Kampftechniken gibt es nur für Gilden – bitte gilde = \"...\" angeben (oder techniken_nur_gilden = false in welt.toml)."
            ));
        }
        if z.waffen_faktor < 0.0 || z.ausdauer_kosten < 0 || z.mana_kosten < 0 {
            fehler.push(format!("{wo}: Kosten und Faktoren dürfen nicht negativ sein."));
        }
        if z.ist_technik() && z.ziel != "gegner" && z.waffen_faktor > 0.0 {
            warn.push(format!("{wo}: Eine Technik mit Waffenschlag braucht ziel = \"gegner\"."));
        }
        for w in &z.wirkungen {
            if !WIRKUNGEN.contains(&w.art.as_str()) {
                let tipp = text::vorschlag(&w.art, WIRKUNGEN.iter().map(|s| s.to_string()).collect::<Vec<_>>().iter())
                    .map(|v| format!(" Meintest du '{v}'?"))
                    .unwrap_or_default();
                fehler.push(format!("{wo}: Wirkung '{}' gibt es nicht.{tipp} Erlaubt: {}", w.art, WIRKUNGEN.join(", ")));
            }
            if w.art == "teleport" {
                raum_verweis(d, &w.raum, &format!("{wo}, Wirkung teleport"), fehler);
            }
            if WIRKUNGEN_MIT_DAUER.contains(&w.art.as_str()) && w.dauer <= 0.0 {
                fehler.push(format!("{wo}: Die Wirkung '{}' braucht eine 'dauer' in Sekunden.", w.art));
            }
            if w.dauer < 0.0 || w.intervall.is_some_and(|i| i <= 0.0) {
                fehler.push(format!("{wo}: 'dauer' und 'intervall' müssen größer als 0 sein."));
            }
            if w.art == "bonus" {
                if w.attribut.is_empty() {
                    fehler.push(format!("{wo}: Die Wirkung 'bonus' braucht ein 'attribut', z.B. \"staerke\"."));
                } else if !d.start.werte.contains_key(&w.attribut) {
                    warn.push(format!("{wo}: Attribut '{}' gibt es in [start.werte] nicht.", w.attribut));
                }
            }
        }
    }

    // --- Geld, Händler und Lehrer ---
    for m in d.monster.values() {
        let wo = format!("[{}] {} '{}'", m.quelle, if m.ist_npc { "NPC" } else { "Monster" }, m.id);
        if let Some(h) = &m.haendler {
            for g in &h.waren {
                verweis(&d.gegenstaende, g, &format!("{wo}, haendler.waren"), "Gegenstand", fehler);
            }
            for t in &h.ankauf_typen {
                if !GEGENSTAND_TYPEN.contains(&t.as_str()) {
                    warn.push(format!("{wo}: unbekannter Typ '{t}' in haendler.ankauf_typen."));
                }
            }
            for g in &h.waren {
                if d.gegenstaende.get(g).is_some_and(|g| g.wert <= 0) {
                    warn.push(format!("{wo}: Die Ware '{g}' hat keinen 'wert' und kostet darum nur 1."));
                }
            }
        }
        for l in &m.lehrt {
            if !d.fertigkeiten.contains_key(&l.id) && !d.zauber.contains_key(&l.id) {
                let alle: Vec<String> = d.fertigkeiten.keys().chain(d.zauber.keys()).cloned().collect();
                let tipp = text::vorschlag(&l.id, alle.iter()).map(|v| format!(" Meintest du '{v}'?")).unwrap_or_default();
                fehler.push(format!("{wo}, lehrt: Eine Fertigkeit oder einen Zauber '{}' gibt es nicht.{tipp}", l.id));
            }
            if !l.gilde.is_empty() {
                verweis(&d.gilden, &l.gilde, &format!("{wo}, lehrt"), "Gilde", fehler);
            }
        }
        if !m.gilde.is_empty() {
            verweis(&d.gilden, &m.gilde, &wo, "Gilde", fehler);
        }
        if !m.beruf.is_empty() {
            verweis(&d.berufe, &m.beruf, &wo, "Beruf", fehler);
        }
        if let GeldDef::Bereich(v) = &m.geld {
            if v.len() > 2 {
                fehler.push(format!("{wo}: 'geld' ist eine Zahl oder [von, bis]."));
            }
        }
    }

    // --- Rassen, Gilden, Berufe ---
    for (art, liste) in [("Rasse", &d.rassen), ("Gilde", &d.gilden), ("Beruf", &d.berufe)] {
        for g in liste.values() {
            let wo = format!("[{}] {art} '{}'", g.quelle, g.id);
            for f in g.fertigkeiten.keys() {
                verweis(&d.fertigkeiten, f, &wo, "Fertigkeit", fehler);
            }
            for z in &g.zauber {
                verweis(&d.zauber, z, &wo, "Zauber", fehler);
            }
            for r in &g.rassen {
                verweis(&d.rassen, r, &wo, "Rasse", fehler);
            }
            for w in g.werte.keys() {
                if !d.start.werte.contains_key(w) {
                    warn.push(format!("{wo}: Wert '{w}' gibt es in [start.werte] nicht."));
                }
            }
        }
    }

    // --- Quests ---
    for q in d.quests.values() {
        let wo = format!("[{}] Quest '{}'", q.quelle, q.id);
        if !QUEST_ARTEN.contains(&q.art.as_str()) {
            fehler.push(format!("{wo}: Art '{}' gibt es nicht. Erlaubt: aufgabe, quest", q.art));
        }
        match d.monster.get(&q.geber) {
            Some(m) if !m.ist_npc => warn.push(format!("{wo}: Der Geber '{}' ist ein Monster, kein NPC.", q.geber)),
            Some(_) => {}
            None => verweis(&d.monster, &q.geber, &format!("{wo}, geber"), "NPC", fehler),
        }
        for v in &q.voraussetzungen {
            verweis(&d.quests, v, &format!("{wo}, voraussetzungen"), "Quest", fehler);
        }
        if !q.gilde.is_empty() {
            verweis(&d.gilden, &q.gilde, &wo, "Gilde", fehler);
        }
        if q.schritte.is_empty() {
            warn.push(format!("{wo}: hat keine Schritte und ist sofort erledigt."));
        }
        for (i, sch) in q.schritte.iter().enumerate() {
            let wo = format!("{wo}, Schritt {}", i + 1);
            match sch.art.as_str() {
                "toeten" => verweis(&d.monster, &sch.ziel, &wo, "Monster", fehler),
                "bringen" => {
                    verweis(&d.gegenstaende, &sch.ziel, &wo, "Gegenstand", fehler);
                    if !sch.an.is_empty() {
                        verweis(&d.monster, &sch.an, &format!("{wo}, an"), "NPC", fehler);
                    }
                }
                "besuchen" => raum_verweis(d, &sch.ziel, &wo, fehler),
                "sprechen" => verweis(&d.monster, &sch.ziel, &wo, "NPC", fehler),
                a => fehler.push(format!("{wo}: Art '{a}' gibt es nicht. Erlaubt: {}", SCHRITT_ARTEN.join(", "))),
            }
            if sch.anzahl == 0 {
                fehler.push(format!("{wo}: 'anzahl' muss mindestens 1 sein."));
            }
        }
        let b = &q.belohnung;
        for g in &b.gegenstaende {
            verweis(&d.gegenstaende, g, &format!("{wo}, Belohnung"), "Gegenstand", fehler);
        }
        for f in b.fertigkeiten.keys() {
            verweis(&d.fertigkeiten, f, &format!("{wo}, Belohnung"), "Fertigkeit", fehler);
        }
        for z in &b.zauber {
            verweis(&d.zauber, z, &format!("{wo}, Belohnung"), "Zauber", fehler);
        }
    }

    // --- welt.toml: Karte und Start ---
    if !d.welt.karten_fertigkeit.is_empty() {
        verweis(&d.fertigkeiten, &d.welt.karten_fertigkeit, "welt.toml (karten_fertigkeit)", "Fertigkeit", fehler);
        if d.welt.karten_fertigkeit_stufen == 0 {
            fehler.push("welt.toml: 'karten_fertigkeit_stufen' muss mindestens 1 sein.".into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn beispiel() -> Daten {
        laden(&Path::new(env!("CARGO_MANIFEST_DIR")).join("daten")).expect("Beispielwelt lädt")
    }

    #[test]
    fn vererbung_aus_vorlagen() {
        let d = beispiel();
        // Vorlagen selbst gibt es nicht in der Welt
        assert!(!d.gegenstaende.contains_key("vorlage_waffe"));
        assert!(!d.monster.contains_key("vorlage_dorfbewohner"));
        // Geerbte Werte
        let dolch = &d.gegenstaende["dolch"];
        assert_eq!((dolch.typ.as_str(), dolch.platz.as_str(), dolch.fertigkeit.as_str()), ("waffe", "waffe", "schwertkampf"));
        assert_eq!(dolch.erholung, Some(1.3)); // eigener Wert schlägt Vorlage
        let wirt = &d.monster["wirt"];
        assert!(wirt.ist_npc && !wirt.ist_angreifbar());
        assert!(!wirt.gespraech.ort.ist_leer()); // aus der Vorlage
        assert!(!wirt.gespraech.beruf.ist_leer()); // eigene Antwort
        assert!(d.monster["ratte"].aggressiv);
        let segen = &d.zauber["segen"];
        assert_eq!((segen.ziel.as_str(), segen.fertigkeit.as_str()), ("raum", "heilkunde"));
    }

    #[test]
    fn neue_arten_aus_der_beispielwelt() {
        let d = beispiel();
        assert!(d.rassen.contains_key("zwerg") && d.gilden.contains_key("lichtorden") && d.berufe.contains_key("schmied"));
        let q = &d.quests["rattenplage"];
        assert_eq!((q.geber.as_str(), q.schritte.len(), q.belohnung.geld), ("wirt", 2, 25));
        let schmied = &d.monster["schmied"];
        assert_eq!(schmied.beruf, "schmied");
        assert!(schmied.haendler.as_ref().is_some_and(|h| h.waren.contains(&"dolch".to_string())));
        assert!(d.monster["hauptmann"].lehrt.iter().any(|l| l.id == "wuchtschlag"));
        assert_eq!(d.monster["magister"].gilde, "magier");
        // Techniken gibt es nur für Gilden
        assert!(d.zauber.values().filter(|z| z.ist_technik()).all(|z| !z.gilde.is_empty()));
        assert!(d.zauber["wuchtschlag"].ist_technik());
        assert_eq!(d.zauber["schutzschild"].wirkungen[0].dauer, 90.0);
        assert!(matches!(d.monster["ratte"].geld, GeldDef::Bereich(_)));
        assert_eq!(d.start.geld, 20);
    }
}
