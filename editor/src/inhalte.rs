//! Der Bereich "Inhalte": Gegenstände, Monster, NPCs, Fertigkeiten, Zauber,
//! Quests, Rassen, Gilden, Berufe, Beschreibungen und Welteinstellungen.
//!
//! Der Editor arbeitet direkt auf den TOML-Dokumenten (toml_edit). Was man im
//! Formular ändert, wird sofort im Dokument geändert; Kommentare und die
//! Reihenfolge bleiben erhalten. Felder, die leer sind, werden nicht geschrieben
//! (dann gilt der Standardwert oder die Vorlage, von der der Eintrag erbt).

use crate::schema::{Ablage, Art, Feld, Typ, Ziel, ARTEN, DETAIL_FELDER};
use crate::welt::{praefix, praefix_voranstellen, text_wert};
use eframe::egui::{self, Color32, RichText};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::{Array, ArrayOfTables, DocumentMut, InlineTable, Item, Table, TableLike, Value};

const GRAU: Color32 = Color32::from_rgb(140, 140, 140);
const ROT: Color32 = Color32::from_rgb(230, 90, 90);
const GELB: Color32 = Color32::from_rgb(230, 190, 90);

pub struct Datei {
    pub rel: String,
    pub doc: DocumentMut,
    original: String,
    /// Beim Speichern löschen (z.B. NPC-Datei eines gelöschten NPCs)
    pub loeschen: bool,
}

impl Datei {
    fn geaendert(&self) -> bool {
        self.loeschen || self.doc.to_string() != self.original
    }
}

/// Ein Eintrag: Datei und Position darin.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Ort {
    pub datei: usize,
    pub index: usize,
}

pub struct NeuDialog {
    id: String,
    datei: String,
    kopie_von: Option<Ort>,
}

pub struct Inhalte {
    pub ordner: PathBuf,
    pub dateien: Vec<Datei>,
    pub art: usize,
    pub auswahl: Option<Ort>,
    suche: String,
    neu: Option<NeuDialog>,
    loeschen_fragen: Option<Ort>,
    pub ladefehler: Vec<String>,
}

// ---------------------------------------------------------------------------
// Laden und Speichern
// ---------------------------------------------------------------------------

fn sammeln(ordner: &Path, basis: &Path, liste: &mut Vec<String>) {
    let Ok(eintraege) = fs::read_dir(ordner) else { return };
    for e in eintraege.flatten() {
        let p = e.path();
        if p.is_dir() {
            sammeln(&p, basis, liste);
        } else if p.extension().is_some_and(|x| x == "toml") {
            let rel = p.strip_prefix(basis).unwrap_or(&p).to_string_lossy().replace('\\', "/");
            // Befehle und Oberwelt haben eigene Bereiche
            if rel != "befehle.toml" && !rel.starts_with("befehle/") && !rel.starts_with("oberwelt/") {
                liste.push(rel);
            }
        }
    }
}

/// Bringt Änderungen in eine Datei, die inzwischen jemand anders geschrieben hat
/// (z.B. der Bereich Räume beim selben Speichern): Nur was wir geändert haben,
/// wird übernommen – Tabelle für Tabelle.
fn zusammenfuehren(frisch: &mut dyn TableLike, alt: &dyn TableLike, mein: &dyn TableLike) {
    let schluessel: BTreeSet<String> = alt.iter().map(|(k, _)| k.to_string()).chain(mein.iter().map(|(k, _)| k.to_string())).collect();
    for k in schluessel {
        let (a, m) = (alt.get(&k), mein.get(&k));
        if a.map(|i| i.to_string()) == m.map(|i| i.to_string()) {
            continue;
        }
        match (a.and_then(|i| i.as_table_like()), m.and_then(|i| i.as_table_like())) {
            (Some(at), Some(mt)) if frisch.get(&k).is_some_and(|i| i.is_table_like()) => {
                if let Some(ft) = frisch.get_mut(&k).and_then(|i| i.as_table_like_mut()) {
                    zusammenfuehren(ft, at, mt);
                }
            }
            _ => match m {
                Some(item) => {
                    frisch.insert(&k, item.clone());
                }
                None => {
                    frisch.remove(&k);
                }
            },
        }
    }
}

impl Inhalte {
    pub fn laden(ordner: &Path) -> Inhalte {
        let mut rel = Vec::new();
        sammeln(ordner, ordner, &mut rel);
        rel.sort();
        let mut dateien = Vec::new();
        let mut ladefehler = Vec::new();
        for r in rel {
            let text = match fs::read_to_string(ordner.join(&r)) {
                Ok(t) => t,
                Err(e) => {
                    ladefehler.push(format!("{r}: {e}"));
                    continue;
                }
            };
            match text.parse::<DocumentMut>() {
                Ok(doc) => dateien.push(Datei { rel: r, original: doc.to_string(), doc, loeschen: false }),
                Err(e) => ladefehler.push(format!("Fehler in '{r}' – die Datei kann hier nicht bearbeitet werden:\n{e}")),
            }
        }
        Inhalte { ordner: ordner.to_path_buf(), dateien, art: 0, auswahl: None, suche: String::new(), neu: None, loeschen_fragen: None, ladefehler }
    }

    pub fn geaendert(&self) -> bool {
        self.dateien.iter().any(|d| d.geaendert())
    }

    /// Speichert alle geänderten Dateien. Gibt die geschriebenen zurück.
    pub fn speichern(&mut self) -> Result<Vec<String>, String> {
        let mut geschrieben = Vec::new();
        for d in &mut self.dateien {
            if !d.geaendert() {
                continue;
            }
            let pfad = self.ordner.join(&d.rel);
            if d.loeschen {
                if pfad.exists() {
                    fs::remove_file(&pfad).map_err(|e| format!("{}: {e}", d.rel))?;
                }
                geschrieben.push(format!("{} (gelöscht)", d.rel));
                continue;
            }
            let mut text = d.doc.to_string();
            // Hat jemand die Datei inzwischen geändert? Dann zusammenführen.
            if let Ok(auf_platte) = fs::read_to_string(&pfad) {
                if auf_platte != d.original {
                    if let (Ok(mut frisch), Ok(alt)) = (auf_platte.parse::<DocumentMut>(), d.original.parse::<DocumentMut>()) {
                        zusammenfuehren(frisch.as_table_mut(), alt.as_table(), d.doc.as_table());
                        text = frisch.to_string();
                    }
                }
            }
            if let Some(o) = pfad.parent() {
                fs::create_dir_all(o).map_err(|e| e.to_string())?;
            }
            fs::write(&pfad, &text).map_err(|e| format!("{}: {e}", d.rel))?;
            d.original = d.doc.to_string();
            geschrieben.push(d.rel.clone());
        }
        Ok(geschrieben)
    }

    // -----------------------------------------------------------------------
    // Einträge finden
    // -----------------------------------------------------------------------

    pub fn orte(&self, art: &Art) -> Vec<Ort> {
        let mut v = Vec::new();
        for (i, d) in self.dateien.iter().enumerate() {
            if d.loeschen {
                continue;
            }
            match art.ablage {
                Ablage::Liste => {
                    if let Some(Item::ArrayOfTables(a)) = d.doc.get(art.schluessel) {
                        v.extend((0..a.len()).map(|index| Ort { datei: i, index }));
                    }
                }
                Ablage::EineDatei => {
                    if matches!(d.doc.get(art.schluessel), Some(Item::Table(_))) {
                        v.push(Ort { datei: i, index: 0 });
                    }
                }
                Ablage::Fest { datei } => {
                    if d.rel == datei {
                        v.push(Ort { datei: i, index: 0 });
                    }
                }
            }
        }
        v
    }

    pub fn tabelle(&self, art: &Art, o: Ort) -> Option<&Table> {
        let doc = &self.dateien.get(o.datei)?.doc;
        match (art.ablage, doc.get(art.schluessel)?) {
            (Ablage::Liste, Item::ArrayOfTables(a)) => a.get(o.index),
            (_, Item::Table(t)) => Some(t),
            _ => None,
        }
    }

    fn tabelle_mut(&mut self, art: &Art, o: Ort) -> Option<&mut Table> {
        let doc = &mut self.dateien.get_mut(o.datei)?.doc;
        if let Ablage::Fest { .. } = art.ablage {
            if doc.get(art.schluessel).is_none() {
                let mut t = Table::new();
                t.decor_mut().set_prefix("\n");
                doc.insert(art.schluessel, Item::Table(t));
            }
        }
        match (art.ablage, doc.get_mut(art.schluessel)?) {
            (Ablage::Liste, Item::ArrayOfTables(a)) => a.get_mut(o.index),
            (_, Item::Table(t)) => Some(t),
            _ => None,
        }
    }

