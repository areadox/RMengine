//! Der Bereich "Befehle" des Editors (Inhalte wie Zauber stehen in inhalte.rs).
//!
//! Felder, die aus einer Vorlage geerbt werden, sind grau. Wer einen eigenen
//! Wert einträgt, überschreibt die Vorlage. Leer lassen = erben.

use crate::befehle::{Befehl, Befehlswerk, Geste};
use crate::befehlsliste::{self, KATEGORIEN, ROLLEN};
use eframe::egui::{self, Color32, RichText};

const GRAU: Color32 = Color32::from_rgb(140, 140, 140);
const GEAENDERT: Color32 = Color32::from_rgb(230, 190, 90);

// ---------------------------------------------------------------------------
// Kleine Bausteine
// ---------------------------------------------------------------------------

/// Eine Wortliste als Text "a, b, c" bearbeiten (der Puffer lebt im egui-Speicher,
/// damit Kommas beim Tippen nicht verschwinden).
fn wortliste(ui: &mut egui::Ui, salt: impl std::hash::Hash + std::fmt::Debug, liste: &mut Vec<String>, hinweis: &str) -> bool {
    let id = ui.make_persistent_id(salt);
    let verbunden = liste.join(", ");
    let mut puffer: String = ui.data_mut(|d| d.get_temp::<String>(id)).unwrap_or_else(|| verbunden.clone());
    let zerlegen = |t: &str| -> Vec<String> { t.split(',').map(|w| w.trim().to_string()).filter(|w| !w.is_empty()).collect() };
    if zerlegen(&puffer) != *liste {
        puffer = verbunden;
    }
    let antwort = crate::kontext::feld(ui, egui::TextEdit::singleline(&mut puffer).hint_text(hinweis).desired_width(f32::INFINITY));
    ui.data_mut(|d| d.insert_temp(id, puffer.clone()));
    if antwort.changed() {
        let neu = zerlegen(&puffer);
        if neu != *liste {
            *liste = neu;
            return true;
        }
    }
    false
}

fn zeile(ui: &mut egui::Ui, beschriftung: &str, inhalt: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal_top(|ui| {
        ui.add_sized([110.0, 20.0], egui::Label::new(beschriftung));
        ui.vertical(|ui| inhalt(ui));
    });
}

// ---------------------------------------------------------------------------
// Befehle und Gesten
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
pub enum BefehlsAuswahl {
    Befehl(&'static str),
    Geste(usize),
}

pub fn befehle_liste(ui: &mut egui::Ui, werk: &mut Befehlswerk, auswahl: &mut Option<BefehlsAuswahl>, suche: &mut String) {
    ui.add_space(4.0);
    if ui.button("➕ Neue Geste").clicked() {
        let mut g = Geste::neu(&werk.standard_datei(true));
        g.woerter = vec!["neue_geste".into()];
        g.selbst = "Du tust etwas.".into();
        g.andere = "{name} tut etwas.".into();
        werk.gesten.push(g);
        *auswahl = Some(BefehlsAuswahl::Geste(werk.gesten.len() - 1));
    }
    crate::kontext::feld(ui, egui::TextEdit::singleline(suche).hint_text("🔍 Suchen …"));
    ui.separator();
    let such = suche.to_lowercase();
    let passt = |woerter: &[String], extra: &str| {
        such.is_empty() || woerter.iter().any(|w| w.to_lowercase().contains(&such)) || extra.to_lowercase().contains(&such)
    };
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.label(RichText::new("Gesten").strong());
        for (n, g) in werk.gesten.iter().enumerate() {
            if !passt(&g.woerter, &g.hilfe) {
                continue;
            }
            let name = g.woerter.first().cloned().unwrap_or_else(|| "(ohne Wort)".into());
            let text = if g.aktiv { RichText::new(name) } else { RichText::new(format!("{name} (aus)")).color(GRAU) };
            if ui.selectable_label(*auswahl == Some(BefehlsAuswahl::Geste(n)), text).clicked() {
                *auswahl = Some(BefehlsAuswahl::Geste(n));
            }
        }
        for kat in KATEGORIEN {
            let in_kat: Vec<&Befehl> = werk.befehle.iter().filter(|b| b.standard().kategorie == *kat).collect();
            if in_kat.iter().all(|b| !passt(&b.woerter, b.id)) {
                continue;
            }
            ui.add_space(6.0);
            ui.label(RichText::new(*kat).strong());
            for b in in_kat {
                if !passt(&b.woerter, b.id) {
                    continue;
                }
                let haupt = b.woerter.first().cloned().unwrap_or_else(|| b.id.to_string());
                let mut text = RichText::new(if haupt == b.id { haupt } else { format!("{haupt}  ({})", b.id) });
                if !b.aktiv {
                    text = text.color(GRAU).strikethrough();
                } else if !b.ist_standard() {
                    text = text.color(GEAENDERT);
                }
                if ui.selectable_label(*auswahl == Some(BefehlsAuswahl::Befehl(b.id)), text).on_hover_text(b.hilfe.as_str()).clicked() {
                    *auswahl = Some(BefehlsAuswahl::Befehl(b.id));
                }
            }
        }
    });
}

