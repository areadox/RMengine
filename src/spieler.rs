//! Spielfiguren: Speichern, Laden und Passwörter.

use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Was eine Figur darf. Höhere Rollen dürfen alles, was niedrigere dürfen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rolle {
    /// Spielt das Spiel
    #[default]
    Spieler,
    /// Baut die Welt: neuladen, springe, erschaffe ... und sieht technische Infos
    Baumeister,
    /// Verwaltet: Rollen vergeben, Spieler hinauswerfen, Verkündungen
    Admin,
}

impl Rolle {
    pub fn aus_text(t: &str) -> Option<Rolle> {
        match crate::text::normalisieren(t).as_str() {
            "spieler" | "player" => Some(Rolle::Spieler),
            "baumeister" | "builder" | "magier" => Some(Rolle::Baumeister),
            "admin" | "administrator" | "gott" => Some(Rolle::Admin),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Rolle::Spieler => "Spieler",
            Rolle::Baumeister => "Baumeister",
            Rolle::Admin => "Admin",
        }
    }

    /// Markierung für "wer" und Co.
    pub fn marke(self) -> &'static str {
        match self {
            Rolle::Spieler => "",
            Rolle::Baumeister => " {hellgelb}[Baumeister]{/}",
            Rolle::Admin => " {hellrot}[Admin]{/}",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SpielerDaten {
    pub name: String,
    pub rolle: Rolle,
    pub passwort_hash: String,
    pub salz: String,
    pub raum: String,
    pub lebenspunkte: i32,
    pub max_lebenspunkte: i32,
    pub mana: i32,
    pub max_mana: i32,
    pub farben: bool,
    pub inventar: Vec<String>,
    pub ausruestung: BTreeMap<String, String>,
    pub fertigkeiten: BTreeMap<String, u32>,
    pub zauber: Vec<String>,
    pub werte: BTreeMap<String, i32>,
    pub besucht: BTreeSet<String>,
    /// Was andere sehen, wenn sie einen ansehen
    pub beschreibung: String,
    /// Wird hinter dem Namen angezeigt, z.B. "der Wanderer"
    pub titel: String,
    /// Sekunden, die man insgesamt in der Welt verbracht hat
    pub spielzeit: u64,
    /// Erstellt am (Sekunden seit 1970)
    pub erstellt: u64,
    pub einstellungen: Einstellungen,
    /// Stufe und Erfahrung in der aktuellen Stufe
    pub stufe: u32,
    pub erfahrung: u64,
    /// Noch zu verteilende Punkte (Befehl: steigere)
    pub freie_punkte: u32,
    pub ausdauer: i32,
    pub max_ausdauer: i32,
    pub tode: u32,
    pub siege: u32,
    /// Liegt gerade tot am Boden und wartet auf Wiederbelebung
    pub ist_tot: bool,
    pub geld: u64,
    pub rasse: String,
    pub gilde: String,
    pub berufe: Vec<String>,
    /// Laufende Aufgaben und Quests: ID -> Fortschritt
    pub quests: BTreeMap<String, QuestStand>,
    /// Erledigte Aufgaben und Quests: ID -> wie oft
    pub erledigt: BTreeMap<String, u32>,
    /// Inhalt der Behälter (Taschen, Rucksäcke), die man bei sich hat
    pub behaelter: Vec<Behaelter>,
    /// Zauber oder Technik, die der Autoangriff von selbst einsetzt (bereite ...)
    pub bereit: String,
    /// Hunger und Durst: wie satt bzw. wie wenig durstig (negativ = noch nicht gesetzt, also voll)
    pub essen: f64,
    pub trinken: f64,
    /// Was sich Skripte für diesen Spieler merken (s:merke("muenze", true))
    pub merker: BTreeMap<String, Merk>,
}

/// Ein gemerkter Wert aus einem Skript: wahr/falsch, Zahl oder Text
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Merk {
    Wahr(bool),
    Zahl(f64),
    Text(String),
}

/// Ein gefüllter Behälter. Leere Behälter haben keinen Eintrag.
/// Hat man zwei gleiche Rucksäcke, gibt es bis zu zwei Einträge mit derselben ID.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Behaelter {
    pub gegenstand: String,
    pub inhalt: Vec<String>,
}

/// Wie weit man mit einer Aufgabe ist.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct QuestStand {
    /// Der aktuelle Schritt (0 = erster)
    pub schritt: usize,
    /// Wie oft das Ziel des Schritts schon erreicht wurde (z.B. 2 von 3 Ratten)
    pub zaehler: u32,
}

/// Alles, was man im Spielerbuch einstellen kann.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Einstellungen {
    pub karte: bool,
    pub kurz: bool,
    pub kartengroesse: i32,
    /// Raumtext neben der Karte (None = wie in welt.toml)
    pub karte_daneben: Option<bool>,
    pub breite: usize,
    /// Zeilenbreite automatisch an das Fenster des Clients anpassen (NAWS)
    pub breite_auto: bool,
    pub prompt: bool,
    pub ankuendigungen: bool,
    pub npc_sprueche: bool,
    /// Im Kampf automatisch weiter angreifen
    pub autoangriff: bool,
    /// Schadenszahlen im Kampf anzeigen
    pub kampfzahlen: bool,
}

