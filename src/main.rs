//! MUD-Treiber – eine Engine, mit der man ein MUD bauen kann, ohne zu programmieren.
//!
//! Aufruf:
//!   mud-treiber [datenordner]            Server starten (Standard: ./daten)
//!   mud-treiber [datenordner] --pruefen  Nur die Weltdaten prüfen

mod befehle;
mod daten;
mod karte;
mod netz;
mod oberwelt;
mod spieler;
mod text;
mod welt;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let nur_pruefen = args.iter().any(|a| a == "--pruefen" || a == "--check");
    let ordner = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("daten"));

    println!("Lade Weltdaten aus '{}' ...", ordner.display());
    let d = match daten::laden(&ordner) {
        Ok(d) => d,
        Err(fehler) => {
            eprintln!("\nDie Weltdaten enthalten {} Fehler:\n", fehler.len());
            for f in fehler {
                eprintln!("  ✗ {f}\n");
            }
            std::process::exit(1);
        }
    };
    for w in &d.warnungen {
        println!("  ! Warnung: {w}");
    }
    println!(
        "'{}': {} Räume, {} Gegenstände, {} Monster, {} Fertigkeiten, {} Zauber.",
        d.welt.name,
        d.raeume.len(),
        d.gegenstaende.len(),
        d.monster.len(),
        d.fertigkeiten.len(),
        d.zauber.len()
    );
    if nur_pruefen {
        println!("Alles in Ordnung.");
        return;
    }

    let welt = Arc::new(Mutex::new(welt::Welt::neu(d, &ordner)));

    let w = welt.clone();
    tokio::select! {
        ergebnis = netz::starten(welt) => {
            if let Err(e) = ergebnis {
                eprintln!("Der Server konnte nicht starten: {e}");
                std::process::exit(1);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            println!("\nServer wird beendet, alle Spieler werden gespeichert ...");
            let w = netz::sperren(&w);
            w.sende_alle("{rot}Der Server wird heruntergefahren. Bis bald!{/}", &[]);
            w.alle_speichern();
        }
    }
}
