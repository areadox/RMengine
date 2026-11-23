//! Spielfiguren: Speichern, Laden und Passwörter.

use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SpielerDaten {
    pub name: String,
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
}

impl Default for SpielerDaten {
    fn default() -> Self {
        SpielerDaten {
            name: String::new(),
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
        }
    }
}

impl SpielerDaten {
    pub fn passwort_setzen(&mut self, pw: &str) {
        self.salz = neues_salz();
        self.passwort_hash = passwort_hash(pw, &self.salz);
    }

    pub fn passwort_stimmt(&self, pw: &str) -> bool {
        passwort_hash(pw, &self.salz) == self.passwort_hash
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

fn neues_salz() -> String {
    let bytes: [u8; 16] = rand::thread_rng().gen();
    hex(&bytes)
}

fn passwort_hash(pw: &str, salz: &str) -> String {
    let mut h = Sha256::digest(format!("{salz}:{pw}").as_bytes()).to_vec();
    for _ in 0..20_000 {
        h = Sha256::digest(&h).to_vec();
    }
    hex(&h)
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
