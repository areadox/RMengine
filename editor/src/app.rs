//! Die Oberfläche des Welteditors.

use crate::befehle::Befehlswerk;
use crate::fernwartung::Arbeitskopie;
use crate::fenster::{self, ServerFenster, UpdateFenster};
use crate::bereiche::{self, BefehlsAuswahl};
use crate::welt::{self, Problem, Raum, Welt, RICHTUNGEN};
use crate::inhalte::Inhalte;
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Key, KeyboardShortcut, Modifiers, Pos2, Rect, Sense, Shape, Stroke,
    StrokeKind, Vec2,
};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const ZELLE: f32 = 120.0;
const OW_ZELLE: f32 = 16.0;

/// Eine Änderung, die nach dem Zeichnen ausgeführt wird (wenn `self` wieder frei ist).
type Aktion = Option<Box<dyn FnOnce(&mut App)>>;

#[derive(Clone)]
struct Stand {
    raeume: Vec<Raum>,
    startraum: String,
    wiederbelebung: String,
}

/// Welcher Teil der Welt gerade bearbeitet wird.
#[derive(Clone, Copy, PartialEq)]
enum Bereich {
    Raeume,
    Inhalte,
    Befehle,
    Skripte,
}

enum Ziehen {
    Verschieben { id: String },
    Verbinden { id: String },
    Schwenken,
}

pub struct App {
    weltliste: crate::weltliste::Weltliste,
    /// Weltauswahl als Fenster (wenn schon eine Welt offen ist)
    welten_fenster: bool,
    neue_welt_eingabe: String,
    /// Wechsel zu dieser Welt, obwohl es ungespeicherte Änderungen gibt?
    wechsel_fragen: Option<PathBuf>,
    server_fenster: ServerFenster,
    update_fenster: UpdateFenster,
    bereich: Bereich,
    inhalte: Option<Inhalte>,
    befehlswerk: Option<Befehlswerk>,
    skriptwerk: Option<crate::skripte::Skriptwerk>,
    befehl_auswahl: Option<BefehlsAuswahl>,
    bereich_suche: String,
    ordner_eingabe: String,
    welt: Option<Welt>,
    ladefehler: Option<String>,
    geladen: Stand,

    auswahl: Option<String>,
    id_eingabe: String,
    id_fehler: Option<String>,

    karte: String,
    ebene: i32,
    kamera: Vec2,
    zoom: f32,
    ziehen: Option<Ziehen>,
    kontext: Option<(Option<String>, (i32, i32))>,
    auto_verbinden: bool,
    werkzeug_verbinden: bool,
    oberwelt_ansicht: bool,
    ow_kamera: Vec2,
    ow_zoom: f32,
    ow_auswahl: Option<(i32, i32)>,
    ow_maus: Option<(i32, i32)>,
    vorige_auswahl: Option<(String, f64)>,

    rueckgaengig: Vec<Stand>,
    wiederholen: Vec<Stand>,
    gruppe: Option<egui::Id>,

    suche: String,
    probleme: Vec<Problem>,
    zeige_probleme: bool,
    zeige_hilfe: bool,
    meldung: Option<(String, bool, f64)>,
    engine: Option<Arc<Mutex<Option<String>>>>,
    engine_ausgabe: Option<String>,
    neue_datei: String,

    loeschen_fragen: Option<String>,
    schliessen_fragen: bool,
    darf_schliessen: bool,
    neu_laden_fragen: bool,
}

impl App {
    pub fn neu(cc: &eframe::CreationContext<'_>, ordner: Option<PathBuf>) -> App {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let weltliste = crate::weltliste::Weltliste::laden();
        // Ohne Angabe: Weltauswahl zeigen (oder die zuletzt benutzte / ./daten öffnen)
        let (ordner, sofort) = match ordner {
            Some(o) => (o, true),
            None if !weltliste.welten.is_empty() && weltliste.beim_start_fragen => (weltliste.zuletzt().unwrap_or_else(|| ordner_suchen(None)), false),
            None => {
                let o = weltliste.zuletzt().unwrap_or_else(|| ordner_suchen(None));
                // Gibt es dort keine Welt, gleich die Auswahl zeigen (ohne Fehlermeldung)
                let da = o.join("welt.toml").exists();
                (o, da)
            }
        };
        let mut app = App {
            weltliste,
            welten_fenster: false,
            neue_welt_eingabe: String::new(),
            wechsel_fragen: None,
            server_fenster: ServerFenster::default(),
            update_fenster: UpdateFenster::default(),
            bereich: Bereich::Raeume,
            inhalte: None,
            befehlswerk: None,
            skriptwerk: None,
            befehl_auswahl: None,
            bereich_suche: String::new(),
            ordner_eingabe: ordner.display().to_string(),
            welt: None,
            ladefehler: None,
            geladen: Stand { raeume: Vec::new(), startraum: String::new(), wiederbelebung: String::new() },
            auswahl: None,
            id_eingabe: String::new(),
            id_fehler: None,
            karte: String::new(),
            ebene: 0,
            kamera: Vec2::ZERO,
            zoom: 1.0,
            ziehen: None,
            kontext: None,
            auto_verbinden: true,
            werkzeug_verbinden: false,
            oberwelt_ansicht: false,
            ow_kamera: Vec2::ZERO,
            ow_zoom: 1.4,
            ow_auswahl: None,
            ow_maus: None,
            vorige_auswahl: None,
            rueckgaengig: Vec::new(),
            wiederholen: Vec::new(),
            gruppe: None,
            suche: String::new(),
            probleme: Vec::new(),
            zeige_probleme: false,
            zeige_hilfe: false,
            meldung: None,
            engine: None,
            engine_ausgabe: None,
            neue_datei: String::new(),
            loeschen_fragen: None,
            schliessen_fragen: false,
            darf_schliessen: false,
            neu_laden_fragen: false,
        };
        if sofort {
            app.laden();
        }
        app
    }

    /// Eine andere Welt öffnen (fragt bei ungespeicherten Änderungen nach)
    fn welt_oeffnen(&mut self, pfad: PathBuf) {
        if self.welt.is_some() && self.geaendert() {
            self.wechsel_fragen = Some(pfad);
            return;
        }
        self.ordner_eingabe = pfad.display().to_string();
        self.auswahl = None;
        self.welten_fenster = false;
        self.laden();
    }

