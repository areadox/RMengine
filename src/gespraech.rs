//! Gespräche mit NPCs.
//!
//! Jeder NPC kann auf Standardfragen antworten (Gruß, Beruf, Aufgabe, ...) und
//! auf eigene Themen. Spieler sprechen ganz natürlich mit ihm:
//!
//!     sage hallo
//!     sage hast du eine aufgabe für mich?
//!     frage wirt nach ratten
//!     rede mit wirt
//!
//! In der NPC-Datei sieht das so aus:
//!
//!     [npc.gespraech]
//!     begruessung = "Willkommen, {spieler}!"
//!     beruf       = "Ich bin der Wirt hier."
//!     aufgabe     = ["Im Keller sind Ratten.", "Frag mich später noch mal."]
//!
//!     [[npc.thema]]
//!     woerter = ["ratten", "keller"]
//!     antwort = "Die Viecher fressen mir den Käse weg!"

use crate::text::normalisieren;
use rand::seq::SliceRandom;
use serde::Deserialize;

/// Ein Text oder mehrere, von denen zufällig einer gewählt wird.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Texte {
    Einer(String),
    Mehrere(Vec<String>),
}

impl Default for Texte {
    fn default() -> Self {
        Texte::Mehrere(Vec::new())
    }
}

impl Texte {
    pub fn ist_leer(&self) -> bool {
        match self {
            Texte::Einer(t) => t.trim().is_empty(),
            Texte::Mehrere(v) => v.iter().all(|t| t.trim().is_empty()),
        }
    }

    pub fn waehle(&self) -> Option<String> {
        match self {
            Texte::Einer(t) if !t.trim().is_empty() => Some(t.clone()),
            Texte::Einer(_) => None,
            Texte::Mehrere(v) => {
                let gefuellt: Vec<&String> = v.iter().filter(|t| !t.trim().is_empty()).collect();
                gefuellt.choose(&mut rand::thread_rng()).map(|t| (*t).clone())
            }
        }
    }
}

/// Antworten auf die Standardfragen.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct GespraechDef {
    /// "hallo", "guten Tag", "moin" ...
    pub begruessung: Texte,
    /// "tschüss", "auf Wiedersehen" ...
    pub abschied: Texte,
    /// "was ist dein Beruf", "was machst du", "wer bist du" ...
    pub beruf: Texte,
    /// "hast du eine Aufgabe für mich", "kann ich helfen" ...
    pub aufgabe: Texte,
    /// "wie heißt du", "dein Name" ...
    pub name: Texte,
    /// "was gibt es Neues", "Gerüchte" ...
    pub neuigkeiten: Texte,
    /// "wo bin ich", "was ist das hier für ein Ort" ...
    pub ort: Texte,
    /// Wenn der NPC mit einer Frage nichts anfangen kann
    pub unbekannt: Texte,
}

/// Ein eigenes Gesprächsthema.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemaDef {
    pub woerter: Vec<String>,
    pub antwort: Texte,
}

/// Die Standardfragen und woran man sie erkennt (Reihenfolge = Vorrang).
const STANDARD: &[(&str, &[&str])] = &[
    (
        "aufgabe",
        &[
            "aufgabe", "aufgaben", "auftrag", "auftraege", "quest", "fuer mich", "kann ich helfen", "brauchst du hilfe",
            "hilfe brauchen", "arbeit fuer mich", "was zu tun", "etwas zu tun", "abenteuer",
        ],
    ),
    (
        "beruf",
        &[
            "beruf", "arbeitest", "deine arbeit", "deine aufgabe hier", "was machst du", "was tust du", "wer bist du",
            "job", "handwerk", "taetigkeit", "womit verdienst",
        ],
    ),
    ("name", &["wie heisst du", "dein name", "deinen namen", "wie ist dein name", "wer bist"]),
    (
        "neuigkeiten",
        &["neuigkeiten", "neues", "geruecht", "geruechte", "klatsch", "tratsch", "was gibts", "was gibt es neues"],
    ),
    ("ort", &["wo bin ich", "was ist das hier", "was fuer ein ort", "dieser ort", "die gegend", "wo sind wir"]),
    (
        "abschied",
        &["tschuess", "wiedersehen", "bis bald", "bis dann", "leb wohl", "lebwohl", "ciao", "bye", "gute nacht", "machs gut"],
    ),
    (
        "begruessung",
        &[
            "hallo", "hi", "hey", "moin", "servus", "gruess", "gruss", "guten tag", "guten morgen", "guten abend",
            "tag", "huhu", "sei gegruesst", "seid gegruesst", "willkommen",
        ],
    ),
];

