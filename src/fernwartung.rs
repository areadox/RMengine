//! Fernwartung: Der Welteditor auf einem anderen Rechner holt die Welt vom
//! Server, und man bearbeitet sie dort und lädt sie wieder hoch.
//!
//! Einrichten auf dem Server:   rmengine --fernwartung
//! Das legt neben dem Datenordner an:
//!   fernwartung.toml   Einstellungen (Port, Hash des Zugangsschlüssels)
//!   fernwartung.crt    Zertifikat für die verschlüsselte Verbindung
//!   fernwartung.key    der geheime Schlüssel dazu (nicht weitergeben!)
//!
//! Sicherheit:
//! - TLS-verschlüsselt; der Editor prüft den Fingerabdruck des Zertifikats.
//! - Ohne den Zugangsschlüssel geht nichts; falsche Versuche werden ausgebremst.
//! - Es sind nur Welt-Dateien im Datenordner erreichbar, keine Spielstände.
//! - Änderungen werden zuerst in einer Kopie geprüft und nur übernommen, wenn
//!   die Welt fehlerfrei lädt.

use crate::befehle;
use crate::daten;
use crate::fernwartung_protokoll::{self as p, Anfrage, Antwort};
use crate::netz::{sperren, GeteilteWelt};
use crate::spieler;
use crate::update;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Konfig {
    pub aktiv: bool,
    pub port: u16,
    pub schluessel_hash: String,
    pub salz: String,
}

impl Default for Konfig {
    fn default() -> Self {
        Konfig { aktiv: false, port: p::STANDARD_PORT, schluessel_hash: String::new(), salz: String::new() }
    }
}

/// Der Ordner, in dem das Programm und der Datenordner liegen.
fn programm_ordner(daten_ordner: &Path) -> PathBuf {
    daten_ordner.parent().map(Path::to_path_buf).filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| PathBuf::from("."))
}

fn konfig_pfad(daten_ordner: &Path) -> PathBuf {
    programm_ordner(daten_ordner).join("fernwartung.toml")
}

pub fn konfig_laden(daten_ordner: &Path) -> Option<Konfig> {
    let text = fs::read_to_string(konfig_pfad(daten_ordner)).ok()?;
    toml::from_str(&text).ok()
}

fn zertifikat_pfade(daten_ordner: &Path) -> (PathBuf, PathBuf) {
    let o = programm_ordner(daten_ordner);
    (o.join("fernwartung.crt"), o.join("fernwartung.key"))
}

/// Fingerabdruck des Zertifikats (zum Vergleichen im Editor).
pub fn fingerabdruck(daten_ordner: &Path) -> Option<String> {
    use rustls::pki_types::pem::PemObject;
    let (crt, _) = zertifikat_pfade(daten_ordner);
    let der = rustls::pki_types::CertificateDer::from_pem_file(crt).ok()?;
    Some(p::fingerabdruck(&der))
}

