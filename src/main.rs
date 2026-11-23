//! RMengine – eine Engine, mit der man ein MUD bauen kann, ohne zu programmieren.
//!
//! Aufruf:
//!   rmengine [datenordner]            Server starten (Standard: ./daten oder daten/ neben dem Programm)
//!   rmengine [datenordner] --pruefen  Nur die Weltdaten prüfen
//!   rmengine --update                 Auf die neueste Version aktualisieren
//!   rmengine --version                Version anzeigen

mod befehle;
mod beschreibung;
mod daten;
mod gespraech;
mod karte;
mod netz;
mod oberwelt;
mod spieler;
mod text;
mod update;
mod welt;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const HILFE: &str = "RMengine – MUD-Treiber

Benutzung:
  rmengine [datenordner]             Server starten
  rmengine [datenordner] --pruefen   Weltdaten nur prüfen
  rmengine --update                  Neueste Version installieren
  rmengine --version                 Version anzeigen
  rmengine --keine-updatepruefung    Beim Start nicht nach Updates suchen

Ohne Angabe wird der Ordner 'daten' im aktuellen Verzeichnis benutzt,
sonst der Ordner 'daten' neben dem Programm.";

#[tokio::main]
async fn main() {
    update::aufraeumen();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let hat = |namen: &[&str]| args.iter().any(|a| namen.contains(&a.as_str()));

    if hat(&["--version", "-V"]) {
        println!("RMengine {}", update::VERSION);
        return;
    }
    if hat(&["--hilfe", "--help", "-h"]) {
        println!("{HILFE}");
        return;
    }
    if hat(&["--update", "--aktualisieren"]) {
        match tokio::task::spawn_blocking(update::aktualisieren).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                eprintln!("Update fehlgeschlagen: {e}");
                warten_und_beenden(1);
            }
            Err(e) => {
                eprintln!("Update fehlgeschlagen: {e}");
                warten_und_beenden(1);
            }
        }
        warten_und_beenden(0);
    }
    let nur_pruefen = hat(&["--pruefen", "--check"]);
    let ordner = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(PathBuf::from)
        .unwrap_or_else(datenordner_suchen);

    println!("RMengine {}", update::VERSION);
    println!("Lade Weltdaten aus '{}' ...", ordner.display());
    let d = match daten::laden(&ordner) {
        Ok(d) => d,
        Err(fehler) => {
            eprintln!("\nDie Weltdaten enthalten {} Fehler:\n", fehler.len());
            for f in fehler {
                eprintln!("  ✗ {f}\n");
            }
            warten_und_beenden(1);
        }
    };
    for w in &d.warnungen {
        println!("  ! Warnung: {w}");
    }
    println!(
        "'{}': {} Räume, {} Gegenstände, {} Monster, {} Fertigkeiten, {} Zauber{}.",
        d.welt.name,
        d.raeume.len(),
        d.gegenstaende.len(),
        d.monster.len(),
        d.fertigkeiten.len(),
        d.zauber.len(),
        if d.oberwelt.is_some() { ", mit Oberwelt" } else { "" }
    );
    if nur_pruefen {
        println!("Alles in Ordnung.");
        warten_und_beenden(0);
    }
    if !hat(&["--keine-updatepruefung"]) {
        update::beim_start_pruefen();
    }

    let welt = Arc::new(Mutex::new(welt::Welt::neu(d, &ordner)));

    let w = welt.clone();
    tokio::select! {
        ergebnis = netz::starten(welt) => {
            if let Err(e) = ergebnis {
                eprintln!("Der Server konnte nicht starten: {e}");
                eprintln!("(Läuft er vielleicht schon? Oder ist der Port in welt.toml belegt?)");
                warten_und_beenden(1);
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

/// ./daten, wenn es den Ordner gibt – sonst daten/ neben dem Programm
/// (so findet ein installiertes Programm seine Welt auch per Doppelklick).
fn datenordner_suchen() -> PathBuf {
    let hier = PathBuf::from("daten");
    if hier.join("welt.toml").exists() {
        return hier;
    }
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|o| o.join("daten")))
        .filter(|p| p.join("welt.toml").exists())
        .unwrap_or(hier)
}

/// Unter Windows schließt sich das Fenster sonst sofort und man kann nichts lesen.
fn warten_und_beenden(code: i32) -> ! {
    if cfg!(windows) && std::env::var_os("PROMPT").is_none() {
        println!("\nDrücke Enter zum Schließen ...");
        let mut s = String::new();
        let _ = std::io::stdin().read_line(&mut s);
    }
    std::process::exit(code);
}