    /// Alle IDs einer Art (ohne Vorlagen, außer `mit_vorlagen`).
    fn ids_von(&self, schluessel: &[&str], mit_vorlagen: bool) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        for s in schluessel {
            if *s == "raum" {
                for d in self.dateien.iter().filter(|d| !d.loeschen) {
                    if let Some(Item::ArrayOfTables(a)) = d.doc.get("raum") {
                        v.extend(a.iter().filter_map(|t| t.get("id")?.as_str().map(String::from)));
                    }
                }
                continue;
            }
            let Some(art) = crate::schema::art(s) else { continue };
            for o in self.orte(art) {
                let Some(t) = self.tabelle(art, o) else { continue };
                let vorlage = t.get("vorlage").and_then(|i| i.as_bool()) == Some(true);
                if vorlage && !mit_vorlagen {
                    continue;
                }
                if let Some(id) = t.get("id").and_then(|i| i.as_str()) {
                    v.push(id.to_string());
                }
            }
        }
        v.sort();
        v.dedup();
        v
    }

    fn ids(&self, ziel: Ziel, art: &Art) -> Vec<String> {
        match ziel {
            Ziel::Gegenstand => self.ids_von(&["gegenstand"], false),
            Ziel::Wesen => self.ids_von(&["monster", "npc"], false),
            Ziel::Npc => self.ids_von(&["npc"], false),
            Ziel::Fertigkeit => self.ids_von(&["fertigkeit"], false),
            Ziel::Zauber => self.ids_von(&["zauber"], false),
            Ziel::Lehrbar => self.ids_von(&["fertigkeit", "zauber"], false),
            Ziel::Raum => self.ids_von(&["raum"], false),
            Ziel::Rasse => self.ids_von(&["rasse"], false),
            Ziel::Gilde => self.ids_von(&["gilde"], false),
            Ziel::Beruf => self.ids_von(&["beruf"], false),
            Ziel::Quest => self.ids_von(&["quest"], false),
            Ziel::Beschreibung => self.ids_von(&["beschreibung"], false),
            Ziel::Gleich => {
                if art.gruppe() == "monster" {
                    self.ids_von(&["monster", "npc"], true)
                } else {
                    self.ids_von(&[art.schluessel], true)
                }
            }
        }
    }

    /// Werte, die ein Eintrag von seinen Vorlagen erbt: Feldname -> Text.
    fn geerbt(&self, art: &Art, t: &Table) -> BTreeMap<String, String> {
        let mut aus = BTreeMap::new();
        let gruppe: Vec<&Art> = ARTEN.iter().filter(|a| a.gruppe() == art.gruppe() && a.hat_id()).collect();
        let mut naechste = t.get("erbt").and_then(|i| i.as_str()).map(String::from);
        let mut tiefe = 0;
        while let Some(id) = naechste.take() {
            tiefe += 1;
            if tiefe > 10 {
                break;
            }
            let eltern = gruppe.iter().find_map(|a| self.orte(a).into_iter().find_map(|o| self.tabelle(a, o).filter(|t| t.get("id").and_then(|i| i.as_str()) == Some(id.as_str()))));
            let Some(e) = eltern else { break };
            for (k, v) in e.iter() {
                if k != "id" && k != "vorlage" && k != "erbt" {
                    aus.entry(k.to_string()).or_insert_with(|| kurz(v));
                }
            }
            naechste = e.get("erbt").and_then(|i| i.as_str()).map(String::from);
        }
        aus
    }

    // -----------------------------------------------------------------------
    // Prüfen
    // -----------------------------------------------------------------------

    /// Doppelte IDs und Verweise ins Leere. (true = Fehler)
    pub fn pruefen(&self) -> Vec<(bool, String)> {
        let mut p = Vec::new();
        let mut kontext: HashMap<&'static str, Vec<String>> = HashMap::new();
        for art in ARTEN {
            let mut gesehen: BTreeMap<String, String> = BTreeMap::new();
            for o in self.orte(art) {
                let Some(t) = self.tabelle(art, o) else { continue };
                let datei = &self.dateien[o.datei].rel;
                if art.hat_id() {
                    let id = t.get("id").and_then(|i| i.as_str()).unwrap_or("");
                    if id.is_empty() {
                        p.push((true, format!("[{datei}] Ein Eintrag in {} hat keine ID.", art.titel)));
                        continue;
                    }
                    if let Some(andere) = gesehen.insert(id.to_string(), datei.clone()) {
                        p.push((true, format!("{} '{id}' gibt es doppelt ({andere} und {datei}).", art.einzahl)));
                    }
                }
                let wo = format!("{} '{}'", art.einzahl, t.get("id").and_then(|i| i.as_str()).unwrap_or(art.titel));
                if art.schluessel == "zauber" && lies_text(t, "typ") == "technik" && lies_text(t, "gilde").is_empty() && t.get("vorlage").and_then(|i| i.as_bool()) != Some(true) {
                    p.push((true, format!("{wo}: Kampftechniken gibt es nur für Gilden – bitte 'Nur für Gilde' wählen.")));
                }
                for feld in art.felder.iter().filter(|f| crate::schema::pflicht(art.schluessel, f.name)) {
                    if t.get(feld.name).is_none_or(|i| i.as_str() == Some("")) {
                        p.push((true, format!("{wo}: Das Feld '{}' fehlt.", feld.titel)));
                    }
                }
                self.verweise_pruefen(art, t, art.felder, &wo, &mut kontext, &mut p);
            }
        }
        p
    }

    fn verweise_pruefen(
        &self,
        art: &Art,
        t: &dyn TableLike,
        felder: &[Feld],
        wo: &str,
        kontext: &mut HashMap<&'static str, Vec<String>>,
        p: &mut Vec<(bool, String)>,
    ) {
        for feld in felder {
            let Some(item) = t.get(feld.name) else { continue };
            match feld.typ {
                Typ::Verweis(z) => {
                    if let Some(s) = item.as_str() {
                        self.verweis(art, z, s, feld, wo, kontext, p);
                    }
                }
                Typ::VerweisListe(z) => {
                    for s in item.as_array().into_iter().flatten().filter_map(|v| v.as_str()) {
                        self.verweis(art, z, s, feld, wo, kontext, p);
                    }
                }
                Typ::VerweisWerte(z) => {
                    if let Some(tl) = item.as_table_like() {
                        for (k, _) in tl.iter() {
                            self.verweis(art, z, k, feld, wo, kontext, p);
                        }
                    }
                }
                Typ::Unter(unter) => {
                    if let Some(tl) = item.as_table_like() {
                        self.verweise_pruefen(art, tl, unter, wo, kontext, p);
                    }
                }
                Typ::Tabellen { felder: unter, .. } => {
                    for (n, tl) in untertabellen(item).into_iter().enumerate() {
                        let wo2 = format!("{wo}, {} Nr. {}", feld.titel, n + 1);
                        for f in unter.iter().filter(|f| crate::schema::pflicht(feld.name, f.name)) {
                            if tl.get(f.name).is_none_or(|i| i.as_str() == Some("")) {
                                p.push((true, format!("{wo2}: Das Feld '{}' fehlt.", f.titel)));
                            }
                        }
                        self.verweise_pruefen(art, tl, unter, &wo2, kontext, p);
                        if unter.iter().any(|f| matches!(f.typ, Typ::SchrittZiel)) {
                            let schritt_art = tl.get("art").and_then(|i| i.as_str()).unwrap_or("");
                            let ziel = tl.get("ziel").and_then(|i| i.as_str()).unwrap_or("");
                            if let Some(z) = schritt_ziel(schritt_art) {
                                let ids = self.ids(z, art);
                                if !ziel.is_empty() && !ids.iter().any(|i| i == ziel) {
                                    p.push((true, format!("{wo2}: Ziel '{ziel}' gibt es nicht.")));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

impl Inhalte {
    #[allow(clippy::too_many_arguments)]
    fn verweis(
        &self,
        art: &Art,
        ziel: Ziel,
        id: &str,
        feld: &Feld,
        wo: &str,
        kontext: &mut HashMap<&'static str, Vec<String>>,
        p: &mut Vec<(bool, String)>,
    ) {
        if id.is_empty() {
            return;
        }
        let ids = kontext.entry(ziel_name(ziel, art)).or_insert_with(|| self.ids(ziel, art));
        if !ids.iter().any(|i| i == id) {
            p.push((true, format!("{wo}, {}: '{id}' gibt es nicht.", feld.titel)));
        }
    }
}

fn ziel_name(z: Ziel, art: &Art) -> &'static str {
    match z {
        Ziel::Gegenstand => "gegenstand",
        Ziel::Wesen => "wesen",
        Ziel::Npc => "npc",
        Ziel::Fertigkeit => "fertigkeit",
        Ziel::Zauber => "zauber",
        Ziel::Lehrbar => "lehrbar",
        Ziel::Raum => "raum",
        Ziel::Rasse => "rasse",
        Ziel::Gilde => "gilde",
        Ziel::Beruf => "beruf",
        Ziel::Quest => "quest",
        Ziel::Beschreibung => "beschreibung",
        Ziel::Gleich => match art.gruppe() {
            "monster" => "gleich:monster",
            "gegenstand" => "gleich:gegenstand",
            "zauber" => "gleich:zauber",
            "beschreibung" => "gleich:beschreibung",
            _ => "gleich",
        },
    }
}

fn schritt_ziel(art: &str) -> Option<Ziel> {
    match art {
        "toeten" => Some(Ziel::Wesen),
        "bringen" => Some(Ziel::Gegenstand),
        "besuchen" => Some(Ziel::Raum),
        "sprechen" => Some(Ziel::Npc),
        _ => None,
    }
}

/// Kurzer Text eines Werts für die Anzeige "(geerbt) ..."
fn kurz(i: &Item) -> String {
    match i {
        Item::Value(Value::String(s)) => s.value().to_string(),
        Item::Value(v) => v.to_string().trim().to_string(),
        Item::Table(_) | Item::ArrayOfTables(_) => "(Tabelle)".into(),
        Item::None => String::new(),
    }
}

/// Untertabellen eines Felds: [[a.b]] oder [ { ... }, { ... } ]
fn untertabellen(item: &Item) -> Vec<&dyn TableLike> {
    match item {
        Item::ArrayOfTables(a) => a.iter().map(|t| t as &dyn TableLike).collect(),
        Item::Value(Value::Array(a)) => a.iter().filter_map(|v| v.as_inline_table()).map(|t| t as &dyn TableLike).collect(),
        _ => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Werte lesen und schreiben (für Tabellen und Inline-Tabellen gleichermaßen)
// ---------------------------------------------------------------------------

fn lies_text(t: &dyn TableLike, k: &str) -> String {
    t.get(k).and_then(|i| i.as_str()).unwrap_or("").to_string()
}

/// Setzt einen Wert; Kommentare und Ausrichtung des alten Werts bleiben.
fn setzen(t: &mut dyn TableLike, k: &str, mut neu: Value) {
    if let Some(Item::Value(alt)) = t.get_mut(k) {
        let decor = alt.decor().clone();
        *neu.decor_mut() = decor;
        *alt = neu;
        return;
    }
    t.insert(k, Item::Value(neu));
}

fn text_setzen(t: &mut dyn TableLike, k: &str, v: &str) {
    if v.is_empty() {
        t.remove(k);
    } else if t.get(k).and_then(|i| i.as_str()) != Some(v) {
        setzen(t, k, text_wert(v));
    }
}

fn liste_lesen(t: &dyn TableLike, k: &str) -> Vec<String> {
    t.get(k).and_then(|i| i.as_array()).map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default()
}

fn liste_setzen(t: &mut dyn TableLike, k: &str, v: &[String]) {
    if v.is_empty() {
        t.remove(k);
        return;
    }
    let mut a = Array::new();
    for x in v {
        a.push(x.as_str());
    }
    setzen(t, k, Value::Array(a));
}

// ---------------------------------------------------------------------------
// Formular-Bausteine
// ---------------------------------------------------------------------------

/// Was die Formulare über die anderen Einträge wissen müssen.
struct Kontext<'a> {
    inhalte: &'a Inhalte,
    art: &'static Art,
    ids: HashMap<&'static str, Vec<String>>,
    geerbt: BTreeMap<String, String>,
}

impl Kontext<'_> {
    fn ids(&mut self, z: Ziel) -> Vec<String> {
        let name = ziel_name(z, self.art);
        let (inhalte, art) = (self.inhalte, self.art);
        self.ids.entry(name).or_insert_with(|| inhalte.ids(z, art)).clone()
    }
}

/// "schritte" aus "quest/schritte/0", "quest" aus "quest"
fn eltern(pfad: &str) -> &str {
    let teile: Vec<&str> = pfad.split('/').collect();
    match teile.as_slice() {
        [.., e, n] if n.parse::<usize>().is_ok() => e,
        [.., letzter] => letzter,
        [] => "",
    }
}

fn beschriftung(ui: &mut egui::Ui, feld: &Feld, pflicht: bool) {
    let titel = if pflicht { format!("{} *", feld.titel) } else { feld.titel.to_string() };
    let l = ui.add_sized([150.0, 18.0], egui::Label::new(titel).truncate());
    if !feld.hilfe.is_empty() {
        l.on_hover_text(feld.hilfe);
    }
}

/// Komma-getrennte Wörter bearbeiten (der Puffer lebt im egui-Speicher).
fn wortliste(ui: &mut egui::Ui, id: egui::Id, liste: &[String], hinweis: &str) -> Option<Vec<String>> {
    let verbunden = liste.join(", ");
    let zerlegen = |t: &str| -> Vec<String> { t.split(',').map(|w| w.trim().to_string()).filter(|w| !w.is_empty()).collect() };
    let mut puffer: String = ui.data_mut(|d| d.get_temp::<String>(id)).unwrap_or_else(|| verbunden.clone());
    if zerlegen(&puffer) != liste {
        puffer = verbunden;
    }
    let a = crate::kontext::feld(ui, egui::TextEdit::singleline(&mut puffer).hint_text(hinweis).desired_width(f32::INFINITY));
    ui.data_mut(|d| d.insert_temp(id, puffer.clone()));
    let neu = zerlegen(&puffer);
    (a.changed() && neu != liste).then_some(neu)
}

/// Eine Zahl als Textfeld: leer = nicht gesetzt.
fn zahl_feld(ui: &mut egui::Ui, id: egui::Id, item: Option<&Item>, komma: bool, hinweis: &str) -> Option<Option<Value>> {
    let aktuell = match item {
        Some(Item::Value(Value::Integer(i))) => i.value().to_string(),
        Some(Item::Value(Value::Float(f))) => f.value().to_string(),
        _ => String::new(),
    };
    let mut puffer: String = ui.data_mut(|d| d.get_temp::<String>(id)).unwrap_or_else(|| aktuell.clone());
    let lesen = |t: &str| -> Option<Option<Value>> {
        let t = t.trim().replace(',', ".");
        if t.is_empty() {
            return Some(None);
        }
        if komma {
            t.parse::<f64>().ok().map(|f| Some(Value::from(f)))
        } else {
            t.parse::<i64>().ok().map(|i| Some(Value::from(i)))
        }
    };
    // Von außen geändert (z.B. Rückgängig/Neu laden)? Dann den Puffer anpassen.
    let passt = match (lesen(&puffer), &aktuell) {
        (Some(Some(v)), a) => v.to_string().trim() == a || puffer.trim() == a,
        (Some(None), a) => a.is_empty(),
        (None, _) => true,
    };
    if !passt {
        puffer = aktuell.clone();
    }
    let ok = lesen(&puffer).is_some();
    let mut feld = egui::TextEdit::singleline(&mut puffer).hint_text(hinweis).desired_width(120.0);
    if !ok {
        feld = feld.text_color(ROT);
    }
    let a = crate::kontext::feld(ui, feld);
    ui.data_mut(|d| d.insert_temp(id, puffer.clone()));
    if !ok {
        ui.label(RichText::new("keine Zahl").color(ROT).small());
    }
    if a.changed() {
        return lesen(&puffer);
    }
    None
}

fn auswahl_box(ui: &mut egui::Ui, id: egui::Id, aktuell: &str, moeglich: &[String], leer: &str) -> Option<String> {
    let mut neu = None;
    let text = if aktuell.is_empty() { leer.to_string() } else { aktuell.to_string() };
    let unbekannt = !aktuell.is_empty() && !moeglich.iter().any(|m| m == aktuell);
    let text = if unbekannt { RichText::new(format!("{text} (fehlt!)")).color(ROT) } else { RichText::new(text) };
    egui::ComboBox::from_id_salt(id).selected_text(text).width(220.0).height(320.0).show_ui(ui, |ui| {
        if ui.selectable_label(aktuell.is_empty(), leer).clicked() {
            neu = Some(String::new());
        }
        for m in moeglich {
            if ui.selectable_label(m == aktuell, m).clicked() {
                neu = Some(m.clone());
            }
        }
    });
    neu
}

/// Ein Feld im Formular. Gibt true zurück, wenn etwas geändert wurde.
fn feld_zeigen(ui: &mut egui::Ui, ctx: &mut Kontext, t: &mut dyn TableLike, feld: &Feld, inline: bool, pfad: &str, oben: bool) -> bool {
    let id = ui.make_persistent_id((pfad, feld.name));
    let pflicht = crate::schema::pflicht(eltern(pfad), feld.name);
    let geerbt = if oben { ctx.geerbt.get(feld.name).cloned() } else { None };
    let hinweis = geerbt.as_ref().map(|g| format!("(geerbt) {g}")).unwrap_or_default();
    let mut geaendert = false;
    match feld.typ {
        Typ::Text | Typ::Mehrzeilig => {
            ui.horizontal(|ui| {
                beschriftung(ui, feld, pflicht);
                let mut v = lies_text(t, feld.name);
                let edit = if matches!(feld.typ, Typ::Mehrzeilig) {
                    egui::TextEdit::multiline(&mut v).desired_rows(3)
                } else {
                    egui::TextEdit::singleline(&mut v)
                };
                if crate::kontext::feld(ui, edit.hint_text(&hinweis).desired_width(f32::INFINITY)).changed() {
                    text_setzen(t, feld.name, &v);
                    geaendert = true;
                }
            });
        }
        Typ::TextOderListe => {
            ui.horizontal(|ui| {
                beschriftung(ui, feld, pflicht);
                let mut v = match t.get(feld.name) {
                    Some(i) if i.is_str() => i.as_str().unwrap_or_default().to_string(),
                    Some(i) => i.as_array().map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join("\n")).unwrap_or_default(),
                    None => String::new(),
                };
                let hinweis = if hinweis.is_empty() { "Mehrere Zeilen = zufällige Auswahl".to_string() } else { hinweis.clone() };
                if crate::kontext::feld(ui, egui::TextEdit::multiline(&mut v).desired_rows(2).hint_text(hinweis).desired_width(f32::INFINITY)).changed() {
                    let zeilen: Vec<String> = v.lines().map(|z| z.trim().to_string()).filter(|z| !z.is_empty()).collect();
                    match zeilen.len() {
                        0 => {
                            t.remove(feld.name);
                        }
                        1 => text_setzen(t, feld.name, &zeilen[0]),
                        _ => liste_setzen(t, feld.name, &zeilen),
                    }
                    geaendert = true;
                }
            });
        }
        Typ::Zahl | Typ::Komma => {
            ui.horizontal(|ui| {
                beschriftung(ui, feld, pflicht);
                if let Some(neu) = zahl_feld(ui, id, t.get(feld.name), matches!(feld.typ, Typ::Komma), &hinweis) {
                    match neu {
                        Some(v) => setzen(t, feld.name, v),
                        None => {
                            t.remove(feld.name);
                        }
                    }
                    geaendert = true;
                }
            });
        }
        Typ::Schalter => {
            ui.horizontal(|ui| {
                beschriftung(ui, feld, pflicht);
                let aktuell = t.get(feld.name).and_then(|i| i.as_bool());
                let standard = match &geerbt {
                    Some(g) => format!("(geerbt: {g})"),
                    None => "(Standard)".into(),
                };
                let text = match aktuell {
                    Some(true) => "ja".to_string(),
                    Some(false) => "nein".to_string(),
                    None => standard.clone(),
                };
                egui::ComboBox::from_id_salt(id).selected_text(text).width(140.0).show_ui(ui, |ui| {
                    for (wert, name) in [(None, standard.as_str()), (Some(true), "ja"), (Some(false), "nein")] {
                        if ui.selectable_label(aktuell == wert, name).clicked() && aktuell != wert {
                            match wert {
                                Some(b) => setzen(t, feld.name, Value::from(b)),
                                None => {
                                    t.remove(feld.name);
                                }
                            }
                            geaendert = true;
                        }
                    }
                });
            });
        }
        Typ::Liste => {
            ui.horizontal(|ui| {
                beschriftung(ui, feld, pflicht);
                let v = liste_lesen(t, feld.name);
                let hinweis = if hinweis.is_empty() { "a, b, c".to_string() } else { hinweis.clone() };
                if let Some(neu) = wortliste(ui, id, &v, &hinweis) {
                    liste_setzen(t, feld.name, &neu);
                    geaendert = true;
                }
            });
        }
        Typ::Auswahl(moeglich) => {
            ui.horizontal(|ui| {
                beschriftung(ui, feld, pflicht);
                let liste: Vec<String> = moeglich.iter().map(|s| s.to_string()).collect();
                let leer = geerbt.as_ref().map(|g| format!("(geerbt: {g})")).unwrap_or_else(|| "(Standard)".into());
                if let Some(neu) = auswahl_box(ui, id, &lies_text(t, feld.name), &liste, &leer) {
                    text_setzen(t, feld.name, &neu);
                    geaendert = true;
                }
            });
        }
        Typ::Verweis(z) => {
            let liste = ctx.ids(z);
            ui.horizontal(|ui| {
                beschriftung(ui, feld, pflicht);
                let leer = geerbt.as_ref().map(|g| format!("(geerbt: {g})")).unwrap_or_else(|| "(keiner)".into());
                if let Some(neu) = auswahl_box(ui, id, &lies_text(t, feld.name), &liste, &leer) {
                    text_setzen(t, feld.name, &neu);
                    geaendert = true;
                }
            });
        }
        Typ::SchrittZiel => {
            let z = schritt_ziel(&lies_text(t, "art"));
            let liste = z.map(|z| ctx.ids(z)).unwrap_or_default();
            ui.horizontal(|ui| {
                beschriftung(ui, feld, pflicht);
                if z.is_none() {
                    ui.label(RichText::new("Erst die Art wählen.").color(GRAU));
                } else if let Some(neu) = auswahl_box(ui, id, &lies_text(t, feld.name), &liste, "(bitte wählen)") {
                    text_setzen(t, feld.name, &neu);
                    geaendert = true;
                }
            });
        }
        Typ::VerweisListe(z) => {
            let liste = ctx.ids(z);
            ui.horizontal_wrapped(|ui| {
                beschriftung(ui, feld, pflicht);
                let mut v = liste_lesen(t, feld.name);
                if v.is_empty() && !hinweis.is_empty() {
                    ui.label(RichText::new(&hinweis).color(GRAU));
                }
                let mut weg = None;
                for (n, e) in v.iter().enumerate() {
                    let farbe = if liste.contains(e) { Color32::LIGHT_GRAY } else { ROT };
                    if ui.small_button(RichText::new(format!("{e} ✖")).color(farbe)).on_hover_text("Entfernen").clicked() {
                        weg = Some(n);
                    }
                }
                if let Some(n) = weg {
                    v.remove(n);
                    liste_setzen(t, feld.name, &v);
                    geaendert = true;
                }
                let frei: Vec<String> = liste.iter().filter(|i| !v.contains(i)).cloned().collect();
                if let Some(neu) = auswahl_box(ui, id, "", &frei, "+ hinzufügen") {
                    if !neu.is_empty() {
                        v.push(neu);
                        liste_setzen(t, feld.name, &v);
                        geaendert = true;
                    }
                }
            });
        }
        Typ::Werte(vorschlaege) => {
            let vorschlaege: Vec<String> = vorschlaege.iter().map(|s| s.to_string()).collect();
            geaendert |= werte_zeigen(ui, ctx, t, feld, inline, id, vorschlaege, false, &hinweis);
        }
        Typ::VerweisWerte(z) => {
            let liste = ctx.ids(z);
            geaendert |= werte_zeigen(ui, ctx, t, feld, inline, id, liste, true, &hinweis);
        }
        Typ::Geld => {
            ui.horizontal(|ui| {
                beschriftung(ui, feld, pflicht);
                let (von, bis) = match t.get(feld.name) {
                    Some(i) if i.as_integer().is_some() => (i.as_integer(), i.as_integer()),
                    Some(i) => {
                        let a: Vec<i64> = i.as_array().map(|a| a.iter().filter_map(|v| v.as_integer()).collect()).unwrap_or_default();
                        (a.first().copied(), a.get(1).copied().or(a.first().copied()))
                    }
                    None => (None, None),
                };
                let mut v = von.map(|x| x.to_string()).unwrap_or_default();
                let mut b = bis.map(|x| x.to_string()).unwrap_or_default();
                ui.label("von");
                let a1 = crate::kontext::feld(ui, egui::TextEdit::singleline(&mut v).desired_width(60.0).hint_text(&hinweis));
                ui.label("bis");
                let a2 = crate::kontext::feld(ui, egui::TextEdit::singleline(&mut b).desired_width(60.0));
                if a1.changed() || a2.changed() {
                    match (v.trim().parse::<i64>().ok(), b.trim().parse::<i64>().ok()) {
                        (None, None) => {
                            t.remove(feld.name);
                        }
                        (Some(x), None) | (None, Some(x)) => setzen(t, feld.name, Value::from(x)),
                        (Some(x), Some(y)) if x == y => setzen(t, feld.name, Value::from(x)),
                        (Some(x), Some(y)) => {
                            let mut a = Array::new();
                            a.push(x);
                            a.push(y);
                            setzen(t, feld.name, Value::Array(a));
                        }
                    }
                    geaendert = true;
                }
            });
        }
        Typ::Tabellen { felder, inline: inline_liste } => {
            geaendert |= tabellen_zeigen(ui, ctx, t, feld, felder, inline || inline_liste, pfad);
        }
        Typ::Unter(felder) => {
            geaendert |= unter_zeigen(ui, ctx, t, feld, felder, inline, pfad);
        }
        Typ::Details => {
            geaendert |= details_zeigen(ui, ctx, t, feld, inline, pfad);
        }
    }
    geaendert
}

/// Name -> Zahl (eigenschaften, werte, fertigkeiten ...)
#[allow(clippy::too_many_arguments)]
fn werte_zeigen(
    ui: &mut egui::Ui,
    _ctx: &mut Kontext,
    t: &mut dyn TableLike,
    feld: &Feld,
    inline: bool,
    id: egui::Id,
    vorschlaege: Vec<String>,
    nur_vorschlaege: bool,
    hinweis: &str,
) -> bool {
    let mut geaendert = false;
    let paare: Vec<(String, i64)> = t
        .get(feld.name)
        .and_then(|i| i.as_table_like())
        .map(|tl| tl.iter().map(|(k, v)| (k.to_string(), v.as_integer().unwrap_or(0))).collect())
        .unwrap_or_default();
    ui.horizontal_wrapped(|ui| {
        beschriftung(ui, feld, false);
        if paare.is_empty() && !hinweis.is_empty() {
            ui.label(RichText::new(hinweis).color(GRAU));
        }
        let mut neu = paare.clone();
        let mut weg = None;
        for (n, (k, v)) in neu.iter_mut().enumerate() {
            let farbe = if !nur_vorschlaege || vorschlaege.contains(k) { Color32::LIGHT_GRAY } else { ROT };
            ui.label(RichText::new(k.as_str()).color(farbe));
            let mut zahl = *v;
            if ui.add(egui::DragValue::new(&mut zahl).speed(0.2)).changed() {
                *v = zahl;
                geaendert = true;
            }
            if ui.small_button("✖").on_hover_text("Entfernen").clicked() {
                weg = Some(n);
            }
            ui.add_space(6.0);
        }
        if let Some(n) = weg {
            neu.remove(n);
            geaendert = true;
        }
        let frei: Vec<String> = vorschlaege.iter().filter(|s| !neu.iter().any(|(k, _)| k == *s)).cloned().collect();
        if let Some(name) = auswahl_box(ui, id.with("neu"), "", &frei, "+ hinzufügen") {
            if !name.is_empty() {
                neu.push((name, 1));
                geaendert = true;
            }
        }
        if !nur_vorschlaege {
            // Eigene Namen (z.B. neue Eigenschaften) eintippen
            let pid = id.with("eigen");
            let mut eigen: String = ui.data_mut(|d| d.get_temp::<String>(pid)).unwrap_or_default();
            let a = crate::kontext::feld(ui, egui::TextEdit::singleline(&mut eigen).hint_text("eigener Name").desired_width(100.0));
            if a.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !eigen.trim().is_empty() {
                neu.push((eigen.trim().to_string(), 1));
                eigen.clear();
                geaendert = true;
            }
            ui.data_mut(|d| d.insert_temp(pid, eigen));
        }
        if geaendert {
            if neu.is_empty() {
                t.remove(feld.name);
            } else if let Some(tl) = t.get_mut(feld.name).and_then(|i| i.as_table_like_mut()) {
                // Vorhandene Tabelle (auch [start.werte]) an Ort und Stelle anpassen
                let alte: Vec<String> = tl.iter().map(|(k, _)| k.to_string()).collect();
                for k in alte {
                    if !neu.iter().any(|(n, _)| *n == k) {
                        tl.remove(&k);
                    }
                }
                for (k, v) in &neu {
                    match tl.get_mut(k) {
                        Some(Item::Value(alt)) => {
                            let d = alt.decor().clone();
                            *alt = Value::from(*v);
                            *alt.decor_mut() = d;
                        }
                        _ => {
                            tl.insert(k, Item::Value(Value::from(*v)));
                        }
                    }
                }
            } else {
                let mut it = InlineTable::new();
                for (k, v) in &neu {
                    it.insert(k, Value::from(*v));
                }
                let _ = inline;
                setzen(t, feld.name, Value::InlineTable(it));
            }
        }
    });
    geaendert
}

/// Liste von Untereinträgen (beute, wirkungen, lehrt, thema, schritte ...)
fn tabellen_zeigen(ui: &mut egui::Ui, ctx: &mut Kontext, t: &mut dyn TableLike, feld: &Feld, felder: &[Feld], inline: bool, pfad: &str) -> bool {
    let mut geaendert = false;
    let anzahl = t.get(feld.name).map(|i| untertabellen(i).len()).unwrap_or(0);
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{} ({anzahl})", feld.titel)).strong()).on_hover_text(feld.hilfe);
        if ui.small_button("+ hinzufügen").clicked() {
            match t.get_mut(feld.name) {
                Some(Item::ArrayOfTables(a)) => {
                    a.push(Table::new());
                }
                Some(Item::Value(Value::Array(a))) => {
                    a.push(InlineTable::new());
                }
                _ if inline => {
                    let mut a = Array::new();
                    a.push(InlineTable::new());
                    t.insert(feld.name, Item::Value(Value::Array(a)));
                }
                _ => {
                    let mut a = ArrayOfTables::new();
                    a.push(Table::new());
                    t.insert(feld.name, Item::ArrayOfTables(a));
                }
            }
            geaendert = true;
        }
    });
    if !feld.hilfe.is_empty() && anzahl == 0 {
        ui.label(RichText::new(feld.hilfe).color(GRAU).small());
    }
    let mut weg = None;
    let mut hoch = None;
    for n in 0..anzahl {
        let unterpfad = format!("{pfad}/{}/{n}", feld.name);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Nr. {}", n + 1)).color(GRAU));
                if ui.small_button("✖ entfernen").clicked() {
                    weg = Some(n);
                }
                if n > 0 && ui.small_button("⬆").on_hover_text("Nach oben").clicked() {
                    hoch = Some(n);
                }
            });
            let unter: Option<&mut dyn TableLike> = match t.get_mut(feld.name) {
                Some(Item::ArrayOfTables(a)) => a.get_mut(n).map(|x| x as &mut dyn TableLike),
                Some(Item::Value(Value::Array(a))) => a.get_mut(n).and_then(|v| v.as_inline_table_mut()).map(|x| x as &mut dyn TableLike),
                _ => None,
            };
            if let Some(unter) = unter {
                for f in felder {
                    geaendert |= feld_zeigen(ui, ctx, unter, f, true, &unterpfad, false);
                }
            }
        });
    }
    if let Some(n) = weg {
        match t.get_mut(feld.name) {
            Some(Item::ArrayOfTables(a)) => {
                a.remove(n);
                if a.is_empty() {
                    t.remove(feld.name);
                }
            }
            Some(Item::Value(Value::Array(a))) => {
                a.remove(n);
                if a.is_empty() {
                    t.remove(feld.name);
                }
            }
            _ => {}
        }
        geaendert = true;
    }
    if let Some(n) = hoch {
        match t.get_mut(feld.name) {
            Some(Item::ArrayOfTables(a)) => {
                let mut alle: Vec<Table> = a.iter().cloned().collect();
                alle.swap(n - 1, n);
                let mut neu = ArrayOfTables::new();
                for x in alle {
                    neu.push(x);
                }
                *a = neu;
            }
            Some(Item::Value(Value::Array(a))) => {
                let mut alle: Vec<Value> = a.iter().cloned().collect();
                alle.swap(n - 1, n);
                let mut neu = Array::new();
                for x in alle {
                    neu.push_formatted(x);
                }
                *a = neu;
            }
            _ => {}
        }
        geaendert = true;
    }
    geaendert
}

