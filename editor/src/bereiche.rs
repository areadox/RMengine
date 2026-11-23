//! Die Bereiche "Zauber" und "Befehle" des Editors.
//!
//! Felder, die aus einer Vorlage geerbt werden, sind grau. Wer einen eigenen
//! Wert einträgt, überschreibt die Vorlage. Leer lassen = erben.

use crate::befehle::{Befehl, Befehlswerk, Geste};
use crate::befehlsliste::{self, KATEGORIEN, ROLLEN};
use crate::zauber::{Wirkung, Wirksam, Zauber, Zauberbuch, WIRKUNGEN, ZIELE};
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
    let antwort = ui.add(egui::TextEdit::singleline(&mut puffer).hint_text(hinweis).desired_width(f32::INFINITY));
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

/// Ein Textfeld, das leer bedeutet "von der Vorlage erben".
fn text_erbend(ui: &mut egui::Ui, wert: &mut Option<String>, geerbt: &str, mehrzeilig: bool) {
    let mut t = wert.clone().unwrap_or_default();
    let hinweis = if geerbt.is_empty() { String::new() } else { format!("(geerbt) {geerbt}") };
    let feld = if mehrzeilig {
        egui::TextEdit::multiline(&mut t).desired_rows(3)
    } else {
        egui::TextEdit::singleline(&mut t)
    };
    if ui.add(feld.hint_text(hinweis).desired_width(f32::INFINITY)).changed() {
        *wert = if t.is_empty() { None } else { Some(t) };
    }
}

/// Eine Zahl, die man erben oder selbst setzen kann.
fn zahl_erbend(ui: &mut egui::Ui, wert: &mut Option<i32>, geerbt: i32, bereich: std::ops::RangeInclusive<i32>) {
    ui.horizontal(|ui| {
        let mut eigen = wert.is_some();
        if ui.checkbox(&mut eigen, "").on_hover_text("Eigener Wert (sonst geerbt/Standard)").changed() {
            *wert = if eigen { Some(geerbt) } else { None };
        }
        match wert {
            Some(v) => {
                ui.add(egui::DragValue::new(v).range(bereich));
            }
            None => {
                ui.label(RichText::new(format!("{geerbt} (geerbt)")).color(GRAU));
            }
        }
    });
}

fn auswahl_erbend(ui: &mut egui::Ui, salt: &str, wert: &mut Option<String>, geerbt: &str, moeglich: &[String]) {
    let text = match wert {
        Some(v) if v.is_empty() => "(keine)".to_string(),
        Some(v) => v.clone(),
        None => format!("{} (geerbt)", if geerbt.is_empty() { "(keine)" } else { geerbt }),
    };
    egui::ComboBox::from_id_salt(salt).selected_text(text).width(220.0).show_ui(ui, |ui| {
        if ui.selectable_label(wert.is_none(), format!("erben: {}", if geerbt.is_empty() { "(keine)" } else { geerbt })).clicked() {
            *wert = None;
        }
        for m in moeglich {
            let anzeige = if m.is_empty() { "(keine)" } else { m.as_str() };
            if ui.selectable_label(wert.as_deref() == Some(m.as_str()), anzeige).clicked() {
                *wert = Some(m.clone());
            }
        }
    });
}

fn zeile(ui: &mut egui::Ui, beschriftung: &str, inhalt: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal_top(|ui| {
        ui.add_sized([110.0, 20.0], egui::Label::new(beschriftung));
        ui.vertical(|ui| inhalt(ui));
    });
}

// ---------------------------------------------------------------------------
// Zauber
// ---------------------------------------------------------------------------