    /// Die Weltauswahl: zuletzt benutzte Welten, andere öffnen, neue anlegen, Server
    fn welten_auswahl(&mut self, ui: &mut egui::Ui) {
        use crate::weltliste::{info, normal, wann};
        let aktuell = self.welt.as_ref().map(|_| normal(Path::new(self.ordner_eingabe.trim())));
        let mut oeffnen = None;
        let mut entfernen = None;
        if self.weltliste.welten.is_empty() {
            ui.label(egui::RichText::new("Noch keine Welt geöffnet.").weak());
        } else {
            ui.label(egui::RichText::new("Zuletzt bearbeitet").strong());
            egui::ScrollArea::vertical().max_height(360.0).id_salt("welten").show(ui, |ui| {
                egui::Grid::new("welten_grid").num_columns(4).spacing([12.0, 6.0]).striped(true).show(ui, |ui| {
                    for e in self.weltliste.welten.clone() {
                        let pfad = PathBuf::from(&e.pfad);
                        let i = info(&pfad);
                        let name = if i.name.is_empty() { pfad.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default() } else { i.name.clone() };
                        let offen = aktuell.as_deref() == Some(e.pfad.as_str());
                        ui.vertical(|ui| {
                            ui.set_min_width(240.0);
                            let mut kopf = egui::RichText::new(if offen { format!("{name}  (offen)") } else { name }).strong();
                            if !i.da {
                                kopf = kopf.color(Color32::from_rgb(150, 150, 150));
                            }
                            ui.add(egui::Label::new(kopf).wrap_mode(egui::TextWrapMode::Extend));
                            // Nur die letzten Ordner zeigen, der ganze Pfad steht im Tooltip
                            let teile: Vec<&str> = e.pfad.split(['/', '\\']).filter(|t| !t.is_empty()).collect();
                            let kurz = if teile.len() > 3 { format!("…/{}", teile[teile.len() - 3..].join("/")) } else { e.pfad.clone() };
                            ui.add(egui::Label::new(egui::RichText::new(kurz).small().weak()).wrap_mode(egui::TextWrapMode::Extend)).on_hover_text(&e.pfad);
                        });
                        match &i.server {
                            Some(a) => {
                                let s = if i.schluessel_gemerkt { "Schlüssel gemerkt" } else { "Schlüssel wird abgefragt" };
                                ui.label(format!("🌐 {a}")).on_hover_text(format!("Arbeitskopie der Fernwartung – {s}"));
                            }
                            None => {
                                ui.label("💻 lokal");
                            }
                        }
                        ui.label(egui::RichText::new(wann(e.zuletzt)).weak());
                        ui.horizontal(|ui| {
                            if i.da {
                                if ui.add_enabled(!offen, egui::Button::new("Öffnen")).clicked() {
                                    oeffnen = Some(pfad.clone());
                                }
                            } else {
                                ui.label(egui::RichText::new("nicht gefunden").color(Color32::from_rgb(240, 110, 110)));
                            }
                            if ui.small_button("✖").on_hover_text("Aus der Liste nehmen (die Dateien bleiben)").clicked() {
                                entfernen = Some(e.pfad.clone());
                            }
                        });
                        ui.end_row();
                    }
                });
            });
            let mut fragen = self.weltliste.beim_start_fragen;
            if ui.checkbox(&mut fragen, "Beim Start diese Auswahl zeigen (sonst die zuletzt benutzte Welt öffnen)").changed() {
                self.weltliste.beim_start_fragen = fragen;
                self.weltliste.speichern();
            }
        }
        if let Some(p) = entfernen {
            self.weltliste.entfernen(&p);
        }
        ui.add_space(10.0);
        ui.separator();
        ui.label(egui::RichText::new("Andere Welt öffnen").strong());
        ui.horizontal(|ui| {
            ui.label("Datenordner (mit welt.toml):");
            let r = crate::kontext::feld(ui, egui::TextEdit::singleline(&mut self.ordner_eingabe).desired_width(380.0));
            if ui.button("Öffnen").clicked() || (r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter))) {
                oeffnen = Some(PathBuf::from(self.ordner_eingabe.trim()));
            }
        });
        ui.add_space(6.0);
        ui.label(egui::RichText::new("Neue leere Welt").strong());
        ui.horizontal(|ui| {
            ui.label("Ordner:");
            crate::kontext::feld(ui, egui::TextEdit::singleline(&mut self.neue_welt_eingabe).hint_text("z.B. C:\\Users\\Du\\Documents\\MeineWelt").desired_width(380.0));
            let ziel = PathBuf::from(self.neue_welt_eingabe.trim());
            let ok = !self.neue_welt_eingabe.trim().is_empty();
            if ui.add_enabled(ok, egui::Button::new("Anlegen und öffnen")).on_hover_text("welt.toml mit allen Einstellungen, ein Startraum und die üblichen Ordner").clicked() {
                match crate::neue_welt::anlegen(&ziel) {
                    Ok(true) => {
                        self.melden(format!("Leere Welt in '{}' angelegt.", ziel.display()), false);
                        oeffnen = Some(ziel);
                    }
                    Ok(false) => {
                        self.melden("Dort gibt es schon eine Welt – sie wird geöffnet.", false);
                        oeffnen = Some(ziel);
                    }
                    Err(e) => self.ladefehler = Some(format!("Die Welt konnte nicht angelegt werden: {e}")),
                }
            }
        });
        ui.add_space(6.0);
        ui.label(egui::RichText::new("Die Welt liegt auf einem Server?").strong());
        ui.horizontal(|ui| {
            if ui.button("🌐 Mit einem Server verbinden").clicked() {
                self.server_fenster.offen = true;
                self.server_fenster.anderen_zeigen();
            }
            ui.label(egui::RichText::new("Einmal verbunden, steht die Welt danach oben in der Liste – mit gemerktem Schlüssel.").weak());
        });
        if let Some(p) = oeffnen {
            self.welt_oeffnen(p);
        }
    }

    fn laden(&mut self) {
        let ordner = PathBuf::from(self.ordner_eingabe.trim());
        // Inhalte und Befehle (Fehler darin hindern nicht am Bearbeiten der Räume)
        let merken = self.inhalte.as_ref().map(|i| i.auswahl_merken());
        let mut inhalte = Inhalte::laden(&ordner);
        if let Some((art, id)) = merken {
            inhalte.auswahl_setzen(art, id);
        }
        for f in &inhalte.ladefehler {
            self.melden(f.clone(), true);
        }
        self.inhalte = Some(inhalte);
        // Skripte (Auswahl bleibt nicht erhalten, die Liste kann sich ändern)
        self.skriptwerk = Some(crate::skripte::Skriptwerk::laden(&ordner));
        self.befehlswerk = match Befehlswerk::laden(&ordner) {
            Ok(b) => Some(b),
            Err(e) => {
                self.melden(format!("Befehle konnten nicht geladen werden: {e}"), true);
                None
            }
        };
        if let (Some(BefehlsAuswahl::Geste(n)), Some(b)) = (self.befehl_auswahl, &self.befehlswerk) {
            if n >= b.gesten.len() {
                self.befehl_auswahl = None;
            }
        }
        match Welt::laden(&ordner) {
            Ok(w) => {
                self.geladen = Stand {
                    raeume: w.raeume.clone(),
                    startraum: w.startraum.clone(),
                    wiederbelebung: w.wiederbelebung.clone(),
                };
                let start = w.raum(&w.startraum).cloned();
                self.welt = Some(w);
                self.ladefehler = None;
                self.rueckgaengig.clear();
                self.wiederholen.clear();
                if self.auswahl.as_ref().is_some_and(|a| self.welt().raum(a).is_none()) {
                    self.auswahl = None;
                }
                if self.auswahl.is_none() {
                    if let Some(s) = start {
                        self.karte = s.karte.clone();
                        self.ebene = s.z;
                        self.kamera = Vec2::new(-(s.x as f32) * ZELLE, s.y as f32 * ZELLE);
                    }
                }
                if let Some(o) = &self.welt().oberwelt {
                    let (x, y) = o.eingaenge.first().map(|e| (e.x, e.y)).unwrap_or((o.breite() / 2, o.hoehe() / 2));
                    self.ow_kamera = -Vec2::new(x as f32, y as f32) * OW_ZELLE;
                } else {
                    self.oberwelt_ansicht = false;
                }
                self.probleme = self.alle_probleme();
                self.weltliste.eintragen(&ordner);
                let n = self.welt().raeume.len();
                self.melden(format!("{n} Räume aus '{}' geladen.", ordner.display()), false);
            }
            Err(e) => {
                self.welt = None;
                self.ladefehler = Some(e);
            }
        }
    }

    fn welt(&self) -> &Welt {
        self.welt.as_ref().expect("Welt geladen")
    }

    fn welt_mut(&mut self) -> &mut Welt {
        self.welt.as_mut().expect("Welt geladen")
    }

    fn melden(&mut self, text: impl Into<String>, fehler: bool) {
        self.meldung = Some((text.into(), fehler, 0.0));
    }

    fn stand(&self) -> Stand {
        Stand {
            raeume: self.welt().raeume.clone(),
            startraum: self.welt().startraum.clone(),
            wiederbelebung: self.welt().wiederbelebung.clone(),
        }
    }

    fn stand_setzen(&mut self, s: Stand) {
        let w = self.welt_mut();
        w.raeume = s.raeume;
        w.startraum = s.startraum;
        w.wiederbelebung = s.wiederbelebung;
    }

    /// Vor jeder Änderung aufrufen: merkt den Zustand für "Rückgängig".
    fn merken(&mut self) {
        let s = self.stand();
        self.rueckgaengig.push(s);
        if self.rueckgaengig.len() > 200 {
            self.rueckgaengig.remove(0);
        }
        self.wiederholen.clear();
        self.gruppe = None;
    }

    fn rueckgaengig_machen(&mut self) {
        if let Some(s) = self.rueckgaengig.pop() {
            let jetzt = self.stand();
            self.wiederholen.push(jetzt);
            self.stand_setzen(s);
            self.nach_aenderung();
            self.gruppe = None;
        }
    }

    fn wiederholen_machen(&mut self) {
        if let Some(s) = self.wiederholen.pop() {
            let jetzt = self.stand();
            self.rueckgaengig.push(jetzt);
            self.stand_setzen(s);
            self.nach_aenderung();
            self.gruppe = None;
        }
    }

    fn nach_aenderung(&mut self) {
        if let Some(a) = &self.auswahl {
            if self.welt().raum(a).is_none() {
                self.auswahl = None;
            }
        }
        self.id_eingabe = self.auswahl.clone().unwrap_or_default();
        self.id_fehler = None;
        self.probleme = self.alle_probleme();
    }

    fn geaendert(&self) -> bool {
        self.welt.as_ref().is_some_and(|w| w.geaendert(&self.geladen.raeume, &self.geladen.startraum, &self.geladen.wiederbelebung))
            || self.inhalte.as_ref().is_some_and(|b| b.geaendert())
            || self.befehlswerk.as_ref().is_some_and(|b| b.geaendert())
            || self.skriptwerk.as_ref().is_some_and(|b| b.geaendert())
    }

    /// Probleme aller Bereiche: Räume, Inhalte und Befehle.
    fn alle_probleme(&self) -> Vec<Problem> {
        let Some(w) = self.welt.as_ref() else { return Vec::new() };
        let mut p = w.pruefen();
        if let Some(b) = &self.inhalte {
            p.extend(b.pruefen().into_iter().map(|(fehler, text)| Problem { fehler, raum: None, text }));
        }
        if let Some(b) = &self.befehlswerk {
            p.extend(b.pruefen().into_iter().map(|(fehler, text)| Problem { fehler, raum: None, text }));
        }
        if let Some(b) = &self.skriptwerk {
            p.extend(b.probleme().into_iter().map(|text| Problem { fehler: true, raum: None, text }));
        }
        p
    }

    fn speichern(&mut self) {
        let Some(w) = self.welt.as_mut() else { return };
        // Erst die Räume, dann Inhalte (führen ihre Änderungen mit denen der Räume
        // zusammen) und Befehle (lesen die Dateien frisch ein)
        let ergebnis = w.speichern().and_then(|mut d| {
            if let Some(b) = self.inhalte.as_mut() {
                d.extend(b.speichern()?);
            }
            if let Some(b) = self.befehlswerk.as_mut() {
                d.extend(b.speichern()?);
            }
            if let Some(b) = self.skriptwerk.as_mut() {
                d.extend(b.speichern()?);
            }
            d.sort();
            d.dedup();
            Ok(d)
        });
        match ergebnis {
            Ok(dateien) => {
                let text = if dateien.is_empty() {
                    "Nichts zu speichern.".to_string()
                } else {
                    format!("Gespeichert: {}", dateien.join(", "))
                };
                let auswahl = self.auswahl.clone();
                self.laden();
                self.auswahl = auswahl.filter(|a| self.welt().raum(a).is_some());
                self.id_eingabe = self.auswahl.clone().unwrap_or_default();
                self.melden(text, false);
            }
            Err(e) => self.melden(format!("Speichern fehlgeschlagen: {e}"), true),
        }
    }

    fn waehlen(&mut self, id: Option<String>) {
        self.auswahl = id;
        self.id_eingabe = self.auswahl.clone().unwrap_or_default();
        self.id_fehler = None;
        self.gruppe = None;
    }

    /// Wählt einen Raum aus und zeigt ihn auf der Karte.
    fn hinspringen(&mut self, id: &str) {
        if let Some(r) = self.welt().raum(id).cloned() {
            self.oberwelt_ansicht = false;
            self.karte = r.karte.clone();
            self.ebene = r.z;
            self.kamera = Vec2::new(-(r.x as f32) * ZELLE, r.y as f32 * ZELLE);
            self.waehlen(Some(id.to_string()));
        }
    }

    /// Neuen Raum an (x, y) auf der aktuellen Karte/Ebene anlegen.
    fn raum_anlegen(&mut self, x: i32, y: i32, verbinden_mit: Option<String>) {
        if self.welt().raum_bei(&self.karte, x, y, self.ebene).is_some() {
            self.melden("Dort ist schon ein Raum.", true);
            return;
        }
        self.merken();
        // Vorlage: verbundener oder ausgewählter Raum, sonst ein Nachbar
        let vorlage = verbinden_mit
            .clone()
            .or_else(|| self.auswahl.clone())
            .and_then(|id| self.welt().raum(&id).cloned())
            .filter(|r| r.karte == self.karte)
            .or_else(|| self.welt().raeume.iter().find(|r| r.karte == self.karte).cloned());
        let basis = vorlage.as_ref().map(|v| v.gebiet.clone()).filter(|g| !g.is_empty()).unwrap_or("raum".into());
        let id = self.welt().neue_id(&basis);
        let datei = vorlage
            .as_ref()
            .map(|v| v.datei.clone())
            .or_else(|| self.welt().dateien.first().cloned())
            .unwrap_or_else(|| "raeume/neu.toml".into());
        let r = Raum {
            id: id.clone(),
            name: "Ein neuer Raum".into(),
            x,
            y,
            z: self.ebene,
            karte: self.karte.clone(),
            gebiet: vorlage.as_ref().map(|v| v.gebiet.clone()).unwrap_or_default(),
            gelaende: vorlage.as_ref().map(|v| v.gelaende.clone()).unwrap_or_default(),
            datei,
            ..Default::default()
        };
        self.welt_mut().raeume.push(r);
        if let Some(von) = verbinden_mit {
            if let Some(v) = self.welt().raum(&von).cloned() {
                let ri = if v.z == self.ebene { welt::richtung_fuer(x - v.x, y - v.y, 0) } else { None };
                if let Some(ri) = ri {
                    if !v.ausgaenge.contains_key(ri) {
                        self.welt_mut().verbinden(&von, ri, &id);
                    }
                }
            }
        }
        self.waehlen(Some(id.clone()));
        self.nach_aenderung();
        self.melden(format!("Raum '{id}' angelegt."), false);
    }

    fn verbinden(&mut self, von: &str, nach: &str) {
        if von == nach {
            return;
        }
        let (Some(a), Some(b)) = (self.welt().raum(von).cloned(), self.welt().raum(nach).cloned()) else { return };
        let ri = if a.karte == b.karte {
            if a.z != b.z && a.x == b.x && a.y == b.y {
                Some(if b.z > a.z { "hoch" } else { "runter" })
            } else {
                welt::richtung_ungefaehr(b.x - a.x, b.y - a.y)
            }
        } else {
            Some("rein")
        };
        let Some(ri) = ri else {
            self.melden("Keine passende Richtung gefunden.", true);
            return;
        };
        if let Some(z) = a.ausgaenge.get(ri) {
            self.melden(format!("'{von}' hat schon einen Ausgang {ri} (nach '{z}'). Erst im Eigenschaftsfeld ändern."), true);
            return;
        }
        self.merken();
        self.welt_mut().verbinden(von, ri, nach);
        self.nach_aenderung();
        let mut text = format!("Verbunden: {von} –{ri}➡ {nach}");
        if !welt::passt_zur_lage(self.welt(), &a, ri, nach) {
            text += "  (Achtung: liegt nicht direkt daneben – im Spiel fehlt die Linie auf der Karte)";
        }
        if self.welt().beidseitig {
            if let Some(anders) = b.ausgaenge.get(welt::gegenteil(ri)).filter(|z| *z != von) {
                text += &format!(
                    "  (Achtung: '{nach}' hat {} schon nach '{anders}', darum gibt es keinen Rückweg)",
                    welt::gegenteil(ri)
                );
            }
        }
        self.melden(text, false);
    }

    fn loeschen(&mut self, id: &str) {
        self.merken();
        self.welt_mut().loeschen(id);
        if self.auswahl.as_deref() == Some(id) {
            self.waehlen(None);
        }
        self.nach_aenderung();
        self.melden(format!("Raum '{id}' gelöscht."), false);
    }

    fn engine_pruefen(&mut self) {
        let ordner = PathBuf::from(self.ordner_eingabe.trim());
        let programm = engine_finden();
        let ergebnis = Arc::new(Mutex::new(None));
        let e2 = ergebnis.clone();
        std::thread::spawn(move || {
            let text = match std::process::Command::new(&programm)
                .arg(&ordner)
                .arg("--pruefen")
                .arg("--keine-updatepruefung")
                .env("PROMPT", "1")
                .stdin(std::process::Stdio::null())
                .output()
            {
                Ok(o) => {
                    let mut t = String::from_utf8_lossy(&o.stdout).to_string();
                    t += &String::from_utf8_lossy(&o.stderr);
                    t
                }
                Err(e) => format!(
                    "Konnte '{}' nicht starten: {e}\n\nLege rmeditor neben das Programm rmengine oder installiere rmengine.",
                    programm.display()
                ),
            };
            *e2.lock().unwrap() = Some(text);
        });
        self.engine = Some(ergebnis);
        self.engine_ausgabe = Some("Prüfe …".into());
    }
}