/// Schreibt fernwartung.toml (nur für den Besitzer lesbar).
fn konfig_schreiben(daten_ordner: &Path, konfig: &Konfig) -> Result<(), String> {
    let text = format!(
        "# Fernwartung für den Welteditor\n\
         # Einschalten/neuer Schlüssel: in der Server-Konsole 'fernwartung ein' bzw. 'fernwartung neu'\n\
         # (oder 'rmengine --fernwartung'). Abschalten: aktiv = false\n{}",
        toml::to_string(konfig).map_err(|e| e.to_string())?
    );
    let pfad = konfig_pfad(daten_ordner);
    fs::write(&pfad, text).map_err(|e| format!("{}: {e}", pfad.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&pfad, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// Einschalten mit einem neuen Zugangsschlüssel (erzeugt beim ersten Mal auch das Zertifikat).
/// Gibt den Schlüssel zurück – er wird nirgends gespeichert, nur sein Hash.
fn neuer_schluessel(daten_ordner: &Path) -> Result<(Konfig, String), String> {
    let (crt, key) = zertifikat_pfade(daten_ordner);
    if !crt.exists() || !key.exists() {
        let neu = rcgen::generate_simple_self_signed(vec![p::SERVERNAME.to_string()]).map_err(|e| e.to_string())?;
        fs::write(&crt, neu.cert.pem()).map_err(|e| format!("{}: {e}", crt.display()))?;
        fs::write(&key, neu.key_pair.serialize_pem()).map_err(|e| format!("{}: {e}", key.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&key, fs::Permissions::from_mode(0o600));
        }
    }
    // Zugangsschlüssel: 5 Gruppen à 5 Zeichen (ohne leicht verwechselbare Zeichen)
    const ZEICHEN: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    let schluessel = (0..5)
        .map(|_| (0..5).map(|_| ZEICHEN[rng.gen_range(0..ZEICHEN.len())] as char).collect::<String>())
        .collect::<Vec<_>>()
        .join("-");
    let alt = konfig_laden(daten_ordner).unwrap_or_default();
    let salz = spieler::neues_salz();
    let konfig = Konfig { aktiv: true, port: alt.port, schluessel_hash: spieler::passwort_hash(&schluessel, &salz), salz };
    konfig_schreiben(daten_ordner, &konfig)?;
    Ok((konfig, schluessel))
}

fn schluessel_anzeigen(daten_ordner: &Path, konfig: &Konfig, schluessel: &str) {
    println!("\nFernwartung eingeschaltet.\n");
    println!("  Port:              {}   (in der Firewall freigeben)", konfig.port);
    println!("  Zugangsschlüssel:  {schluessel}");
    println!("                     (wird nur jetzt angezeigt – gut aufheben!)");
    println!("  Fingerabdruck:     {}", fingerabdruck(daten_ordner).unwrap_or_default());
    println!("\nIm Welteditor: 🌐 Server → Adresse dieses Rechners, Port {} und den Schlüssel eingeben.", konfig.port);
    println!("Der Editor zeigt beim ersten Verbinden den Fingerabdruck – er muss mit dem oben übereinstimmen.");
}

/// `rmengine --fernwartung`: einschalten und einen neuen Zugangsschlüssel erzeugen.
pub fn einrichten(daten_ordner: &Path) -> Result<(), String> {
    let (konfig, schluessel) = neuer_schluessel(daten_ordner)?;
    schluessel_anzeigen(daten_ordner, &konfig, &schluessel);
    println!("Starte den Server (neu), damit die Fernwartung aktiv wird.");
    Ok(())
}

/// Abschalten (der Schlüssel bleibt gespeichert). false = war nicht eingerichtet.
pub fn abschalten(daten_ordner: &Path) -> Result<bool, String> {
    let Some(mut k) = konfig_laden(daten_ordner) else { return Ok(false) };
    k.aktiv = false;
    konfig_schreiben(daten_ordner, &k)?;
    Ok(true)
}

/// Läuft die Fernwartung gerade?
static LAEUFT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static STOPP: tokio::sync::Notify = tokio::sync::Notify::const_new();

/// Der Befehl `fernwartung [ein|neu|aus]` in der Server-Konsole – wirkt sofort, ohne Neustart.
pub fn konsole(welt: &GeteilteWelt, daten_ordner: &Path, was: &str) {
    use std::sync::atomic::Ordering;
    let ergebnis: Result<(), String> = (|| {
        match crate::text::normalisieren(was).as_str() {
            "ein" | "an" | "on" => {
                match konfig_laden(daten_ordner).filter(|k| !k.schluessel_hash.is_empty()) {
                    Some(mut k) => {
                        k.aktiv = true;
                        konfig_schreiben(daten_ordner, &k)?;
                        println!("Fernwartung eingeschaltet (Zugangsschlüssel unverändert, einen neuen gibt es mit 'fernwartung neu').");
                    }
                    None => {
                        let (k, s) = neuer_schluessel(daten_ordner)?;
                        schluessel_anzeigen(daten_ordner, &k, &s);
                    }
                }
            }
            "neu" | "schluessel" => {
                let (k, s) = neuer_schluessel(daten_ordner)?;
                schluessel_anzeigen(daten_ordner, &k, &s);
                println!("Der alte Schlüssel gilt ab sofort nicht mehr.");
            }
            "aus" | "off" => {
                if abschalten(daten_ordner)? {
                    if LAEUFT.load(Ordering::SeqCst) {
                        STOPP.notify_one();
                    }
                    println!("Fernwartung abgeschaltet.");
                } else {
                    println!("Die Fernwartung war nicht eingerichtet.");
                }
                return Ok(());
            }
            "" => {
                match konfig_laden(daten_ordner) {
                    Some(k) if k.aktiv && !k.schluessel_hash.is_empty() => println!(
                        "Fernwartung ist eingeschaltet, Port {}{}. Fingerabdruck: {}",
                        k.port,
                        if LAEUFT.load(Ordering::SeqCst) { "" } else { " (läuft gerade nicht – Port belegt?)" },
                        fingerabdruck(daten_ordner).unwrap_or_default()
                    ),
                    Some(_) => println!("Fernwartung ist abgeschaltet. Einschalten: fernwartung ein"),
                    None => println!("Fernwartung ist nicht eingerichtet. Einschalten: fernwartung ein"),
                }
                println!("Befehle: fernwartung ein | neu (neuer Schlüssel) | aus");
                return Ok(());
            }
            _ => {
                println!("Benutzung: fernwartung [ein|neu|aus]");
                return Ok(());
            }
        }
        // Eingeschaltet: starten, falls sie noch nicht läuft
        if !LAEUFT.load(Ordering::SeqCst) {
            tokio::spawn(starten(welt.clone()));
        }
        Ok(())
    })();
    if let Err(e) = ergebnis {
        println!("Fernwartung: {e}");
    }
}

/// Startet die Fernwartung, wenn sie eingerichtet und eingeschaltet ist.
pub async fn starten(welt: GeteilteWelt) {
    use std::sync::atomic::Ordering;
    if LAEUFT.swap(true, Ordering::SeqCst) {
        return; // läuft schon
    }
    lauschen(welt).await;
    LAEUFT.store(false, Ordering::SeqCst);
}

async fn lauschen(welt: GeteilteWelt) {
    let ordner = sperren(&welt).daten_ordner.clone();
    let Some(konfig) = konfig_laden(&ordner).filter(|k| k.aktiv && !k.schluessel_hash.is_empty()) else { return };
    let tls = match tls_config(&ordner) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Fernwartung: kann nicht starten: {e}");
            return;
        }
    };
    let listener = match TcpListener::bind(("0.0.0.0", konfig.port)).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Fernwartung: Port {} ist nicht verfügbar: {e}", konfig.port);
            return;
        }
    };
    println!(
        "Fernwartung aktiv auf Port {} (Fingerabdruck {}).",
        konfig.port,
        fingerabdruck(&ordner).unwrap_or_default()
    );
    let annehmer = tokio_rustls::TlsAcceptor::from(tls);
    loop {
        let (sock, adresse) = tokio::select! {
            r = listener.accept() => match r {
                Ok(v) => v,
                Err(_) => continue,
            },
            _ = STOPP.notified() => {
                println!("Fernwartung beendet.");
                return;
            }
        };
        // Die Einstellungen bei jeder Verbindung neu lesen: ein neuer Schlüssel gilt sofort
        let Some(konfig) = konfig_laden(&ordner).filter(|k| k.aktiv && !k.schluessel_hash.is_empty()) else {
            continue;
        };
        let konfig = Arc::new(konfig);
        let (welt, annehmer) = (welt.clone(), annehmer.clone());
        tokio::spawn(async move {
            if let Err(e) = verbindung(sock, annehmer, welt, konfig, adresse.to_string()).await {
                if e.contains("HandshakeFailure") || e.contains("CertificateUnknown") || e.contains("BadCertificate") {
                    // Der Editor hat nur den Fingerabdruck abgefragt oder ihn nicht akzeptiert
                    println!("Fernwartung {adresse}: Verbindung ohne Anmeldung beendet (Fingerabdruck-Prüfung im Editor).");
                } else {
                    eprintln!("Fernwartung {adresse}: {e}");
                }
            }
        });
    }
}