/// Linke Spalte: Liste aller Zauber und Vorlagen.
pub fn zauber_liste(ui: &mut egui::Ui, buch: &mut Zauberbuch, auswahl: &mut Option<usize>, suche: &mut String) {
    ui.add_space(4.0);
    if ui.button("➕ Neuer Zauber").clicked() {
        let datei = buch
            .zauber
            .iter()
            .find(|z| !z.vorlage)
            .map(|z| z.datei.clone())
            .unwrap_or_else(|| "zauber/zauber.toml".into());
        let id = buch.neue_id("neuer_zauber");
        buch.zauber.push(Zauber {
            id,
            name: Some("Neuer Zauber".into()),
            mana_kosten: Some(5),
            wirkungen: Some(vec![Wirkung { art: "heilen".into(), wert: 10, raum: String::new() }]),
            datei,
            ..Default::default()
        });
        *auswahl = Some(buch.zauber.len() - 1);
    }
    ui.add(egui::TextEdit::singleline(suche).hint_text("🔍 Suchen …"));
    ui.separator();
    let such = suche.to_lowercase();
    egui::ScrollArea::vertical().show(ui, |ui| {
        for vorlagen in [false, true] {
            ui.label(RichText::new(if vorlagen { "Vorlagen" } else { "Zauber" }).strong());
            for i in 0..buch.zauber.len() {
                let z = &buch.zauber[i];
                if z.vorlage != vorlagen {
                    continue;
                }
                let name = buch.wirksam(i).name;
                if !such.is_empty() && !name.to_lowercase().contains(&such) && !z.id.contains(&such) {
                    continue;
                }
                let text = if z.erbt.is_empty() { name } else { format!("{name}  · erbt {}", z.erbt) };
                if ui.selectable_label(*auswahl == Some(i), text).on_hover_text(&z.id).clicked() {
                    *auswahl = Some(i);
                }
            }
            ui.add_space(6.0);
        }
    });
}