fn engine_finden() -> PathBuf {
    let name = if cfg!(windows) { "rmengine.exe" } else { "rmengine" };
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|o| o.join(name)))
        .filter(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from(name))
}

pub fn farbe(name: &str) -> Color32 {
    match name {
        "schwarz" => Color32::from_rgb(70, 70, 70),
        "rot" => Color32::from_rgb(190, 60, 60),
        "gruen" => Color32::from_rgb(60, 160, 60),
        "gelb" => Color32::from_rgb(190, 170, 40),
        "blau" => Color32::from_rgb(60, 90, 200),
        "magenta" => Color32::from_rgb(170, 70, 170),
        "cyan" => Color32::from_rgb(50, 160, 170),
        "weiss" => Color32::from_rgb(190, 190, 190),
        "grau" => Color32::from_rgb(130, 130, 130),
        "hellrot" => Color32::from_rgb(240, 100, 100),
        "hellgruen" => Color32::from_rgb(120, 220, 120),
        "hellgelb" => Color32::from_rgb(240, 225, 110),
        "hellblau" => Color32::from_rgb(110, 160, 250),
        "hellmagenta" => Color32::from_rgb(230, 130, 230),
        "hellcyan" => Color32::from_rgb(120, 225, 235),
        "hellweiss" => Color32::from_rgb(245, 245, 245),
        _ => Color32::from_rgb(150, 150, 150),
    }
}

fn abdunkeln(c: Color32, f: f32) -> Color32 {
    Color32::from_rgb((c.r() as f32 * f) as u8, (c.g() as f32 * f) as u8, (c.b() as f32 * f) as u8)
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // Aktionen aus dem Rechtsklick-Menü der Textfelder (Ausschneiden, Rückgängig ...)
        crate::kontext::ereignisse_einspielen(&ctx);

        // Schließen mit ungespeicherten Änderungen
        if ctx.input(|i| i.viewport().close_requested()) && self.geaendert() && !self.darf_schliessen {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.schliessen_fragen = true;
        }

        // Ergebnis der Engine-Prüfung abholen
        if let Some(e) = self.engine.clone() {
            let fertig = e.lock().unwrap().take();
            if let Some(t) = fertig {
                self.engine_ausgabe = Some(t);
                self.engine = None;
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }

        if self.welt.is_none() {
            egui::CentralPanel::default().show(ui, |ui| self.startseite(ui));
            self.netz_fenster(&ctx);
            return;
        }

        self.tastatur(&ctx);
        self.oben(ui);
        self.unten(ui);

        // Inhalte und Befehle: Liste links, Formular in der Mitte
        if self.bereich != Bereich::Raeume {
            let bereich = self.bereich;
            egui::Panel::left("bereich_links").default_size(280.0).show(ui, |ui| match bereich {
                Bereich::Inhalte => {
                    if let Some(b) = self.inhalte.as_mut() {
                        b.liste(ui);
                    }
                }
                Bereich::Skripte => {
                    if let Some(b) = self.skriptwerk.as_mut() {
                        b.liste(ui);
                    }
                }
                _ => match self.befehlswerk.as_mut() {
                    Some(b) => bereiche::befehle_liste(ui, b, &mut self.befehl_auswahl, &mut self.bereich_suche),
                    None => {
                        ui.label("Die Befehle konnten nicht geladen werden (siehe Meldung unten).");
                    }
                },
            });
            egui::CentralPanel::default().show(ui, |ui| match bereich {
                Bereich::Inhalte => {
                    if let Some(b) = self.inhalte.as_mut() {
                        b.formular(ui);
                    }
                }
                Bereich::Skripte => {
                    if let Some(b) = self.skriptwerk.as_mut() {
                        b.formular(ui);
                    }
                }
                _ => {
                    if let Some(b) = self.befehlswerk.as_mut() {
                        bereiche::befehl_formular(ui, b, &mut self.befehl_auswahl);
                    }
                }
            });
            // Prüfung laufend aktualisieren (Änderungen passieren direkt im Formular)
            self.probleme = self.alle_probleme();
            self.fenster(&ctx);
            self.netz_fenster(&ctx);
            return;
        }

        egui::Panel::left("links").default_size(230.0).show(ui, |ui| self.links(ui));
        egui::Panel::right("rechts").default_size(380.0).size_range(320.0..=800.0).show(ui, |ui| {
            if self.oberwelt_ansicht {
                egui::ScrollArea::vertical().show(ui, |ui| self.ow_feld_panel(ui));
            } else if self.auswahl.is_some() {
                egui::ScrollArea::vertical().show(ui, |ui| self.eigenschaften(ui));
            } else {
                ui.add_space(12.0);
                ui.label(egui::RichText::new("Kein Raum ausgewählt.").strong());
                ui.add_space(6.0);
                ui.label("Klicke auf einen Raum, um ihn zu bearbeiten, oder doppelklicke auf ein leeres Feld, um einen neuen Raum anzulegen.");
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Mehr unter ❓ Hilfe.").weak());
            }
        });
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| {
            if self.oberwelt_ansicht {
                self.oberwelt_zeichnen(ui);
            } else {
                self.karte_zeichnen(ui);
            }
        });
        self.fenster(&ctx);
        self.netz_fenster(&ctx);
    }
}