pub fn befehl_formular(ui: &mut egui::Ui, werk: &mut Befehlswerk, auswahl: &mut Option<BefehlsAuswahl>) {
    let rollen: Vec<String> = ROLLEN.iter().map(|r| r.to_string()).collect();
    match *auswahl {
        None => {
            ui.add_space(20.0);
            ui.label("Wähle links einen Befehl oder eine Geste aus.");
            ui.add_space(8.0);
            ui.label(RichText::new("Gelb = angepasst, durchgestrichen = abgeschaltet. Gespeichert wird in daten/befehle/ – nur das, was vom Standard abweicht.").color(GRAU));
        }
        Some(BefehlsAuswahl::Befehl(id)) => {
            let Some(b) = werk.befehle.iter_mut().find(|b| b.id == id) else { return };
            let st = befehlsliste::finde(id).expect("eingebaut");
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(6.0);
                ui.heading(b.woerter.first().cloned().unwrap_or_else(|| id.to_string()));
                ui.label(RichText::new(format!("Eingebauter Befehl '{id}' · {}", st.kategorie)).color(GRAU));
                ui.add_space(6.0);
                zeile(ui, "Aktiv", |ui| {
                    ui.checkbox(&mut b.aktiv, "Befehl ist eingeschaltet");
                });
                zeile(ui, "Wörter", |ui| {
                    wortliste(ui, ("befehl_woerter", id), &mut b.woerter, "Hauptname, Abkürzungen …");
                    ui.label(RichText::new(format!("Das erste Wort ist der Hauptname. Standard: {}", st.woerter.join(", "))).color(GRAU).small());
                });
                zeile(ui, "Rolle", |ui| {
                    egui::ComboBox::from_id_salt("befehl_rolle").selected_text(&b.rolle).show_ui(ui, |ui| {
                        for r in &rollen {
                            ui.selectable_value(&mut b.rolle, r.clone(), r);
                        }
                    });
                    ui.label(RichText::new(format!("Wer ihn benutzen darf (Standard: {})", st.rolle)).color(GRAU).small());
                });
                zeile(ui, "Aufruf", |ui| {
                    crate::kontext::feld(ui, egui::TextEdit::singleline(&mut b.syntax).hint_text(st.syntax).desired_width(f32::INFINITY));
                });
                zeile(ui, "Hilfetext", |ui| {
                    crate::kontext::feld(ui, egui::TextEdit::multiline(&mut b.hilfe).desired_rows(2).hint_text(st.hilfe).desired_width(f32::INFINITY));
                });
                ui.add_space(10.0);
                if ui.add_enabled(!b.ist_standard(), egui::Button::new("↺ Auf Standard zurücksetzen")).clicked() {
                    b.woerter = st.woerter.iter().map(|w| w.to_string()).collect();
                    b.rolle = st.rolle.into();
                    b.aktiv = true;
                    b.hilfe = st.hilfe.into();
                    b.syntax = st.syntax.into();
                }
            });
        }
        Some(BefehlsAuswahl::Geste(n)) => {
            if n >= werk.gesten.len() {
                *auswahl = None;
                return;
            }
            let mut dateien = werk.dateien.clone();
            let standard = werk.standard_datei(true);
            if !dateien.contains(&standard) {
                dateien.push(standard);
            }
            let mut loeschen = false;
            let g = &mut werk.gesten[n];
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(6.0);
                ui.heading(format!("Geste: {}", g.woerter.first().cloned().unwrap_or_default()));
                ui.add_space(6.0);
                zeile(ui, "Aktiv", |ui| {
                    ui.checkbox(&mut g.aktiv, "Geste ist eingeschaltet");
                });
                zeile(ui, "Wörter", |ui| {
                    wortliste(ui, ("geste_woerter", n), &mut g.woerter, "z.B. winke, winken");
                });
                zeile(ui, "Hilfetext", |ui| {
                    crate::kontext::feld(ui, egui::TextEdit::singleline(&mut g.hilfe).desired_width(f32::INFINITY));
                });
                zeile(ui, "Rolle", |ui| {
                    egui::ComboBox::from_id_salt("geste_rolle").selected_text(&g.rolle).show_ui(ui, |ui| {
                        for r in &rollen {
                            ui.selectable_value(&mut g.rolle, r.clone(), r);
                        }
                    });
                });
                zeile(ui, "Datei", |ui| {
                    egui::ComboBox::from_id_salt("geste_datei").selected_text(&g.datei).width(240.0).show_ui(ui, |ui| {
                        for d in &dateien {
                            ui.selectable_value(&mut g.datei, d.clone(), d);
                        }
                    });
                });
                ui.separator();
                ui.label(RichText::new("{name} = wer die Geste macht · {ziel} = „den Wirt“ · {ziel_dativ} = „dem Wirt“").color(GRAU).small());
                ui.label(RichText::new("Ohne Ziel (z.B. „winke“)").strong());
                zeile(ui, "für dich", |ui| {
                    crate::kontext::feld(ui, egui::TextEdit::singleline(&mut g.selbst).hint_text("Du winkst.").desired_width(f32::INFINITY));
                });
                zeile(ui, "für die anderen", |ui| {
                    crate::kontext::feld(ui, egui::TextEdit::singleline(&mut g.andere).hint_text("{name} winkt.").desired_width(f32::INFINITY));
                });
                ui.label(RichText::new("Mit Ziel (z.B. „winke wirt“) – leer lassen, wenn es keins gibt").strong());
                zeile(ui, "für dich", |ui| {
                    crate::kontext::feld(ui, egui::TextEdit::singleline(&mut g.selbst_ziel).hint_text("Du winkst {ziel_dativ} zu.").desired_width(f32::INFINITY));
                });
                zeile(ui, "für das Ziel", |ui| {
                    crate::kontext::feld(ui, egui::TextEdit::singleline(&mut g.ziel).hint_text("{name} winkt dir zu.").desired_width(f32::INFINITY));
                });
                zeile(ui, "für die anderen", |ui| {
                    crate::kontext::feld(ui, egui::TextEdit::singleline(&mut g.andere_ziel).hint_text("{name} winkt {ziel_dativ} zu.").desired_width(f32::INFINITY));
                });

                // Vorschau
                ui.separator();
                ui.label(RichText::new("Vorschau (Bert winkt dem Wirt)").strong());
                let vorschau = |t: &str| {
                    let t = t.replace("{name}", "Bert").replace("{ziel_dativ}", "dem Wirt").replace("{ziel}", "den Wirt");
                    let mut c = t.chars();
                    c.next().map(|e| e.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
                };
                for (wer, t) in [("Bert", &g.selbst), ("andere", &g.andere), ("Bert", &g.selbst_ziel), ("andere", &g.andere_ziel)] {
                    if !t.trim().is_empty() {
                        ui.label(format!("{wer}: {}", vorschau(t)));
                    }
                }
                ui.add_space(10.0);
                ui.menu_button("🗑 Löschen …", |ui| {
                    ui.label("Geste wirklich löschen?");
                    if ui.button("Ja, löschen").clicked() {
                        loeschen = true;
                        ui.close();
                    }
                });
            });
            if loeschen {
                werk.gesten.remove(n);
                *auswahl = None;
            }
        }
    }
}