/// Eine Untertabelle wie [npc.haendler] oder [quest.belohnung]
fn unter_zeigen(ui: &mut egui::Ui, ctx: &mut Kontext, t: &mut dyn TableLike, feld: &Feld, felder: &[Feld], inline: bool, pfad: &str) -> bool {
    let mut geaendert = false;
    ui.add_space(4.0);
    let da = t.get(feld.name).is_some_and(|i| i.is_table_like());
    let geerbt = ctx.geerbt.contains_key(feld.name);
    ui.horizontal(|ui| {
        ui.label(RichText::new(feld.titel).strong()).on_hover_text(feld.hilfe);
        if !da {
            if geerbt {
                ui.label(RichText::new("(geerbt)").color(GRAU));
            }
            if ui.small_button("+ anlegen").clicked() {
                if inline {
                    t.insert(feld.name, Item::Value(Value::InlineTable(InlineTable::new())));
                } else {
                    let mut neu = Table::new();
                    neu.set_implicit(false);
                    neu.decor_mut().set_prefix("\n");
                    t.insert(feld.name, Item::Table(neu));
                }
                geaendert = true;
            }
        } else if ui.small_button("✖ entfernen").clicked() {
            t.remove(feld.name);
            geaendert = true;
        }
    });
    if !da && !geaendert {
        if !feld.hilfe.is_empty() {
            ui.label(RichText::new(feld.hilfe).color(GRAU).small());
        }
        return geaendert;
    }
    let ist_inline = matches!(t.get(feld.name), Some(Item::Value(_)));
    if let Some(unter) = t.get_mut(feld.name).and_then(|i| i.as_table_like_mut()) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            for f in felder {
                geaendert |= feld_zeigen(ui, ctx, unter, f, ist_inline, &format!("{pfad}/{}", feld.name), false);
            }
        });
    }
    geaendert
}