impl App {
    /// Die Fenster für Fernwartung und Updates.
    fn netz_fenster(&mut self, ctx: &egui::Context) {
        if self.welten_fenster && self.welt.is_some() {
            let mut offen = true;
            egui::Window::new("🌍 Welten").open(&mut offen).min_width(680.0).default_width(720.0).collapsible(false).show(ctx, |ui| {
                self.welten_auswahl(ui);
            });
            self.welten_fenster &= offen;
        }
        if let Some(pfad) = self.wechsel_fragen.clone() {
            egui::Window::new("Ungespeicherte Änderungen").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                ui.label(format!("Die aktuelle Welt hat ungespeicherte Änderungen.\nVor dem Wechsel zu '{}' speichern?", pfad.display()));
                ui.horizontal(|ui| {
                    if ui.button("Speichern und wechseln").clicked() {
                        self.speichern();
                        self.wechsel_fragen = None;
                        if !self.geaendert() {
                            self.welt_oeffnen(pfad.clone());
                        }
                    }
                    if ui.button("Verwerfen und wechseln").clicked() {
                        self.wechsel_fragen = None;
                        self.welt = None;
                        self.welt_oeffnen(pfad.clone());
                    }
                    if ui.button("Abbrechen").clicked() {
                        self.wechsel_fragen = None;
                    }
                });
            });
        }
        let daten = PathBuf::from(self.ordner_eingabe.trim());
        let ungespeichert = self.geaendert();
        if self.server_fenster.offen {
            match self.server_fenster.zeichnen(ctx, &daten, ungespeichert) {
                Some(fenster::Aktion::Oeffnen(pfad)) => {
                    self.ordner_eingabe = pfad.display().to_string();
                    self.auswahl = None;
                    self.laden();
                }
                Some(fenster::Aktion::Speichern) => self.speichern(),
                None => {}
            }
        }
        if self.update_fenster.offen {
            let server = self.server_fenster.verbindung(&daten).map(|(_, k, s)| (k, s));
            if self.update_fenster.zeichnen(ctx, server, ungespeichert) {
                // Mit der neuen Version neu starten
                let _ = std::process::Command::new(crate::update::eigener_pfad()).arg(&self.ordner_eingabe).spawn();
                self.darf_schliessen = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}

impl App {
    fn startseite(&mut self, ui: &mut egui::Ui) {
        ui.heading("RMengine – Welteditor");
        ui.add_space(4.0);
        ui.label("Welche Welt möchtest du bearbeiten?");
        if let Some(f) = &self.ladefehler {
            ui.add_space(8.0);
            ui.colored_label(Color32::from_rgb(240, 110, 110), f);
        }
        ui.add_space(8.0);
        self.welten_auswahl(ui);
        ui.add_space(12.0);
        if ui.button("⟳ Nach Updates suchen").clicked() {
            self.update_fenster.oeffnen(ui.ctx());
        }
    }

    fn tastatur(&mut self, ctx: &egui::Context) {
        let speichern = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
        let undo = KeyboardShortcut::new(Modifiers::COMMAND, Key::Z);
        let redo1 = KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
        let redo2 = KeyboardShortcut::new(Modifiers::COMMAND, Key::Y);
        let textfeld = ctx.memory(|m| m.focused().is_some());
        if ctx.input_mut(|i| i.consume_shortcut(&speichern)) {
            self.speichern();
        }
        if !textfeld && self.bereich == Bereich::Raeume {
            if ctx.input_mut(|i| i.consume_shortcut(&redo1) || i.consume_shortcut(&redo2)) {
                self.wiederholen_machen();
            } else if ctx.input_mut(|i| i.consume_shortcut(&undo)) {
                self.rueckgaengig_machen();
            }
            if ctx.input(|i| i.key_pressed(Key::Delete)) {
                self.loeschen_fragen = self.auswahl.clone();
            }
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                self.waehlen(None);
            }
            if ctx.input(|i| i.key_pressed(Key::PageUp)) {
                self.ebene += 1;
            }
            if ctx.input(|i| i.key_pressed(Key::PageDown)) {
                self.ebene -= 1;
            }
        }
    }

    fn oben(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("oben").show(ui, |ui| {
            ui.horizontal(|ui| {
                for (b, name) in [(Bereich::Raeume, "🗺 Räume"), (Bereich::Inhalte, "📚 Inhalte"), (Bereich::Befehle, "⌨ Befehle"), (Bereich::Skripte, "📜 Skripte")] {
                    if ui.selectable_label(self.bereich == b, egui::RichText::new(name).strong()).clicked() && self.bereich != b {
                        self.bereich = b;
                        self.bereich_suche.clear();
                    }
                }
                ui.separator();
                let geaendert = self.geaendert();
                let text = if geaendert { "💾 Speichern*" } else { "💾 Speichern" };
                if ui.add_enabled(geaendert, egui::Button::new(text)).on_hover_text("Strg+S").clicked() {
                    self.speichern();
                }
                if ui.button("⟳ Neu laden").on_hover_text("Dateien neu einlesen").clicked() {
                    if geaendert {
                        self.neu_laden_fragen = true;
                    } else {
                        self.laden();
                    }
                }
                ui.separator();
                if ui.add_enabled(!self.rueckgaengig.is_empty(), egui::Button::new("↺")).on_hover_text("Rückgängig (Strg+Z)").clicked() {
                    self.rueckgaengig_machen();
                }
                if ui.add_enabled(!self.wiederholen.is_empty(), egui::Button::new("↻")).on_hover_text("Wiederholen (Strg+Y)").clicked() {
                    self.wiederholen_machen();
                }
                ui.separator();
                let fehler = self.probleme.iter().filter(|p| p.fehler).count();
                let warn = self.probleme.len() - fehler;
                let text = format!("⚠ {fehler} Fehler, {warn} Hinweise");
                let farbe = if fehler > 0 {
                    Color32::from_rgb(240, 110, 110)
                } else if warn > 0 {
                    Color32::from_rgb(230, 190, 90)
                } else {
                    Color32::from_rgb(120, 210, 120)
                };
                if ui.add(egui::Button::new(egui::RichText::new(text).color(farbe))).clicked() {
                    self.zeige_probleme = !self.zeige_probleme;
                }
                if ui.button("▶ Mit Engine prüfen").on_hover_text("Startet 'rmengine --pruefen' für den gespeicherten Stand").clicked() {
                    self.engine_pruefen();
                }
                ui.separator();
                // Nur die letzten Ordner zeigen, damit die Knöpfe rechts Platz haben
                let teile: Vec<&str> = self.ordner_eingabe.split(['/', '\\']).filter(|t| !t.is_empty()).collect();
                let kurz = if teile.len() > 3 { format!("…/{}", teile[teile.len() - 3..].join("/")) } else { self.ordner_eingabe.clone() };
                ui.label(egui::RichText::new(kurz).weak()).on_hover_text(&self.ordner_eingabe);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("❓ Hilfe").clicked() {
                        self.zeige_hilfe = !self.zeige_hilfe;
                    }
                    if ui.button("⟳ Updates").on_hover_text("Nach einer neuen Version suchen").clicked() {
                        self.update_fenster.oeffnen(ui.ctx());
                    }
                    if ui.button("🌍 Welten").on_hover_text("Eine andere Welt öffnen oder eine neue anlegen").clicked() {
                        self.welten_fenster = !self.welten_fenster;
                    }
                    let server = Arbeitskopie::zu_datenordner(Path::new(self.ordner_eingabe.trim()));
                    let text = match &server {
                        Some((_, k)) => format!("🌐 {}", k.adresse),
                        None => "🌐 Server".to_string(),
                    };
                    if ui.button(text).on_hover_text("Fernwartung: Welt von einem Server holen und wieder hochladen").clicked() {
                        self.server_fenster.offen = !self.server_fenster.offen;
                    }
                });
            });
        });
    }

    fn unten(&mut self, ui: &mut egui::Ui) {
        let jetzt = ui.input(|i| i.time);
        egui::Panel::bottom("unten").show(ui, |ui| {
            ui.horizontal(|ui| {
                if self.oberwelt_ansicht {
                    let pos = self.ow_maus.map_or("–".to_string(), |(x, y)| format!("x = {x}, y = {y}   (ow:{x}:{y})"));
                    ui.label(format!("Oberwelt   Maus: {pos}   Zoom: {:.0} %", self.ow_zoom * 100.0));
                } else {
                ui.label(format!(
                    "Karte: {}   Ebene: {}   Zoom: {:.0} %",
                    if self.karte.is_empty() { "Hauptkarte" } else { &self.karte },
                    self.ebene,
                    self.zoom * 100.0
                ));
                }
                if let Some((text, fehler, seit)) = &mut self.meldung {
                    if *seit == 0.0 {
                        *seit = jetzt;
                    }
                    if jetzt - *seit < 8.0 {
                        ui.separator();
                        let f = if *fehler { Color32::from_rgb(240, 110, 110) } else { Color32::from_rgb(150, 210, 150) };
                        ui.colored_label(f, text.as_str());
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
                    }
                }
            });
        });
    }

    fn links(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.strong("Karte");
        let karten = self.welt().karten();
        egui::ComboBox::from_id_salt("karte")
            .width(200.0)
            .selected_text(if self.oberwelt_ansicht {
                "🌍 Oberwelt".to_string()
            } else if self.karte.is_empty() {
                "Hauptkarte".to_string()
            } else {
                self.karte.clone()
            })
            .show_ui(ui, |ui| {
                for k in karten {
                    let name = if k.is_empty() { "Hauptkarte".to_string() } else { k.clone() };
                    if ui.selectable_label(!self.oberwelt_ansicht && self.karte == k, name).clicked() {
                        self.karte = k;
                        self.kamera = Vec2::ZERO;
                        self.oberwelt_ansicht = false;
                    }
                }
                if self.welt.as_ref().is_some_and(|w| w.oberwelt.is_some())
                    && ui.selectable_label(self.oberwelt_ansicht, "🌍 Oberwelt (karte.txt)").clicked()
                {
                    self.oberwelt_ansicht = true;
                }
            });
        if self.oberwelt_ansicht {
            ui.add_space(6.0);
            ui.label(egui::RichText::new("Koordinaten wie in oberwelt.toml:\nx = Spalte, y = Zeile (ab 1).").weak());
        }
        if !self.oberwelt_ansicht {
        ui.add_space(6.0);
        ui.strong("Ebene (z)");
        ui.horizontal_wrapped(|ui| {
            if ui.small_button("⏶").on_hover_text("Eine Ebene höher (Bild ↑)").clicked() {
                self.ebene += 1;
            }
            if ui.small_button("⏷").on_hover_text("Eine Ebene tiefer (Bild ↓)").clicked() {
                self.ebene -= 1;
            }
            for e in self.welt().ebenen(&self.karte) {
                if ui.selectable_label(self.ebene == e, e.to_string()).clicked() {
                    self.ebene = e;
                }
            }
        });
        ui.add_space(6.0);
        ui.strong("Ziehen mit der Maus");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.werkzeug_verbinden, false, "✋ Verschieben");
            ui.selectable_value(&mut self.werkzeug_verbinden, true, "🔗 Verbinden");
        })
        .response
        .on_hover_text("Verbinden: von einem Raum zu einem anderen ziehen legt einen Ausgang an.\nAuf ein leeres Feld ziehen legt dort einen neuen, verbundenen Raum an.\n(Shift + Ziehen verbindet immer.)");
        ui.add_space(6.0);
        ui.checkbox(&mut self.auto_verbinden, "Neue Räume mit Auswahl verbinden")
            .on_hover_text("Doppelklick neben den ausgewählten Raum legt einen Raum an und verbindet beide.");
        }
        ui.separator();

        ui.strong("Räume");
        crate::kontext::feld(ui, egui::TextEdit::singleline(&mut self.suche).hint_text("🔍 Suchen …"));
        let suche = self.suche.to_lowercase();
        let mut springen = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let mut gebiete: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
            for r in &self.welt().raeume {
                if suche.is_empty() || r.id.contains(&suche) || r.name.to_lowercase().contains(&suche) {
                    let g = if r.gebiet.is_empty() { "(ohne Gebiet)".into() } else { r.gebiet.clone() };
                    gebiete.entry(g).or_default().push((r.id.clone(), r.name.clone()));
                }
            }
            for (g, raeume) in gebiete {
                egui::CollapsingHeader::new(format!("{g} ({})", raeume.len()))
                    .default_open(true)
                    .show(ui, |ui| {
                        for (id, name) in raeume {
                            let gewaehlt = self.auswahl.as_deref() == Some(id.as_str());
                            let start = if self.welt().startraum == id { "★ " } else { "" };
                            let eintrag = ui.selectable_label(gewaehlt, format!("{start}{name}")).on_hover_text(&id);
                            eintrag.context_menu(|ui| {
                                if ui.button("Hinspringen").clicked() {
                                    springen = Some(id.clone());
                                    ui.close();
                                }
                                if ui.button("📋 ID kopieren").clicked() {
                                    ui.ctx().copy_text(id.clone());
                                    ui.close();
                                }
                                if ui.button("Name kopieren").clicked() {
                                    ui.ctx().copy_text(name.clone());
                                    ui.close();
                                }
                            });
                            if eintrag.clicked() {
                                springen = Some(id);
                            }
                        }
                    });
            }
        });
        if let Some(id) = springen {
            self.hinspringen(&id);
        }
    }

    // -----------------------------------------------------------------------
    // Eigenschaften des ausgewählten Raums
    // -----------------------------------------------------------------------

    fn eigenschaften(&mut self, ui: &mut egui::Ui) {
        let Some(id) = self.auswahl.clone() else { return };
        let Some(i) = self.welt().index(&id) else { return };
        let vorher = self.welt().raeume[i].clone();
        let mut r = vorher.clone();
        let mut aktion: Aktion = None;

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.heading(&r.name);
            if self.welt().startraum == r.id {
                ui.label(egui::RichText::new("★ Startraum").color(Color32::GOLD));
            }
            if self.welt().wiederbelebung == r.id {
                ui.label(egui::RichText::new("✚ Wiederbelebung").color(Color32::LIGHT_GREEN))
                    .on_hover_text("Hier erwachen Spieler nach dem Tod ([kampf] wiederbelebung_raum in welt.toml).");
            }
        });
        ui.add_space(4.0);

        egui::Grid::new("felder").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
            ui.label("ID");
            ui.vertical(|ui| {
                let rid = crate::kontext::feld(ui, egui::TextEdit::singleline(&mut self.id_eingabe).desired_width(200.0));
                if rid.lost_focus() && self.id_eingabe != r.id {
                    let neu = self.id_eingabe.trim().to_string();
                    let alt = r.id.clone();
                    aktion = Some(Box::new(move |app: &mut App| {
                        let vorher = app.stand();
                        match app.welt_mut().umbenennen(&alt, &neu) {
                            Ok(()) => {
                                app.rueckgaengig.push(vorher);
                                app.wiederholen.clear();
                                app.auswahl = Some(neu.clone());
                                app.nach_aenderung();
                                app.melden(format!("'{alt}' heißt jetzt '{neu}' (alle Verweise angepasst)."), false);
                            }
                            Err(e) => app.id_fehler = Some(e),
                        }
                    }));
                }
                if let Some(f) = &self.id_fehler {
                    ui.colored_label(Color32::from_rgb(240, 110, 110), f);
                }
            });
            ui.end_row();

            ui.label("Name");
            crate::kontext::feld(ui, egui::TextEdit::singleline(&mut r.name).desired_width(f32::INFINITY));
            ui.end_row();

            ui.label("Gebiet");
            ui.horizontal(|ui| {
                crate::kontext::feld(ui, egui::TextEdit::singleline(&mut r.gebiet).desired_width(150.0));
                let gebiete = self.welt().gebiete();
                ui.menu_button("⏷", |ui| {
                    for g in gebiete {
                        if ui.button(&g).clicked() {
                            r.gebiet = g;
                            ui.close();
                        }
                    }
                });
            });
            ui.end_row();

            ui.label("Gelände");
            let gelaende = self.welt().gelaende.clone();
            let anzeige = |id: &str| {
                if id.is_empty() {
                    "(keins)".to_string()
                } else {
                    gelaende.get(id).map(|g| format!("{} – {}", g.name, id)).unwrap_or_else(|| format!("? {id}"))
                }
            };
            egui::ComboBox::from_id_salt("gelaende").width(200.0).selected_text(anzeige(&r.gelaende)).show_ui(ui, |ui| {
                ui.selectable_value(&mut r.gelaende, String::new(), "(keins)");
                for (gid, g) in &gelaende {
                    ui.selectable_value(&mut r.gelaende, gid.clone(), format!("{} {} – {}", g.symbol, g.name, gid));
                }
            });
            ui.end_row();

            ui.label("Symbol");
            ui.horizontal(|ui| {
                ui.add_sized([30.0, 20.0], egui::TextEdit::singleline(&mut r.symbol).char_limit(1));
                ui.label("Klammern");
                ui.add_sized([30.0, 20.0], egui::TextEdit::singleline(&mut r.klammern).char_limit(2))
                    .on_hover_text("Kästchen auf der Stadtkarte, z.B. () oder []. Leer = vom Gelände.");
            });
            ui.end_row();

            ui.label("Farbe");
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("farbe")
                    .width(110.0)
                    .selected_text(if r.farbe.is_empty() { "(Gelände)" } else { &r.farbe })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut r.farbe, String::new(), "(Gelände)");
                        for f in welt::FARBEN {
                            ui.selectable_value(&mut r.farbe, f.to_string(), egui::RichText::new(f).color(farbe(f)));
                        }
                    });
            });
            ui.end_row();

            ui.label("Position");
            ui.horizontal(|ui| {
                ui.label("x");
                ui.add(egui::DragValue::new(&mut r.x));
                ui.label("y");
                ui.add(egui::DragValue::new(&mut r.y));
                ui.label("z");
                ui.add(egui::DragValue::new(&mut r.z));
            });
            ui.end_row();

            ui.label("Karte");
            crate::kontext::feld(ui, egui::TextEdit::singleline(&mut r.karte).hint_text("(Hauptkarte)").desired_width(150.0))
                .on_hover_text("Eigene Karte, z.B. für das Innere eines Hauses. Leer = gemeinsame Karte.");
            ui.end_row();

            ui.label("Datei");
            ui.horizontal(|ui| {
                let dateien = self.welt().dateien.clone();
                egui::ComboBox::from_id_salt("datei").width(170.0).selected_text(&r.datei).show_ui(ui, |ui| {
                    for d in dateien {
                        ui.selectable_value(&mut r.datei, d.clone(), d);
                    }
                });
                ui.menu_button("Neu…", |ui| {
                    ui.label("Neue Datei (relativ zum Datenordner):");
                    crate::kontext::feld(ui, egui::TextEdit::singleline(&mut self.neue_datei).hint_text("raeume/burg.toml"));
                    if ui.button("Raum dorthin verschieben").clicked() {
                        let mut d = self.neue_datei.trim().replace('\\', "/");
                        if !d.is_empty() {
                            if !d.ends_with(".toml") {
                                d += ".toml";
                            }
                            r.datei = d.clone();
                            let w = self.welt_mut();
                            if !w.dateien.contains(&d) {
                                w.dateien.push(d);
                                w.dateien.sort();
                            }
                            ui.close();
                        }
                    }
                });
            });
            ui.end_row();
        });

        ui.add_space(8.0);
        ui.strong("Beschreibung");
        crate::kontext::feld(
            ui,
            egui::TextEdit::multiline(&mut r.beschreibung)
                .desired_rows(6)
                .desired_width(f32::INFINITY)
                .hint_text("Was sieht man hier? Farben: {gruen}Text{/}"),
        );
        egui::CollapsingHeader::new(if r.beschreibung_nacht.is_empty() { "Bei Nacht (leer = wie am Tag)" } else { "Bei Nacht (eigene Beschreibung)" })
            .id_salt("nacht")
            .show(ui, |ui| {
                crate::kontext::feld(
                    ui,
                    egui::TextEdit::multiline(&mut r.beschreibung_nacht)
                        .desired_rows(4)
                        .desired_width(f32::INFINITY)
                        .hint_text("So sieht der Raum nachts aus (mit Tag und Nacht in Welt → Zeit)."),
                );
            });
        ui.horizontal(|ui| {
            ui.label("Drinnen oder draußen:");
            let text = match r.drinnen {
                None => "wie das Gelände",
                Some(true) => "drinnen (kein Wetter)",
                Some(false) => "draußen",
            };
            egui::ComboBox::from_id_salt("drinnen").selected_text(text).show_ui(ui, |ui| {
                ui.selectable_value(&mut r.drinnen, None, "wie das Gelände");
                ui.selectable_value(&mut r.drinnen, Some(true), "drinnen (kein Wetter)");
                ui.selectable_value(&mut r.drinnen, Some(false), "draußen");
            });
        });

        ui.add_space(8.0);
        ui.strong("Ausgänge");
        let alle = self.welt().alle_ausgaenge(&r.id);
        egui::Grid::new("ausgaenge").num_columns(3).spacing([6.0, 3.0]).show(ui, |ui| {
            for ri in RICHTUNGEN {
                ui.label(ri);
                let mut ziel = r.ausgaenge.get(ri).cloned().unwrap_or_default();
                let auto = alle.get(ri).filter(|(_, a)| *a).map(|(z, _)| z.clone());
                let bekannt = ziel.is_empty() || self.welt().ziel_gueltig(&ziel);
                let mut te = egui::TextEdit::singleline(&mut ziel).desired_width(160.0);
                if let Some(a) = &auto {
                    te = te.hint_text(format!("auto: {a}"));
                }
                if !bekannt {
                    te = te.text_color(Color32::from_rgb(240, 110, 110));
                }
                let resp = crate::kontext::feld(ui, te.desired_width(162.0));
                if resp.changed() {
                    if ziel.trim().is_empty() {
                        r.ausgaenge.remove(ri);
                    } else {
                        r.ausgaenge.insert(ri.to_string(), ziel.trim().to_string());
                    }
                }
                let sprung = r.ausgaenge.get(ri).cloned().or(auto).filter(|z| self.welt().index(z).is_some());
                if let Some(z) = sprung {
                    if ui.small_button("➡").on_hover_text(format!("Zu '{z}' springen")).clicked() {
                        aktion = Some(Box::new(move |app: &mut App| app.hinspringen(&z)));
                    }
                } else {
                    ui.label("");
                }
                ui.end_row();
            }
        });
        if self.welt().beidseitig {
            ui.label(egui::RichText::new("Grau = Rückweg, den die Engine automatisch anlegt.").small().weak());
        }

        ui.add_space(8.0);
        ui.strong("Sinne");
        egui::Grid::new("sinne").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
            for (name, feld) in [("Geruch", &mut r.geruch), ("Geräusch", &mut r.geraeusch), ("Gefühl", &mut r.gefuehl)] {
                ui.label(name);
                crate::kontext::feld(ui, egui::TextEdit::multiline(feld).desired_rows(1).desired_width(f32::INFINITY));
                ui.end_row();
            }
        });

        ui.add_space(8.0);
        let w = self.welt();
        let (gg, mo, np, bi) = (w.gegenstaende.clone(), w.monster.clone(), w.npcs.clone(), w.bibliothek.clone());
        liste_bearbeiten(ui, "Gegenstände", "gegenstaende", &mut r.gegenstaende, &gg, true);
        liste_bearbeiten(ui, "Monster", "monster", &mut r.monster, &mo, true);
        liste_bearbeiten(ui, "NPCs", "npcs", &mut r.npcs, &np, false);
        liste_bearbeiten(ui, "Bibliothek (nutzt)", "nutzt", &mut r.nutzt, &bi, false);

        if r.details > 0 {
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(format!(
                    "{} Detail(s) in der Datei – bleiben beim Speichern erhalten (im Texteditor bearbeiten).",
                    r.details
                ))
                .weak(),
            );
        }

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if self.welt().startraum != r.id && ui.button("★ Als Startraum").clicked() {
                let id = r.id.clone();
                aktion = Some(Box::new(move |app: &mut App| {
                    app.merken();
                    app.welt_mut().startraum = id;
                    app.nach_aenderung();
                }));
            }
            if self.welt().wiederbelebung != r.id && ui.button("✚ Als Wiederbelebungsort").clicked() {
                let id = r.id.clone();
                aktion = Some(Box::new(move |app: &mut App| {
                    app.merken();
                    app.welt_mut().wiederbelebung = id;
                    app.nach_aenderung();
                }));
            }
            if ui.button("🗑 Löschen").clicked() {
                self.loeschen_fragen = Some(r.id.clone());
            }
        });

        // Änderungen übernehmen (Tippen in einem Feld = ein Rückgängig-Schritt)
        if r != vorher {
            let fokus = ui.ctx().memory(|m| m.focused());
            if fokus.is_none() || fokus != self.gruppe {
                let mut s = self.stand();
                s.raeume[i] = vorher;
                self.rueckgaengig.push(s);
                self.wiederholen.clear();
                self.gruppe = fokus;
            }
            self.welt_mut().raeume[i] = r;
            self.probleme = self.alle_probleme();
        }
        if let Some(a) = aktion {
            a(self);
        }
    }

    // -----------------------------------------------------------------------
    // Die Karte
    // -----------------------------------------------------------------------

    fn karte_zeichnen(&mut self, ui: &mut egui::Ui) {
        let (rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_rgb(24, 26, 30));

        let zelle = ZELLE * self.zoom;
        let mitte = rect.center() + self.kamera * self.zoom;
        let zu_bild = |x: i32, y: i32| Pos2::new(mitte.x + x as f32 * zelle, mitte.y - y as f32 * zelle);
        let zu_feld = |p: Pos2| {
            let v = (p - mitte) / zelle;
            (v.x.round() as i32, (-v.y).round() as i32)
        };
        let box_groesse = Vec2::new(zelle * 0.68, zelle * 0.46);

        // Zoom mit dem Mausrad
        if let Some(maus) = resp.hover_pos() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            let zoom = ui.input(|i| i.zoom_delta());
            let faktor = (scroll / 400.0).exp() * zoom;
            if (faktor - 1.0).abs() > 1e-4 {
                let alt = self.zoom;
                self.zoom = (self.zoom * faktor).clamp(0.2, 3.0);
                // Punkt unter der Maus festhalten
                let vor = (maus - rect.center()) / alt - self.kamera;
                self.kamera = (maus - rect.center()) / self.zoom - vor;
            }
        }

        // Raster
        let raster = Color32::from_rgb(36, 39, 45);
        let (x0, y1) = zu_feld(rect.left_top());
        let (x1, y0) = zu_feld(rect.right_bottom());
        if zelle > 25.0 {
            for x in x0 - 1..=x1 + 1 {
                let px = zu_bild(x, 0).x + zelle / 2.0;
                painter.line_segment([Pos2::new(px, rect.top()), Pos2::new(px, rect.bottom())], Stroke::new(1.0, raster));
            }
            for y in y0 - 1..=y1 + 1 {
                let py = zu_bild(0, y).y + zelle / 2.0;
                painter.line_segment([Pos2::new(rect.left(), py), Pos2::new(rect.right(), py)], Stroke::new(1.0, raster));
            }
        }
        // Ursprung
        let o = zu_bild(0, 0);
        painter.circle_filled(o, 2.0, Color32::from_rgb(70, 75, 85));

        let welt = self.welt.as_ref().unwrap();
        let ebene: Vec<&Raum> = welt.raeume.iter().filter(|r| r.karte == self.karte && r.z == self.ebene).collect();
        let hier: HashSet<&str> = ebene.iter().map(|r| r.id.as_str()).collect();

        // Räume anderer Ebenen schwach andeuten
        for r in welt.raeume.iter().filter(|r| r.karte == self.karte && (r.z - self.ebene).abs() == 1) {
            let b = Rect::from_center_size(zu_bild(r.x, r.y), box_groesse * 0.9);
            painter.rect_stroke(b, 4.0, Stroke::new(1.0, Color32::from_rgb(55, 60, 68)), StrokeKind::Inside);
        }

        // Verbindungen
        let mut gezeichnet = HashSet::new();
        for r in &ebene {
            for (ri, ziel) in &r.ausgaenge {
                let Some(z) = welt.raum(ziel) else { continue };
                if !hier.contains(ziel.as_str()) || welt::versatz(ri).is_some_and(|v| v.2 != 0) || ri == "rein" || ri == "raus" {
                    continue;
                }
                let paar = if r.id < z.id { (r.id.clone(), z.id.clone()) } else { (z.id.clone(), r.id.clone()) };
                let a = zu_bild(r.x, r.y);
                let b = zu_bild(z.x, z.y);
                let zurueck_explizit = z.ausgaenge.get(welt::gegenteil(ri)) == Some(&r.id);
                let zurueck_irgendwo = z.ausgaenge.values().any(|v| *v == r.id);
                let zurueck_auto = welt.beidseitig && !z.ausgaenge.contains_key(welt::gegenteil(ri));
                let beidseitig = zurueck_explizit || zurueck_auto || zurueck_irgendwo;
                let passt = welt::passt_zur_lage(welt, r, ri, ziel);
                let farbe = if !passt {
                    Color32::from_rgb(230, 150, 60)
                } else if beidseitig {
                    Color32::from_rgb(150, 155, 165)
                } else {
                    Color32::from_rgb(230, 190, 90)
                };
                let breite = (2.0 * self.zoom).clamp(1.0, 3.0);
                if !gezeichnet.insert(paar) && beidseitig {
                    continue;
                }
                if passt {
                    painter.line_segment([a, b], Stroke::new(breite, farbe));
                } else {
                    painter.extend(Shape::dashed_line(&[a, b], Stroke::new(breite, farbe), 6.0, 4.0));
                }
                if !beidseitig {
                    // Einbahnstraße: Pfeil zur Mitte
                    let m = a + (b - a) * 0.5;
                    let d = (b - a).normalized() * 10.0 * self.zoom.max(0.6);
                    painter.arrow(m - d, d * 1.5, Stroke::new(breite, farbe));
                }
            }
        }

        // Räume
        let maus = resp.hover_pos();
        let mut unter_maus: Option<String> = None;
        for r in &ebene {
            let mittel = zu_bild(r.x, r.y);
            let b = Rect::from_center_size(mittel, box_groesse);
            if maus.is_some_and(|m| b.expand(4.0).contains(m)) {
                unter_maus = Some(r.id.clone());
            }
            let g = welt.gelaende.get(&r.gelaende);
            let fname = if !r.farbe.is_empty() { r.farbe.as_str() } else { g.map_or("", |g| g.farbe.as_str()) };
            let grund = farbe(fname);
            let gewaehlt = self.auswahl.as_deref() == Some(r.id.as_str());
            let fehlerhaft = self.probleme.iter().any(|p| p.fehler && p.raum.as_deref() == Some(r.id.as_str()));
            painter.rect_filled(b, CornerRadius::same(6), abdunkeln(grund, 0.35));
            let rand = if gewaehlt {
                Stroke::new(3.0, Color32::from_rgb(255, 230, 80))
            } else if fehlerhaft {
                Stroke::new(2.0, Color32::from_rgb(240, 80, 80))
            } else {
                Stroke::new(1.5, grund)
            };
            painter.rect_stroke(b, CornerRadius::same(6), rand, StrokeKind::Middle);

            let symbol = if !r.symbol.is_empty() { r.symbol.clone() } else { g.map_or(String::new(), |g| g.symbol.clone()) };
            if zelle > 45.0 {
                painter.text(
                    b.center_top() + Vec2::new(0.0, 4.0 * self.zoom),
                    Align2::CENTER_TOP,
                    &symbol,
                    FontId::monospace(16.0 * self.zoom),
                    grund,
                );
                let name = kuerzen(&r.name, (box_groesse.x / (6.5 * self.zoom)).max(4.0) as usize);
                painter.text(
                    b.center_bottom() - Vec2::new(0.0, 4.0 * self.zoom),
                    Align2::CENTER_BOTTOM,
                    name,
                    FontId::proportional(11.0 * self.zoom),
                    Color32::from_rgb(230, 230, 230),
                );
            } else {
                painter.text(b.center(), Align2::CENTER_CENTER, &symbol, FontId::monospace(14.0 * self.zoom.max(0.6)), grund);
            }
            if welt.startraum == r.id {
                painter.text(b.left_top() + Vec2::new(3.0, 1.0), Align2::LEFT_TOP, "★", FontId::proportional(12.0 * self.zoom.max(0.7)), Color32::GOLD);
            }
            if welt.wiederbelebung == r.id {
                painter.text(b.left_top() + Vec2::new(3.0 + 14.0 * self.zoom.max(0.7), 1.0), Align2::LEFT_TOP, "✚", FontId::proportional(12.0 * self.zoom.max(0.7)), Color32::LIGHT_GREEN);
            }
            // Ausgänge, die nicht auf dieser Ebene liegen: hoch/runter/rein/raus
            let mut marken = Vec::new();
            for (ri, (ziel, _)) in welt.alle_ausgaenge(&r.id) {
                let andere_ebene = welt.raum(&ziel).is_some_and(|z| z.karte != r.karte || z.z != r.z);
                if matches!(ri.as_str(), "hoch" | "runter" | "rein" | "raus") || andere_ebene {
                    let zeichen = match ri.as_str() {
                        "hoch" => "⏶",
                        "runter" => "⏷",
                        "raus" if welt.raum(&ziel).is_none() => "🌍",
                        "rein" | "raus" => "🚪",
                        _ => "🔗",
                    };
                    marken.push(zeichen);
                }
            }
            if !marken.is_empty() && zelle > 45.0 {
                painter.text(
                    b.right_top() + Vec2::new(-3.0, 1.0),
                    Align2::RIGHT_TOP,
                    marken.join(""),
                    FontId::proportional(11.0 * self.zoom.max(0.7)),
                    Color32::from_rgb(140, 200, 255),
                );
            }
        }

        // Tooltip
        if let (Some(id), None) = (&unter_maus, &self.ziehen) {
            if let Some(r) = welt.raum(id) {
                let aus: Vec<String> = welt
                    .alle_ausgaenge(id)
                    .into_iter()
                    .map(|(ri, (z, auto))| format!("{ri} ➡ {z}{}", if auto { " (auto)" } else { "" }))
                    .collect();
                let text = format!("{} ({})\n{}, {}, {}\n{}", r.name, r.id, r.x, r.y, r.z, aus.join("\n"));
                resp.clone().on_hover_text_at_pointer(text);
            }
        }

        // ---- Bedienung ----
        let shift = ui.input(|i| i.modifiers.shift);
        if resp.drag_started_by(egui::PointerButton::Primary) {
            self.ziehen = Some(match &unter_maus {
                Some(id) if shift || self.werkzeug_verbinden => Ziehen::Verbinden { id: id.clone() },
                Some(id) => Ziehen::Verschieben { id: id.clone() },
                None => Ziehen::Schwenken,
            });
        }
        if resp.drag_started_by(egui::PointerButton::Secondary) || resp.drag_started_by(egui::PointerButton::Middle) {
            self.ziehen = Some(Ziehen::Schwenken);
        }

        let zeiger = ui.input(|i| i.pointer.interact_pos()).or(maus);
        match &self.ziehen {
            Some(Ziehen::Schwenken) => {
                self.kamera += resp.drag_delta() / self.zoom;
            }
            Some(Ziehen::Verschieben { id }) => {
                if let (Some(p), Some(r)) = (zeiger, welt.raum(id)) {
                    let (fx, fy) = zu_feld(p);
                    let frei = welt.raum_bei(&self.karte, fx, fy, self.ebene).is_none_or(|j| welt.raeume[j].id == *id);
                    let ziel = Rect::from_center_size(zu_bild(fx, fy), box_groesse);
                    let f = if frei { Color32::from_rgb(120, 220, 120) } else { Color32::from_rgb(240, 90, 90) };
                    painter.rect_stroke(ziel, CornerRadius::same(6), Stroke::new(2.0, f), StrokeKind::Middle);
                    painter.line_segment([zu_bild(r.x, r.y), ziel.center()], Stroke::new(1.0, f));
                }
            }
            Some(Ziehen::Verbinden { id }) => {
                if let (Some(p), Some(r)) = (zeiger, welt.raum(id)) {
                    painter.line_segment([zu_bild(r.x, r.y), p], Stroke::new(2.5, Color32::from_rgb(120, 200, 255)));
                    if unter_maus.is_none() {
                        let (fx, fy) = zu_feld(p);
                        let ziel = Rect::from_center_size(zu_bild(fx, fy), box_groesse);
                        painter.rect_stroke(ziel, CornerRadius::same(6), Stroke::new(1.5, Color32::from_rgb(120, 200, 255)), StrokeKind::Middle);
                        painter.text(ziel.center(), Align2::CENTER_CENTER, "+", FontId::proportional(20.0 * self.zoom), Color32::from_rgb(120, 200, 255));
                    }
                }
            }
            None => {}
        }

        if resp.drag_started_by(egui::PointerButton::Primary) && unter_maus.is_some() {
            self.waehlen(unter_maus.clone());
        }
        if resp.drag_stopped() {
            let ziehen = self.ziehen.take();
            let p = ui.input(|i| i.pointer.interact_pos()).or(maus);
            match (ziehen, p) {
                (Some(Ziehen::Verschieben { id }), Some(p)) => {
                    let (fx, fy) = zu_feld(p);
                    let r = self.welt().raum(&id).cloned();
                    if let Some(r) = r {
                        if (r.x, r.y) != (fx, fy) {
                            if self.welt().raum_bei(&self.karte, fx, fy, self.ebene).is_some() {
                                self.melden("Dort ist schon ein Raum.", true);
                            } else {
                                self.merken();
                                let i = self.welt().index(&id).unwrap();
                                self.welt_mut().raeume[i].x = fx;
                                self.welt_mut().raeume[i].y = fy;
                                self.nach_aenderung();
                            }
                        }
                    }
                }
                (Some(Ziehen::Verbinden { id }), Some(p)) => match unter_maus.clone() {
                    Some(ziel) => self.verbinden(&id, &ziel),
                    None => {
                        let (fx, fy) = zu_feld(p);
                        self.raum_anlegen(fx, fy, Some(id));
                    }
                },
                _ => {}
            }
        }

        if resp.clicked() {
            if unter_maus.is_none() && self.auswahl.is_some() {
                // Für einen Doppelklick: die Auswahl vor dem ersten Klick merken
                let t = ui.input(|i| i.time);
                self.vorige_auswahl = self.auswahl.clone().map(|a| (a, t));
            }
            self.waehlen(unter_maus.clone());
        }
        if resp.double_clicked() && unter_maus.is_none() {
            if let Some(p) = maus {
                let (fx, fy) = zu_feld(p);
                let mit = if self.auto_verbinden {
                    {
                        let t = ui.input(|i| i.time);
                        self.vorige_auswahl.take().filter(|(_, z)| t - z < 1.0).map(|(a, _)| a)
                    }
                    .or(self.auswahl.clone()).filter(|a| {
                        self.welt().raum(a).is_some_and(|r| {
                            r.karte == self.karte && r.z == self.ebene && (r.x - fx).abs() <= 1 && (r.y - fy).abs() <= 1
                        })
                    })
                } else {
                    None
                };
                self.raum_anlegen(fx, fy, mit);
            }
        }
        if resp.secondary_clicked() {
            if let Some(p) = maus {
                self.kontext = Some((unter_maus.clone(), zu_feld(p)));
            }
        }
        let mut aktion: Aktion = None;
        resp.context_menu(|ui| {
            let Some((raum, (fx, fy))) = self.kontext.clone() else { return };
            match raum {
                Some(id) => {
                    ui.label(egui::RichText::new(&id).strong());
                    if ui.button("📋 ID kopieren").clicked() {
                        ui.ctx().copy_text(id.clone());
                        ui.close();
                        return;
                    }
                    if ui.button("★ Als Startraum festlegen").clicked() {
                        aktion = Some(Box::new(move |app: &mut App| {
                            app.merken();
                            app.welt_mut().startraum = id;
                            app.nach_aenderung();
                        }));
                        ui.close();
                        return;
                    }
                    if ui.button("✚ Als Wiederbelebungsort festlegen").clicked() {
                        aktion = Some(Box::new(move |app: &mut App| {
                            app.merken();
                            app.welt_mut().wiederbelebung = id;
                            app.nach_aenderung();
                        }));
                        ui.close();
                        return;
                    }
                    let nachbarn: Vec<String> = self
                        .welt()
                        .alle_ausgaenge(&id)
                        .into_values()
                        .map(|(z, _)| z)
                        .filter(|z| self.welt().index(z).is_some())
                        .collect::<std::collections::BTreeSet<_>>()
                        .into_iter()
                        .collect();
                    if !nachbarn.is_empty() {
                        ui.menu_button("✂ Verbindung trennen", |ui| {
                            for n in nachbarn {
                                if ui.button(&n).clicked() {
                                    let a = id.clone();
                                    aktion = Some(Box::new(move |app: &mut App| {
                                        app.merken();
                                        app.welt_mut().trennen(&a, &n);
                                        app.nach_aenderung();
                                        app.melden(format!("Verbindung {a} – {n} getrennt."), false);
                                    }));
                                    ui.close();
                                }
                            }
                        });
                    }
                    if ui.button("🗑 Löschen").clicked() {
                        self.loeschen_fragen = Some(id.clone());
                        ui.close();
                    }
                }
                None => {
                    if ui.button(format!("➕ Neuer Raum hier ({fx}, {fy})")).clicked() {
                        aktion = Some(Box::new(move |app: &mut App| app.raum_anlegen(fx, fy, None)));
                        ui.close();
                    }
                }
            }
        });
        if let Some(a) = aktion {
            a(self);
        }

        // Leere Welt / Hinweis
        if self.welt().raeume.iter().all(|r| r.karte != self.karte || r.z != self.ebene) {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "Auf dieser Ebene gibt es noch keine Räume.\nDoppelklick legt einen neuen Raum an.",
                FontId::proportional(16.0),
                Color32::from_rgb(120, 125, 135),
            );
        }
    }

    // -----------------------------------------------------------------------
    // Die Oberwelt (karte.txt)
    // -----------------------------------------------------------------------

    fn oberwelt_zeichnen(&mut self, ui: &mut egui::Ui) {
        let (rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_rgb(18, 20, 24));
        let Some(ow) = self.welt.as_ref().and_then(|w| w.oberwelt.clone()) else { return };

        // Zoom und Schwenken
        if let Some(maus) = resp.hover_pos() {
            let faktor = (ui.input(|i| i.smooth_scroll_delta.y) / 400.0).exp() * ui.input(|i| i.zoom_delta());
            if (faktor - 1.0).abs() > 1e-4 {
                let alt = self.ow_zoom;
                self.ow_zoom = (self.ow_zoom * faktor).clamp(0.4, 4.0);
            
                let vor = (maus - rect.center()) / alt - self.ow_kamera;
                self.ow_kamera = (maus - rect.center()) / self.ow_zoom - vor;
            }
        }
        if resp.dragged() {
            self.ow_kamera += resp.drag_delta() / self.ow_zoom;
        }

        let zelle = OW_ZELLE * self.ow_zoom;
        let ursprung = rect.center() + self.ow_kamera * self.ow_zoom;
        // Feld (x, y) mit x = Spalte, y = Zeile (ab 1); Mittelpunkt des Feldes:
        let zu_bild = |x: i32, y: i32| Pos2::new(ursprung.x + x as f32 * zelle, ursprung.y + y as f32 * zelle);
        let zu_feld = |p: Pos2| {
            let v = (p - ursprung) / zelle;
            (v.x.round() as i32, v.y.round() as i32)
        };

        // Sichtbarer Bereich
        let (x0, y0) = zu_feld(rect.left_top());
        let (x1, y1) = zu_feld(rect.right_bottom());
        let schrift = FontId::monospace(zelle * 0.85);
        for y in y0.max(1)..=y1.min(ow.hoehe()) {
            for x in x0.max(1)..=x1.min(ow.breite()) {
                let Some(c) = ow.zeichen(x, y) else { continue };
                let f = ow.felder.get(&c);
                let farbe_feld = farbe(f.map_or("", |f| f.farbe.as_str()));
                let p = zu_bild(x, y);
                let feld = Rect::from_center_size(p, Vec2::splat(zelle));
                let begehbar = f.is_none_or(|f| f.begehbar);
                painter.rect_filled(feld, 0.0, abdunkeln(farbe_feld, if begehbar { 0.22 } else { 0.12 }));
                let symbol = f.map_or(c, |f| f.symbol).to_string();
                painter.text(p, Align2::CENTER_CENTER, symbol, schrift.clone(), farbe_feld);
            }
        }
        // Raster ab ausreichender Größe
        if zelle >= 14.0 {
            let linie = Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 0, 90));
            for x in x0.max(1)..=x1.min(ow.breite()) + 1 {
                let px = zu_bild(x, 0).x - zelle / 2.0;
                painter.line_segment([Pos2::new(px, zu_bild(0, 1).y - zelle / 2.0), Pos2::new(px, zu_bild(0, ow.hoehe()).y + zelle / 2.0)], linie);
            }
            for y in y0.max(1)..=y1.min(ow.hoehe()) + 1 {
                let py = zu_bild(0, y).y - zelle / 2.0;
                painter.line_segment([Pos2::new(zu_bild(1, 0).x - zelle / 2.0, py), Pos2::new(zu_bild(ow.breite(), 0).x + zelle / 2.0, py)], linie);
            }
        }

        // Eingänge und Orte: erst die Kästchen, dann die Beschriftungen (Orte links, Eingänge rechts)
        let punkte: Vec<(&welt::OwPunkt, Color32, bool)> = ow
            .orte
            .iter()
            .map(|o| (o, Color32::from_rgb(120, 200, 255), false))
            .chain(ow.eingaenge.iter().map(|e| (e, Color32::from_rgb(255, 210, 80), true)))
            .collect();
        for (pkt, rahmen, _) in &punkte {
            let p = zu_bild(pkt.x, pkt.y);
            let feld = Rect::from_center_size(p, Vec2::splat(zelle));
            painter.rect_filled(feld, 2.0, Color32::from_rgb(20, 20, 20));
            painter.rect_stroke(feld, 2.0, Stroke::new(2.0, *rahmen), StrokeKind::Middle);
            // Ohne eigenes Symbol zeigt die Engine das Gelände
            let sym = if pkt.symbol.is_empty() {
                ow.zeichen(pkt.x, pkt.y).map(|c| ow.felder.get(&c).map_or(c, |f| f.symbol)).unwrap_or(' ').to_string()
            } else {
                pkt.symbol.clone()
            };
            let f = if pkt.farbe.is_empty() { *rahmen } else { farbe(&pkt.farbe) };
            painter.text(p, Align2::CENTER_CENTER, sym, FontId::monospace(zelle * 0.85), f);
        }
        if zelle >= 10.0 {
            for (pkt, rahmen, rechts) in &punkte {
                let feld = Rect::from_center_size(zu_bild(pkt.x, pkt.y), Vec2::splat(zelle));
                let g = painter.layout_no_wrap(pkt.name.clone(), FontId::proportional(12.0), *rahmen);
                let pos = if *rechts {
                    feld.right_center() + Vec2::new(5.0, -g.size().y / 2.0)
                } else {
                    feld.left_center() - Vec2::new(5.0 + g.size().x, g.size().y / 2.0)
                };
                painter.rect_filled(Rect::from_min_size(pos - Vec2::new(3.0, 1.0), g.size() + Vec2::new(6.0, 2.0)), 3.0, Color32::from_black_alpha(190));
                painter.galley(pos, g, *rahmen);
            }
        }
        // Ausgänge aus Räumen, die direkt auf Oberwelt-Felder zeigen (z.B. raus = "ow:36:11")
        let mut ow_ziele: Vec<(i32, i32, String)> = Vec::new();
        for r in &self.welt().raeume {
            for (ri, z) in &r.ausgaenge {
                if let Some((x, y)) = welt::oberwelt_feld(z) {
                    ow_ziele.push((x, y, format!("{} ({ri})", r.id)));
                }
            }
        }
        for (x, y, _) in &ow_ziele {
            let feld = Rect::from_center_size(zu_bild(*x, *y), Vec2::splat(zelle));
            painter.rect_stroke(feld, 2.0, Stroke::new(2.0, Color32::from_rgb(230, 130, 255)), StrokeKind::Middle);
        }

        // Auswahl
        if let Some((x, y)) = self.ow_auswahl {
            let feld = Rect::from_center_size(zu_bild(x, y), Vec2::splat(zelle));
            painter.rect_stroke(feld.expand(2.0), 2.0, Stroke::new(2.5, Color32::WHITE), StrokeKind::Middle);
        }

        // Maus: Fadenkreuz, Koordinaten, Tooltip
        self.ow_maus = None;
        if let Some(m) = resp.hover_pos() {
            let (x, y) = zu_feld(m);
            if x >= 1 && y >= 1 && x <= ow.breite() && y <= ow.hoehe() {
                self.ow_maus = Some((x, y));
                let feld = Rect::from_center_size(zu_bild(x, y), Vec2::splat(zelle));
                let hell = Color32::from_rgba_unmultiplied(255, 255, 255, 25);
                painter.rect_filled(Rect::from_x_y_ranges(rect.x_range(), feld.y_range()), 0.0, hell);
                painter.rect_filled(Rect::from_x_y_ranges(feld.x_range(), rect.y_range()), 0.0, hell);
                painter.rect_stroke(feld, 0.0, Stroke::new(1.5, Color32::WHITE), StrokeKind::Middle);
                let mut text = format!("x = {x}, y = {y}");
                match ow.zeichen(x, y) {
                    Some(c) => {
                        let f = ow.felder.get(&c);
                        text += &format!("\n'{c}' {}", f.map_or("(nicht erklärt)".to_string(), |f| f.name.clone()));
                        if !ow.begehbar(x, y) {
                            text += "  – nicht begehbar";
                        }
                    }
                    None => text += "\n(leer)",
                }
                for e in ow.eingaenge.iter().filter(|e| (e.x, e.y) == (x, y)) {
                    text += &format!("\nEingang: {} ➡ {}", e.name, e.raum);
                }
                for o in ow.orte.iter().filter(|o| (o.x, o.y) == (x, y)) {
                    text += &format!("\nOrt: {}", o.name);
                }
                for (_, _, von) in ow_ziele.iter().filter(|(a, b, _)| (*a, *b) == (x, y)) {
                    text += &format!("\nAusgang von {von}");
                }
                resp.clone().on_hover_text_at_pointer(text);
                if resp.clicked() {
                    self.ow_auswahl = Some((x, y));
                }
            }
        }

        // Lineale oben und links mit Spalten- und Zeilennummern
        let lineal = 22.0;
        let grund = Color32::from_rgb(32, 35, 41);
        let oben = Rect::from_min_size(rect.min, Vec2::new(rect.width(), lineal));
        let links = Rect::from_min_size(rect.min, Vec2::new(34.0, rect.height()));
        painter.rect_filled(oben, 0.0, grund);
        painter.rect_filled(links, 0.0, grund);
        let schritt = if zelle >= 24.0 { 1 } else if zelle >= 12.0 { 5 } else { 10 };
        let klein = FontId::monospace(10.0);
        let normal = Color32::from_rgb(150, 155, 165);
        let hervor = Color32::from_rgb(255, 230, 80);
        for x in x0.max(1)..=x1.min(ow.breite()) {
            let maus = self.ow_maus.is_some_and(|m| m.0 == x);
            if x % schritt == 0 || x == 1 || maus {
                let p = Pos2::new(zu_bild(x, 0).x, oben.center().y);
                if p.x > links.right() + 6.0 {
                    painter.text(p, Align2::CENTER_CENTER, x.to_string(), klein.clone(), if maus { hervor } else { normal });
                }
            }
        }
        for y in y0.max(1)..=y1.min(ow.hoehe()) {
            let maus = self.ow_maus.is_some_and(|m| m.1 == y);
            if y % schritt == 0 || y == 1 || maus {
                let p = Pos2::new(links.center().x, zu_bild(0, y).y);
                if p.y > oben.bottom() + 4.0 {
                    painter.text(p, Align2::CENTER_CENTER, y.to_string(), klein.clone(), if maus { hervor } else { normal });
                }
            }
        }
        painter.rect_filled(Rect::from_min_size(rect.min, Vec2::new(34.0, lineal)), 0.0, grund);
        painter.text(rect.min + Vec2::new(17.0, 11.0), Align2::CENTER_CENTER, "y\\x", klein, normal);
    }

    fn ow_feld_panel(&mut self, ui: &mut egui::Ui) {
        let Some(ow) = self.welt().oberwelt.clone() else { return };
        ui.add_space(4.0);
        ui.heading(if ow.name.is_empty() { "Oberwelt".to_string() } else { ow.name.clone() });
        ui.label(egui::RichText::new(format!("karte.txt: {} Spalten × {} Zeilen", ow.breite(), ow.hoehe())).weak());
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(Color32::from_rgb(255, 210, 80), "■ Eingang");
            ui.colored_label(Color32::from_rgb(120, 200, 255), "■ Ort");
            ui.colored_label(Color32::from_rgb(230, 130, 255), "■ Ziel eines Raum-Ausgangs (ow:x:y)");
        });
        ui.add_space(8.0);
        let Some((x, y)) = self.ow_auswahl else {
            ui.label("Klicke auf ein Feld, um seine Koordinaten zu sehen.");
            ui.add_space(6.0);
            ui.label(egui::RichText::new("Mausrad = zoomen, Ziehen = verschieben.\nDie Koordinate unter der Maus steht unten in der Statuszeile und im Lineal.").weak());
            return;
        };
        ui.label(egui::RichText::new(format!("x = {x}, y = {y}")).size(20.0).strong());
        let id = format!("ow:{x}:{y}");
        ui.horizontal(|ui| {
            ui.monospace(&id);
            if ui.small_button("📋 Kopieren").clicked() {
                ui.ctx().copy_text(id.clone());
                self.meldung = Some((format!("'{id}' kopiert."), false, 0.0));
            }
            if ui.small_button("📋 x/y kopieren").on_hover_text("Für [[eingang]] oder [[ort]] in oberwelt.toml").clicked() {
                ui.ctx().copy_text(format!("x = {x}\ny = {y}"));
                self.meldung = Some(("x/y kopiert.".into(), false, 0.0));
            }
        });
        ui.add_space(6.0);
        match ow.zeichen(x, y) {
            Some(c) => {
                let f = ow.felder.get(&c);
                ui.label(format!("Zeichen '{c}': {}", f.map_or("nicht in oberwelt.toml erklärt", |f| f.name.as_str())));
                if ow.begehbar(x, y) {
                    ui.label("begehbar");
                } else {
                    ui.colored_label(Color32::from_rgb(240, 110, 110), "nicht begehbar");
                }
            }
            None => {
                ui.label("Hier ist nichts (Leerzeichen oder außerhalb der Karte).");
            }
        }
        for e in ow.eingaenge.iter().filter(|e| (e.x, e.y) == (x, y)) {
            ui.add_space(6.0);
            ui.label(egui::RichText::new(format!("Eingang: {}", e.name)).strong());
            let raum = e.raum.clone();
            if ui.button(format!("➡ Zum Raum '{raum}'")).clicked() {
                self.hinspringen(&raum);
            }
        }
        for o in ow.orte.iter().filter(|o| (o.x, o.y) == (x, y)) {
            ui.add_space(6.0);
            ui.label(egui::RichText::new(format!("Ort: {}", o.name)).strong());
        }

        // Ausgang eines Raums auf dieses Feld legen
        ui.add_space(10.0);
        ui.separator();
        match self.auswahl.clone().and_then(|a| self.welt().raum(&a).cloned()) {
            Some(r) if ow.begehbar(x, y) => {
                ui.label(format!("Zuletzt gewählter Raum: {} ({})", r.name, r.id));
                let aktuell = r.ausgaenge.get("raus").cloned();
                if let Some(a) = &aktuell {
                    ui.label(egui::RichText::new(format!("raus führt bisher zu '{a}'")).weak());
                }
                if aktuell.as_deref() != Some(id.as_str())
                    && ui.button(format!("🌍 Als Ausgang 'raus' von '{}' setzen", r.id)).clicked()
                {
                    self.merken();
                    let i = self.welt().index(&r.id).unwrap();
                    self.welt_mut().raeume[i].ausgaenge.insert("raus".into(), id.clone());
                    self.nach_aenderung();
                    self.melden(format!("'{}': raus ➡ {id}", r.id), false);
                }
            }
            Some(_) => {
                ui.label(egui::RichText::new("Auf nicht begehbare Felder kann kein Ausgang führen.").weak());
            }
            None => {
                ui.label(egui::RichText::new("Tipp: Wähle vorher einen Raum aus, dann kannst du hier seinen Ausgang 'raus' auf dieses Feld legen.").weak());
            }
        }
    }

    // -----------------------------------------------------------------------
    // Fenster und Rückfragen
    // -----------------------------------------------------------------------

    fn fenster(&mut self, ctx: &egui::Context) {
        if self.zeige_probleme {
            let mut offen = true;
            let mut springen = None;
            egui::Window::new("Prüfung").open(&mut offen).default_width(480.0).show(ctx, |ui| {
                if self.probleme.is_empty() {
                    ui.colored_label(Color32::from_rgb(120, 210, 120), "Keine Probleme gefunden.");
                }
                egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                    for p in &self.probleme {
                        let (zeichen, f) = if p.fehler {
                            ("✖", Color32::from_rgb(240, 110, 110))
                        } else {
                            ("⚠", Color32::from_rgb(230, 190, 90))
                        };
                        let r = ui.add(egui::Button::new(egui::RichText::new(format!("{zeichen} {}", p.text)).color(f)).frame(false));
                        if r.clicked() {
                            springen = p.raum.clone();
                        }
                    }
                });
            });
            self.zeige_probleme = offen;
            if let Some(id) = springen {
                self.hinspringen(&id);
            }
        }

        if let Some(text) = self.engine_ausgabe.clone() {
            let mut offen = true;
            egui::Window::new("Prüfung durch rmengine").open(&mut offen).default_width(560.0).show(ctx, |ui| {
                if self.geaendert() {
                    ui.colored_label(Color32::from_rgb(230, 190, 90), "Hinweis: Es gibt ungespeicherte Änderungen. Geprüft wird der gespeicherte Stand.");
                }
                egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                    ui.add(egui::Label::new(egui::RichText::new(text).monospace()).wrap());
                });
            });
            if !offen {
                self.engine_ausgabe = None;
            }
        }

        if self.zeige_hilfe {
            egui::Window::new("Hilfe").open(&mut self.zeige_hilfe).default_width(420.0).show(ctx, |ui| {
                ui.label(HILFE);
            });
        }

        if let Some(id) = self.loeschen_fragen.clone() {
            let mut antwort = None;
            egui::Modal::new(egui::Id::unique("loeschen")).show(ctx, |ui| {
                ui.heading("Raum löschen?");
                ui.label(format!("'{id}' und alle Ausgänge dorthin werden entfernt. (Rückgängig mit Strg+Z)"));
                ui.horizontal(|ui| {
                    if ui.button("Löschen").clicked() {
                        antwort = Some(true);
                    }
                    if ui.button("Abbrechen").clicked() || ui.input(|i| i.key_pressed(Key::Escape)) {
                        antwort = Some(false);
                    }
                });
            });
            match antwort {
                Some(true) => {
                    self.loeschen_fragen = None;
                    self.loeschen(&id);
                }
                Some(false) => self.loeschen_fragen = None,
                None => {}
            }
        }

        if self.neu_laden_fragen {
            egui::Modal::new(egui::Id::unique("neuladen")).show(ctx, |ui| {
                ui.heading("Neu laden?");
                ui.label("Ungespeicherte Änderungen gehen verloren.");
                ui.horizontal(|ui| {
                    if ui.button("Verwerfen und neu laden").clicked() {
                        self.neu_laden_fragen = false;
                        self.laden();
                    }
                    if ui.button("Abbrechen").clicked() {
                        self.neu_laden_fragen = false;
                    }
                });
            });
        }

        if self.schliessen_fragen {
            egui::Modal::new(egui::Id::unique("schliessen")).show(ctx, |ui| {
                ui.heading("Ungespeicherte Änderungen");
                ui.label("Möchtest du vor dem Beenden speichern?");
                ui.horizontal(|ui| {
                    if ui.button("💾 Speichern und beenden").clicked() {
                        self.speichern();
                        if !self.geaendert() {
                            self.darf_schliessen = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        self.schliessen_fragen = false;
                    }
                    if ui.button("Verwerfen").clicked() {
                        self.darf_schliessen = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if ui.button("Abbrechen").clicked() {
                        self.schliessen_fragen = false;
                    }
                });
            });
        }
    }
}