impl Default for Einstellungen {
    fn default() -> Self {
        Einstellungen {
            karte: true,
            kurz: false,
            kartengroesse: 0,
            karte_daneben: None,
            breite: 78,
            breite_auto: true,
            prompt: true,
            ankuendigungen: true,
            npc_sprueche: true,
            autoangriff: true,
            kampfzahlen: true,
        }
    }
}

impl Default for SpielerDaten {
    fn default() -> Self {
        SpielerDaten {
            name: String::new(),
            rolle: Rolle::Spieler,
            passwort_hash: String::new(),
            salz: String::new(),
            raum: String::new(),
            lebenspunkte: 100,
            max_lebenspunkte: 100,
            mana: 50,
            max_mana: 50,
            farben: true,
            inventar: Vec::new(),
            ausruestung: BTreeMap::new(),
            fertigkeiten: BTreeMap::new(),
            zauber: Vec::new(),
            werte: BTreeMap::new(),
            besucht: BTreeSet::new(),
            beschreibung: String::new(),
            titel: String::new(),
            spielzeit: 0,
            erstellt: 0,
            einstellungen: Einstellungen::default(),
            stufe: 1,
            erfahrung: 0,
            freie_punkte: 0,
            ausdauer: 100,
            max_ausdauer: 100,
            tode: 0,
            siege: 0,
            ist_tot: false,
            geld: 0,
            rasse: String::new(),
            gilde: String::new(),
            berufe: Vec::new(),
            quests: BTreeMap::new(),
            erledigt: BTreeMap::new(),
            behaelter: Vec::new(),
            bereit: String::new(),
            essen: -1.0,
            trinken: -1.0,
            merker: BTreeMap::new(),
        }
    }
}

impl SpielerDaten {
    /// "Bert der Wanderer"
    pub fn voller_name(&self) -> String {
        if self.titel.is_empty() {
            self.name.clone()
        } else {
            format!("{} {}", self.name, self.titel)
        }
    }

    /// Passwort mit Argon2 speichern (das Salz steckt im Hash selbst)
    pub fn passwort_setzen(&mut self, pw: &str) {
        use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
        let salz = SaltString::generate(&mut OsRng);
        match argon2::Argon2::default().hash_password(pw.as_bytes(), &salz) {
            Ok(h) => {
                self.passwort_hash = h.to_string();
                self.salz.clear();
            }
            // Sollte nie passieren – dann wenigstens das alte Verfahren
            Err(_) => {
                self.salz = neues_salz();
                self.passwort_hash = passwort_hash(pw, &self.salz);
            }
        }
    }

    pub fn passwort_stimmt(&self, pw: &str) -> bool {
        use argon2::password_hash::{PasswordHash, PasswordVerifier};
        if self.passwort_hash.starts_with("$argon2") {
            PasswordHash::new(&self.passwort_hash).is_ok_and(|h| argon2::Argon2::default().verify_password(pw.as_bytes(), &h).is_ok())
        } else {
            passwort_hash(pw, &self.salz) == self.passwort_hash
        }
    }

    /// Noch mit dem alten Verfahren gespeichert? (wird beim Einloggen umgestellt)
    pub fn passwort_veraltet(&self) -> bool {
        !self.passwort_hash.starts_with("$argon2")
    }
}

/// Prüft einen gewünschten Namen und gibt ihn schön geschrieben zurück.
pub fn name_pruefen(name: &str) -> Result<String, String> {
    let name = name.trim();
    let laenge = name.chars().count();
    if !(3..=16).contains(&laenge) {
        return Err("Der Name muss 3 bis 16 Buchstaben lang sein.".into());
    }
    if !name.chars().all(|c| c.is_alphabetic()) {
        return Err("Der Name darf nur Buchstaben enthalten.".into());
    }
    let klein = name.to_lowercase();
    Ok(crate::text::gross(&klein))
}

fn datei(verzeichnis: &Path, name: &str) -> PathBuf {
    verzeichnis.join(format!("{}.toml", name.to_lowercase()))
}

pub fn existiert(verzeichnis: &Path, name: &str) -> bool {
    datei(verzeichnis, name).exists()
}

pub fn laden(verzeichnis: &Path, name: &str) -> Result<SpielerDaten, String> {
    let pfad = datei(verzeichnis, name);
    let inhalt = fs::read_to_string(&pfad).map_err(|e| format!("Kann {} nicht lesen: {e}", pfad.display()))?;
    toml::from_str(&inhalt).map_err(|e| format!("Spielerdatei {} ist beschädigt: {e}", pfad.display()))
}

pub fn speichern(verzeichnis: &Path, sp: &SpielerDaten) -> Result<(), String> {
    fs::create_dir_all(verzeichnis).map_err(|e| e.to_string())?;
    let inhalt = toml::to_string_pretty(sp).map_err(|e| e.to_string())?;
    let pfad = datei(verzeichnis, &sp.name);
    // Erst in eine Zwischendatei schreiben, damit bei einem Absturz nichts kaputtgeht.
    let tmp = pfad.with_extension("toml.tmp");
    fs::write(&tmp, inhalt).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &pfad).map_err(|e| e.to_string())
}

pub(crate) fn neues_salz() -> String {
    let bytes: [u8; 16] = rand::thread_rng().gen();
    hex(&bytes)
}

pub(crate) fn passwort_hash(pw: &str, salz: &str) -> String {
    let mut h = Sha256::digest(format!("{salz}:{pw}").as_bytes()).to_vec();
    for _ in 0..20_000 {
        h = Sha256::digest(&h).to_vec();
    }
    hex(&h)
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
