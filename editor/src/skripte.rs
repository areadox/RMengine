//! Bereich "Skripte": Lua-Dateien in daten/skripte bearbeiten und prüfen.

use eframe::egui::{self, Color32, RichText};
use std::fs;
use std::path::{Path, PathBuf};

const VORSPANN: &str = include_str!("../../src/skripte_vorspann.lua");
const GRAU: Color32 = Color32::from_rgb(150, 150, 150);
const ROT: Color32 = Color32::from_rgb(240, 110, 110);
const GRUEN: Color32 = Color32::from_rgb(120, 200, 120);

const NEUES_SKRIPT: &str = r#"-- Ein neues Skript. Beispiel: ein Befehl, den es nur in einem Raum gibt.
befehl{
  woerter = {"klopf", "klopfe"},
  raum    = "dorfplatz",          -- ohne raum: überall
  syntax  = "klopf an den brunnen",
  hilfe   = "An den Brunnen klopfen",
  funktion = function(s, rest, raum)
    if not rest:find("brunnen") then
      return false                 -- nicht gemeint: dann gelten die normalen Befehle
    end
    s:sende("Du klopfst an den Brunnen. Tief unten hallt es hohl.")
    s:andere(s.name .. " klopft an den Brunnen.")
  end,
}
"#;

pub struct Datei {
    /// relativ zum Datenordner, z.B. "skripte/taverne.lua"
    pub rel: String,
    pub text: String,
    gespeichert: String,
}

pub struct Skriptwerk {
    ordner: PathBuf,
    pub dateien: Vec<Datei>,
    auswahl: Option<usize>,
    neu_name: String,
    /// Ergebnis der letzten Prüfung: (ok, Meldungen)
    pruefung: Option<(bool, Vec<String>)>,
    geloescht: Vec<String>,
}

impl Skriptwerk {
    pub fn laden(ordner: &Path) -> Skriptwerk {
        let mut dateien = Vec::new();
        fn sammeln(basis: &Path, o: &Path, aus: &mut Vec<Datei>) {
            let Ok(e) = fs::read_dir(o) else { return };
            let mut e: Vec<_> = e.flatten().map(|e| e.path()).collect();
            e.sort();
            for p in e {
                if p.is_dir() {
                    sammeln(basis, &p, aus);
                } else if p.extension().is_some_and(|x| x == "lua") {
                    let text = fs::read_to_string(&p).unwrap_or_default();
                    let rel = p.strip_prefix(basis).unwrap_or(&p).display().to_string().replace('\\', "/");
                    aus.push(Datei { rel, gespeichert: text.clone(), text });
                }
            }
        }
        sammeln(ordner, &ordner.join("skripte"), &mut dateien);
        Skriptwerk { ordner: ordner.to_path_buf(), dateien, auswahl: None, neu_name: String::new(), pruefung: None, geloescht: Vec::new() }
    }

    pub fn geaendert(&self) -> bool {
        !self.geloescht.is_empty() || self.dateien.iter().any(|d| d.text != d.gespeichert)
    }

    pub fn speichern(&mut self) -> Result<Vec<String>, String> {
        let mut aus = Vec::new();
        for rel in self.geloescht.drain(..) {
            let _ = fs::remove_file(self.ordner.join(&rel));
            aus.push(rel);
        }
        for d in &mut self.dateien {
            if d.text != d.gespeichert {
                let pfad = self.ordner.join(&d.rel);
                if let Some(o) = pfad.parent() {
                    fs::create_dir_all(o).map_err(|e| e.to_string())?;
                }
                fs::write(&pfad, &d.text).map_err(|e| format!("{}: {e}", d.rel))?;
                d.gespeichert = d.text.clone();
                aus.push(d.rel.clone());
            }
        }
        if !aus.is_empty() {
            self.pruefung = Some(pruefen(&self.dateien));
        }
        Ok(aus)
    }

    /// Probleme für die Prüfung oben im Editor
    pub fn probleme(&self) -> Vec<String> {
        match &self.pruefung {
            Some((false, m)) => m.clone(),
            _ => Vec::new(),
        }
    }

