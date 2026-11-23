//! Ausführliche Beschreibungen: Details mit Unterdetails, Sinne (riechen,
//! hören, tasten) und die Bibliothek allgemeiner Beschreibungen.
//!
//! Ein Detail kann einfach ein Text sein:
//!
//!     [raum.details]
//!     altar = "Auf dem Altar liegen frische Blumen."
//!
//! oder sehr ausführlich, mit beliebig tief verschachtelten Unterdetails:
//!
//!     [raum.details.brunnen]
//!     text      = "Ein alter Brunnen aus grauem Stein."
//!     woerter   = ["schacht", "ziehbrunnen"]
//!     geraeusch = "Tief unten plätschert Wasser."
//!     gefuehl   = "Der Stein ist kühl und feucht."
//!
//!     [raum.details.brunnen.details]
//!     rand  = "Jemand hat 'Erbaut im Jahre 312' in den Rand geritzt."
//!     eimer = { text = "Ein Holzeimer an einer Kette.", geruch = "Er riecht modrig." }

use crate::text::normalisieren;
use serde::Deserialize;
use std::collections::BTreeMap;

/// Die drei Sinne neben dem Sehen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sinn {
    Riechen,
    Hoeren,
    Tasten,
}

impl Sinn {
    /// "Du riechst nichts Besonderes."
    pub fn nichts(self) -> &'static str {
        match self {
            Sinn::Riechen => "Du riechst nichts Besonderes.",
            Sinn::Hoeren => "Du hörst nichts Besonderes.",
            Sinn::Tasten => "Du fühlst nichts Besonderes.",
        }
    }

    /// "Bert schnuppert."
    pub fn handlung(self) -> &'static str {
        match self {
            Sinn::Riechen => "schnuppert",
            Sinn::Hoeren => "lauscht",
            Sinn::Tasten => "tastet herum",
        }
    }
}

/// Ein Detail: entweder nur ein Text oder eine ausführliche Beschreibung.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Detail {
    Text(String),
    Voll(DetailDef),
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct DetailDef {
    pub text: String,
    /// Weitere Wörter, unter denen man das Detail findet
    pub woerter: Vec<String>,
    pub geruch: String,
    pub geraeusch: String,
    pub gefuehl: String,
    /// Allgemeine Beschreibungen aus der Bibliothek, die hier mit gelten
    pub nutzt: Vec<String>,
    pub details: BTreeMap<String, Detail>,
}

impl Detail {
    pub fn text(&self) -> &str {
        match self {
            Detail::Text(t) => t,
            Detail::Voll(d) => &d.text,
        }
    }

    pub fn sinn(&self, s: Sinn) -> &str {
        match self {
            Detail::Text(_) => "",
            Detail::Voll(d) => match s {
                Sinn::Riechen => &d.geruch,
                Sinn::Hoeren => &d.geraeusch,
                Sinn::Tasten => &d.gefuehl,
            },
        }
    }

    fn woerter(&self) -> &[String] {
        match self {
            Detail::Text(_) => &[],
            Detail::Voll(d) => &d.woerter,
        }
    }

    pub fn kinder(&self) -> Option<&BTreeMap<String, Detail>> {
        match self {
            Detail::Text(_) => None,
            Detail::Voll(d) => Some(&d.details),
        }
    }

    pub fn nutzt(&self) -> &[String] {
        match self {
            Detail::Text(_) => &[],
            Detail::Voll(d) => &d.nutzt,
        }
    }
}

/// Ein Eintrag der Bibliothek (`daten/beschreibungen/*.toml`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BeschreibungDef {
    pub id: String,
    #[serde(default)]
    pub woerter: Vec<String>,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub geruch: String,
    #[serde(default)]
    pub geraeusch: String,
    #[serde(default)]
    pub gefuehl: String,
    #[serde(default)]
    pub nutzt: Vec<String>,
    #[serde(default)]
    pub details: BTreeMap<String, Detail>,
    #[serde(skip)]
    pub quelle: String,
}

impl BeschreibungDef {
    /// Als Detail, damit überall gleich gesucht werden kann.
    pub fn als_detail(&self) -> Detail {
        let mut woerter = vec![self.id.replace('_', " ")];
        woerter.extend(self.woerter.iter().cloned());
        Detail::Voll(DetailDef {
            text: self.text.clone(),
            woerter,
            geruch: self.geruch.clone(),
            geraeusch: self.geraeusch.clone(),
            gefuehl: self.gefuehl.clone(),
            nutzt: self.nutzt.clone(),
            details: self.details.clone(),
        })
    }
}

/// Sucht ein Detail – erst auf der obersten Ebene, dann in allen Unterdetails.
pub fn suchen<'a>(details: &'a BTreeMap<String, Detail>, wort: &str) -> Option<(&'a str, &'a Detail)> {
    let wort = normalisieren(wort);
    if wort.is_empty() {
        return None;
    }
    for (name, d) in details {
        if normalisieren(name) == wort || d.woerter().iter().any(|w| normalisieren(w) == wort) {
            return Some((name, d));
        }
    }
    for d in details.values() {
        if let Some(kinder) = d.kinder() {
            if let Some(f) = suchen(kinder, &wort) {
                return Some(f);
            }
        }
    }
    None
}

/// Alle Verweise (`nutzt`) in einem Baum von Details, zum Prüfen.
pub fn alle_verweise(details: &BTreeMap<String, Detail>, aus: &mut Vec<String>) {
    for d in details.values() {
        aus.extend(d.nutzt().iter().cloned());
        if let Some(k) = d.kinder() {
            alle_verweise(k, aus);
        }
    }
}

/// Namen der Unterdetails (für Baumeister).
pub fn unterdetails(d: &Detail) -> Vec<String> {
    d.kinder().map(|k| k.keys().cloned().collect()).unwrap_or_default()
}
