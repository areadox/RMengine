//! Die Fenster "🌐 Server" (Fernwartung) und "⟳ Updates" des Editors.
//!
//! Alles, was übers Netz geht, läuft in einem eigenen Thread, damit der
//! Editor nicht einfriert. Das Ergebnis wird in jedem Bild abgeholt.

use crate::fernwartung::{self as fw, Arbeitskopie, Hochgeladen, VerbindungsFehler};
use crate::fernwartung_protokoll::Anfrage;
use crate::update;
use eframe::egui::{self, Color32, RichText};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const GRUEN: Color32 = Color32::from_rgb(120, 210, 120);
const ROT: Color32 = Color32::from_rgb(240, 110, 110);
const GELB: Color32 = Color32::from_rgb(230, 190, 90);
const GRAU: Color32 = Color32::from_rgb(150, 150, 150);

type Postfach<T> = Arc<Mutex<Option<T>>>;

fn im_hintergrund<T: Send + 'static>(ctx: &egui::Context, arbeit: impl FnOnce() -> T + Send + 'static) -> Postfach<T> {
    let fach: Postfach<T> = Arc::new(Mutex::new(None));
    let f2 = fach.clone();
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let e = arbeit();
        *f2.lock().unwrap() = Some(e);
        ctx.request_repaint();
    });
    fach
}

fn abholen<T>(fach: &Option<Postfach<T>>) -> Option<T> {
    fach.as_ref().and_then(|f| f.lock().unwrap().take())
}

/// Was der Editor nach einer Aktion im Fenster tun soll.
pub enum Aktion {
    /// Diesen Datenordner öffnen (nach dem Holen)
    Oeffnen(PathBuf),
    /// Erst speichern (vor dem Hochladen)
    Speichern,
}

enum Ergebnis {
    Fingerabdruck(Result<(), VerbindungsFehler>),
    Geholt(Result<String, VerbindungsFehler>, PathBuf),
    Hochgeladen(Result<Hochgeladen, VerbindungsFehler>),
    Bericht(Result<(bool, String), VerbindungsFehler>),
}

// ---------------------------------------------------------------------------
// Server (Fernwartung)
// ---------------------------------------------------------------------------

pub struct ServerFenster {
    pub offen: bool,
    adresse: String,
    pub schluessel: String,
    merken: bool,
    ordner: String,
    neuladen: bool,
    laeuft: Option<(String, Postfach<Ergebnis>)>,
    ausgabe: Option<(bool, String)>,
    fingerabdruck_frage: Option<String>,
    konflikt: Option<Vec<String>>,
    nicht_uebernommen: bool,
    holen_bestaetigen: Option<usize>,
    hochladen_nach_speichern: bool,
    /// Formular für einen anderen Server zeigen, obwohl gerade eine Arbeitskopie offen ist
    anderer: bool,
}

impl Default for ServerFenster {
    fn default() -> Self {
        ServerFenster {
            offen: false,
            adresse: String::new(),
            schluessel: String::new(),
            merken: true,
            ordner: String::new(),
            neuladen: true,
            laeuft: None,
            ausgabe: None,
            fingerabdruck_frage: None,
            konflikt: None,
            nicht_uebernommen: false,
            holen_bestaetigen: None,
            hochladen_nach_speichern: false,
            anderer: false,
        }
    }
}

impl ServerFenster {
    /// Die Arbeitskopie, in der der Editor gerade arbeitet (falls es eine ist).
    pub fn verbindung(&self, daten: &Path) -> Option<(PathBuf, Arbeitskopie, String)> {
        let (ordner, kopie) = Arbeitskopie::zu_datenordner(daten)?;
        let schluessel = if self.schluessel.trim().is_empty() { kopie.schluessel.clone().unwrap_or_default() } else { self.schluessel.clone() };
        Some((ordner, kopie, schluessel))
    }