    pub fn liste(&mut self, ui: &mut egui::Ui) {
        ui.heading("📜 Skripte");
        ui.label(RichText::new("Lua-Dateien in daten/skripte. Damit baust du Befehle und Ereignisse, die es nur in bestimmten Räumen gibt.").color(GRAU).small());
        ui.add_space(6.0);
        for (i, d) in self.dateien.iter().enumerate() {
            let name = d.rel.strip_prefix("skripte/").unwrap_or(&d.rel);
            let text = if d.text != d.gespeichert { format!("{name} *") } else { name.to_string() };
            if ui.selectable_label(self.auswahl == Some(i), text).clicked() {
                self.auswahl = Some(i);
            }
        }
        if self.dateien.is_empty() {
            ui.label(RichText::new("Noch keine Skripte.").color(GRAU));
        }
        ui.add_space(10.0);
        ui.separator();
        ui.label("Neues Skript:");
        ui.horizontal(|ui| {
            crate::kontext::feld(ui, egui::TextEdit::singleline(&mut self.neu_name).hint_text("z.B. taverne").desired_width(150.0));
            let name: String = self
                .neu_name
                .trim()
                .trim_end_matches(".lua")
                .chars()
                .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c.to_ascii_lowercase() } else { '_' })
                .collect();
            let rel = format!("skripte/{name}.lua");
            let frei = !name.is_empty() && !self.dateien.iter().any(|d| d.rel == rel);
            if ui.add_enabled(frei, egui::Button::new("+ Anlegen")).clicked() {
                self.dateien.push(Datei { rel, text: NEUES_SKRIPT.to_string(), gespeichert: String::new() });
                self.auswahl = Some(self.dateien.len() - 1);
                self.neu_name.clear();
            }
        });
    }

    pub fn formular(&mut self, ui: &mut egui::Ui) {
        let Some(i) = self.auswahl.filter(|i| *i < self.dateien.len()) else {
            ui.add_space(12.0);
            ui.label(RichText::new("Wähle links ein Skript oder lege ein neues an.").strong());
            ui.add_space(8.0);
            spickzettel(ui);
            return;
        };
        ui.horizontal(|ui| {
            ui.heading(&self.dateien[i].rel);
            if ui.button("✔ Prüfen").on_hover_text("Findet Tippfehler, ohne die Engine zu starten").clicked() {
                self.pruefung = Some(pruefen(&self.dateien));
            }
            if ui.button("🗑 Löschen").on_hover_text("Wird beim Speichern gelöscht").clicked() {
                let d = self.dateien.remove(i);
                if !d.gespeichert.is_empty() {
                    self.geloescht.push(d.rel);
                }
                self.auswahl = None;
                return;
            }
        });
        if self.auswahl.is_none() {
            return;
        }
        if let Some((ok, meldungen)) = &self.pruefung {
            if *ok {
                ui.label(RichText::new("✔ Alle Skripte sind in Ordnung.").color(GRUEN));
            } else {
                for m in meldungen {
                    ui.label(RichText::new(m).color(ROT));
                }
            }
        }
        ui.add_space(4.0);
        egui::ScrollArea::vertical().id_salt("skript_text").max_height(ui.available_height() - 220.0).show(ui, |ui| {
            let d = &mut self.dateien[i];
            let antwort = crate::kontext::feld(
                ui,
                egui::TextEdit::multiline(&mut d.text).code_editor().desired_rows(28).desired_width(f32::INFINITY).lock_focus(true),
            );
            if antwort.changed() {
                self.pruefung = None;
            }
        });
        ui.add_space(6.0);
        egui::ScrollArea::vertical().id_salt("spick").show(ui, |ui| {
            egui::CollapsingHeader::new("Spickzettel: Was ein Skript kann").default_open(false).show(ui, spickzettel);
        });
    }
}

