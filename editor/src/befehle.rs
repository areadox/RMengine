//! Befehle und Gesten im Editor: daten/befehle.toml und daten/befehle/*.toml.
//!
//! Gespeichert wird nur, was vom Standard abweicht. Kommentare bleiben erhalten.

use crate::befehlsliste::{self, EINGEBAUT, RICHTUNGSWOERTER, ROLLEN};
use crate::welt::{kommentar_ans_ende, kommentare_umverteilen, praefix, praefix_voranstellen, s, setze_liste, setze_text, wert_setzen};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::{ArrayOfTables, DocumentMut, Item, Table, Value};

/// Wie die Engine Wörter vergleicht.
pub fn normalisieren(t: &str) -> String {
    t.trim().to_lowercase().replace('ä', "ae").replace('ö', "oe").replace('ü', "ue").replace('ß', "ss")
}

#[derive(Debug, Clone, PartialEq)]
pub struct Befehl {
    pub id: &'static str,
    pub woerter: Vec<String>,
    pub rolle: String,
    pub aktiv: bool,
    pub hilfe: String,
    pub syntax: String,
}

impl Befehl {
    pub fn standard(&self) -> &'static befehlsliste::Eingebaut {
        befehlsliste::finde(self.id).expect("eingebaut")
    }
    fn aus_standard(e: &'static befehlsliste::Eingebaut) -> Befehl {
        Befehl {
            id: e.id,
            woerter: e.woerter.iter().map(|w| w.to_string()).collect(),
            rolle: e.rolle.to_string(),
            aktiv: true,
            hilfe: e.hilfe.to_string(),
            syntax: e.syntax.to_string(),
        }
    }
    pub fn ist_standard(&self) -> bool {
        *self == Befehl::aus_standard(self.standard())
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Geste {
    pub woerter: Vec<String>,
    pub hilfe: String,
    pub rolle: String,
    pub aktiv: bool,
    pub selbst: String,
    pub andere: String,
    pub selbst_ziel: String,
    pub ziel: String,
    pub andere_ziel: String,
    pub datei: String,
    /// Position in der Datei beim Laden (None = neu)
    pub herkunft: Option<usize>,
}

impl Geste {
    pub fn neu(datei: &str) -> Geste {
        Geste { rolle: "spieler".into(), aktiv: true, datei: datei.into(), ..Default::default() }
    }
}

pub struct Befehlswerk {
    pub ordner: PathBuf,
    pub befehle: Vec<Befehl>,
    pub gesten: Vec<Geste>,
    geladen: (Vec<Befehl>, Vec<Geste>),
    /// In welcher Datei steht [befehl.<id>]?
    ort: HashMap<String, String>,
    pub dateien: Vec<String>,
}

fn datei_liste(ordner: &Path) -> Vec<String> {
    let mut liste = Vec::new();
    if ordner.join("befehle.toml").exists() {
        liste.push("befehle.toml".to_string());
    }
    if let Ok(e) = fs::read_dir(ordner.join("befehle")) {
        let mut teil: Vec<String> = e
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "toml"))
            .map(|e| format!("befehle/{}", e.file_name().to_string_lossy()))
            .collect();
        teil.sort();
        liste.extend(teil);
    }
    liste
}

impl Befehlswerk {
    pub fn laden(ordner: &Path) -> Result<Befehlswerk, String> {
        let mut befehle: Vec<Befehl> = EINGEBAUT.iter().map(Befehl::aus_standard).collect();
        let mut gesten = Vec::new();
        let mut ort = HashMap::new();
        let dateien = datei_liste(ordner);
        for rel in &dateien {
            let text = fs::read_to_string(ordner.join(rel)).map_err(|e| format!("{rel}: {e}"))?;
            let doc: DocumentMut = text.parse().map_err(|e| format!("Fehler in '{rel}':\n{e}"))?;
            if let Some(t) = doc.get("befehl").and_then(|i| i.as_table()) {
                for (id, item) in t.iter() {
                    let Some(a) = item.as_table() else { continue };
                    ort.insert(id.to_string(), rel.clone());
                    let Some(b) = befehle.iter_mut().find(|b| b.id == id) else { continue };
                    if a.contains_key("woerter") {
                        b.woerter = crate::welt::liste(a, "woerter");
                    }
                    if let Some(r) = a.get("rolle").and_then(|i| i.as_str()) {
                        b.rolle = r.into();
                    }
                    if let Some(x) = a.get("aktiv").and_then(|i| i.as_bool()) {
                        b.aktiv = x;
                    }
                    if let Some(h) = a.get("hilfe").and_then(|i| i.as_str()) {
                        b.hilfe = h.into();
                    }
                    if let Some(h) = a.get("syntax").and_then(|i| i.as_str()) {
                        b.syntax = h.into();
                    }
                }
            }
            if let Some(Item::ArrayOfTables(a)) = doc.get("geste") {
                for (n, t) in a.iter().enumerate() {
                    gesten.push(Geste {
                        woerter: crate::welt::liste(t, "woerter"),
                        hilfe: s(t, "hilfe"),
                        rolle: t.get("rolle").and_then(|i| i.as_str()).unwrap_or("spieler").into(),
                        aktiv: t.get("aktiv").and_then(|i| i.as_bool()).unwrap_or(true),
                        selbst: s(t, "selbst"),
                        andere: s(t, "andere"),
                        selbst_ziel: s(t, "selbst_ziel"),
                        ziel: s(t, "ziel"),
                        andere_ziel: s(t, "andere_ziel"),
                        datei: rel.clone(),
                        herkunft: Some(n),
                    });
                }
            }
        }
        Ok(Befehlswerk {
            ordner: ordner.to_path_buf(),
            geladen: (befehle.clone(), gesten.clone()),
            befehle,
            gesten,
            ort,
            dateien,
        })
    }