fn kuerzen(t: &str, max: usize) -> String {
    if t.chars().count() <= max {
        t.to_string()
    } else {
        let mut s: String = t.chars().take(max.saturating_sub(1)).collect();
        s.push('…');
        s
    }
}

/// Eine Liste von IDs bearbeiten (Gegenstände, Monster …).
fn liste_bearbeiten(
    ui: &mut egui::Ui,
    titel: &str,
    salt: &str,
    liste: &mut Vec<String>,
    bekannt: &std::collections::BTreeSet<String>,
    doppelt: bool,
) {
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new(titel).strong());
        let mut weg = None;
        for (i, e) in liste.iter().enumerate() {
            let f = if bekannt.contains(e) { Color32::from_rgb(200, 200, 200) } else { Color32::from_rgb(240, 110, 110) };
            if ui.add(egui::Button::new(egui::RichText::new(format!("{e} ✖")).color(f)).small()).on_hover_text("Entfernen").clicked() {
                weg = Some(i);
            }
        }
        if let Some(i) = weg {
            liste.remove(i);
        }
        ui.menu_button("➕", |ui| {
            ui.set_max_height(300.0);
            egui::ScrollArea::vertical().id_salt(salt).show(ui, |ui| {
                for b in bekannt {
                    if (doppelt || !liste.contains(b)) && ui.button(b).clicked() {
                        liste.push(b.clone());
                        ui.close();
                    }
                }
                if bekannt.is_empty() {
                    ui.label("(keine vorhanden)");
                }
            });
        });
    });
}