/// Alle Skripte so laden, wie es die Engine tut (ohne Welt) – findet Tipp- und Aufruffehler.
fn pruefen(dateien: &[Datei]) -> (bool, Vec<String>) {
    use mlua::{Lua, LuaOptions, StdLib, Table};
    let mut fehler = Vec::new();
    let lua = match Lua::new_with(StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::UTF8, LuaOptions::default()) {
        Ok(l) => l,
        Err(e) => return (false, vec![format!("Lua konnte nicht gestartet werden: {e}")]),
    };
    let _ = lua.set_memory_limit(64 * 1024 * 1024);
    if let Err(e) = lua.load(VORSPANN).set_name("vorspann").exec() {
        return (false, vec![format!("Vorspann: {e}")]);
    }
    for d in dateien {
        if let Ok(rm) = lua.globals().get::<Table>("_RM") {
            let _ = rm.set("datei", d.rel.as_str());
        }
        if let Err(e) = lua.load(&d.text).set_name(format!("={}", d.rel)).exec() {
            let t = match &e {
                mlua::Error::SyntaxError { message, .. } => message.clone(),
                mlua::Error::RuntimeError(m) => m.clone(),
                andere => andere.to_string(),
            };
            fehler.push(t.lines().next().unwrap_or_default().to_string());
        }
    }
    (fehler.is_empty(), fehler)
}

fn spickzettel(ui: &mut egui::Ui) {
    let zeilen: &[(&str, &str)] = &[
        ("befehl{ woerter = {...}, raum = \"id\", funktion = function(s, rest, raum) ... end }", "Ein Befehl (raum/gebiet weglassen = überall). return false = nicht gemeint."),
        ("bei_betreten(\"raum\", function(s, raum) ... end)", "Wenn jemand den Raum betritt (\"*\" = jeder Raum)"),
        ("bei_verlassen(\"raum\", function(s, raum) ... end)", "Wenn jemand den Raum verlässt"),
        ("bei_sagen(\"raum\", function(s, text, raum) ... end)", "Wenn jemand im Raum etwas sagt"),
        ("bei_sieg(\"monster\", function(s, monster, raum) ... end)", "Wenn jemand dieses Monster besiegt"),
        ("bei_takt(function() ... end)", "Alle paar Sekunden"),
        ("spaeter(sekunden, function() ... end)", "Etwas später ausführen"),
        ("s.name  s.raum  s.stufe  s.lp  s.max_lp  s.mp  s.geld", "Werte des Spielers"),
        ("s.rasse  s.gilde  s.berufe  s.inventar  s.angelegt  s.merker  s.fertigkeiten", ""),
        ("s:sende(text)   s:andere(text)", "An den Spieler / an alle anderen im Raum"),
        ("s:hat(id)   s:gib(id)   s:nimm(id)", "Gegenstände"),
        ("s:geld_aendern(n)   s:zahle(n)", "Geld (zahle gibt false, wenn es nicht reicht)"),
        ("s:heile(n)   s:schade(n)   s:mana(n)   s:erfahrung(n)", "Werte ändern"),
        ("s:gehe(raum, text_weg, text_an)", "In einen anderen Raum versetzen"),
        ("s:merke(name, wert)   s.merker.name", "Dauerhaft merken (z.B. \"schon gefunden\")"),
        ("s:befehl(\"schau\")   s:zauber(id)   s:fortschritt(ziel)", "Befehl ausführen, Zauberwirkung, Quest-Schritt art = \"skript\""),
        ("raum.id  raum.name  raum.gegenstaende  raum.monster  raum.spieler", "Der Raum"),
        ("welt.sende_raum(raum, text)   welt.sende_alle(text)", "Nachrichten"),
        ("welt.lege(raum, id)   welt.entferne(raum, id)   welt.erschaffe(raum, monster)", "Gegenstände und Monster"),
        ("welt.merke(name, wert)   welt.merker.name", "Für die ganze Welt merken"),
        ("welt.zeit.stunde  .tageszeit  .jahreszeit  .wetter", "Tag und Nacht (wenn eingeschaltet)"),
        ("math.random(1, 6)   string.find   print(...)", "Lua selbst; print schreibt in die Server-Konsole"),
    ];
    egui::Grid::new("spickzettel").num_columns(2).spacing([12.0, 4.0]).striped(true).show(ui, |ui| {
        for (code, text) in zeilen {
            ui.label(RichText::new(*code).monospace());
            ui.label(RichText::new(*text).color(GRAU));
            ui.end_row();
        }
    });
    ui.label(RichText::new("Mehr in der ANLEITUNG, Kapitel „Skripte“.").color(GRAU));
}