    fn starten(&mut self, ctx: &egui::Context, was: &str, arbeit: impl FnOnce() -> Ergebnis + Send + 'static) {
        self.ausgabe = None;
        self.konflikt = None;
        self.nicht_uebernommen = false;
        self.laeuft = Some((was.to_string(), im_hintergrund(ctx, arbeit)));
    }

    fn hochladen_starten(&mut self, ctx: &egui::Context, ordner: PathBuf, schluessel: String, trotz_fehlern: bool, ueberschreiben: bool) {
        let neuladen = self.neuladen;
        self.starten(ctx, "Lade hoch und lasse den Server prüfen …", move || {
            Ergebnis::Hochgeladen(fw::hochladen(&ordner, &schluessel, neuladen, trotz_fehlern, ueberschreiben))
        });
    }

    pub fn zeichnen(&mut self, ctx: &egui::Context, daten: &Path, ungespeichert: bool) -> Option<Aktion> {
        let mut aktion = None;

        // Ergebnis der Hintergrundarbeit abholen
        if let Some((_, fach)) = &self.laeuft {
            let fertig = fach.lock().unwrap().take();
            if let Some(e) = fertig {
                self.laeuft = None;
                match e {
                    Ergebnis::Fingerabdruck(Err(VerbindungsFehler::NeuerFingerabdruck(fp))) => self.fingerabdruck_frage = Some(fp),
                    Ergebnis::Fingerabdruck(Err(f)) => self.ausgabe = Some((false, fw::fehlertext(&f))),
                    Ergebnis::Fingerabdruck(Ok(())) => self.ausgabe = Some((false, "Unerwartet: Der Server hat ohne Prüfung angenommen.".into())),
                    Ergebnis::Geholt(Ok(t), pfad) => {
                        self.anderer = false;
                        self.ausgabe = Some((true, format!("{t}\nDu arbeitest jetzt in der Arbeitskopie:\n{}", pfad.display())));
                        aktion = Some(Aktion::Oeffnen(pfad.join("daten")));
                    }
                    Ergebnis::Geholt(Err(f), _) | Ergebnis::Bericht(Err(f)) | Ergebnis::Hochgeladen(Err(f)) => {
                        self.ausgabe = Some((false, fw::fehlertext(&f)))
                    }
                    Ergebnis::Hochgeladen(Ok(Hochgeladen::Konflikt(p))) => self.konflikt = Some(p),
                    Ergebnis::Hochgeladen(Ok(Hochgeladen::Fertig { ok, text })) => {
                        self.nicht_uebernommen = text.starts_with("NICHT");
                        self.ausgabe = Some((ok, text));
                    }
                    Ergebnis::Bericht(Ok(b)) => self.ausgabe = Some(b),
                }
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(200));
            }
        }

        let verbunden = if self.anderer { None } else { self.verbindung(daten) };

        // Nach dem Speichern automatisch hochladen
        if self.hochladen_nach_speichern && !ungespeichert && self.laeuft.is_none() {
            self.hochladen_nach_speichern = false;
            if let Some((ordner, _, schluessel)) = verbunden.clone() {
                self.hochladen_starten(ctx, ordner, schluessel, false, false);
            }
        }

