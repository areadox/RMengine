//! Der Telnet-Server: nimmt Verbindungen an, erledigt das Einloggen
//! und leitet Eingaben an die Befehle weiter.

use crate::befehle::{self, Folge};
use crate::spieler;
use crate::text::fuer_telnet;
use crate::welt::Welt;
use std::io;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

pub type GeteilteWelt = Arc<Mutex<Welt>>;

/// Holt die Welt, auch wenn ein früherer Fehler das Schloss "vergiftet" hat.
pub fn sperren(w: &GeteilteWelt) -> MutexGuard<'_, Welt> {
    w.lock().unwrap_or_else(|e| e.into_inner())
}

const ECHO_AUS: [u8; 3] = [255, 251, 1]; // IAC WILL ECHO
const ECHO_AN: [u8; 3] = [255, 252, 1]; // IAC WONT ECHO

pub async fn starten(welt: GeteilteWelt) -> io::Result<()> {
    let port = sperren(&welt).d.welt.port;
    let listener = TcpListener::bind(("0.0.0.0", port)).await?;
    println!("Der Server läuft auf Port {port}.");
    println!("Verbinde dich z.B. mit:  telnet localhost {port}   (oder mit einem MUD-Programm wie Mudlet)");

    // Der Takt der Welt
    let w = welt.clone();
    tokio::spawn(async move {
        loop {
            let sekunden = sperren(&w).d.welt.takt_sekunden.max(1);
            tokio::time::sleep(Duration::from_secs(sekunden)).await;
            sperren(&w).takt();
        }
    });

    loop {
        let (sock, adresse) = listener.accept().await?;
        let w = welt.clone();
        tokio::spawn(async move {
            if let Err(e) = verbindung(sock, w).await {
                eprintln!("Verbindung {adresse} beendet: {e}");
            }
        });
    }
}

async fn verbindung(sock: TcpStream, welt: GeteilteWelt) -> io::Result<()> {
    let (lesen, mut schreiben) = sock.into_split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let schreiber = tokio::spawn(async move {
        while let Some(b) = rx.recv().await {
            if schreiben.write_all(&b).await.is_err() {
                break;
            }
        }
        let _ = schreiben.shutdown().await;
    });
    let mut leser = BufReader::new(lesen);
    let sende = |t: &str| {
        let _ = tx.send(fuer_telnet(t, true, false));
    };

    let (titel, willkommen) = {
        let w = sperren(&welt);
        (w.d.welt.name.clone(), w.d.welt.willkommen.clone())
    };
    sende(&format!("\n{{fett}}{{hellgelb}}*** {titel} ***{{/}}\n\n{willkommen}\n\n"));

    // --- Name ---
    let name = loop {
        sende("Wie heißt deine Figur? ");
        let Some(zeile) = zeile_lesen(&mut leser).await? else { return Ok(()) };
        match spieler::name_pruefen(&zeile) {
            Ok(n) => break n,
            Err(m) => sende(&format!("{m}\n")),
        }
    };

    // --- Passwort ---
    let verzeichnis = sperren(&welt).spieler_verzeichnis();
    let sp = if spieler::existiert(&verzeichnis, &name) {
        let sp = match spieler::laden(&verzeichnis, &name) {
            Ok(sp) => sp,
            Err(e) => {
                eprintln!("{e}");
                sende("Deine Spielerdatei ist beschädigt. Bitte melde dich beim Betreiber.\n");
                return Ok(());
            }
        };
        let mut versuche = 0;
        loop {
            sende(&format!("Willkommen zurück, {name}! Passwort: "));
            let _ = tx.send(ECHO_AUS.to_vec());
            let Some(pw) = zeile_lesen(&mut leser).await? else { return Ok(()) };
            let _ = tx.send(ECHO_AN.to_vec());
            sende("\n");
            if sp.passwort_stimmt(&pw) {
                break sp;
            }
            versuche += 1;
            if versuche >= 3 {
                sende("Falsches Passwort. Auf Wiedersehen!\n");
                drop(tx);
                let _ = schreiber.await;
                return Ok(());
            }
            sende("Falsches Passwort.\n");
        }
    } else {
        sende(&format!("Eine neue Figur namens {name} wird erschaffen.\n"));
        loop {
            sende("Wähle ein Passwort (mindestens 4 Zeichen): ");
            let _ = tx.send(ECHO_AUS.to_vec());
            let Some(pw1) = zeile_lesen(&mut leser).await? else { return Ok(()) };
            sende("\nNoch einmal: ");
            let Some(pw2) = zeile_lesen(&mut leser).await? else { return Ok(()) };
            let _ = tx.send(ECHO_AN.to_vec());
            sende("\n");
            if pw1.chars().count() < 4 {
                sende("Das Passwort ist zu kurz.\n");
            } else if pw1 != pw2 {
                sende("Die Passwörter stimmen nicht überein.\n");
            } else {
                let sp = sperren(&welt).neuer_spieler(&name, &pw1);
                break sp;
            }
        }
    };

    // --- In die Welt ---
    let anmeldung = {
        let mut w = sperren(&welt);
        let ergebnis = w.anmelden(sp, tx.clone());
        if let Ok(id) = ergebnis {
            w.sende(id, "{grau}Tippe 'hilfe' für eine Liste der Befehle.{/}\n");
            befehle::raum_anzeigen(&w, id);
            befehle::prompt(&w, id);
            if let Some(s) = w.spieler.get(&id) {
                w.speichern(&s.sp);
            }
        }
        ergebnis
    };
    let sid = match anmeldung {
        Ok(id) => id,
        Err(m) => {
            sende(&format!("{m}\n"));
            drop(tx);
            let _ = schreiber.await;
            return Ok(());
        }
    };
    drop(tx);

    // --- Hauptschleife ---
    loop {
        let zeile = match zeile_lesen(&mut leser).await {
            Ok(Some(z)) => z,
            _ => break,
        };
        let ende = {
            let mut w = sperren(&welt);
            matches!(befehle::ausfuehren(&mut w, sid, &zeile), Folge::Beenden)
        };
        if ende {
            break;
        }
    }
    sperren(&welt).abmelden(sid);
    let _ = schreiber.await;
    Ok(())
}

/// Liest eine Zeile und entfernt dabei Telnet-Steuerzeichen.
async fn zeile_lesen(leser: &mut BufReader<OwnedReadHalf>) -> io::Result<Option<String>> {
    let mut puffer = Vec::new();
    let n = (&mut *leser).take(4096).read_until(b'\n', &mut puffer).await?;
    if n == 0 {
        return Ok(None);
    }
    Ok(Some(telnet_bereinigen(&puffer)))
}

fn telnet_bereinigen(roh: &[u8]) -> String {
    let mut aus = Vec::with_capacity(roh.len());
    let mut i = 0;
    while i < roh.len() {
        let b = roh[i];
        if b == 255 {
            // IAC: Telnet-Befehl überspringen
            match roh.get(i + 1) {
                Some(255) => {
                    aus.push(255);
                    i += 2;
                }
                Some(251..=254) => i += 3,
                Some(250) => {
                    // Unterverhandlung bis IAC SE
                    i += 2;
                    while i + 1 < roh.len() && !(roh[i] == 255 && roh[i + 1] == 240) {
                        i += 1;
                    }
                    i += 2;
                }
                _ => i += 2,
            }
            continue;
        }
        if b >= 32 || b == b'\t' {
            aus.push(b);
        } else if b == 8 || b == 127 {
            aus.pop(); // Rücktaste
        }
        i += 1;
    }
    String::from_utf8_lossy(&aus).trim().to_string()
}