/// Passt ein Stichwort zum Satz? Ganze Wörter oder Wortgruppen.
fn kommt_vor(satz: &str, stichwort: &str) -> bool {
    let stichwort = normalisieren(stichwort);
    if stichwort.contains(' ') {
        return format!(" {satz} ").contains(&format!(" {stichwort} "));
    }
    satz.split_whitespace().any(|wort| wort == stichwort || (stichwort.len() >= 5 && wort.starts_with(&stichwort)))
}

/// Satzzeichen weg, Kleinbuchstaben, Umlaute ersetzen.
pub fn satz_vorbereiten(t: &str) -> String {
    normalisieren(t)
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Fragt der Satz nach einer Aufgabe ("hast du eine Aufgabe für mich")?
pub fn fragt_nach_aufgabe(frage: &str) -> bool {
    let satz = satz_vorbereiten(frage);
    STANDARD.iter().filter(|(art, _)| *art == "aufgabe").any(|(_, woerter)| woerter.iter().any(|w| kommt_vor(&satz, w)))
}

/// Was antwortet der NPC? `None` = er versteht die Frage nicht.
pub fn antwort(g: &GespraechDef, themen: &[ThemaDef], frage: &str) -> Option<String> {
    let satz = satz_vorbereiten(frage);
    if satz.is_empty() {
        return g.begruessung.waehle();
    }
    // Eigene Themen haben Vorrang
    for t in themen {
        if t.woerter.iter().any(|w| kommt_vor(&satz, w)) {
            if let Some(a) = t.antwort.waehle() {
                return Some(a);
            }
        }
    }
    for (art, stichwoerter) in STANDARD {
        if stichwoerter.iter().any(|w| kommt_vor(&satz, w)) {
            let texte = match *art {
                "aufgabe" => &g.aufgabe,
                "beruf" => &g.beruf,
                "name" => &g.name,
                "neuigkeiten" => &g.neuigkeiten,
                "ort" => &g.ort,
                "abschied" => &g.abschied,
                _ => &g.begruessung,
            };
            if let Some(a) = texte.waehle() {
                return Some(a);
            }
        }
    }
    None
}

/// Worüber man mit dem NPC reden kann (für "rede mit ...").
pub fn themenliste(g: &GespraechDef, themen: &[ThemaDef]) -> Vec<String> {
    let mut aus = Vec::new();
    for (name, t) in [
        ("Beruf", &g.beruf),
        ("Aufgabe", &g.aufgabe),
        ("Name", &g.name),
        ("Neuigkeiten", &g.neuigkeiten),
        ("Ort", &g.ort),
    ] {
        if !t.ist_leer() {
            aus.push(name.to_string());
        }
    }
    for t in themen {
        if let Some(w) = t.woerter.first() {
            aus.push(crate::text::gross(w));
        }
    }
    aus
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wirt() -> GespraechDef {
        GespraechDef {
            begruessung: Texte::Einer("gruss".into()),
            beruf: Texte::Einer("beruf".into()),
            aufgabe: Texte::Einer("aufgabe".into()),
            ..Default::default()
        }
    }

    #[test]
    fn standardfragen() {
        let g = wirt();
        assert_eq!(antwort(&g, &[], "Hallo!").as_deref(), Some("gruss"));
        assert_eq!(antwort(&g, &[], "Was ist dein Beruf?").as_deref(), Some("beruf"));
        assert_eq!(antwort(&g, &[], "Hallo, hast du eine Aufgabe für mich?").as_deref(), Some("aufgabe"));
        assert_eq!(antwort(&g, &[], "hast du arbeit für mich").as_deref(), Some("aufgabe"));
        assert_eq!(antwort(&g, &[], "was machst du so").as_deref(), Some("beruf"));
        assert_eq!(antwort(&g, &[], "Wie spät ist es?"), None);
    }

    #[test]
    fn eigene_themen_gehen_vor() {
        let themen = vec![ThemaDef { woerter: vec!["Ratten".into()], antwort: Texte::Einer("ratten!".into()) }];
        assert_eq!(antwort(&wirt(), &themen, "hallo, was ist mit den ratten?").as_deref(), Some("ratten!"));
    }
}
