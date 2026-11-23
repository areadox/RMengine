//! Anpassbare Befehle: daten/befehle.toml
//!
//!     # Eingebaute Befehle ändern
//!     [befehl.toete]
//!     woerter = ["toete", "kill", "k"]   # ersetzt alle Wörter, das erste ist der Hauptname
//!     rolle   = "spieler"                # spieler | baumeister | admin
//!     aktiv   = true                     # false = abgeschaltet
//!     hilfe   = "Angreifen"
//!     syntax  = "toete <monster>"
//!
//!     # Eigene Befehle: Gesten
//!     [[geste]]
//!     woerter     = ["verbeuge", "verbeugen"]
//!     hilfe       = "Sich verbeugen"
//!     selbst      = "Du verbeugst dich."
//!     andere      = "{name} verbeugt sich."
//!     selbst_ziel = "Du verbeugst dich vor {ziel}."
//!     ziel        = "{name} verbeugt sich vor dir."
//!     andere_ziel = "{name} verbeugt sich vor {ziel}."

use crate::befehlsliste::{self, EINGEBAUT};
use crate::text::{self, normalisieren};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Anpassung {
    pub woerter: Option<Vec<String>>,
    pub rolle: Option<String>,
    pub aktiv: Option<bool>,
    pub hilfe: Option<String>,
    pub syntax: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GesteDef {
    pub woerter: Vec<String>,
    #[serde(default)]
    pub hilfe: String,
    #[serde(default = "standard_rolle")]
    pub rolle: String,
    #[serde(default = "wahr")]
    pub aktiv: bool,
    /// Ohne Ziel: was man selbst liest und was die anderen im Raum lesen
    #[serde(default)]
    pub selbst: String,
    #[serde(default)]
    pub andere: String,
    /// Mit Ziel ("winke bert"): für sich, für das Ziel und für die anderen
    #[serde(default)]
    pub selbst_ziel: String,
    #[serde(default)]
    pub ziel: String,
    #[serde(default)]
    pub andere_ziel: String,
}

fn standard_rolle() -> String {
    "spieler".into()
}
fn wahr() -> bool {
    true
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
struct Datei {
    befehl: BTreeMap<String, Anpassung>,
    geste: Vec<GesteDef>,
}

#[derive(Debug, Clone, Copy)]
pub enum Treffer {
    Befehl(&'static str),
    Geste(usize),
}

#[derive(Debug, Clone, Default)]
pub struct Befehle {
    pub anpassung: BTreeMap<String, Anpassung>,
    pub gesten: Vec<GesteDef>,
    /// Eingetipptes Wort -> Befehl oder Geste
    wort: HashMap<String, Treffer>,
}

impl Befehle {
    fn a(&self, id: &str) -> Option<&Anpassung> {
        self.anpassung.get(id)
    }

    pub fn woerter(&self, id: &str) -> Vec<String> {
        match self.a(id).and_then(|a| a.woerter.clone()) {
            Some(w) => w,
            None => befehlsliste::finde(id).map(|e| e.woerter.iter().map(|s| s.to_string()).collect()).unwrap_or_default(),
        }
    }

    pub fn hauptname(&self, id: &str) -> String {
        self.woerter(id).into_iter().next().unwrap_or_else(|| id.to_string())
    }

    pub fn rolle(&self, id: &str) -> String {
        self.a(id)
            .and_then(|a| a.rolle.clone())
            .or_else(|| befehlsliste::finde(id).map(|e| e.rolle.to_string()))
            .unwrap_or_else(standard_rolle)
    }

    pub fn aktiv(&self, id: &str) -> bool {
        self.a(id).and_then(|a| a.aktiv).unwrap_or(true)
    }

    pub fn hilfe(&self, id: &str) -> String {
        self.a(id)
            .and_then(|a| a.hilfe.clone())
            .or_else(|| befehlsliste::finde(id).map(|e| e.hilfe.to_string()))
            .unwrap_or_default()
    }

    pub fn syntax(&self, id: &str) -> String {
        self.a(id)
            .and_then(|a| a.syntax.clone())
            .or_else(|| befehlsliste::finde(id).map(|e| e.syntax.to_string()))
            .unwrap_or_default()
    }

    /// Was ist mit dem eingetippten Wort gemeint?
    pub fn aufloesen(&self, wort: &str) -> Option<Treffer> {
        self.wort.get(&normalisieren(wort)).copied()
    }
}

/// Die Dateien mit Befehlen: daten/befehle.toml und alle .toml in daten/befehle/
pub fn dateien(ordner: &Path) -> Vec<std::path::PathBuf> {
    let mut liste = Vec::new();
    let einzeln = ordner.join("befehle.toml");
    if einzeln.exists() {
        liste.push(einzeln);
    }
    if let Ok(e) = fs::read_dir(ordner.join("befehle")) {
        let mut teil: Vec<_> =
            e.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "toml")).collect();
        teil.sort();
        liste.extend(teil);
    }
    liste
}

/// Lädt alle Befehlsdateien (fehlen sie, gelten die Standardwerte).
pub fn laden(ordner: &Path, fehler: &mut Vec<String>, warn: &mut Vec<String>) -> Befehle {
    let mut anpassung: BTreeMap<String, Anpassung> = BTreeMap::new();
    let mut gesten = Vec::new();
    for pfad in dateien(ordner) {
        let name = pfad.strip_prefix(ordner).unwrap_or(&pfad).display().to_string();
        let datei: Datei = match fs::read_to_string(&pfad) {
            Ok(t) => match toml::from_str(&t) {
                Ok(d) => d,
                Err(e) => {
                    fehler.push(format!("Fehler in '{name}':\n{}", crate::daten::toml_fehler_uebersetzen(&e)));
                    continue;
                }
            },
            Err(e) => {
                fehler.push(format!("Kann '{name}' nicht lesen: {e}"));
                continue;
            }
        };
        for (id, a) in datei.befehl {
            match anpassung.entry(id) {
                std::collections::btree_map::Entry::Occupied(e) => {
                    fehler.push(format!("[{name}] [befehl.{}] steht in mehreren Dateien.", e.key()));
                }
                std::collections::btree_map::Entry::Vacant(e) => {
                    e.insert(a);
                }
            }
        }
        gesten.extend(datei.geste);
    }

    let mut b = Befehle { anpassung, gesten, wort: HashMap::new() };
    let ids: Vec<String> = EINGEBAUT.iter().map(|e| e.id.to_string()).collect();

    // Unbekannte Befehle und ungültige Werte
    for (id, a) in &b.anpassung {
        let wo = format!("Befehle [befehl.{id}]");
        if befehlsliste::finde(id).is_none() {
            let tipp = text::vorschlag(id, ids.iter()).map(|v| format!(" Meintest du '{v}'?")).unwrap_or_default();
            fehler.push(format!("{wo}: Einen eingebauten Befehl '{id}' gibt es nicht.{tipp}"));
        }
        if let Some(r) = &a.rolle {
            if !befehlsliste::ROLLEN.contains(&r.as_str()) {
                fehler.push(format!("{wo}: Rolle '{r}' gibt es nicht. Erlaubt: spieler, baumeister, admin"));
            }
        }
        if a.woerter.as_ref().is_some_and(|w| w.iter().all(|x| x.trim().is_empty())) {
            fehler.push(format!("{wo}: 'woerter' darf nicht leer sein. Zum Abschalten: aktiv = false"));
        }
        if matches!(id.as_str(), "hilfe" | "ende") && a.aktiv == Some(false) {
            warn.push(format!("{wo}: Ohne diesen Befehl kommen Spieler kaum zurecht."));
        }
    }
    for (i, g) in b.gesten.iter().enumerate() {
        let wo = format!("Befehle, Geste '{}'", g.woerter.first().cloned().unwrap_or_else(|| (i + 1).to_string()));
        if g.woerter.iter().all(|x| x.trim().is_empty()) {
            fehler.push(format!("{wo}: 'woerter' fehlt."));
        }
        if g.selbst.trim().is_empty() && g.selbst_ziel.trim().is_empty() {
            fehler.push(format!("{wo}: Es fehlt 'selbst' (oder 'selbst_ziel' für Gesten mit Ziel)."));
        }
        if !befehlsliste::ROLLEN.contains(&g.rolle.as_str()) {
            fehler.push(format!("{wo}: Rolle '{}' gibt es nicht. Erlaubt: spieler, baumeister, admin", g.rolle));
        }
    }

    // Wörter zuordnen und Doppelte finden
    let mut belegt: HashMap<String, String> = HashMap::new();
    let mut eintragen = |b: &mut Befehle, wort: &str, treffer: Treffer, name: String| {
        let w = normalisieren(wort);
        if w.is_empty() {
            return;
        }
        if befehlsliste::RICHTUNGSWOERTER.contains(&w.as_str()) {
            warn.push(format!("Befehle: '{w}' ({name}) ist auch eine Richtung – zum Gehen muss man dann 'gehe {w}' tippen."));
        }
        if let Some(anderer) = belegt.get(&w) {
            if *anderer != name {
                fehler.push(format!("Befehle: Das Wort '{w}' gehört zu '{anderer}' und zu '{name}'."));
            }
            return;
        }
        belegt.insert(w.clone(), name);
        b.wort.insert(w, treffer);
    };
    for e in EINGEBAUT {
        if !b.aktiv(e.id) {
            continue;
        }
        for w in b.woerter(e.id) {
            eintragen(&mut b, &w, Treffer::Befehl(e.id), format!("Befehl {}", e.id));
        }
    }
    let gesten = b.gesten.clone();
    for (i, g) in gesten.iter().enumerate() {
        if !g.aktiv {
            continue;
        }
        let name = format!("Geste {}", g.woerter.first().cloned().unwrap_or_default());
        for w in &g.woerter {
            eintragen(&mut b, w, Treffer::Geste(i), name.clone());
        }
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standardwoerter_ohne_datei() {
        let (mut f, mut w) = (Vec::new(), Vec::new());
        let b = laden(Path::new("/gibt/es/nicht"), &mut f, &mut w);
        assert!(f.is_empty(), "{f:?}");
        assert!(w.is_empty(), "{w:?}");
        assert!(matches!(b.aufloesen("kill"), Some(Treffer::Befehl("toete"))));
        assert!(matches!(b.aufloesen("Töte"), Some(Treffer::Befehl("toete"))));
        assert!(b.aufloesen("norden").is_none());
    }

    #[test]
    fn eingebaute_woerter_sind_eindeutig() {
        let mut gesehen = std::collections::HashSet::new();
        for e in EINGEBAUT {
            assert_eq!(e.woerter[0], e.id, "Hauptname von {}", e.id);
            for w in e.woerter {
                assert!(gesehen.insert(*w), "'{w}' doppelt");
                assert!(!befehlsliste::RICHTUNGSWOERTER.contains(w), "'{w}' ist eine Richtung");
            }
        }
    }
}