        let mut offen = self.offen;
        egui::Window::new("🌐 Server (Fernwartung)").open(&mut offen).default_width(560.0).show(ctx, |ui| {
            match &verbunden {
                Some((ordner, kopie, schluessel)) => {
                    ui.label(RichText::new(format!("Verbunden mit {}", kopie.adresse)).strong());
                    ui.label(RichText::new(format!("Arbeitskopie: {}", ordner.display())).color(GRAU).small());
                    ui.label(RichText::new(format!("Fingerabdruck: {}…", kopie.fingerabdruck.chars().take(23).collect::<String>())).color(GRAU).small());
                    ui.add_space(6.0);
                    let aenderungen = fw::lokale_aenderungen(ordner, kopie);
                    if aenderungen.is_empty() && !ungespeichert {
                        ui.label(RichText::new("Keine Änderungen seit dem letzten Holen/Hochladen.").color(GRUEN));
                    } else {
                        let mut t = format!("{} geänderte Datei(en) zum Hochladen", aenderungen.len());
                        if ungespeichert {
                            t.push_str(" – dazu ungespeicherte Änderungen im Editor (werden vorher gespeichert)");
                        }
                        ui.label(RichText::new(t).color(GELB));
                    }
                    ui.horizontal(|ui| {
                        ui.label("Zugangsschlüssel:");
                        let mut s = if self.schluessel.is_empty() { schluessel.clone() } else { self.schluessel.clone() };
                        if ui.add(egui::TextEdit::singleline(&mut s).password(true).desired_width(260.0)).changed() {
                            self.schluessel = s;
                        }
                    });
                    ui.checkbox(&mut self.neuladen, "Nach dem Hochladen im laufenden Spiel neu laden");
                    ui.add_space(4.0);
                    let frei = self.laeuft.is_none();
                    ui.horizontal(|ui| {
                        if ui.add_enabled(frei, egui::Button::new("⬆ Hochladen")).on_hover_text("Geänderte Dateien auf den Server übertragen. Der Server prüft sie vorher.").clicked() {
                            if ungespeichert {
                                self.hochladen_nach_speichern = true;
                                aktion = Some(Aktion::Speichern);
                            } else {
                                self.hochladen_starten(ctx, ordner.clone(), schluessel.clone(), false, false);
                            }
                        }
                        if ui.add_enabled(frei, egui::Button::new("⬇ Neu vom Server holen")).clicked() {
                            if aenderungen.is_empty() && !ungespeichert {
                                let (o, a, fp, s, m) = (ordner.clone(), kopie.adresse.clone(), kopie.fingerabdruck.clone(), schluessel.clone(), kopie.schluessel.is_some());
                                self.starten(ctx, "Hole die Welt vom Server …", move || Ergebnis::Geholt(fw::holen(&o, &a, &fp, &s, m), o.clone()));
                            } else {
                                self.holen_bestaetigen = Some(aenderungen.len());
                            }
                        }
                        if ui.add_enabled(frei, egui::Button::new("Auf dem Server prüfen")).clicked() {
                            let (k, s) = (kopie.clone(), schluessel.clone());
                            self.starten(ctx, "Der Server prüft die Welt …", move || Ergebnis::Bericht(fw::einfach(&k, &s, Anfrage::Pruefen)));
                        }
                        if ui.add_enabled(frei, egui::Button::new("Im Spiel neu laden")).clicked() {
                            let (k, s) = (kopie.clone(), schluessel.clone());
                            self.starten(ctx, "Der Server lädt die Welt neu …", move || Ergebnis::Bericht(fw::einfach(&k, &s, Anfrage::Neuladen)));
                        }
                    });
                    if let Some(n) = self.holen_bestaetigen {
                        ui.add_space(6.0);
                        ui.label(RichText::new(format!("Achtung: {n} lokale Änderung(en) sind noch nicht hochgeladen und gehen beim Holen verloren.")).color(ROT));
                        ui.horizontal(|ui| {
                            if ui.button("Trotzdem holen").clicked() {
                                self.holen_bestaetigen = None;
                                let (o, a, fp, s, m) = (ordner.clone(), kopie.adresse.clone(), kopie.fingerabdruck.clone(), schluessel.clone(), kopie.schluessel.is_some());
                                self.starten(ctx, "Hole die Welt vom Server …", move || Ergebnis::Geholt(fw::holen(&o, &a, &fp, &s, m), o.clone()));
                            }
                            if ui.button("Abbrechen").clicked() {
                                self.holen_bestaetigen = None;
                            }
                        });
                    }
                    if let Some(pfade) = self.konflikt.clone() {
                        ui.add_space(6.0);
                        ui.label(RichText::new("Auf dem Server wurden inzwischen dieselben Dateien geändert:").color(ROT));
                        for p in &pfade {
                            ui.label(format!("  • {p}"));
                        }
                        ui.label("Hochladen würde diese Änderungen auf dem Server überschreiben.");
                        ui.horizontal(|ui| {
                            if ui.button("Trotzdem überschreiben").clicked() {
                                self.hochladen_starten(ctx, ordner.clone(), schluessel.clone(), false, true);
                            }
                            if ui.button("Abbrechen").clicked() {
                                self.konflikt = None;
                            }
                        });
                    }
                    if self.nicht_uebernommen {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Die Welt auf dem Server bleibt unverändert.").color(GRAU));
                            if ui.button("Trotz Fehlern übernehmen …").on_hover_text("Nicht empfohlen: Beim nächsten Neustart lädt der Server die Welt dann nicht.").clicked() {
                                self.hochladen_starten(ctx, ordner.clone(), schluessel.clone(), true, false);
                            }
                        });
                    }
                    ui.add_space(6.0);
                    ui.separator();
                    if ui.small_button("Mit einem anderen Server verbinden …").clicked() {
                        self.adresse.clear();
                        self.ordner.clear();
                        self.schluessel.clear();
                        self.ausgabe = None;
                        self.anderer = true;
                    }
                }
                None => {
                    ui.label(RichText::new("Mit einem Server verbinden").strong());
                    ui.label(RichText::new("Auf dem Server einmal 'rmengine --fernwartung' ausführen. Das zeigt den Zugangsschlüssel und den Fingerabdruck an.").color(GRAU).small());
                    ui.add_space(6.0);
                    egui::Grid::new("server_grid").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                        ui.label("Adresse:");
                        if ui.add(egui::TextEdit::singleline(&mut self.adresse).hint_text("meinserver.de oder 1.2.3.4 (Port 4001)").desired_width(320.0)).changed() && !self.adresse.is_empty() {
                            self.ordner = fw::standard_ordner(&self.adresse).display().to_string();
                        }
                        ui.end_row();
                        ui.label("Zugangsschlüssel:");
                        ui.add(egui::TextEdit::singleline(&mut self.schluessel).password(true).hint_text("XXXXX-XXXXX-…").desired_width(320.0));
                        ui.end_row();
                        ui.label("Arbeitskopie:");
                        ui.add(egui::TextEdit::singleline(&mut self.ordner).desired_width(320.0));
                        ui.end_row();
                    });
                    ui.checkbox(&mut self.merken, "Schlüssel auf diesem PC merken");
                    ui.add_space(4.0);
                    let bereit = self.laeuft.is_none() && !self.adresse.trim().is_empty() && !self.schluessel.trim().is_empty() && !self.ordner.trim().is_empty();
                    if ui.add_enabled(bereit, egui::Button::new("Verbinden und Welt holen")).clicked() {
                        let ordner = PathBuf::from(self.ordner.trim());
                        // Diesen Server schon einmal benutzt? Dann den gemerkten Fingerabdruck nehmen.
                        let bekannt = std::fs::read_to_string(Arbeitskopie::datei(&ordner))
                            .ok()
                            .and_then(|t| serde_json::from_str::<Arbeitskopie>(&t).ok())
                            .filter(|k| k.adresse == self.adresse.trim());
                        let (a, s, m) = (self.adresse.trim().to_string(), self.schluessel.trim().to_string(), self.merken);
                        match bekannt {
                            Some(k) => self.starten(ctx, "Hole die Welt vom Server …", move || Ergebnis::Geholt(fw::holen(&ordner, &a, &k.fingerabdruck, &s, m), ordner.clone())),
                            None => self.starten(ctx, "Verbinde …", move || Ergebnis::Fingerabdruck(fw::Sitzung::oeffnen(&a, None, "").map(|_| ()))),
                        }
                    }
                    if let Some(fp) = self.fingerabdruck_frage.clone() {
                        ui.add_space(8.0);
                        ui.label(RichText::new("Erste Verbindung zu diesem Server").strong());
                        ui.label("Vergleiche den Fingerabdruck mit dem, den 'rmengine --fernwartung' auf dem Server angezeigt hat:");
                        ui.label(RichText::new(&fp).monospace().color(GELB));
                        ui.horizontal(|ui| {
                            if ui.button("Stimmt überein – verbinden").clicked() {
                                self.fingerabdruck_frage = None;
                                let ordner = PathBuf::from(self.ordner.trim());
                                let (a, s, m) = (self.adresse.trim().to_string(), self.schluessel.trim().to_string(), self.merken);
                                self.starten(ctx, "Hole die Welt vom Server …", move || Ergebnis::Geholt(fw::holen(&ordner, &a, &fp, &s, m), ordner.clone()));
                            }
                            if ui.button("Abbrechen").clicked() {
                                self.fingerabdruck_frage = None;
                            }
                        });
                    }
                }
            }
            if let Some((was, _)) = &self.laeuft {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(was);
                });
            }
            if let Some((ok, text)) = &self.ausgabe {
                ui.add_space(6.0);
                ui.separator();
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    ui.label(RichText::new(text).color(if *ok { GRUEN } else { ROT }));
                });
            }
        });
        self.offen = offen;
        aktion
    }
}