fn tls_config(ordner: &Path) -> Result<Arc<rustls::ServerConfig>, String> {
    use rustls::pki_types::pem::PemObject;
    use rustls::pki_types::{CertificateDer, PrivateKeyDer};
    let (crt, key) = zertifikat_pfade(ordner);
    let zert = CertificateDer::from_pem_file(&crt).map_err(|e| format!("{}: {e}", crt.display()))?;
    let schl = PrivateKeyDer::from_pem_file(&key).map_err(|e| format!("{}: {e}", key.display()))?;
    let cfg = rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .with_no_client_auth()
        .with_single_cert(vec![zert], schl)
        .map_err(|e| e.to_string())?;
    Ok(Arc::new(cfg))
}

async fn lesen<R: tokio::io::AsyncBufRead + Unpin>(r: &mut R) -> Result<Option<Anfrage>, String> {
    let mut puffer = Vec::new();
    let n = r.take(p::MAX_NACHRICHT).read_until(b'\n', &mut puffer).await.map_err(|e| e.to_string())?;
    if n == 0 {
        return Ok(None);
    }
    serde_json::from_slice(&puffer).map(Some).map_err(|e| format!("Unverständliche Anfrage: {e}"))
}

async fn senden<W: tokio::io::AsyncWrite + Unpin>(w: &mut W, a: &Antwort) -> Result<(), String> {
    let mut zeile = serde_json::to_vec(a).map_err(|e| e.to_string())?;
    zeile.push(b'\n');
    w.write_all(&zeile).await.map_err(|e| e.to_string())?;
    w.flush().await.map_err(|e| e.to_string())
}

