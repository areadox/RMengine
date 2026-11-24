//! Rechtsklick-Menüs: Ausschneiden, Kopieren, Einfügen, Alles markieren und
//! Rückgängig für alle Textfelder des Editors.
//!
//! egui verarbeitet Ausschneiden/Einfügen als Eingabe-Ereignisse des Textfelds,
//! das gerade den Fokus hat. Das Menü merkt sich deshalb, was zu tun ist, und
//! erledigt es im nächsten Bild: erst dem Feld den Fokus geben, dann das
//! Ereignis schicken. (Der Klick auf den Menüpunkt landet sonst zusätzlich im
//! Feld darunter und nimmt ihm den Fokus weg.)

use eframe::egui::{self, text::CCursor, text::CCursorRange, Event, Key, Modifiers, Response, TextEdit, Ui, ViewportCommand};

#[derive(Clone)]
enum Aktion {
    Ereignis(Event),
    Einfuegen,
    AllesMarkieren(usize),
}

#[derive(Clone, Default)]
struct Wartend(Vec<(egui::Id, Aktion)>);

fn wartend_id() -> egui::Id {
    egui::Id::NULL.with("kontext_wartend")
}

/// Am Anfang jedes Bilds aufrufen: erledigt, was im Menü gewählt wurde.
pub fn ereignisse_einspielen(ctx: &egui::Context) {
    let Some(Wartend(liste)) = ctx.data_mut(|d| d.remove_temp::<Wartend>(wartend_id())) else { return };
    for (feld, aktion) in liste {
        ctx.memory_mut(|m| m.request_focus(feld));
        match aktion {
            Aktion::Ereignis(e) => ctx.input_mut(|i| i.events.push(e)),
            Aktion::Einfuegen => ctx.send_viewport_cmd(ViewportCommand::RequestPaste),
            // Erst nach dem Zeichnen des Felds markieren (beim Fokussieren setzt egui den Cursor neu)
            Aktion::AllesMarkieren(n) => {
                ctx.data_mut(|d| d.insert_temp(feld.with("alles_markieren"), n));
            }
        }
    }
    ctx.request_repaint();
}

fn merken(ctx: &egui::Context, feld: egui::Id, a: Aktion) {
    ctx.data_mut(|d| d.get_temp_mut_or_default::<Wartend>(wartend_id()).0.push((feld, a)));
    ctx.request_repaint();
}

/// Ein Textfeld mit Rechtsklick-Menü (statt `ui.add(TextEdit::...)`).
pub fn feld(ui: &mut Ui, edit: TextEdit) -> Response {
    let ausgabe = edit.show(ui);
    let antwort = ausgabe.response.response.clone();
    let text = ausgabe.galley.text().to_string();
    let id = antwort.id;
    // Ein Rechtsklick hebt in egui die Markierung auf. Deshalb merken wir uns die
    // letzte Markierung und stellen sie beim Rechtsklick wieder her.
    let merk_id = id.with("letzte_auswahl");
    let mut auswahl = ausgabe.state.cursor.char_range();
    if let Some(n) = ui.data_mut(|d| d.remove_temp::<usize>(id.with("alles_markieren"))) {
        let alles = CCursorRange::two(CCursor::new(0), CCursor::new(n));
        let mut zustand = ausgabe.state.clone();
        zustand.cursor.set_char_range(Some(alles));
        zustand.store(ui.ctx(), id);
        ui.data_mut(|d| d.insert_temp(merk_id, alles));
        auswahl = Some(alles);
        ui.ctx().request_repaint();
    }
    let rechte_taste = ui.input(|i| i.pointer.secondary_down() || i.pointer.secondary_released());
    let menue_offen = antwort.context_menu_opened() || antwort.secondary_clicked();
    let gemerkt = ui.data(|d| d.get_temp::<CCursorRange>(merk_id));
    if menue_offen {
        // Während das Menü offen ist, gilt die Markierung von vor dem Rechtsklick
        if let Some(alt) = gemerkt {
            if auswahl != Some(alt) {
                let mut zustand = ausgabe.state.clone();
                zustand.cursor.set_char_range(Some(alt));
                zustand.store(ui.ctx(), id);
            }
            auswahl = Some(alt);
        }
    } else if antwort.has_focus() && !rechte_taste {
        match auswahl.filter(|r| !r.is_empty()) {
            Some(r) => {
                ui.data_mut(|d| d.insert_temp(merk_id, r));
            }
            // Markierung aufgehoben (Klick, Tippen): vergessen
            None => {
                ui.data_mut(|d| d.remove_temp::<CCursorRange>(merk_id));
            }
        }
    }
    let markiert: String = auswahl
        .filter(|r| !r.is_empty())
        .map(|r| {
            let (p, q) = (r.primary.index.0, r.secondary.index.0);
            let (a, b) = (p.min(q), p.max(q));
            text.chars().skip(a).take(b - a).collect()
        })
        .unwrap_or_default();
    antwort.context_menu(|ui| {
        let hat_auswahl = !markiert.is_empty();
        if ui.add_enabled(hat_auswahl, egui::Button::new("✂ Ausschneiden")).clicked() {
            merken(ui.ctx(), id, Aktion::Ereignis(Event::Cut));
            ui.close();
        }
        if ui.add_enabled(hat_auswahl, egui::Button::new("📋 Kopieren")).clicked() {
            ui.ctx().copy_text(markiert.clone());
            ui.close();
        }
        if ui.button("📥 Einfügen").clicked() {
            merken(ui.ctx(), id, Aktion::Einfuegen);
            ui.close();
        }
        ui.separator();
        if ui.button("Alles markieren").clicked() {
            merken(ui.ctx(), id, Aktion::AllesMarkieren(text.chars().count()));
            ui.close();
        }
        if ui.add_enabled(!text.is_empty(), egui::Button::new("Alles kopieren")).clicked() {
            ui.ctx().copy_text(text.clone());
            ui.close();
        }
        ui.separator();
        if ui.button("Rückgängig").on_hover_text("Strg+Z").clicked() {
            let e = Event::Key { key: Key::Z, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::COMMAND };
            merken(ui.ctx(), id, Aktion::Ereignis(e));
            ui.close();
        }
    });
    antwort
}