// ---------------------------------------------------------------------------
// Updates
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct UpdateFenster {
    pub offen: bool,
    suche: Option<Postfach<Result<update::Release, String>>>,
    release: Option<Result<update::Release, String>>,
    engine_lokal: bool,
    /// Version der Engine neben dem Editor (None = keine da oder unbekannt)
    engine_version: Option<String>,
    engine_server: bool,
    laeuft: Option<Postfach<(bool, String)>>,
    ausgabe: Option<(bool, String)>,
    neustart_noetig: bool,
}

impl UpdateFenster {
    pub fn oeffnen(&mut self, ctx: &egui::Context) {
        self.offen = true;
        self.ausgabe = None;
        self.release = None;
        self.engine_version = update::programm_pfad(update::ENGINE)
            .filter(|p| p.exists())
            .and_then(|p| std::process::Command::new(p).arg("--version").output().ok())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().trim_start_matches("RMengine").trim().to_string());
        self.engine_lokal = self.engine_version.is_some();
        self.engine_server = true;
        self.suche = Some(im_hintergrund(ctx, || update::neueste_version(std::time::Duration::from_secs(20))));
    }

    /// `server`: Arbeitskopie und Schlüssel, wenn der Editor mit einem Server arbeitet.
    /// Gibt true zurück, wenn der Editor neu gestartet werden soll.
    pub fn zeichnen(&mut self, ctx: &egui::Context, server: Option<(Arbeitskopie, String)>, ungespeichert: bool) -> bool {
        if let Some(r) = abholen(&self.suche) {
            self.suche = None;
            self.release = Some(r);
        }
        if let Some(e) = abholen(&self.laeuft) {
            self.laeuft = None;
            self.neustart_noetig = e.0 && e.1.contains("rmeditor aktualisiert");
            self.ausgabe = Some(e);
        }
        if self.suche.is_some() || self.laeuft.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
        let mut neu_starten = false;
        let mut offen = self.offen;
        egui::Window::new("⟳ Updates").open(&mut offen).default_width(460.0).show(ctx, |ui| {
            ui.label(format!("Installiert: Version {}", update::VERSION));
            ui.add_space(4.0);
            let mut engine_alt = false;
            let neuer = match &self.release {
                None => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Suche nach Updates …");
                    });
                    false
                }
                Some(Err(e)) => {
                    ui.label(RichText::new(e).color(ROT));
                    false
                }
                Some(Ok(r)) => {
                    let neuer = update::ist_neuer(&r.version, update::VERSION);
                    engine_alt = self.engine_version.as_ref().is_some_and(|v| update::ist_neuer(&r.version, v));
                    if neuer {
                        ui.label(RichText::new(format!("Neue Version {} verfügbar!", r.version)).strong().color(GELB));
                        if let Some(n) = update::neuerungen(&r.notizen) {
                            ui.label(RichText::new(n).small());
                        }
                    } else {
                        ui.label(RichText::new(format!("Der Editor ist auf dem neuesten Stand ({}).", r.version)).color(GRUEN));
                    }
                    neuer
                }
            };
            ui.add_space(6.0);
            ui.label(RichText::new("Was soll aktualisiert werden?").strong());
            ui.add_enabled(false, egui::Checkbox::new(&mut neuer.clone(), "Welteditor (dieses Programm)"));
            ui.add_enabled_ui(self.engine_version.is_some(), |ui| {
                let text = match &self.engine_version {
                    Some(v) if engine_alt => format!("Engine auf diesem PC (Version {v} – veraltet)"),
                    Some(v) => format!("Engine auf diesem PC (Version {v})"),
                    None => "Engine auf diesem PC".to_string(),
                };
                ui.checkbox(&mut self.engine_lokal, text).on_disabled_hover_text("Neben dem Editor liegt keine Engine (rmengine).");
            });
            let server_moeglich = server.as_ref().is_some_and(|(_, s)| !s.trim().is_empty());
            ui.add_enabled_ui(server_moeglich, |ui| {
                let text = match &server {
                    Some((k, _)) => format!("Engine auf dem Server ({})", k.adresse),
                    None => "Engine auf dem Server".to_string(),
                };
                ui.checkbox(&mut self.engine_server, text).on_disabled_hover_text(
                    "Nur möglich, wenn du über 🌐 Server mit einer Fernwartung arbeitest (und der Schlüssel eingetragen ist).",
                );
                ui.label(RichText::new("Der Server speichert dabei alle Spieler und startet mit der neuen Version neu.").color(GRAU).small());
            });
            if ungespeichert {
                ui.label(RichText::new("Bitte erst speichern – nach dem Update startet der Editor neu.").color(GELB));
            }
            ui.add_space(6.0);
            let was_zu_tun = neuer || (engine_alt && self.engine_lokal) || (server_moeglich && self.engine_server);
            if ui.add_enabled(was_zu_tun && self.laeuft.is_none() && !ungespeichert && self.release.as_ref().is_some_and(|r| r.is_ok()), egui::Button::new("Jetzt aktualisieren")).clicked() {
                let release = match &self.release {
                    Some(Ok(r)) => r.clone(),
                    _ => return,
                };
                let mut lokal = Vec::new();
                if neuer {
                    lokal.push(update::EDITOR);
                }
                if self.engine_lokal && (engine_alt || neuer) {
                    lokal.push(update::ENGINE);
                }
                let server = if self.engine_server && server_moeglich { server.clone() } else { None };
                self.laeuft = Some(im_hintergrund(ctx, move || {
                    let mut log = Vec::new();
                    let mut ok = true;
                    if !lokal.is_empty() {
                        if let Err(e) = update::programme_aktualisieren(&release, &lokal, &mut |t| log.push(t)) {
                            ok = false;
                            log.push(format!("Fehler: {e}"));
                        }
                    }
                    if let Some((kopie, schluessel)) = server {
                        log.push(format!("Server {}:", kopie.adresse));
                        match fw::einfach(&kopie, &schluessel, Anfrage::Aktualisieren) {
                            Ok((o, t)) => {
                                ok &= o;
                                log.push(t);
                            }
                            Err(f) => {
                                ok = false;
                                log.push(fw::fehlertext(&f));
                            }
                        }
                    }
                    (ok, log.join("\n"))
                }));
            }
            if self.laeuft.is_some() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Aktualisiere …");
                });
            }
            if let Some((ok, text)) = &self.ausgabe {
                ui.separator();
                ui.label(RichText::new(text).color(if *ok { GRUEN } else { ROT }));
                if self.neustart_noetig && ui.button("Editor jetzt neu starten").clicked() {
                    neu_starten = true;
                }
            }
        });
        self.offen = offen;
        neu_starten
    }
}