async fn verbindung(
    sock: TcpStream,
    annehmer: tokio_rustls::TlsAcceptor,
    welt: GeteilteWelt,
    konfig: Arc<Konfig>,
    adresse: String,
) -> Result<(), String> {
    let tls = tokio::time::timeout(Duration::from_secs(15), annehmer.accept(sock))
        .await
        .map_err(|_| "Zeitüberschreitung beim Verbindungsaufbau".to_string())?
        .map_err(|e| e.to_string())?;
    let (r, mut w) = tokio::io::split(tls);
    let mut leser = BufReader::new(r);

    // Anmeldung
    let anmeldung = tokio::time::timeout(Duration::from_secs(30), lesen(&mut leser))
        .await
        .map_err(|_| "keine Anmeldung".to_string())??;
    match anmeldung {
        Some(Anfrage::Anmelden { schluessel, .. })
            if spieler::passwort_hash(schluessel.trim(), &konfig.salz) == konfig.schluessel_hash =>
        {
            let (version, name, online) = {
                let w = sperren(&welt);
                (update::VERSION.to_string(), w.d.welt.name.clone(), w.spieler.len())
            };
            println!("Fernwartung: Anmeldung von {adresse}");
            senden(&mut w, &Antwort::Angemeldet { version, welt: name, spieler_online: online }).await?;
        }
        _ => {
            // Falsche Versuche bremsen
            tokio::time::sleep(Duration::from_secs(3)).await;
            println!("Fernwartung: falscher Zugangsschlüssel von {adresse}");
            senden(&mut w, &Antwort::Fehler { text: "Der Zugangsschlüssel stimmt nicht.".into() }).await?;
            return Ok(());
        }
    }

    // Anfragen bearbeiten
    loop {
        let anfrage = match tokio::time::timeout(Duration::from_secs(600), lesen(&mut leser)).await {
            Ok(Ok(Some(a))) => a,
            Ok(Ok(None)) | Err(_) => return Ok(()),
            Ok(Err(e)) => {
                senden(&mut w, &Antwort::Fehler { text: e }).await?;
                return Ok(());
            }
        };
        let (antwort, neustart) = bearbeiten(anfrage, &welt, &adresse).await;
        senden(&mut w, &antwort).await?;
        if neustart {
            neu_starten(welt.clone());
            return Ok(());
        }
    }
}

/// Alle Welt-Dateien (relativ, mit '/') eines Ordners.
fn dateien_sammeln(basis: &Path, ordner: &Path, aus: &mut BTreeMap<String, PathBuf>) {
    let Ok(eintraege) = fs::read_dir(ordner) else { return };
    for e in eintraege.flatten() {
        let pfad = e.path();
        if pfad.is_dir() {
            dateien_sammeln(basis, &pfad, aus);
        } else if let Ok(rel) = pfad.strip_prefix(basis) {
            let rel = rel.to_string_lossy().replace('\\', "/");
            if p::pfad_erlaubt(&rel) {
                aus.insert(rel, pfad);
            }
        }
    }
}