    pub fn geaendert(&self) -> bool {
        self.befehle != self.geladen.0 || self.gesten != self.geladen.1
    }

    /// Wohin neue Anpassungen und Gesten kommen.
    pub fn standard_datei(&self, gesten: bool) -> String {
        if self.dateien == ["befehle.toml"] {
            return "befehle.toml".into();
        }
        if gesten { "befehle/gesten.toml".into() } else { "befehle/anpassungen.toml".into() }
    }

    /// Wie die Engine: doppelte Wörter, Richtungen, fehlende Texte.
    pub fn pruefen(&self) -> Vec<(bool, String)> {
        let mut p = Vec::new();
        let mut belegt: HashMap<String, String> = HashMap::new();
        let mut eintragen = |p: &mut Vec<(bool, String)>, wort: &str, wer: String| {
            let w = normalisieren(wort);
            if w.is_empty() {
                return;
            }
            if RICHTUNGSWOERTER.contains(&w.as_str()) {
                p.push((false, format!("'{w}' ({wer}) ist auch eine Richtung – zum Gehen muss man dann 'gehe {w}' tippen.")));
            }
            match belegt.get(&w) {
                Some(anderer) if *anderer != wer => p.push((true, format!("Das Wort '{w}' gehört zu {anderer} und zu {wer}."))),
                Some(_) => {}
                None => {
                    belegt.insert(w, wer);
                }
            }
        };
        for b in &self.befehle {
            if !ROLLEN.contains(&b.rolle.as_str()) {
                p.push((true, format!("Befehl '{}': Rolle '{}' gibt es nicht.", b.id, b.rolle)));
            }
            if b.woerter.iter().all(|w| w.trim().is_empty()) {
                p.push((true, format!("Befehl '{}': Es fehlt mindestens ein Wort. Zum Abschalten 'aktiv' ausschalten.", b.id)));
            }
            if !b.aktiv {
                if matches!(b.id, "hilfe" | "ende") {
                    p.push((false, format!("Befehl '{}' ist abgeschaltet – ohne ihn kommen Spieler kaum zurecht.", b.id)));
                }
                continue;
            }
            for w in &b.woerter {
                eintragen(&mut p, w, format!("Befehl '{}'", b.id));
            }
        }
        for (i, g) in self.gesten.iter().enumerate() {
            let name = g.woerter.first().cloned().unwrap_or_else(|| format!("Nr. {}", i + 1));
            if g.woerter.iter().all(|w| w.trim().is_empty()) {
                p.push((true, format!("Geste {name}: Es fehlen die Wörter.")));
            }
            if g.selbst.trim().is_empty() && g.selbst_ziel.trim().is_empty() {
                p.push((true, format!("Geste '{name}': Es fehlt der Text 'selbst' (oder 'selbst_ziel').")));
            }
            if !ROLLEN.contains(&g.rolle.as_str()) {
                p.push((true, format!("Geste '{name}': Rolle '{}' gibt es nicht.", g.rolle)));
            }
            if g.aktiv {
                for w in &g.woerter {
                    eintragen(&mut p, w, format!("Geste '{name}'"));
                }
            }
        }
        p
    }

