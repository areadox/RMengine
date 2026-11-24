//! Das Protokoll der Fernwartung zwischen Welteditor und Server.
//!
//! Diese Datei benutzen Engine UND Welteditor (dort per `#[path]` eingebunden).
//! Die Verbindung ist mit TLS verschlüsselt. Jede Nachricht ist eine Zeile JSON.
//! Zuerst meldet sich der Editor mit dem Zugangsschlüssel an.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const STANDARD_PORT: u16 = 4001;
/// Größte erlaubte Nachricht (eine ganze Welt passt bequem hinein).
pub const MAX_NACHRICHT: u64 = 64 * 1024 * 1024;
/// Name im Zertifikat (der Editor prüft den Fingerabdruck, nicht den Namen).
pub const SERVERNAME: &str = "rmengine";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "art", rename_all = "snake_case")]
pub enum Anfrage {
    Anmelden { schluessel: String, version: String },
    /// Alle Welt-Dateien mit Inhalt
    Holen,
    /// Änderungen übernehmen. Der Server prüft sie VORHER in einer Kopie.
    Uebernehmen {
        /// Pfad -> neuer Inhalt (None = Datei löschen)
        aenderungen: BTreeMap<String, Option<String>>,
        /// Pfad -> Hash, den die Datei auf dem Server haben muss (None = darf es nicht geben).
        /// So merkt der Server, wenn dort inzwischen jemand anders etwas geändert hat.
        erwartet: BTreeMap<String, Option<String>>,
        /// Nach dem Übernehmen im laufenden Spiel neu laden
        neuladen: bool,
        /// Auch übernehmen, wenn die Prüfung Fehler findet
        trotz_fehlern: bool,
        /// Auch übernehmen, wenn sich die Dateien auf dem Server geändert haben
        ueberschreiben: bool,
    },
    Pruefen,
    Neuladen,
    /// Die Engine auf dem Server aktualisieren und neu starten
    Aktualisieren,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "art", rename_all = "snake_case")]
pub enum Antwort {
    Angemeldet { version: String, welt: String, spieler_online: usize },
    Fehler { text: String },
    Dateien { dateien: BTreeMap<String, String> },
    /// Ergebnis von Prüfen, Neuladen, Übernehmen oder Aktualisieren
    Bericht { ok: bool, text: String },
    /// Diese Dateien wurden auf dem Server inzwischen anders geändert
    Konflikt { pfade: Vec<String> },
}

/// Darf diese Datei über die Fernwartung gelesen oder geschrieben werden?
/// Nur Welt-Dateien im Datenordner – keine Spielstände, keine Programme.
pub fn pfad_erlaubt(p: &str) -> bool {
    let endung_ok = p.ends_with(".toml") || p.ends_with(".txt") || p.ends_with(".md");
    !p.is_empty()
        && p.len() < 300
        && endung_ok
        && !p.starts_with('/')
        && !p.contains('\\')
        && !p.contains(':')
        && p.split('/').all(|t| !t.is_empty() && t != "." && t != ".." && !t.starts_with('.'))
}

/// Hash einer Datei, um Änderungen zu erkennen.
pub fn hash(inhalt: &str) -> String {
    Sha256::digest(inhalt.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// Fingerabdruck eines Zertifikats, z.B. "3F:A2:...".
pub fn fingerabdruck(der: &[u8]) -> String {
    Sha256::digest(der).iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(":")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nur_weltdateien_erlaubt() {
        for ok in ["welt.toml", "raeume/dorf.toml", "oberwelt/karte.txt", "befehle/gesten.toml"] {
            assert!(pfad_erlaubt(ok), "{ok}");
        }
        for nein in [
            "../spieler/admin.toml", "/etc/passwd", "raeume/../../x.toml", "spieler.sh", "a\\b.toml", "",
            ".git/config.toml", "C:/x.toml", "raeume//dorf.toml", "rmengine",
        ] {
            assert!(!pfad_erlaubt(nein), "{nein}");
        }
    }
}
