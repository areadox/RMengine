//! Die Liste der Welten, die der Editor schon geöffnet hat (lokal und Fernwartung).
//!
//! Gespeichert in
//!   Windows: %APPDATA%\RMengine\editor.json
//!   Linux:   ~/.config/rmengine/editor.json

use crate::fernwartung::Arbeitskopie;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Eintrag {
    /// Der Datenordner (mit welt.toml)
    pub pfad: String,
    /// Zuletzt geöffnet (Sekunden seit 1970)
    pub zuletzt: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Weltliste {
    pub welten: Vec<Eintrag>,
    /// Beim Start die Auswahl zeigen (sonst die zuletzt benutzte Welt öffnen)
    pub beim_start_fragen: bool,
}

impl Default for Weltliste {
    fn default() -> Self {
        Weltliste { welten: Vec::new(), beim_start_fragen: true }
    }
}

/// Was man über eine Welt in der Liste anzeigt
pub struct Info {
    pub name: String,
    pub da: bool,
    /// Adresse des Servers, wenn es eine Arbeitskopie der Fernwartung ist
    pub server: Option<String>,
    pub schluessel_gemerkt: bool,
}

fn datei() -> PathBuf {
    let basis = std::env::var_os("APPDATA")
        .map(|a| PathBuf::from(a).join("RMengine"))
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(|c| PathBuf::from(c).join("rmengine")))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config").join("rmengine")))
        .unwrap_or_else(|| PathBuf::from("."));
    basis.join("editor.json")
}

pub fn jetzt() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Pfade vergleichbar machen (absolut, ohne ./)
pub fn normal(p: &Path) -> String {
    std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf()).components().collect::<PathBuf>().display().to_string()
}

impl Weltliste {
    pub fn laden() -> Weltliste {
        fs::read_to_string(datei()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }

    pub fn speichern(&self) {
        let d = datei();
        if let Some(o) = d.parent() {
            let _ = fs::create_dir_all(o);
        }
        if let Ok(t) = serde_json::to_string_pretty(self) {
            let _ = fs::write(d, t);
        }
    }

    /// Eine geöffnete Welt nach vorne stellen und speichern.
    pub fn eintragen(&mut self, ordner: &Path) {
        let pfad = normal(ordner);
        self.welten.retain(|e| e.pfad != pfad);
        self.welten.insert(0, Eintrag { pfad, zuletzt: jetzt() });
        self.welten.truncate(30);
        self.speichern();
    }

    pub fn entfernen(&mut self, pfad: &str) {
        self.welten.retain(|e| e.pfad != pfad);
        self.speichern();
    }

    pub fn zuletzt(&self) -> Option<PathBuf> {
        self.welten.iter().map(|e| PathBuf::from(&e.pfad)).find(|p| p.join("welt.toml").exists())
    }
}

/// Name der Welt (aus welt.toml) und ob sie zu einem Server gehört
pub fn info(pfad: &Path) -> Info {
    let text = fs::read_to_string(pfad.join("welt.toml")).ok();
    let name = text
        .as_deref()
        .and_then(|t| t.parse::<toml_edit::DocumentMut>().ok())
        .and_then(|d| d.get("welt")?.get("name")?.as_str().map(String::from))
        .unwrap_or_default();
    let kopie = Arbeitskopie::zu_datenordner(pfad).map(|(_, k)| k);
    Info {
        name,
        da: text.is_some(),
        schluessel_gemerkt: kopie.as_ref().is_some_and(|k| k.schluessel.is_some()),
        server: kopie.map(|k| k.adresse),
    }
}

/// "heute", "gestern", "vor 5 Tagen" ...
pub fn wann(sekunden: u64) -> String {
    let tage = jetzt().saturating_sub(sekunden) / 86_400;
    match tage {
        0 => "heute".into(),
        1 => "gestern".into(),
        n if n < 60 => format!("vor {n} Tagen"),
        n => format!("vor {} Monaten", n / 30),
    }
}