/// Mitte: alle Felder eines Zaubers.
pub fn zauber_formular(ui: &mut egui::Ui, buch: &mut Zauberbuch, auswahl: &mut Option<usize>, raeume: &[String]) {
    let Some(i) = *auswahl else {
        ui.add_space(20.0);
        ui.label("Wähle links einen Zauber aus oder lege einen neuen an.");
        ui.add_space(8.0);
        ui.label(RichText::new("Zauber stehen in daten/zauber/. Vorlagen (z.B. vorlage_heilzauber) liefern gemeinsame Werte, die einzelne Zauber mit 'erbt' übernehmen.").color(GRAU));
        return;
    };
    if i >= buch.zauber.len() {
        *auswahl = None;
        return;
    }
    let wirksam = buch.wirksam(i);
    let geerbt: Wirksam = buch.geerbt(i).unwrap_or_else(|| Wirksam { ziel: "selbst".into(), ..Default::default() });
    let ids: Vec<String> = buch.zauber.iter().filter(|z| z.id != buch.zauber[i].id).map(|z| z.id.clone()).collect();
    let fertigkeiten: Vec<String> = std::iter::once(String::new()).chain(buch.fertigkeiten.iter().cloned()).collect();
    let dateien = buch.dateien.clone();
    let mut loeschen = false;
    let mut kopieren = false;

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.heading(&wirksam.name);
            if buch.zauber[i].vorlage {
                ui.label(RichText::new("Vorlage").color(GEAENDERT));
            }
        });
        ui.add_space(6.0);
        let z = &mut buch.zauber[i];

        zeile(ui, "ID", |ui| {
            ui.add(egui::TextEdit::singleline(&mut z.id).desired_width(220.0));
            if z.herkunft.as_ref().is_some_and(|h| *h != z.id) {
                ui.label(RichText::new("Beim Speichern werden Startzauber, Monster und Vorlagen-Verweise angepasst. Spieler, die den Zauber schon kennen, verlieren ihn.").color(GEAENDERT).small());
            }
        });
        zeile(ui, "Datei", |ui| {
            egui::ComboBox::from_id_salt("zauber_datei").selected_text(&z.datei).width(260.0).show_ui(ui, |ui| {
                for d in &dateien {
                    ui.selectable_value(&mut z.datei, d.clone(), d);
                }
            });
        });
        zeile(ui, "Erbt von", |ui| {
            let text = if z.erbt.is_empty() { "(nichts)".to_string() } else { z.erbt.clone() };
            egui::ComboBox::from_id_salt("zauber_erbt").selected_text(text).width(220.0).show_ui(ui, |ui| {
                ui.selectable_value(&mut z.erbt, String::new(), "(nichts)");
                for id in &ids {
                    ui.selectable_value(&mut z.erbt, id.clone(), id);
                }
            });
            ui.checkbox(&mut z.vorlage, "Nur Vorlage (erscheint nicht im Spiel)");
        });
        ui.separator();
        zeile(ui, "Name", |ui| text_erbend(ui, &mut z.name, &geerbt.name, false));
        zeile(ui, "Beschreibung", |ui| text_erbend(ui, &mut z.beschreibung, &geerbt.beschreibung, true));
        zeile(ui, "Suchwörter", |ui| {
            let mut eigen = z.schluesselwoerter.is_some();
            if ui.checkbox(&mut eigen, "eigene").changed() {
                z.schluesselwoerter = if eigen { Some(geerbt.schluesselwoerter.clone()) } else { None };
            }
            match &mut z.schluesselwoerter {
                Some(l) => {
                    wortliste(ui, ("zauber_woerter", i), l, "z.B. heilung, heilen");
                }
                None => {
                    ui.label(RichText::new(format!("(geerbt) {}", geerbt.schluesselwoerter.join(", "))).color(GRAU));
                }
            }
        });
        zeile(ui, "Mana-Kosten", |ui| zahl_erbend(ui, &mut z.mana_kosten, geerbt.mana_kosten, 0..=9999));
        zeile(ui, "Ziel", |ui| {
            let ziele: Vec<String> = ZIELE.iter().map(|s| s.to_string()).collect();
            auswahl_erbend(ui, "zauber_ziel", &mut z.ziel, &geerbt.ziel, &ziele);
            ui.label(RichText::new("selbst · spieler (oder sich selbst) · gegner (Monster) · raum (alle Spieler hier)").color(GRAU).small());
        });
        zeile(ui, "Fertigkeit", |ui| auswahl_erbend(ui, "zauber_fert", &mut z.fertigkeit, &geerbt.fertigkeit, &fertigkeiten));
        zeile(ui, "Mindest-Stufe", |ui| zahl_erbend(ui, &mut z.mindest_stufe, geerbt.mindest_stufe, 0..=1000));
        zeile(ui, "Abklingzeit (s)", |ui| zahl_erbend(ui, &mut z.abklingzeit, geerbt.abklingzeit, 0..=86400));

        ui.separator();
        ui.label(RichText::new("Wirkungen").strong());
        let mut eigen = z.wirkungen.is_some();
        if ui.checkbox(&mut eigen, "eigene Wirkungen (sonst von der Vorlage)").changed() {
            z.wirkungen = if eigen { Some(geerbt.wirkungen.clone()) } else { None };
        }
        match &mut z.wirkungen {
            None => {
                for w in &geerbt.wirkungen {
                    ui.label(RichText::new(format!("(geerbt) {} {}", w.art, if w.art == "teleport" { w.raum.clone() } else { w.wert.to_string() })).color(GRAU));
                }
            }
            Some(liste) => {
                let mut weg = None;
                for (n, w) in liste.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt(("wirkung_art", n)).selected_text(&w.art).width(110.0).show_ui(ui, |ui| {
                            for a in WIRKUNGEN {
                                ui.selectable_value(&mut w.art, a.to_string(), *a);
                            }
                        });
                        if w.art == "teleport" {
                            egui::ComboBox::from_id_salt(("wirkung_raum", n)).selected_text(&w.raum).width(200.0).show_ui(ui, |ui| {
                                for r in raeume {
                                    ui.selectable_value(&mut w.raum, r.clone(), r);
                                }
                            });
                        } else {
                            ui.add(egui::DragValue::new(&mut w.wert).range(0..=100000).prefix("Wert "));
                        }
                        if ui.small_button("✖").clicked() {
                            weg = Some(n);
                        }
                    });
                }
                if let Some(n) = weg {
                    liste.remove(n);
                }
                if ui.button("➕ Wirkung").clicked() {
                    liste.push(Wirkung { art: "heilen".into(), wert: 10, raum: String::new() });
                }
                ui.label(RichText::new("heilen/mana/schaden: Wert · teleport: Zielraum").color(GRAU).small());
            }
        }

        ui.separator();
        ui.label(RichText::new("Texte").strong());
        ui.label(RichText::new("{name} = wer zaubert, {ziel} = das Ziel, {zauber} = Name des Zaubers").color(GRAU).small());
        zeile(ui, "für dich", |ui| text_erbend(ui, &mut z.text_selbst, &geerbt.text_selbst, false));
        zeile(ui, "für das Ziel", |ui| text_erbend(ui, &mut z.text_ziel, &geerbt.text_ziel, false));
        zeile(ui, "für den Raum", |ui| text_erbend(ui, &mut z.text_raum, &geerbt.text_raum, false));

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui.button("Kopie anlegen").clicked() {
                kopieren = true;
            }
            ui.menu_button("🗑 Löschen …", |ui| {
                ui.label("Zauber wirklich löschen?");
                if ui.button("Ja, löschen").clicked() {
                    loeschen = true;
                    ui.close();
                }
            });
        });
    });

    if kopieren {
        let mut k = buch.zauber[i].clone();
        k.id = buch.neue_id(&format!("{}_kopie", k.id));
        k.herkunft = None;
        k.name = Some(format!("{} (Kopie)", buch.wirksam(i).name));
        buch.zauber.push(k);
        *auswahl = Some(buch.zauber.len() - 1);
    }
    if loeschen {
        buch.zauber.remove(i);
        *auswahl = None;
    }
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
    ui.add(egui::TextEdit::singleline(suche).hint_text("🔍 Suchen …"));
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
                    ui.add(egui::TextEdit::singleline(&mut b.syntax).hint_text(st.syntax).desired_width(f32::INFINITY));
                });
                zeile(ui, "Hilfetext", |ui| {
                    ui.add(egui::TextEdit::multiline(&mut b.hilfe).desired_rows(2).hint_text(st.hilfe).desired_width(f32::INFINITY));
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
                    ui.add(egui::TextEdit::singleline(&mut g.hilfe).desired_width(f32::INFINITY));
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
                    ui.add(egui::TextEdit::singleline(&mut g.selbst).hint_text("Du winkst.").desired_width(f32::INFINITY));
                });
                zeile(ui, "für die anderen", |ui| {
                    ui.add(egui::TextEdit::singleline(&mut g.andere).hint_text("{name} winkt.").desired_width(f32::INFINITY));
                });
                ui.label(RichText::new("Mit Ziel (z.B. „winke wirt“) – leer lassen, wenn es keins gibt").strong());
                zeile(ui, "für dich", |ui| {
                    ui.add(egui::TextEdit::singleline(&mut g.selbst_ziel).hint_text("Du winkst {ziel_dativ} zu.").desired_width(f32::INFINITY));
                });
                zeile(ui, "für das Ziel", |ui| {
                    ui.add(egui::TextEdit::singleline(&mut g.ziel).hint_text("{name} winkt dir zu.").desired_width(f32::INFINITY));
                });
                zeile(ui, "für die anderen", |ui| {
                    ui.add(egui::TextEdit::singleline(&mut g.andere_ziel).hint_text("{name} winkt {ziel_dativ} zu.").desired_width(f32::INFINITY));
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
