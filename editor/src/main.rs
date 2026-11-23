//! RMengine – Welteditor
//!
//! Aufruf:
//!   rmeditor               sucht ./daten oder daten/ neben dem Programm
//!   rmeditor <datenordner> öffnet den angegebenen Ordner

#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod welt;

fn main() -> eframe::Result {
    let ordner = app::ordner_suchen(std::env::args().nth(1));
    let optionen = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("RMengine – Welteditor")
            .with_inner_size([1400.0, 850.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        "rmeditor",
        optionen,
        Box::new(move |cc| Ok(Box::new(app::App::neu(cc, ordner)))),
    )
}