fn ordner_kopieren(von: &Path, nach: &Path) -> std::io::Result<()> {
    fs::create_dir_all(nach)?;
    for e in fs::read_dir(von)?.flatten() {
        let pfad = e.path();
        if pfad.is_dir() {
            ordner_kopieren(&pfad, &nach.join(e.file_name()))?;
        } else {
            fs::copy(&pfad, nach.join(e.file_name()))?;
        }
    }
    Ok(())
}

fn aenderungen_anwenden(ordner: &Path, aenderungen: &BTreeMap<String, Option<String>>) -> Result<(), String> {
    for (rel, inhalt) in aenderungen {
        let pfad = ordner.join(rel);
        match inhalt {
            Some(text) => {
                if let Some(o) = pfad.parent() {
                    fs::create_dir_all(o).map_err(|e| format!("{rel}: {e}"))?;
                }
                let tmp = pfad.with_extension("fernwartung.tmp");
                fs::write(&tmp, text).map_err(|e| format!("{rel}: {e}"))?;
                fs::rename(&tmp, &pfad).map_err(|e| format!("{rel}: {e}"))?;
            }
            None => {
                if pfad.exists() {
                    fs::remove_file(&pfad).map_err(|e| format!("{rel}: {e}"))?;
                }
            }
        }
    }
    Ok(())
}

fn pruef_bericht(ergebnis: &Result<daten::Daten, Vec<String>>) -> (bool, String) {
    match ergebnis {
        Ok(d) => {
            let mut t = format!(
                "Alles in Ordnung: {} Räume, {} Gegenstände, {} Monster, {} Zauber.",
                d.raeume.len(),
                d.gegenstaende.len(),
                d.monster.len(),
                d.zauber.len()
            );
            for w in &d.warnungen {
                t.push_str(&format!("\n! {w}"));
            }
            (true, t)
        }
        Err(f) => (false, format!("{} Fehler:\n{}", f.len(), f.iter().map(|x| format!("✗ {x}")).collect::<Vec<_>>().join("\n"))),
    }
}

/// Die laufende Welt neu laden und die Spieler informieren.
fn welt_neu_laden(welt: &GeteilteWelt) -> (bool, String) {
    let mut w = sperren(welt);
    match w.neuladen() {
        Ok(warnungen) => {
            w.sende_alle("{grau}[Die Welt verändert sich ...]{/}", &[]);
            for id in w.spieler.keys().copied().collect::<Vec<_>>() {
                befehle::raum_anzeigen(&w, id);
                befehle::prompt(&w, id);
            }
            (true, format!("Im laufenden Spiel neu geladen ({} Spieler online, {} Hinweise).", w.spieler.len(), warnungen.len()))
        }
        Err(f) => (false, format!("Neuladen abgebrochen, die alte Welt läuft weiter:\n{}", f.join("\n"))),
    }
}

