//! Der Telnet-Server: nimmt Verbindungen an, erledigt das Einloggen
//! und leitet Eingaben an die Befehle weiter.

use crate::befehle::{self, Folge};
use crate::spieler;
use crate::text::fuer_telnet;
use crate::welt::Welt;
use std::io;
use std::sync::{Arc, Mutex, MutexGuard};
use crate::telnet::{self, Ausgabe, Ereignis};
use flate2::write::ZlibEncoder;
use std::collections::VecDeque;
use std::io::Write;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

pub type GeteilteWelt = Arc<Mutex<Welt>>;

/// Holt die Welt, auch wenn ein früherer Fehler das Schloss "vergiftet" hat.
pub fn sperren(w: &GeteilteWelt) -> MutexGuard<'_, Welt> {
    w.lock().unwrap_or_else(|e| e.into_inner())
}

const ECHO_AUS: [u8; 3] = [255, 251, 1]; // IAC WILL ECHO
const ECHO_AN: [u8; 3] = [255, 252, 1]; // IAC WONT ECHO

/// Seit wann der Server läuft (Unix-Zeit, für MSSP)
static STARTZEIT: std::sync::OnceLock<u64> = std::sync::OnceLock::new();

pub async fn starten(welt: GeteilteWelt) -> io::Result<()> {
    let port = sperren(&welt).d.welt.port;
    let _ = STARTZEIT.set(SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0));
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

    // Fernwartung für den Welteditor (nur, wenn mit --fernwartung eingerichtet)
    tokio::spawn(crate::fernwartung::starten(welt.clone()));

    // Der Kampftakt: mehrmals pro Sekunde, damit Kämpfe in Echtzeit laufen
    let w = welt.clone();
    tokio::spawn(async move {
        let mut intervall = tokio::time::interval(Duration::from_millis(200));
        loop {
            intervall.tick().await;
            crate::kampf::takt(&mut sperren(&w));
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
    let _ = sock.set_nodelay(true);
    let (lesen, schreiben) = sock.into_split();
    let (tx, rx) = mpsc::unbounded_channel::<Ausgabe>();
    let schreiber = tokio::spawn(schreiben_schleife(schreiben, rx));
    let proto = telnet::Geteilt::default();
    // Anbieten, was wir können – der Client sucht sich aus, was er versteht
    telnet::senden(&tx, telnet::begruessung());
    let mut leser = Leser {
        lesen,
        zerleger: telnet::Zerleger::default(),
        zeilen: VecDeque::new(),
        tx: tx.clone(),
        proto: proto.clone(),
        welt: welt.clone(),
        sid: None,
    };
    let sende = |t: &str| {
        let b = telnet::sperren(&proto).kodieren(fuer_telnet(t, true, false));
        telnet::senden(&tx, b);
    };
    let echo = |b: [u8; 3]| telnet::senden(&tx, b.to_vec());
    let (titel, willkommen) = {
        let w = sperren(&welt);
        (w.d.welt.name.clone(), w.d.welt.willkommen.clone())
    };
    sende(&format!("\n{{fett}}{{hellgelb}}*** {titel} ***{{/}}\n\n{willkommen}\n\n"));

    // --- Name ---
    let name = loop {
        sende("Wie heißt deine Figur? ");
        let Some(zeile) = leser.zeile().await? else { return Ok(()) };
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
            echo(ECHO_AUS);
            let Some(pw) = leser.zeile().await? else { return Ok(()) };
            echo(ECHO_AN);
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
            echo(ECHO_AUS);
            let Some(pw1) = leser.zeile().await? else { return Ok(()) };
            sende("\nNoch einmal: ");
            let Some(pw2) = leser.zeile().await? else { return Ok(()) };
            echo(ECHO_AN);
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
        let ergebnis = w.anmelden(sp, tx.clone(), proto.clone());
        if let Ok(id) = ergebnis {
            crate::clientdaten::angemeldet(&w, id);
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
    leser.sid = Some(sid);

    // --- Hauptschleife ---
    loop {
        let zeile = match leser.zeile().await {
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

/// Schreibt alles zum Client – nach MCCP2-Aushandlung komprimiert.
async fn schreiben_schleife(mut schreiben: OwnedWriteHalf, mut rx: mpsc::UnboundedReceiver<Ausgabe>) {
    let mut zlib: Option<ZlibEncoder<Vec<u8>>> = None;
    while let Some(erste) = rx.recv().await {
        // Alles einsammeln, was gerade ansteht: ein Paket statt vieler kleiner
        let mut stapel = vec![erste];
        while let Ok(a) = rx.try_recv() {
            stapel.push(a);
        }
        let mut raus = Vec::new();
        for a in stapel {
            match a {
                Ausgabe::Daten(b) => match zlib.as_mut() {
                    Some(z) => {
                        let _ = z.write_all(&b);
                    }
                    None => raus.extend(b),
                },
                Ausgabe::KompressionStarten => {
                    if zlib.is_none() {
                        zlib = Some(ZlibEncoder::new(Vec::new(), flate2::Compression::default()));
                    }
                }
            }
        }
        if let Some(z) = zlib.as_mut() {
            let _ = z.flush(); // Sync-Flush: der Client kann sofort alles entpacken
            raus.extend(std::mem::take(z.get_mut()));
        }
        if !raus.is_empty() && schreiben.write_all(&raus).await.is_err() {
            return;
        }
    }
    // Komprimierten Datenstrom ordentlich beenden
    if let Some(z) = zlib {
        if let Ok(rest) = z.finish() {
            let _ = schreiben.write_all(&rest).await;
        }
    }
    let _ = schreiben.shutdown().await;
}

/// Liest vom Client: Eingabezeilen und nebenbei die Telnet-Aushandlung.
struct Leser {
    lesen: OwnedReadHalf,
    zerleger: telnet::Zerleger,
    zeilen: VecDeque<String>,
    tx: telnet::Sender,
    proto: telnet::Geteilt,
    welt: GeteilteWelt,
    sid: Option<u64>,
}

impl Leser {
    /// Die nächste Eingabezeile (None = Verbindung beendet).
    async fn zeile(&mut self) -> io::Result<Option<String>> {
        let mut puffer = [0u8; 4096];
        loop {
            if let Some(z) = self.zeilen.pop_front() {
                return Ok(Some(z));
            }
            let n = self.lesen.read(&mut puffer).await?;
            if n == 0 {
                return Ok(None);
            }
            let mut ereignisse = Vec::new();
            self.zerleger.fuettern(&puffer[..n], &mut ereignisse);
            for e in ereignisse {
                let aktion = match e {
                    Ereignis::Zeile(z) => {
                        if self.zeilen.len() < 100 {
                            self.zeilen.push_back(z);
                        }
                        None
                    }
                    Ereignis::Befehl(b, opt) => telnet::befehl(&mut telnet::sperren(&self.proto), &self.tx, b, opt),
                    Ereignis::Unter(opt, d) => telnet::unter(&mut telnet::sperren(&self.proto), &self.tx, opt, &d),
                };
                if let Some(a) = aktion {
                    self.ausfuehren(a);
                }
            }
        }
    }

    /// Antworten, für die man die Welt braucht.
    fn ausfuehren(&self, a: telnet::Aktion) {
        let w = sperren(&self.welt);
        match a {
            telnet::Aktion::Mssp => {
                let werte = crate::clientdaten::mssp(&w, STARTZEIT.get().copied().unwrap_or(0));
                telnet::senden(&self.tx, telnet::mssp(&werte));
            }
            telnet::Aktion::Msdp(befehle) => crate::clientdaten::msdp_befehle(&w, self.sid, &self.proto, &self.tx, befehle),
            telnet::Aktion::Gmcp(paket, _) => {
                // Der Client möchte Werte sofort (z.B. nach dem Laden eines Skripts)
                if let Some(sid) = self.sid {
                    if paket.eq_ignore_ascii_case("Char.Request") || paket.eq_ignore_ascii_case("Core.Supports.Set") {
                        crate::clientdaten::werte(&w, sid);
                    }
                }
            }
        }
    }
}