    pub fn speichern(&mut self) -> Result<Vec<String>, String> {
        if !self.geaendert() {
            return Ok(Vec::new());
        }
        let mut docs: BTreeMap<String, DocumentMut> = BTreeMap::new();
        let mut alle: BTreeSet<String> = self.dateien.iter().cloned().collect();
        alle.extend(self.gesten.iter().map(|g| g.datei.clone()));
        alle.insert(self.standard_datei(false));
        for rel in &alle {
            let pfad = self.ordner.join(rel);
            let doc = if pfad.exists() {
                fs::read_to_string(&pfad).map_err(|e| format!("{rel}: {e}"))?.parse().map_err(|e| format!("{rel}: {e}"))?
            } else {
                DocumentMut::new()
            };
            docs.insert(rel.clone(), doc);
        }
        let vorher: BTreeMap<String, String> = docs.iter().map(|(k, d)| (k.clone(), d.to_string())).collect();

        // Anpassungen eingebauter Befehle
        for b in &self.befehle {
            let st = Befehl::aus_standard(b.standard());
            let datei = self.ort.get(b.id).cloned().unwrap_or_else(|| self.standard_datei(false));
            let doc = docs.get_mut(&datei).expect("geladen");
            if *b == st {
                let mut uebrig = None;
                if let Some(t) = doc.get_mut("befehl").and_then(|i| i.as_table_mut()) {
                    if let Some(Item::Table(weg)) = t.remove(b.id) {
                        // Kommentar über der gelöschten Anpassung an die nächste weitergeben
                        let p = praefix(&weg);
                        let pos = weg.position().unwrap_or(isize::MAX);
                        let naechste = t
                            .iter_mut()
                            .filter_map(|(_, i)| i.as_table_mut())
                            .filter(|n| n.position().unwrap_or(isize::MAX) > pos)
                            .min_by_key(|n| n.position().unwrap_or(isize::MAX));
                        match naechste {
                            Some(n) => praefix_voranstellen(n, &p),
                            None => uebrig = Some(p),
                        }
                    }
                }
                if let Some(p) = uebrig {
                    kommentar_ans_ende(doc, &p);
                }
                continue;
            }
            if doc.get("befehl").is_none() {
                let mut t = Table::new();
                t.set_implicit(true);
                doc.insert("befehl", Item::Table(t));
            }
            let oben = doc.get_mut("befehl").and_then(|i| i.as_table_mut()).ok_or("befehl ist keine Tabelle")?;
            if !oben.contains_key(b.id) {
                let mut t = Table::new();
                t.decor_mut().set_prefix("\n");
                oben.insert(b.id, Item::Table(t));
            }
            let t = oben.get_mut(b.id).and_then(|i| i.as_table_mut()).ok_or("Befehl ist keine Tabelle")?;
            if b.woerter != st.woerter {
                setze_liste(t, "woerter", &b.woerter);
            } else {
                t.remove("woerter");
            }
            if b.rolle != st.rolle {
                setze_text(t, "rolle", &b.rolle, true);
            } else {
                t.remove("rolle");
            }
            if !b.aktiv {
                if t.get("aktiv").and_then(|i| i.as_bool()) != Some(false) {
                    wert_setzen(t, "aktiv", Value::from(false));
                }
            } else {
                t.remove("aktiv");
            }
            for (k, wert, standard) in [("hilfe", &b.hilfe, &st.hilfe), ("syntax", &b.syntax, &st.syntax)] {
                if wert != standard {
                    setze_text(t, k, wert, true);
                } else {
                    t.remove(k);
                }
            }
        }

        // Gesten: je Datei neu zusammenstellen, vorhandene Tabellen (mit Kommentaren) wiederverwenden
        for (rel, doc) in docs.iter_mut() {
            let alte: Vec<Table> = match doc.get("geste") {
                Some(Item::ArrayOfTables(a)) => a.iter().cloned().collect(),
                _ => Vec::new(),
            };
            let meine: Vec<&Geste> = self.gesten.iter().filter(|g| g.datei == *rel).collect();
            if alte.is_empty() && meine.is_empty() {
                continue;
            }
            let bleibt: Vec<bool> = (0..alte.len()).map(|n| meine.iter().any(|g| g.herkunft == Some(n))).collect();
            let (extra, rest) = kommentare_umverteilen(&alte, &bleibt);
            let mut neu = ArrayOfTables::new();
            for g in meine {
                let mut t = g.herkunft.and_then(|n| alte.get(n).cloned()).unwrap_or_else(|| {
                    let mut t = Table::new();
                    t.decor_mut().set_prefix("\n");
                    t
                });
                geste_schreiben(&mut t, g);
                if let Some(p) = g.herkunft.and_then(|n| extra.get(&n)) {
                    praefix_voranstellen(&mut t, p);
                }
                neu.push(t);
            }
            if neu.is_empty() {
                doc.remove("geste");
            } else {
                doc.insert("geste", Item::ArrayOfTables(neu));
            }
            kommentar_ans_ende(doc, &rest);
        }

        let mut geschrieben = Vec::new();
        for (rel, doc) in &docs {
            let mut t = doc.to_string();
            if vorher.get(rel) == Some(&t) || (t.trim().is_empty() && !self.ordner.join(rel).exists()) {
                continue;
            }
            if !t.ends_with('\n') {
                t.push('\n');
            }
            let pfad = self.ordner.join(rel);
            if let Some(o) = pfad.parent() {
                fs::create_dir_all(o).map_err(|e| format!("{rel}: {e}"))?;
            }
            fs::write(&pfad, t).map_err(|e| format!("{rel}: {e}"))?;
            geschrieben.push(rel.clone());
        }
        Ok(geschrieben)
    }
}