/// Details: Name -> Text, oder ausführlich mit Sinnen und Unter-Details.
fn details_zeigen(ui: &mut egui::Ui, ctx: &mut Kontext, t: &mut dyn TableLike, feld: &Feld, inline: bool, pfad: &str) -> bool {
    let mut geaendert = false;
    let namen: Vec<String> = t.get(feld.name).and_then(|i| i.as_table_like()).map(|tl| tl.iter().map(|(k, _)| k.to_string()).collect()).unwrap_or_default();
    let mut pfad_id = ui.make_persistent_id((pfad, feld.name, "neu"));
    egui::CollapsingHeader::new(RichText::new(format!("{} ({})", feld.titel, namen.len())).strong())
        .id_salt((pfad, feld.name))
        .show(ui, |ui| {
            ui.label(RichText::new("Was man mit 'schau <name>' genauer ansehen kann.").color(GRAU).small());
            let mut weg = None;
            for name in &namen {
                let unterpfad = format!("{pfad}/{}/{name}", feld.name);
                let Some(tl) = t.get_mut(feld.name).and_then(|i| i.as_table_like_mut()) else { break };
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(name).strong());
                        if ui.small_button("✖").on_hover_text("Detail entfernen").clicked() {
                            weg = Some(name.clone());
                        }
                        let ist_text = tl.get(name).is_some_and(|i| i.is_str());
                        if ist_text && ui.small_button("ausführlich …").on_hover_text("Mit Wörtern, Sinnen und Unter-Details").clicked() {
                            let text = tl.get(name).and_then(|i| i.as_str()).unwrap_or_default().to_string();
                            let mut it = InlineTable::new();
                            it.insert("text", text_wert(&text));
                            if inline {
                                tl.insert(name, Item::Value(Value::InlineTable(it)));
                            } else {
                                let mut neu = it.into_table();
                                neu.set_implicit(false);
                                tl.insert(name, Item::Table(neu));
                            }
                            geaendert = true;
                        }
                    });
                    if tl.get(name).is_some_and(|i| i.is_str()) {
                        let mut v = tl.get(name).and_then(|i| i.as_str()).unwrap_or_default().to_string();
                        if crate::kontext::feld(ui, egui::TextEdit::multiline(&mut v).desired_rows(2).desired_width(f32::INFINITY)).changed() {
                            setzen(tl, name, text_wert(&v));
                            geaendert = true;
                        }
                    } else {
                        let ist_inline = matches!(tl.get(name), Some(Item::Value(_)));
                        if let Some(unter) = tl.get_mut(name).and_then(|i| i.as_table_like_mut()) {
                            for f in DETAIL_FELDER {
                                geaendert |= feld_zeigen(ui, ctx, unter, f, ist_inline, &unterpfad, false);
                            }
                        }
                    }
                });
            }
            if let Some(name) = weg {
                if let Some(tl) = t.get_mut(feld.name).and_then(|i| i.as_table_like_mut()) {
                    tl.remove(&name);
                    if tl.is_empty() {
                        t.remove(feld.name);
                    }
                }
                geaendert = true;
            }
            ui.horizontal(|ui| {
                pfad_id = pfad_id.with("name");
                let mut neu: String = ui.data_mut(|d| d.get_temp::<String>(pfad_id)).unwrap_or_default();
                crate::kontext::feld(ui, egui::TextEdit::singleline(&mut neu).hint_text("neues Detail, z.B. brunnen").desired_width(180.0));
                let name = neu.trim().to_lowercase();
                if ui.add_enabled(!name.is_empty() && !namen.contains(&name), egui::Button::new("+ Detail")).clicked() {
                    if t.get(feld.name).is_none() {
                        if inline {
                            t.insert(feld.name, Item::Value(Value::InlineTable(InlineTable::new())));
                        } else {
                            let mut neu_t = Table::new();
                            neu_t.set_implicit(false);
                            t.insert(feld.name, Item::Table(neu_t));
                        }
                    }
                    if let Some(tl) = t.get_mut(feld.name).and_then(|i| i.as_table_like_mut()) {
                        tl.insert(&name, Item::Value(Value::from("")));
                    }
                    neu.clear();
                    geaendert = true;
                }
                ui.data_mut(|d| d.insert_temp(pfad_id, neu));
            });
        });
    geaendert
}

