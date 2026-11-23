//! Zauber im Editor: laden, mit Vererbung anzeigen, prüfen und speichern.
//!
//! Jedes Feld ist entweder in der Datei gesetzt (`Some`) oder nicht (`None`).
//! Nicht gesetzte Felder kommen aus der Vorlage (`erbt`) oder dem Standardwert –
//! der Editor zeigt sie grau an und schreibt sie nicht in die Datei.

use crate::welt::{
    dateien_sammeln, id_gueltig, kommentar_ans_ende, kommentare_umverteilen, liste, neue_tabelle, praefix_voranstellen, s,
    setze_liste, setze_text, wert_setzen,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::{Array, ArrayOfTables, DocumentMut, InlineTable, Item, Table, Value};

pub const ZIELE: &[&str] = &["selbst", "spieler", "gegner", "raum"];
pub const WIRKUNGEN: &[&str] = &["heilen", "mana", "schaden", "teleport"];

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Wirkung {
    pub art: String,
    pub wert: i32,
    pub raum: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Zauber {
    pub id: String,
    pub erbt: String,
    pub vorlage: bool,
    pub name: Option<String>,
    pub beschreibung: Option<String>,
    pub schluesselwoerter: Option<Vec<String>>,
    pub mana_kosten: Option<i32>,
    pub ziel: Option<String>,
    pub fertigkeit: Option<String>,
    pub mindest_stufe: Option<i32>,
    pub abklingzeit: Option<i32>,
    pub wirkungen: Option<Vec<Wirkung>>,
    pub text_selbst: Option<String>,
    pub text_ziel: Option<String>,
    pub text_raum: Option<String>,
    /// Datei relativ zum Datenordner
    pub datei: String,
    /// ID beim Laden (None = neu)
    pub herkunft: Option<String>,
}

/// Die tatsächlichen Werte eines Zaubers (nach Vererbung und Standardwerten).
#[derive(Debug, Clone, Default)]
pub struct Wirksam {
    pub name: String,
    pub beschreibung: String,
    pub schluesselwoerter: Vec<String>,
    pub mana_kosten: i32,
    pub ziel: String,
    pub fertigkeit: String,
    pub mindest_stufe: i32,
    pub abklingzeit: i32,
    pub wirkungen: Vec<Wirkung>,
    pub text_selbst: String,
    pub text_ziel: String,
    pub text_raum: String,
}

pub struct Zauberbuch {
    pub ordner: PathBuf,
    pub zauber: Vec<Zauber>,
    geladen: Vec<Zauber>,
    pub fertigkeiten: BTreeSet<String>,
    /// Alle Dateien, in denen Zauber stehen dürfen
    pub dateien: Vec<String>,
}

fn zahl(t: &Table, k: &str) -> Option<i32> {
    t.get(k).and_then(|i| i.as_integer()).map(|v| v as i32)
}
fn text(t: &Table, k: &str) -> Option<String> {
    t.get(k).and_then(|i| i.as_str()).map(String::from)
}

fn wirkungen_lesen(t: &Table) -> Option<Vec<Wirkung>> {
    let aus_tabelle = |tl: &dyn toml_edit::TableLike| Wirkung {
        art: tl.get("art").and_then(|i| i.as_str()).unwrap_or_default().to_string(),
        wert: tl.get("wert").and_then(|i| i.as_integer()).unwrap_or(0) as i32,
        raum: tl.get("raum").and_then(|i| i.as_str()).unwrap_or_default().to_string(),
    };
    match t.get("wirkungen")? {
        Item::Value(Value::Array(a)) => Some(a.iter().filter_map(|v| v.as_inline_table()).map(|it| aus_tabelle(it)).collect()),
        Item::ArrayOfTables(a) => Some(a.iter().map(|t| aus_tabelle(t)).collect()),
        _ => None,
    }
}

fn zauber_lesen(t: &Table, datei: &str) -> Zauber {
    let id = s(t, "id");
    Zauber {
        id: id.clone(),
        erbt: s(t, "erbt"),
        vorlage: t.get("vorlage").and_then(|i| i.as_bool()).unwrap_or(false),
        name: text(t, "name"),
        beschreibung: text(t, "beschreibung"),
        schluesselwoerter: t.get("schluesselwoerter").map(|_| liste(t, "schluesselwoerter")),
        mana_kosten: zahl(t, "mana_kosten"),
        ziel: text(t, "ziel"),
        fertigkeit: text(t, "fertigkeit"),
        mindest_stufe: zahl(t, "mindest_stufe"),
        abklingzeit: zahl(t, "abklingzeit"),
        wirkungen: wirkungen_lesen(t),
        text_selbst: text(t, "text_selbst"),
        text_ziel: text(t, "text_ziel"),
        text_raum: text(t, "text_raum"),
        datei: datei.to_string(),
        herkunft: Some(id),
    }
}

impl Zauberbuch {
    pub fn laden(ordner: &Path) -> Result<Zauberbuch, String> {
        let mut dateien = Vec::new();
        dateien_sammeln(ordner, ordner, &mut dateien);
        dateien.sort();
        let mut zauber = Vec::new();
        let mut fertigkeiten = BTreeSet::new();
        for rel in &dateien {
            let text = fs::read_to_string(ordner.join(rel)).map_err(|e| format!("{rel}: {e}"))?;
            let doc: DocumentMut = text.parse().map_err(|e| format!("Fehler in '{rel}':\n{e}"))?;
            if let Some(Item::ArrayOfTables(a)) = doc.get("zauber") {
                zauber.extend(a.iter().map(|t| zauber_lesen(t, rel)));
            }
            if let Some(Item::ArrayOfTables(a)) = doc.get("fertigkeit") {
                fertigkeiten.extend(a.iter().map(|t| s(t, "id")).filter(|i| !i.is_empty()));
            }
        }
        Ok(Zauberbuch { ordner: ordner.to_path_buf(), geladen: zauber.clone(), zauber, fertigkeiten, dateien })
    }

    pub fn index(&self, id: &str) -> Option<usize> {
        self.zauber.iter().position(|z| z.id == id)
    }

    pub fn geaendert(&self) -> bool {
        self.zauber != self.geladen
    }

    /// Die Werte nach Vererbung (Kreise werden abgebrochen).
    pub fn wirksam(&self, i: usize) -> Wirksam {
        let mut kette = vec![i];
        let mut j = i;
        while let Some(e) = self.index(&self.zauber[j].erbt) {
            if kette.contains(&e) {
                break;
            }
            kette.push(e);
            j = e;
        }
        let mut w = Wirksam { ziel: "selbst".into(), ..Default::default() };
        for &k in kette.iter().rev() {
            let z = &self.zauber[k];
            macro_rules! ueber {
                ($f:ident) => {
                    if let Some(v) = &z.$f {
                        w.$f = v.clone();
                    }
                };
            }
            ueber!(name);
            ueber!(beschreibung);
            ueber!(schluesselwoerter);
            ueber!(mana_kosten);
            ueber!(ziel);
            ueber!(fertigkeit);
            ueber!(mindest_stufe);
            ueber!(abklingzeit);
            ueber!(wirkungen);
            ueber!(text_selbst);
            ueber!(text_ziel);
            ueber!(text_raum);
        }
        w
    }

    /// Wirksame Werte der Vorlage (für die grauen Hinweise im Editor).
    pub fn geerbt(&self, i: usize) -> Option<Wirksam> {
        self.index(&self.zauber[i].erbt).map(|e| self.wirksam(e))
    }

    pub fn neue_id(&self, basis: &str) -> String {
        if self.index(basis).is_none() && id_gueltig(basis) {
            return basis.to_string();
        }
        (1..).map(|n| format!("{basis}_{n}")).find(|id| self.index(id).is_none()).unwrap()
    }

    /// Prüft die Zauber. `raum_gueltig` sagt, ob ein Teleport-Ziel existiert.
    pub fn pruefen(&self, raum_gueltig: &dyn Fn(&str) -> bool) -> Vec<String> {
        let mut p = Vec::new();
        let mut gesehen = BTreeSet::new();
        for (i, z) in self.zauber.iter().enumerate() {
            let wo = format!("Zauber '{}'", z.id);
            if !id_gueltig(&z.id) {
                p.push(format!("{wo}: ungültige ID (nur a-z, 0-9 und _)."));
            }
            if !gesehen.insert(z.id.clone()) {
                p.push(format!("{wo} gibt es doppelt."));
            }
            if !z.erbt.is_empty() {
                if self.index(&z.erbt).is_none() {
                    p.push(format!("{wo}: erbt von '{}', den es nicht gibt.", z.erbt));
                } else {
                    // Kreis?
                    let mut j = i;
                    let mut schritte = 0;
                    while let Some(e) = self.index(&self.zauber[j].erbt) {
                        schritte += 1;
                        if e == i || schritte > self.zauber.len() {
                            p.push(format!("{wo}: Die Vererbung dreht sich im Kreis."));
                            break;
                        }
                        j = e;
                    }
                }
            }
            if z.vorlage {
                continue;
            }
            let w = self.wirksam(i);
            if w.name.trim().is_empty() {
                p.push(format!("{wo}: Es fehlt der Name."));
            }
            if !ZIELE.contains(&w.ziel.as_str()) {
                p.push(format!("{wo}: Ziel '{}' gibt es nicht.", w.ziel));
            }
            if !w.fertigkeit.is_empty() && !self.fertigkeiten.contains(&w.fertigkeit) {
                p.push(format!("{wo}: Fertigkeit '{}' gibt es nicht.", w.fertigkeit));
            }
            if w.mana_kosten < 0 {
                p.push(format!("{wo}: Mana-Kosten können nicht negativ sein."));
            }
            if w.wirkungen.is_empty() {
                p.push(format!("{wo}: hat keine Wirkung (Hinweis)."));
            }
            for wi in &w.wirkungen {
                if !WIRKUNGEN.contains(&wi.art.as_str()) {
                    p.push(format!("{wo}: Wirkung '{}' gibt es nicht.", wi.art));
                }
                if wi.art == "teleport" && !raum_gueltig(&wi.raum) {
                    p.push(format!("{wo}: Teleport-Ziel '{}' gibt es nicht.", wi.raum));
                }
            }
        }
        p
    }

    /// Schreibt alle Änderungen. Die Dateien werden frisch gelesen, damit
    /// vorher gespeicherte Raum-Änderungen erhalten bleiben.
    pub fn speichern(&mut self) -> Result<Vec<String>, String> {
        if !self.geaendert() {
            return Ok(Vec::new());
        }
        let umbenannt: BTreeMap<String, String> = self
            .zauber
            .iter()
            .filter_map(|z| z.herkunft.as_ref().filter(|h| **h != z.id).map(|h| (h.clone(), z.id.clone())))
            .collect();
        let geloescht: BTreeSet<String> = self
            .geladen
            .iter()
            .filter_map(|z| z.herkunft.clone())
            .filter(|h| !self.zauber.iter().any(|z| z.herkunft.as_ref() == Some(h)))
            .collect();

        let mut docs: BTreeMap<String, DocumentMut> = BTreeMap::new();
        let mut alle: BTreeSet<String> = self.dateien.iter().cloned().collect();
        alle.extend(self.zauber.iter().map(|z| z.datei.clone()));
        alle.insert("welt.toml".into());
        for rel in &alle {
            let pfad = self.ordner.join(rel);
            let doc = if pfad.exists() {
                fs::read_to_string(&pfad)
                    .map_err(|e| format!("{rel}: {e}"))?
                    .parse::<DocumentMut>()
                    .map_err(|e| format!("{rel}: {e}"))?
            } else {
                DocumentMut::new()
            };
            docs.insert(rel.clone(), doc);
        }
        let vorher: BTreeMap<String, String> = docs.iter().map(|(k, d)| (k.clone(), d.to_string())).collect();

        // 1. Gelöschte und verschobene Zauber aus ihren alten Dateien entfernen
        for (rel, doc) in docs.iter_mut() {
            let alte: Vec<Table> = match doc.get("zauber") {
                Some(Item::ArrayOfTables(a)) => a.iter().cloned().collect(),
                _ => continue,
            };
            let bleibt: Vec<bool> = alte
                .iter()
                .map(|t| {
                    let id = s(t, "id");
                    // gelöscht oder in eine andere Datei verschoben?
                    !geloescht.contains(&id)
                        && !self.zauber.iter().any(|z| z.herkunft.as_deref() == Some(id.as_str()) && z.datei != *rel)
                })
                .collect();
            if bleibt.iter().all(|b| *b) {
                continue;
            }
            let (extra, rest) = kommentare_umverteilen(&alte, &bleibt);
            let mut neu = ArrayOfTables::new();
            for (i, mut t) in alte.into_iter().enumerate() {
                if bleibt[i] {
                    if let Some(p) = extra.get(&i) {
                        praefix_voranstellen(&mut t, p);
                    }
                    neu.push(t);
                }
            }
            if neu.is_empty() {
                doc.remove("zauber");
            } else {
                doc.insert("zauber", Item::ArrayOfTables(neu));
            }
            kommentar_ans_ende(doc, &rest);
        }

        // 2. Geänderte Zauber schreiben, neue anhängen
        for z in &self.zauber {
            let alt = z.herkunft.as_ref().and_then(|h| self.geladen.iter().find(|g| g.herkunft.as_ref() == Some(h)));
            if alt == Some(z) {
                continue;
            }
            let doc = docs.get_mut(&z.datei).ok_or("Datei fehlt")?;
            if doc.get("zauber").is_none() {
                doc.insert("zauber", Item::ArrayOfTables(ArrayOfTables::new()));
            }
            let Some(Item::ArrayOfTables(a)) = doc.get_mut("zauber") else {
                return Err(format!("{}: 'zauber' ist kein [[zauber]]-Abschnitt", z.datei));
            };
            let such = z.herkunft.clone().unwrap_or_default();
            let pos = a.iter().position(|t| !such.is_empty() && s(t, "id") == such);
            match pos {
                Some(p) => zauber_schreiben(a.get_mut(p).unwrap(), z),
                None => {
                    let mut t = neue_tabelle();
                    zauber_schreiben(&mut t, z);
                    a.push(t);
                }
            }
        }

        // 3. Verweise auf umbenannte Zauber mitziehen
        if !umbenannt.is_empty() {
            for doc in docs.values_mut() {
                verweise_umbenennen(doc, &umbenannt);
            }
        }

        // Schreiben, was sich geändert hat
        let mut geschrieben = Vec::new();
        for (rel, doc) in &docs {
            let mut t = doc.to_string();
            if vorher.get(rel) == Some(&t) {
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

fn setze_zahl_opt(t: &mut Table, k: &str, v: Option<i32>) {
    let alt = t.get(k).and_then(|i| i.as_integer()).map(|x| x as i32);
    if alt == v {
        return;
    }
    match v {
        Some(v) => wert_setzen(t, k, Value::from(v as i64)),
        None => {
            t.remove(k);
        }
    }
}

fn setze_text_opt(t: &mut Table, k: &str, v: &Option<String>) {
    match v {
        Some(v) => {
            if t.get(k).and_then(|i| i.as_str()) != Some(v.as_str()) {
                setze_text(t, k, v, true);
            }
        }
        None => {
            t.remove(k);
        }
    }
}

fn zauber_schreiben(t: &mut Table, z: &Zauber) {
    setze_text(t, "id", &z.id, true);
    setze_text(t, "erbt", &z.erbt, false);
    if z.vorlage {
        if t.get("vorlage").and_then(|i| i.as_bool()) != Some(true) {
            wert_setzen(t, "vorlage", Value::from(true));
        }
    } else {
        t.remove("vorlage");
    }
    setze_text_opt(t, "name", &z.name);
    setze_text_opt(t, "beschreibung", &z.beschreibung);
    match &z.schluesselwoerter {
        Some(l) if l.is_empty() => {
            wert_setzen(t, "schluesselwoerter", Value::Array(Array::new()));
        }
        Some(l) => setze_liste(t, "schluesselwoerter", l),
        None => {
            t.remove("schluesselwoerter");
        }
    }
    setze_zahl_opt(t, "mana_kosten", z.mana_kosten);
    setze_text_opt(t, "ziel", &z.ziel);
    setze_text_opt(t, "fertigkeit", &z.fertigkeit);
    setze_zahl_opt(t, "mindest_stufe", z.mindest_stufe);
    setze_zahl_opt(t, "abklingzeit", z.abklingzeit);
    match &z.wirkungen {
        Some(w) => {
            if wirkungen_lesen(t).as_ref() != Some(w) {
                let mut a = Array::new();
                for wi in w {
                    let mut it = InlineTable::new();
                    it.insert("art", Value::from(wi.art.as_str()));
                    if wi.art == "teleport" {
                        it.insert("raum", Value::from(wi.raum.as_str()));
                    } else {
                        it.insert("wert", Value::from(wi.wert as i64));
                    }
                    it.fmt();
                    a.push(Value::InlineTable(it));
                }
                t.remove("wirkungen");
                wert_setzen(t, "wirkungen", Value::Array(a));
            }
        }
        None => {
            t.remove("wirkungen");
        }
    }
    setze_text_opt(t, "text_selbst", &z.text_selbst);
    setze_text_opt(t, "text_ziel", &z.text_ziel);
    setze_text_opt(t, "text_raum", &z.text_raum);
}

/// Verweise auf umbenannte Zauber: [start] zauber, Monster/NPCs (zauber = [...]) und erbt.
fn verweise_umbenennen(doc: &mut DocumentMut, umbenannt: &BTreeMap<String, String>) {
    fn liste_umbenennen(t: &mut Table, k: &str, umbenannt: &BTreeMap<String, String>) {
        let alt = liste(t, k);
        if alt.iter().any(|x| umbenannt.contains_key(x)) {
            let neu: Vec<String> = alt.iter().map(|x| umbenannt.get(x).cloned().unwrap_or_else(|| x.clone())).collect();
            setze_liste(t, k, &neu);
        }
    }
    if let Some(st) = doc.get_mut("start").and_then(|i| i.as_table_mut()) {
        liste_umbenennen(st, "zauber", umbenannt);
    }
    for art in ["monster", "zauber"] {
        if let Some(Item::ArrayOfTables(a)) = doc.get_mut(art) {
            for t in a.iter_mut() {
                if art == "monster" {
                    liste_umbenennen(t, "zauber", umbenannt);
                } else if let Some(neu) = umbenannt.get(&s(t, "erbt")) {
                    wert_setzen(t, "erbt", Value::from(neu.as_str()));
                }
            }
        }
    }
    if let Some(t) = doc.get_mut("npc").and_then(|i| i.as_table_mut()) {
        liste_umbenennen(t, "zauber", umbenannt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kopie(name: &str) -> PathBuf {
        let von = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../daten");
        let ziel = std::env::temp_dir().join(format!("rmeditor_zauber_{name}_{}", std::process::id()));
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
    fn laden_vererben_pruefen() {
        let b = Zauberbuch::laden(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../daten")).unwrap();
        let i = b.index("kleine_heilung").unwrap();
        let w = b.wirksam(i);
        assert_eq!(w.fertigkeit, "heilkunde"); // aus vorlage_heilzauber
        assert_eq!(w.ziel, "spieler");
        assert_eq!(w.mana_kosten, 10);
        assert!(b.pruefen(&|r| r == "dorfplatz").is_empty(), "{:?}", b.pruefen(&|r| r == "dorfplatz"));
    }

    #[test]
    fn unveraendert_speichern_aendert_nichts() {
        let ordner = kopie("unveraendert");
        let mut b = Zauberbuch::laden(&ordner).unwrap();
        assert!(b.speichern().unwrap().is_empty());
        let _ = fs::remove_dir_all(ordner);
    }

    #[test]
    fn aendern_neu_umbenennen() {
        let ordner = kopie("aendern");
        let mut b = Zauberbuch::laden(&ordner).unwrap();
        let i = b.index("feuerpfeil").unwrap();
        b.zauber[i].mana_kosten = Some(12);
        b.zauber[i].id = "flammenpfeil".into();
        b.zauber.push(Zauber {
            id: "eisstrahl".into(),
            erbt: "vorlage_kampfzauber".into(),
            name: Some("Eisstrahl".into()),
            mana_kosten: Some(9),
            wirkungen: Some(vec![Wirkung { art: "schaden".into(), wert: 10, raum: String::new() }]),
            datei: "zauber/zauber.toml".into(),
            ..Default::default()
        });
        let geschrieben = b.speichern().unwrap();
        assert!(geschrieben.contains(&"zauber/zauber.toml".to_string()));
        // Startzauber und die zaubernde Spinne wurden mit umbenannt
        assert!(geschrieben.contains(&"welt.toml".to_string()));
        assert!(fs::read_to_string(ordner.join("welt.toml")).unwrap().contains("\"flammenpfeil\""));
        assert!(fs::read_to_string(ordner.join("monster/wald.toml")).unwrap().contains("\"flammenpfeil\""));
        let text = fs::read_to_string(ordner.join("zauber/zauber.toml")).unwrap();
        assert!(text.contains("#  Zaubersprüche")); // Kommentar bleibt
        let b2 = Zauberbuch::laden(&ordner).unwrap();
        let j = b2.index("flammenpfeil").unwrap();
        assert_eq!(b2.wirksam(j).mana_kosten, 12);
        let k = b2.index("eisstrahl").unwrap();
        assert_eq!(b2.wirksam(k).fertigkeit, "magie");
        let _ = fs::remove_dir_all(ordner);
    }

    #[test]
    fn ersten_zauber_loeschen_behaelt_dateikommentar() {
        let ordner = kopie("loeschen");
        let mut b = Zauberbuch::laden(&ordner).unwrap();
        let i = b.index("kleine_heilung").unwrap();
        b.zauber.remove(i);
        b.speichern().unwrap();
        let text = fs::read_to_string(ordner.join("zauber/zauber.toml")).unwrap();
        assert!(text.contains("#  Zaubersprüche"), "{text}");
        assert!(!text.contains("\"kleine_heilung\""));
        let _ = fs::remove_dir_all(ordner);
    }
}