fn geste_schreiben(t: &mut Table, g: &Geste) {
    let alt = crate::welt::liste(t, "woerter");
    if alt != g.woerter {
        setze_liste(t, "woerter", &g.woerter);
    }
    setze_text(t, "hilfe", &g.hilfe, false);
    if g.rolle != "spieler" {
        setze_text(t, "rolle", &g.rolle, true);
    } else {
        t.remove("rolle");
    }
    if !g.aktiv {
        wert_setzen(t, "aktiv", Value::from(false));
    } else {
        t.remove("aktiv");
    }
    for (k, v) in [
        ("selbst", &g.selbst),
        ("andere", &g.andere),
        ("selbst_ziel", &g.selbst_ziel),
        ("ziel", &g.ziel),
        ("andere_ziel", &g.andere_ziel),
    ] {
        setze_text(t, k, v, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kopie(name: &str) -> PathBuf {
        let von = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../daten");
        let ziel = std::env::temp_dir().join(format!("rmeditor_befehle_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&ziel);
        fn kopieren(von: &Path, nach: &Path) {
            fs::create_dir_all(nach).unwrap();
            for e in fs::read_dir(von).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    kopieren(&p, &nach.join(e.file_name()));
                } else {
                    fs::copy(&p, nach.join(e.file_name())).unwrap();
                }
            }
        }
        kopieren(&von, &ziel);
        ziel
    }

    #[test]
    fn beispiel_laden_und_pruefen() {
        let w = Befehlswerk::laden(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../daten")).unwrap();
        assert!(w.befehle.iter().find(|b| b.id == "toete").unwrap().woerter.contains(&"attacke".to_string()));
        assert!(w.gesten.iter().any(|g| g.woerter.first().map(String::as_str) == Some("winke")));
        let p = w.pruefen();
        assert!(p.is_empty(), "{p:?}");
    }

    #[test]
    fn speichern_nur_abweichungen() {
        let ordner = kopie("speichern");
        let mut w = Befehlswerk::laden(&ordner).unwrap();
        assert!(w.speichern().unwrap().is_empty());

        // Weltkarte nur für Baumeister, toete zurück auf Standard, neue Geste
        w.befehle.iter_mut().find(|b| b.id == "weltkarte").unwrap().rolle = "baumeister".into();
        let toete = w.befehle.iter_mut().find(|b| b.id == "toete").unwrap();
        *toete = Befehl::aus_standard(befehlsliste::finde("toete").unwrap());
        let mut g = Geste::neu("befehle/gesten.toml");
        g.woerter = vec!["klatsche".into()];
        g.selbst = "Du klatschst.".into();
        w.gesten.push(g);
        w.speichern().unwrap();

        let a = fs::read_to_string(ordner.join("befehle/anpassungen.toml")).unwrap();
        assert!(a.contains("[befehl.weltkarte]") && a.contains("rolle = \"baumeister\""), "{a}");
        assert!(!a.contains("[befehl.toete]"), "{a}");
        assert!(a.contains("#  Eingebaute Befehle anpassen")); // Kommentar bleibt
        let g = fs::read_to_string(ordner.join("befehle/gesten.toml")).unwrap();
        assert!(g.contains("klatsche") && g.contains("#  Gesten: eigene Befehle mit Texten"));

        let w2 = Befehlswerk::laden(&ordner).unwrap();
        assert_eq!(w2.befehle.iter().find(|b| b.id == "weltkarte").unwrap().rolle, "baumeister");
        assert!(w2.pruefen().iter().all(|(f, _)| !f));
        let _ = fs::remove_dir_all(ordner);
    }

    #[test]
    fn doppeltes_wort_wird_gemeldet() {
        let mut w = Befehlswerk::laden(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../daten")).unwrap();
        w.befehle.iter_mut().find(|b| b.id == "karte").unwrap().woerter.push("kill".into());
        assert!(w.pruefen().iter().any(|(f, t)| *f && t.contains("'kill'")));
    }
}