// ---------------------------------------------------------------------------
// Der Bereich: Arten, Liste, Formular
// ---------------------------------------------------------------------------

impl Inhalte {
    fn art_def(&self) -> &'static Art {
        &ARTEN[self.art.min(ARTEN.len() - 1)]
    }

    /// Name und ID eines Eintrags für die Liste
    fn titel(&self, art: &Art, o: Ort) -> (String, String) {
        let Some(t) = self.tabelle(art, o) else { return (String::new(), String::new()) };
        let id = lies_text(t, "id");
        let name = lies_text(t, "name");
        let name = if name.is_empty() { lies_text(t, "woerter") } else { name };
        (id, name)
    }

    /// Links: welche Art, und die Einträge.
    pub fn liste(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        egui::ComboBox::from_id_salt("inhalte_art")
            .selected_text(RichText::new(format!("{} {}", self.art_def().symbol, self.art_def().titel)).strong())
            .width(240.0)
            .height(500.0)
            .show_ui(ui, |ui| {
                for (i, a) in ARTEN.iter().enumerate() {
                    if ui.selectable_label(self.art == i, format!("{} {}", a.symbol, a.titel)).clicked() && self.art != i {
                        self.art = i;
                        self.auswahl = None;
                        self.suche.clear();
                    }
                }
            });
        let art = self.art_def();
        if !art.hilfe.is_empty() {
            ui.label(RichText::new(art.hilfe).color(GRAU).small());
        }
        ui.separator();
        if !art.hat_id() {
            // Feste Einstellungen: es gibt nur den einen Eintrag
            self.auswahl = self.orte(art).into_iter().next();
            if self.auswahl.is_none() {
                ui.label(RichText::new("welt.toml fehlt oder hat Fehler.").color(ROT));
            }
            return;
        }
        ui.horizontal(|ui| {
            if ui.button("+ Neu").clicked() {
                self.neu = Some(NeuDialog { id: String::new(), datei: self.standard_datei(art), kopie_von: None });
            }
            if let Some(o) = self.auswahl {
                if ui.button("Kopieren").clicked() {
                    let (id, _) = self.titel(art, o);
                    let datei = self.dateien[o.datei].rel.clone();
                    let datei = if art.ablage == Ablage::EineDatei { format!("{}/{id}_kopie.toml", art.ordner) } else { datei };
                    self.neu = Some(NeuDialog { id: format!("{id}_kopie"), datei, kopie_von: Some(o) });
                }
                if ui.button("🗑 Löschen").clicked() {
                    self.loeschen_fragen = Some(o);
                }
            }
        });
        crate::kontext::feld(ui, egui::TextEdit::singleline(&mut self.suche).hint_text("Suchen …").desired_width(f32::INFINITY));
        ui.separator();
        let such = self.suche.to_lowercase();
        let orte = self.orte(art);
        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut letzte_datei = None;
            for o in orte {
                let (id, name) = self.titel(art, o);
                if !such.is_empty() && !id.to_lowercase().contains(&such) && !name.to_lowercase().contains(&such) {
                    continue;
                }
                if art.ablage == Ablage::Liste && letzte_datei != Some(o.datei) {
                    ui.label(RichText::new(&self.dateien[o.datei].rel).color(GRAU).small());
                    letzte_datei = Some(o.datei);
                }
                let vorlage = self.tabelle(art, o).and_then(|t| t.get("vorlage")).and_then(|i| i.as_bool()) == Some(true);
                let text = if name.is_empty() { id.clone() } else { format!("{name}  ({id})") };
                let text = if vorlage { RichText::new(format!("{text} [Vorlage]")).italics() } else { RichText::new(text) };
                let eintrag = ui.selectable_label(self.auswahl == Some(o), text);
                if eintrag.clicked() {
                    self.auswahl = Some(o);
                }
                eintrag.context_menu(|ui| {
                    if ui.button("Kopie anlegen").clicked() {
                        self.auswahl = Some(o);
                        let datei = self.dateien[o.datei].rel.clone();
                        let datei = if art.ablage == Ablage::EineDatei { format!("{}/{id}_kopie.toml", art.ordner) } else { datei };
                        self.neu = Some(NeuDialog { id: format!("{id}_kopie"), datei, kopie_von: Some(o) });
                        ui.close();
                    }
                    if ui.button("📋 ID kopieren").on_hover_text("Zum Einfügen in andere Felder, z.B. bei Räumen").clicked() {
                        ui.ctx().copy_text(id.clone());
                        ui.close();
                    }
                    if !name.is_empty() && ui.button("Name kopieren").clicked() {
                        ui.ctx().copy_text(name.clone());
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("🗑 Löschen …").clicked() {
                        self.auswahl = Some(o);
                        self.loeschen_fragen = Some(o);
                        ui.close();
                    }
                });
            }
        });
    }

    fn standard_datei(&self, art: &Art) -> String {
        match art.ablage {
            Ablage::EineDatei => format!("{}/neu.toml", art.ordner),
            _ => self
                .auswahl
                .and_then(|o| self.dateien.get(o.datei))
                .filter(|d| d.doc.get(art.schluessel).is_some())
                .map(|d| d.rel.clone())
                .or_else(|| self.orte(art).last().map(|o| self.dateien[o.datei].rel.clone()))
                .unwrap_or_else(|| format!("{}/{}.toml", art.ordner, art.ordner)),
        }
    }

    /// Dateien, in denen Einträge dieser Art stehen (für den Neu-Dialog).
    fn dateien_fuer(&self, art: &Art) -> Vec<String> {
        let mut v: Vec<String> = self.orte(art).into_iter().map(|o| self.dateien[o.datei].rel.clone()).collect();
        v.dedup();
        v
    }

    /// Mitte: das Formular des ausgewählten Eintrags.
    pub fn formular(&mut self, ui: &mut egui::Ui) {
        let art = self.art_def();
        self.dialoge(ui);
        let Some(o) = self.auswahl.filter(|o| self.tabelle(art, *o).is_some() || !art.hat_id()) else {
            ui.add_space(12.0);
            ui.label(RichText::new(format!("{} {}", art.symbol, art.titel)).heading());
            ui.add_space(6.0);
            ui.label("Wähle links einen Eintrag aus oder lege mit „+ Neu“ einen an.");
            ui.add_space(6.0);
            ui.label(RichText::new("Leere Felder werden nicht gespeichert – dann gilt der Standardwert oder die Vorlage (erbt). Graue Werte kommen aus der Vorlage.").color(GRAU));
            return;
        };
        let geerbt = self.tabelle(art, o).map(|t| self.geerbt(art, t)).unwrap_or_default();
        let datei = self.dateien.get(o.datei).map(|d| d.rel.clone()).unwrap_or_default();
        // Die Formulare brauchen Lesezugriff auf alle Einträge (für Auswahllisten)
        // und Schreibzugriff auf den einen – deshalb wird der Eintrag kurz herausgenommen.
        let Some(mut tabelle) = self.tabelle_mut(art, o).map(std::mem::take) else { return };
        // Beim Herausnehmen bleiben Kommentare und Position im Table-Objekt selbst
        let mut geaendert = false;
        {
            let mut ctx = Kontext { inhalte: self, art, ids: HashMap::new(), geerbt };
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (id, name) = (lies_text(&tabelle, "id"), lies_text(&tabelle, "name"));
                    let titel = if art.hat_id() { if name.is_empty() { id } else { name } } else { art.titel.to_string() };
                    ui.heading(format!("{} {titel}", art.symbol));
                    ui.label(RichText::new(&datei).color(GRAU).small());
                });
                ui.add_space(4.0);
                for feld in art.felder {
                    geaendert |= feld_zeigen(ui, &mut ctx, &mut tabelle, feld, false, art.schluessel, true);
                }
                // Felder, die der Editor nicht kennt, bleiben erhalten – hier nur zur Info
                let bekannt: BTreeSet<&str> = art.felder.iter().map(|f| f.name).chain(["id"]).collect();
                let fremd: Vec<String> = tabelle.iter().map(|(k, _)| k.to_string()).filter(|k| !bekannt.contains(k.as_str())).collect();
                if !fremd.is_empty() {
                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("Weitere Felder in der Datei (bleiben erhalten): {}", fremd.join(", "))).color(GRAU).small());
                }
            });
        }
        if let Some(t) = self.tabelle_mut(art, o) {
            *t = tabelle;
        }
        let _ = geaendert;
    }

    fn dialoge(&mut self, ui: &mut egui::Ui) {
        let art = self.art_def();
        // Neu / Kopieren
        if let Some(mut d) = self.neu.take() {
            let mut offen = true;
            let mut fertig = false;
            let dateien = self.dateien_fuer(art);
            egui::Window::new(if d.kopie_von.is_some() { "Kopie anlegen" } else { "Neuer Eintrag" })
                .collapsible(false)
                .resizable(false)
                .open(&mut offen)
                .show(ui.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("ID:");
                        crate::kontext::feld(ui, egui::TextEdit::singleline(&mut d.id).hint_text("z.B. eisenschwert").desired_width(200.0));
                    });
                    let gueltig = crate::welt::id_gueltig(&d.id);
                    let vergeben = self.ids_von(&[art.schluessel], true).contains(&d.id)
                        || (art.gruppe() == "monster" && self.ids_von(&["monster", "npc"], true).contains(&d.id));
                    if !d.id.is_empty() && !gueltig {
                        ui.label(RichText::new("Nur Kleinbuchstaben, Ziffern und _ (keine Umlaute).").color(ROT));
                    }
                    if vergeben {
                        ui.label(RichText::new("Diese ID gibt es schon.").color(ROT));
                    }
                    if art.ablage == Ablage::EineDatei {
                        d.datei = format!("{}/{}.toml", art.ordner, if d.id.is_empty() { "neu" } else { &d.id });
                        ui.label(RichText::new(format!("Datei: {}", d.datei)).color(GRAU));
                    } else {
                        ui.horizontal(|ui| {
                            ui.label("Datei:");
                            egui::ComboBox::from_id_salt("neu_datei").selected_text(&d.datei).width(260.0).show_ui(ui, |ui| {
                                for f in &dateien {
                                    ui.selectable_value(&mut d.datei, f.clone(), f);
                                }
                            });
                        });
                        ui.horizontal(|ui| {
                            ui.label("oder neue Datei:");
                            crate::kontext::feld(ui, egui::TextEdit::singleline(&mut d.datei).desired_width(260.0));
                        });
                        if !d.datei.ends_with(".toml") {
                            ui.label(RichText::new("Der Dateiname muss auf .toml enden.").color(ROT));
                        }
                    }
                    let erlaubt = gueltig && !vergeben && d.datei.ends_with(".toml") && !d.datei.contains("..");
                    if ui.add_enabled(erlaubt, egui::Button::new("Anlegen")).clicked() {
                        fertig = true;
                    }
                });
            if fertig {
                self.anlegen(art, &d);
            } else if offen {
                self.neu = Some(d);
            }
        }
        // Löschen
        if let Some(o) = self.loeschen_fragen {
            let (id, name) = self.titel(art, o);
            let mut offen = true;
            let mut ja = false;
            egui::Window::new("Löschen?").collapsible(false).resizable(false).open(&mut offen).show(ui.ctx(), |ui| {
                ui.label(format!("{} '{}' wirklich löschen?", art.einzahl, if name.is_empty() { &id } else { &name }));
                if art.ablage == Ablage::EineDatei {
                    ui.label(RichText::new(format!("Die Datei {} wird beim Speichern gelöscht.", self.dateien[o.datei].rel)).color(GELB));
                }
                ui.label(RichText::new("Verweise darauf (z.B. in Räumen) musst du selbst entfernen.").color(GRAU));
                if ui.button("Ja, löschen").clicked() {
                    ja = true;
                }
            });
            if ja {
                self.entfernen(art, o);
                self.loeschen_fragen = None;
            } else if !offen {
                self.loeschen_fragen = None;
            }
        }
    }

    fn anlegen(&mut self, art: &Art, d: &NeuDialog) {
        let mut t = match d.kopie_von.and_then(|o| self.tabelle(art, o)) {
            Some(alt) => alt.clone(),
            None => Table::new(),
        };
        t.decor_mut().set_prefix("\n");
        t.set_implicit(false);
        setzen(&mut t, "id", Value::from(d.id.as_str()));
        if t.get("name").is_none() && art.felder.iter().any(|f| f.name == "name") {
            setzen(&mut t, "name", Value::from(d.id.replace('_', " ").as_str()));
        }
        t.remove("vorlage");
        // Datei suchen oder anlegen
        let i = match self.dateien.iter().position(|x| x.rel == d.datei) {
            Some(i) => {
                self.dateien[i].loeschen = false;
                i
            }
            None => {
                self.dateien.push(Datei { rel: d.datei.clone(), doc: DocumentMut::new(), original: String::new(), loeschen: false });
                self.dateien.len() - 1
            }
        };
        let doc = &mut self.dateien[i].doc;
        let index = match art.ablage {
            Ablage::EineDatei => {
                t.decor_mut().set_prefix("");
                doc.insert(art.schluessel, Item::Table(t));
                0
            }
            _ => {
                if !matches!(doc.get(art.schluessel), Some(Item::ArrayOfTables(_))) {
                    doc.insert(art.schluessel, Item::ArrayOfTables(ArrayOfTables::new()));
                }
                let Some(Item::ArrayOfTables(a)) = doc.get_mut(art.schluessel) else { return };
                a.push(t);
                a.len() - 1
            }
        };
        self.auswahl = Some(Ort { datei: i, index });
        self.suche.clear();
    }

    fn entfernen(&mut self, art: &Art, o: Ort) {
        match art.ablage {
            Ablage::EineDatei => {
                if let Some(d) = self.dateien.get_mut(o.datei) {
                    d.loeschen = true;
                }
            }
            Ablage::Liste => {
                let Some(d) = self.dateien.get_mut(o.datei) else { return };
                if let Some(Item::ArrayOfTables(a)) = d.doc.get_mut(art.schluessel) {
                    // Kommentar über dem Eintrag an den nächsten weitergeben
                    let p = a.get(o.index).map(praefix).unwrap_or_default();
                    a.remove(o.index);
                    if let Some(n) = a.get_mut(o.index) {
                        praefix_voranstellen(n, &p);
                    } else if p.contains('#') {
                        crate::welt::kommentar_ans_ende(&mut d.doc, &p);
                    }
                }
                if matches!(d.doc.get(art.schluessel), Some(Item::ArrayOfTables(a)) if a.is_empty()) {
                    d.doc.remove(art.schluessel);
                }
            }
            Ablage::Fest { .. } => {}
        }
        self.auswahl = None;
    }

    /// Nach dem Neuladen wieder denselben Eintrag auswählen.
    pub fn auswahl_merken(&self) -> (usize, Option<String>) {
        let art = self.art_def();
        (self.art, self.auswahl.map(|o| self.titel(art, o).0))
    }

    pub fn auswahl_setzen(&mut self, art: usize, id: Option<String>) {
        self.art = art.min(ARTEN.len() - 1);
        let a = self.art_def();
        self.auswahl = id.and_then(|id| self.orte(a).into_iter().find(|o| self.titel(a, *o).0 == id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kopie(name: &str) -> PathBuf {
        let von = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../daten");
        let ziel = std::env::temp_dir().join(format!("rmeditor_inhalte_{name}_{}", std::process::id()));
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
    fn beispielwelt_laden_und_pruefen() {
        let i = Inhalte::laden(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../daten"));
        assert!(i.ladefehler.is_empty(), "{:?}", i.ladefehler);
        let p = i.pruefen();
        assert!(p.is_empty(), "{p:?}");
        let npc = crate::schema::art("npc").unwrap();
        assert!(i.orte(npc).len() >= 4);
        assert!(i.ids(Ziel::Raum, npc).contains(&"taverne".to_string()));
        assert!(i.ids(Ziel::Lehrbar, npc).contains(&"wuchtschlag".to_string()));
    }

    #[test]
    fn anlegen_aendern_loeschen_speichern() {
        let ordner = kopie("speichern");
        let mut i = Inhalte::laden(&ordner);
        assert!(!i.geaendert());
        let g = crate::schema::art("gegenstand").unwrap();
        i.anlegen(g, &NeuDialog { id: "eisenschwert".into(), datei: "gegenstaende/ausruestung.toml".into(), kopie_von: None });
        let o = i.auswahl.unwrap();
        let t = i.tabelle_mut(g, o).unwrap();
        setzen(t, "wert", Value::from(99));
        let npc = crate::schema::art("npc").unwrap();
        i.anlegen(npc, &NeuDialog { id: "baecker".into(), datei: "npcs/baecker.toml".into(), kopie_von: None });
        assert!(i.geaendert());
        i.speichern().unwrap();
        let text = fs::read_to_string(ordner.join("gegenstaende/ausruestung.toml")).unwrap();
        assert!(text.contains("id = \"eisenschwert\"") && text.contains("wert = 99"), "{text}");
        assert!(text.contains("#  Waffen und Rüstungen") || text.contains('#'), "Kommentare bleiben");
        assert!(fs::read_to_string(ordner.join("npcs/baecker.toml")).unwrap().contains("[npc]"));

        // NPC löschen: Datei verschwindet
        let mut i = Inhalte::laden(&ordner);
        let o = i.orte(npc).into_iter().find(|o| i.titel(npc, *o).0 == "baecker").unwrap();
        i.entfernen(npc, o);
        i.speichern().unwrap();
        assert!(!ordner.join("npcs/baecker.toml").exists());
        let _ = fs::remove_dir_all(ordner);
    }

    #[test]
    fn zusammenfuehren_behaelt_fremde_aenderungen() {
        let alt: DocumentMut = "[welt]\nname = \"A\"\nstartraum = \"x\"\n".parse().unwrap();
        let mut mein = alt.clone();
        mein["welt"]["name"] = toml_edit::value("B");
        let mut frisch: DocumentMut = "[welt]\nname = \"A\"\nstartraum = \"y\"\n".parse().unwrap();
        zusammenfuehren(frisch.as_table_mut(), alt.as_table(), mein.as_table());
        let t = frisch.to_string();
        assert!(t.contains("name = \"B\"") && t.contains("startraum = \"y\""), "{t}");
    }
}