async fn bearbeiten(anfrage: Anfrage, welt: &GeteilteWelt, adresse: &str) -> (Antwort, bool) {
    let ordner = sperren(welt).daten_ordner.clone();
    match anfrage {
        Anfrage::Anmelden { .. } => (Antwort::Fehler { text: "Schon angemeldet.".into() }, false),
        Anfrage::Holen => {
            let mut pfade = BTreeMap::new();
            dateien_sammeln(&ordner, &ordner, &mut pfade);
            let mut dateien = BTreeMap::new();
            for (rel, pfad) in pfade {
                if let Ok(t) = fs::read_to_string(&pfad) {
                    dateien.insert(rel, t);
                }
            }
            (Antwort::Dateien { dateien }, false)
        }
        Anfrage::Pruefen => {
            let o = ordner.clone();
            let ergebnis = tokio::task::spawn_blocking(move || daten::laden(&o)).await;
            let (ok, text) = match ergebnis {
                Ok(e) => pruef_bericht(&e),
                Err(e) => (false, e.to_string()),
            };
            (Antwort::Bericht { ok, text }, false)
        }
        Anfrage::Neuladen => {
            let (ok, text) = welt_neu_laden(welt);
            (Antwort::Bericht { ok, text }, false)
        }
        Anfrage::Uebernehmen { aenderungen, erwartet, neuladen, trotz_fehlern, ueberschreiben } => {
            if let Some(schlecht) = aenderungen.keys().chain(erwartet.keys()).find(|p| !p::pfad_erlaubt(p)) {
                return (Antwort::Fehler { text: format!("Diese Datei ist nicht erlaubt: {schlecht}") }, false);
            }
            // Hat jemand anders inzwischen auf dem Server etwas geändert?
            if !ueberschreiben {
                let konflikte: Vec<String> = erwartet
                    .iter()
                    .filter(|(rel, erw)| {
                        let jetzt = fs::read_to_string(ordner.join(rel)).ok().map(|t| p::hash(&t));
                        jetzt != **erw
                    })
                    .map(|(rel, _)| rel.clone())
                    .collect();
                if !konflikte.is_empty() {
                    return (Antwort::Konflikt { pfade: konflikte }, false);
                }
            }
            // Erst in einer Kopie prüfen
            let probe = programm_ordner(&ordner).join(".fernwartung-probe");
            let o = ordner.clone();
            let a = aenderungen.clone();
            let pr = probe.clone();
            let ergebnis = tokio::task::spawn_blocking(move || {
                let _ = fs::remove_dir_all(&pr);
                ordner_kopieren(&o, &pr).map_err(|e| e.to_string())?;
                aenderungen_anwenden(&pr, &a)?;
                let e = daten::laden(&pr);
                let _ = fs::remove_dir_all(&pr);
                Ok::<_, String>(pruef_bericht(&e))
            })
            .await;
            let (ok, mut text) = match ergebnis {
                Ok(Ok(b)) => b,
                Ok(Err(e)) => return (Antwort::Fehler { text: e }, false),
                Err(e) => return (Antwort::Fehler { text: e.to_string() }, false),
            };
            if !ok && !trotz_fehlern {
                return (
                    Antwort::Bericht { ok: false, text: format!("NICHT übernommen – die Welt hätte Fehler:\n{text}") },
                    false,
                );
            }
            if let Err(e) = aenderungen_anwenden(&ordner, &aenderungen) {
                return (Antwort::Fehler { text: e }, false);
            }
            println!("Fernwartung: {} Datei(en) von {adresse} übernommen.", aenderungen.len());
            text = format!("{} Datei(en) übernommen. {text}", aenderungen.len());
            let mut alles_ok = ok;
            if neuladen && ok {
                let (n_ok, n_text) = welt_neu_laden(welt);
                alles_ok &= n_ok;
                text.push_str(&format!("\n{n_text}"));
            }
            (Antwort::Bericht { ok: alles_ok, text }, false)
        }
        Anfrage::Aktualisieren => {
            let ergebnis = tokio::task::spawn_blocking(|| {
                let r = update::neueste_version(Duration::from_secs(20))?;
                if !update::ist_neuer(&r.version, update::VERSION) {
                    return Ok((false, format!("Die Engine auf dem Server ist schon auf dem neuesten Stand ({}).", update::VERSION)));
                }
                let mut log = Vec::new();
                update::programme_aktualisieren(&r, &[update::ENGINE, update::EDITOR], &mut |t| log.push(t))?;
                Ok::<_, String>((true, format!("Engine auf {} aktualisiert. Der Server startet jetzt neu.\n{}", r.version, log.join("\n"))))
            })
            .await;
            match ergebnis {
                Ok(Ok((neustart, text))) => (Antwort::Bericht { ok: true, text }, neustart),
                Ok(Err(e)) => (Antwort::Bericht { ok: false, text: format!("Update fehlgeschlagen: {e}") }, false),
                Err(e) => (Antwort::Bericht { ok: false, text: e.to_string() }, false),
            }
        }
    }
}

/// Nach einem Update: Spieler speichern, Bescheid geben und mit der neuen Version neu starten.
fn neu_starten(welt: GeteilteWelt) {
    tokio::spawn(async move {
        {
            let w = sperren(&welt);
            w.sende_alle("{hellrot}Der Server wird für ein Update neu gestartet. Bitte gleich wieder verbinden!{/}", &[]);
            w.alle_speichern();
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
        println!("Neustart nach Update ...");
        update::neustart();
    });
}