pub fn ordner_suchen(arg: Option<String>) -> PathBuf {
    if let Some(a) = arg {
        return PathBuf::from(a);
    }
    let hier = PathBuf::from("daten");
    if hier.join("welt.toml").exists() {
        return hier;
    }
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|o| o.join("daten")))
        .filter(|p: &PathBuf| Path::new(p).join("welt.toml").exists())
        .unwrap_or(hier)
}

const HILFE: &str = "\
Karte
  • Klick auf einen Raum: auswählen
  • Raum ziehen: verschieben
  • Shift + Raum ziehen: Ausgang zu einem anderen Raum anlegen
    (auf ein leeres Feld loslassen = neuen Raum anlegen und verbinden)
  • Doppelklick auf ein leeres Feld: neuen Raum anlegen
    (neben dem ausgewählten Raum wird er automatisch verbunden)
  • Rechtsklick: Menü (Startraum, Verbindung trennen, Löschen)
  • Leere Fläche ziehen oder rechte/mittlere Maustaste: Karte verschieben
  • Mausrad: zoomen
  • Bild ↑ / Bild ↓: Ebene wechseln

Linien
  grau = Weg in beide Richtungen
  gelb mit Pfeil = Einbahnstraße
  orange gestrichelt = Ziel liegt nicht im Nachbarfeld dieser Richtung
                       (die Engine zeichnet die Verbindung dann nicht)
  ★ = Startraum, ✚ = Wiederbelebungsort (Rechtsklick zum Festlegen)
  ⏶ ⏷ = Ausgänge hoch / runter
  🚪 = rein / raus (z.B. in ein Haus mit eigener Karte)
  🌍 = raus auf die Oberwelt (aus oberwelt.toml)
  🔗 = Ausgang zu einer anderen Ebene oder Karte

Oberwelt
  Links unter „Karte“ die 🌍 Oberwelt wählen. Sie zeigt karte.txt mit allen
  Feldern, Eingängen (gelb), Orten (blau) und Feldern, auf die
  Ausgänge von Räumen zeigen (lila). Die Koordinate unter der Maus
  steht im Lineal und in der Statuszeile (x = Spalte, y = Zeile, ab 1).
  Ein Klick auf ein Feld zeigt rechts die Details: ow:x:y kopieren oder als
  Ausgang 'raus' des zuletzt gewählten Raums setzen.

Tasten
  Strg+S speichern, Strg+Z rückgängig, Strg+Y wiederholen,
  Entf löschen, Esc Auswahl aufheben

Beim Speichern bleiben Kommentare, Details und alles, was der Editor
nicht kennt, in den TOML-Dateien erhalten.";
