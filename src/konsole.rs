//! Die Server-Konsole: Befehle direkt im Fenster der Engine eintippen
//! (neuladen, wer, verkuende, speichern, ende ...).

use crate::netz::{sperren, GeteilteWelt};
use crate::spieler::{self, Rolle};
use tokio::io::{AsyncBufReadExt, BufReader};

const HILFE: &str = "Befehle der Server-Konsole:
  neuladen               Weltdaten neu einlesen (Spieler bleiben im Spiel)
  pruefen                Weltdaten prüfen, ohne sie zu übernehmen
  wer                    Wer ist gerade im Spiel?
  verkuende <text>       Nachricht an alle Spieler
  rolle <name> <rolle>   Rolle vergeben: spieler, baumeister oder admin
  speichern              Alle Spieler sofort speichern
  version                Version anzeigen
  ende                   Alle speichern und den Server beenden";

/// Windows: Umlaute richtig anzeigen und dem Fenster einen Titel geben.
pub fn fenster_vorbereiten(titel: &str) {
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        extern "system" {
            fn SetConsoleOutputCP(id: u32) -> i32;
            fn SetConsoleCP(id: u32) -> i32;
            fn SetConsoleTitleW(titel: *const u16) -> i32;
        }
        let breit: Vec<u16> = titel.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: einfache Windows-Aufrufe ohne Zeiger außer dem nullterminierten Titel
        unsafe {
            SetConsoleOutputCP(65001);
            SetConsoleCP(65001);
            SetConsoleTitleW(breit.as_ptr());
        }
    }
    #[cfg(not(windows))]
    let _ = titel;
}

/// Liest Befehle aus dem Konsolenfenster. Ohne Konsole (z.B. als Dienst) endet das still.
pub async fn starten(welt: GeteilteWelt) {
    println!("Server-Konsole: Tippe hier 'hilfe' für Befehle (z.B. neuladen, wer, ende).");
    let mut zeilen = BufReader::new(tokio::io::stdin()).lines();
    while let Ok(Some(zeile)) = zeilen.next_line().await {
        let zeile = zeile.trim().to_string();
        if zeile.is_empty() {
            continue;
        }
        let (befehl, rest) = zeile.split_once(char::is_whitespace).unwrap_or((&zeile, ""));
        let rest = rest.trim();
        let mut w = sperren(&welt);
        match crate::text::normalisieren(befehl).as_str() {
            "hilfe" | "help" | "?" => println!("{HILFE}"),
            "neuladen" | "reload" => match w.neuladen() {
                Ok(warnungen) => {
                    for t in &warnungen {
                        println!("  ! Warnung: {t}");
                    }
                    println!("Weltdaten neu geladen.");
                    w.sende_alle("{grau}Die Welt wurde neu geladen.{/}", &[]);
                }
                Err(fehler) => {
                    println!("Neuladen abgebrochen – die alte Welt läuft weiter. {} Fehler:", fehler.len());
                    for f in fehler {
                        println!("  ✗ {f}");
                    }
                }
            },
            "pruefen" | "check" => match crate::daten::laden(&w.daten_ordner) {
                Ok(d) => {
                    for t in &d.warnungen {
                        println!("  ! Warnung: {t}");
                    }
                    println!("Alles in Ordnung.");
                }
                Err(fehler) => {
                    println!("{} Fehler:", fehler.len());
                    for f in fehler {
                        println!("  ✗ {f}");
                    }
                }
            },
            "wer" | "who" | "spieler" => {
                if w.spieler.is_empty() {
                    println!("Niemand ist im Spiel.");
                }
                for s in w.spieler.values() {
                    let client = crate::telnet::sperren(&s.proto).client_name();
                    println!("  {:<20} {:<10} Raum: {:<20} ({client})", s.sp.name, s.rolle.name(), s.sp.raum);
                }
            }
            "verkuende" | "sag" | "sage" => {
                if rest.is_empty() {
                    println!("Was willst du verkünden?");
                } else {
                    w.sende_alle(&format!("\n{{hellgelb}}*** Verkündung: {rest} ***{{/}}\n"), &[]);
                    println!("Verkündet.");
                }
            }
            "rolle" => rolle(&mut w, rest),
            "speichern" | "speichernalle" => {
                w.alle_speichern();
                println!("{} Spieler gespeichert.", w.spieler.len());
            }
            "version" => println!("RMengine {}", crate::update::VERSION),
            "ende" | "quit" | "exit" | "stopp" | "stop" => {
                println!("Server wird beendet, alle Spieler werden gespeichert ...");
                w.sende_alle("{rot}Der Server wird heruntergefahren. Bis bald!{/}", &[]);
                w.alle_speichern();
                std::process::exit(0);
            }
            _ => println!("Unbekannter Befehl '{befehl}'. Tippe 'hilfe'."),
        }
    }
}

fn rolle(w: &mut crate::welt::Welt, rest: &str) {
    let mut teile = rest.split_whitespace();
    let (Some(name), Some(r)) = (teile.next(), teile.next()) else {
        println!("Benutzung: rolle <name> <spieler|baumeister|admin>");
        return;
    };
    let Some(neu) = Rolle::aus_text(r) else {
        println!("Rolle '{r}' gibt es nicht. Erlaubt: spieler, baumeister, admin");
        return;
    };
    // Im Spiel?
    if let Some(sid) = w.spieler.iter().find(|(_, s)| s.sp.name.eq_ignore_ascii_case(name)).map(|(id, _)| *id) {
        let s = w.spieler.get_mut(&sid).expect("gefunden");
        s.sp.rolle = neu;
        let sp = s.sp.clone();
        let tatsaechlich = w.rolle_von(&sp);
        let s = w.spieler.get_mut(&sid).expect("gefunden");
        s.rolle = tatsaechlich;
        s.sende(&format!("{{hellgelb}}Du hast jetzt die Rolle {}.{{/}}", tatsaechlich.name()));
        w.speichern(&sp);
        println!("{} hat jetzt die Rolle {}.", sp.name, tatsaechlich.name());
        return;
    }
    // Sonst in der gespeicherten Datei ändern
    let verzeichnis = w.spieler_verzeichnis();
    match spieler::laden(&verzeichnis, name) {
        Ok(mut sp) => {
            sp.rolle = neu;
            w.speichern(&sp);
            println!("{} hat jetzt die Rolle {} (gilt beim nächsten Einloggen).", sp.name, neu.name());
        }
        Err(_) => println!("Eine Figur '{name}' gibt es nicht."),
    }
}
