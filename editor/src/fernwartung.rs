//! Fernwartung im Editor: Welt vom Server holen, hier bearbeiten, wieder hochladen.
//!
//! Die Welt liegt dabei als "Arbeitskopie" auf diesem Rechner:
//!   <arbeitskopie>/daten/        die Weltdateien (der Editor öffnet diesen Ordner)
//!   <arbeitskopie>/server.json   Adresse, Fingerabdruck und Stand der letzten Übertragung
//!
//! Beim Hochladen schickt der Editor nur geänderte Dateien. Der Server prüft sie
//! zuerst in einer Kopie und übernimmt sie nur, wenn die Welt fehlerfrei lädt.

use crate::fernwartung_protokoll::{self as p, Anfrage, Antwort};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Was über die Verbindung zum Server gespeichert wird.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Arbeitskopie {
    pub adresse: String,
    pub fingerabdruck: String,
    /// Nur gespeichert, wenn "Schlüssel merken" an ist
    #[serde(default)]
    pub schluessel: Option<String>,
    /// Stand beim letzten Holen/Hochladen: Pfad -> Hash
    #[serde(default)]
    pub dateien: BTreeMap<String, String>,
}

impl Arbeitskopie {
    pub fn datei(ordner: &Path) -> PathBuf {
        ordner.join("server.json")
    }
    /// Gehört der Datenordner (…/daten) zu einer Arbeitskopie?
    pub fn zu_datenordner(daten: &Path) -> Option<(PathBuf, Arbeitskopie)> {
        let ordner = daten.parent()?.to_path_buf();
        let text = fs::read_to_string(Self::datei(&ordner)).ok()?;
        serde_json::from_str(&text).ok().map(|a| (ordner, a))
    }
    pub fn speichern(&self, ordner: &Path) -> Result<(), String> {
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(Self::datei(ordner), text).map_err(|e| e.to_string())
    }
}

/// Vorschlag für den Ordner der Arbeitskopie, z.B. Dokumente\RMengine-Server\meinserver
pub fn standard_ordner(adresse: &str) -> PathBuf {
    let host: String = adresse
        .split(':')
        .next()
        .unwrap_or("server")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' })
        .collect();
    let basis = std::env::var_os("USERPROFILE")
        .map(|h| PathBuf::from(h).join("Documents"))
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."));
    basis.join("RMengine-Server").join(if host.is_empty() { "server".into() } else { host })
}

// ---------------------------------------------------------------------------
// Verbindung
// ---------------------------------------------------------------------------

pub enum VerbindungsFehler {
    /// Erste Verbindung: Der Benutzer soll den Fingerabdruck bestätigen
    NeuerFingerabdruck(String),
    /// Der Server zeigt ein anderes Zertifikat als beim letzten Mal
    FalscherFingerabdruck { erwartet: String, bekommen: String },
    Text(String),
}

impl From<String> for VerbindungsFehler {
    fn from(t: String) -> Self {
        VerbindungsFehler::Text(t)
    }
}

/// Akzeptiert genau das Zertifikat mit dem gemerkten Fingerabdruck.
#[derive(Debug)]
struct Fingerabdruckpruefer {
    erwartet: Option<String>,
    gesehen: Mutex<Option<String>>,
    anbieter: Arc<rustls::crypto::CryptoProvider>,
}

impl rustls::client::danger::ServerCertVerifier for Fingerabdruckpruefer {
    fn verify_server_cert(
        &self,
        zert: &rustls::pki_types::CertificateDer<'_>,
        _: &[rustls::pki_types::CertificateDer<'_>],
        _: &rustls::pki_types::ServerName<'_>,
        _: &[u8],
        _: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        let fp = p::fingerabdruck(zert);
        *self.gesehen.lock().unwrap() = Some(fp.clone());
        if self.erwartet.as_deref() == Some(fp.as_str()) {
            Ok(rustls::client::danger::ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::General("Fingerabdruck".into()))
        }
    }
    fn verify_tls12_signature(
        &self,
        nachricht: &[u8],
        zert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(nachricht, zert, dss, &self.anbieter.signature_verification_algorithms)
    }
    fn verify_tls13_signature(
        &self,
        nachricht: &[u8],
        zert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(nachricht, zert, dss, &self.anbieter.signature_verification_algorithms)
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.anbieter.signature_verification_algorithms.supported_schemes()
    }
}

pub struct Sitzung {
    strom: BufReader<rustls::StreamOwned<rustls::ClientConnection, TcpStream>>,
    pub version: String,
    pub welt: String,
    pub spieler_online: usize,
}

fn adresse_mit_port(adresse: &str) -> String {
    let a = adresse.trim();
    // "host" oder "host:port" (IPv6 in eckigen Klammern: "[::1]:4001")
    if a.rsplit_once(':').is_some_and(|(h, port)| port.parse::<u16>().is_ok() && (!h.contains(':') || h.ends_with(']'))) {
        a.to_string()
    } else {
        format!("{a}:{}", p::STANDARD_PORT)
    }
}

impl Sitzung {
    /// Verbinden und anmelden. Ohne `fingerabdruck` wird nur der Fingerabdruck
    /// abgefragt (VerbindungsFehler::NeuerFingerabdruck), der Schlüssel aber NICHT gesendet.
    pub fn oeffnen(adresse: &str, fingerabdruck: Option<&str>, schluessel: &str) -> Result<Sitzung, VerbindungsFehler> {
        let ziel = adresse_mit_port(adresse);
        let sockadresse = ziel
            .to_socket_addrs()
            .map_err(|e| format!("Adresse '{ziel}' nicht gefunden: {e}"))?
            .next()
            .ok_or_else(|| format!("Adresse '{ziel}' nicht gefunden"))?;
        let tcp = TcpStream::connect_timeout(&sockadresse, Duration::from_secs(10))
            .map_err(|e| format!("Keine Verbindung zu {ziel}: {e}\n(Läuft der Server? Ist der Port in der Firewall frei?)"))?;
        let _ = tcp.set_read_timeout(Some(Duration::from_secs(180)));
        let _ = tcp.set_write_timeout(Some(Duration::from_secs(60)));

        let anbieter = Arc::new(rustls::crypto::ring::default_provider());
        let pruefer = Arc::new(Fingerabdruckpruefer {
            erwartet: fingerabdruck.map(String::from),
            gesehen: Mutex::new(None),
            anbieter: anbieter.clone(),
        });
        let cfg = rustls::ClientConfig::builder_with_provider(anbieter)
            .with_safe_default_protocol_versions()
            .map_err(|e| e.to_string())?
            .dangerous()
            .with_custom_certificate_verifier(pruefer.clone())
            .with_no_client_auth();
        let name = rustls::pki_types::ServerName::try_from(p::SERVERNAME).map_err(|e| e.to_string())?;
        let verbindung = rustls::ClientConnection::new(Arc::new(cfg), name).map_err(|e| e.to_string())?;
        let mut strom = rustls::StreamOwned::new(verbindung, tcp);

        // Handschlag erzwingen, um den Fingerabdruck zu prüfen
        while strom.conn.is_handshaking() {
            if let Err(e) = strom.conn.complete_io(&mut strom.sock) {
                let gesehen = pruefer.gesehen.lock().unwrap().clone();
                return Err(match (gesehen, fingerabdruck) {
                    (Some(g), None) => VerbindungsFehler::NeuerFingerabdruck(g),
                    (Some(g), Some(e)) if g != e => VerbindungsFehler::FalscherFingerabdruck { erwartet: e.into(), bekommen: g },
                    _ => VerbindungsFehler::Text(format!("Verschlüsselte Verbindung fehlgeschlagen: {e}")),
                });
            }
        }

        let mut sitzung = Sitzung { strom: BufReader::new(strom), version: String::new(), welt: String::new(), spieler_online: 0 };
        let antwort = sitzung.anfrage(&Anfrage::Anmelden {
            schluessel: schluessel.trim().to_string(),
            version: crate::update::VERSION.to_string(),
        })?;
        match antwort {
            Antwort::Angemeldet { version, welt, spieler_online } => {
                sitzung.version = version;
                sitzung.welt = welt;
                sitzung.spieler_online = spieler_online;
                Ok(sitzung)
            }
            Antwort::Fehler { text } => Err(VerbindungsFehler::Text(text)),
            _ => Err(VerbindungsFehler::Text("Unerwartete Antwort vom Server.".into())),
        }
    }

    pub fn anfrage(&mut self, a: &Anfrage) -> Result<Antwort, String> {
        let mut zeile = serde_json::to_vec(a).map_err(|e| e.to_string())?;
        zeile.push(b'\n');
        let s = self.strom.get_mut();
        s.write_all(&zeile).map_err(|e| format!("Senden fehlgeschlagen: {e}"))?;
        s.flush().map_err(|e| e.to_string())?;
        let mut puffer = Vec::new();
        (&mut self.strom).take(p::MAX_NACHRICHT).read_until(b'\n', &mut puffer).map_err(|e| format!("Empfangen fehlgeschlagen: {e}"))?;
        if puffer.is_empty() {
            return Err("Der Server hat die Verbindung beendet.".into());
        }
        serde_json::from_slice(&puffer).map_err(|e| format!("Unverständliche Antwort vom Server: {e}"))
    }
}

// ---------------------------------------------------------------------------
// Arbeitskopie: Dateien vergleichen, holen, hochladen
// ---------------------------------------------------------------------------

/// Alle Welt-Dateien im lokalen Datenordner: Pfad -> Inhalt
pub fn lokale_dateien(daten: &Path) -> BTreeMap<String, String> {
    fn sammeln(basis: &Path, ordner: &Path, aus: &mut BTreeMap<String, String>) {
        let Ok(e) = fs::read_dir(ordner) else { return };
        for e in e.flatten() {
            let pfad = e.path();
            if pfad.is_dir() {
                sammeln(basis, &pfad, aus);
            } else if let Ok(rel) = pfad.strip_prefix(basis) {
                let rel = rel.to_string_lossy().replace('\\', "/");
                if p::pfad_erlaubt(&rel) {
                    if let Ok(t) = fs::read_to_string(&pfad) {
                        aus.insert(rel, t);
                    }
                }
            }
        }
    }
    let mut aus = BTreeMap::new();
    sammeln(daten, daten, &mut aus);
    aus
}

/// Was hat sich lokal seit dem letzten Holen/Hochladen geändert? Pfad -> neuer Inhalt (None = gelöscht)
pub fn lokale_aenderungen(ordner: &Path, kopie: &Arbeitskopie) -> BTreeMap<String, Option<String>> {
    let lokal = lokale_dateien(&ordner.join("daten"));
    let mut aus = BTreeMap::new();
    for (pfad, inhalt) in &lokal {
        if kopie.dateien.get(pfad) != Some(&p::hash(inhalt)) {
            aus.insert(pfad.clone(), Some(inhalt.clone()));
        }
    }
    for pfad in kopie.dateien.keys() {
        if !lokal.contains_key(pfad) {
            aus.insert(pfad.clone(), None);
        }
    }
    aus
}

/// Holt die ganze Welt vom Server in die Arbeitskopie.
pub fn holen(ordner: &Path, adresse: &str, fingerabdruck: &str, schluessel: &str, merken: bool) -> Result<String, VerbindungsFehler> {
    let mut s = Sitzung::oeffnen(adresse, Some(fingerabdruck), schluessel)?;
    let dateien = match s.anfrage(&Anfrage::Holen)? {
        Antwort::Dateien { dateien } => dateien,
        Antwort::Fehler { text } => return Err(VerbindungsFehler::Text(text)),
        _ => return Err(VerbindungsFehler::Text("Unerwartete Antwort vom Server.".into())),
    };
    if !dateien.contains_key("welt.toml") {
        return Err(VerbindungsFehler::Text("Auf dem Server fehlt welt.toml.".into()));
    }
    let daten = ordner.join("daten");
    // Lokale Welt-Dateien, die es auf dem Server nicht (mehr) gibt, entfernen
    for pfad in lokale_dateien(&daten).keys() {
        if !dateien.contains_key(pfad) {
            let _ = fs::remove_file(daten.join(pfad));
        }
    }
    for (pfad, inhalt) in &dateien {
        if !p::pfad_erlaubt(pfad) {
            continue; // nur zur Sicherheit – der Server schickt so etwas nicht
        }
        let ziel = daten.join(pfad);
        if let Some(o) = ziel.parent() {
            fs::create_dir_all(o).map_err(|e| format!("{}: {e}", o.display()))?;
        }
        fs::write(&ziel, inhalt).map_err(|e| format!("{}: {e}", ziel.display()))?;
    }
    let kopie = Arbeitskopie {
        adresse: adresse.trim().to_string(),
        fingerabdruck: fingerabdruck.to_string(),
        schluessel: merken.then(|| schluessel.trim().to_string()),
        dateien: dateien.iter().map(|(k, v)| (k.clone(), p::hash(v))).collect(),
    };
    kopie.speichern(ordner)?;
    Ok(format!(
        "{} Dateien von '{}' geholt (Engine {}, {} Spieler online).",
        dateien.len(),
        s.welt,
        s.version,
        s.spieler_online
    ))
}

pub enum Hochgeladen {
    Fertig { ok: bool, text: String },
    Konflikt(Vec<String>),
}

/// Lädt alle lokalen Änderungen hoch.
pub fn hochladen(
    ordner: &Path,
    schluessel: &str,
    neuladen: bool,
    trotz_fehlern: bool,
    ueberschreiben: bool,
) -> Result<Hochgeladen, VerbindungsFehler> {
    let text = fs::read_to_string(Arbeitskopie::datei(ordner)).map_err(|e| e.to_string())?;
    let mut kopie: Arbeitskopie = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let aenderungen = lokale_aenderungen(ordner, &kopie);
    if aenderungen.is_empty() {
        return Ok(Hochgeladen::Fertig { ok: true, text: "Es gibt keine Änderungen zum Hochladen.".into() });
    }
    let erwartet: BTreeMap<String, Option<String>> =
        aenderungen.keys().map(|p| (p.clone(), kopie.dateien.get(p).cloned())).collect();
    let mut s = Sitzung::oeffnen(&kopie.adresse, Some(&kopie.fingerabdruck), schluessel)?;
    let antwort = s.anfrage(&Anfrage::Uebernehmen { aenderungen: aenderungen.clone(), erwartet, neuladen, trotz_fehlern, ueberschreiben })?;
    match antwort {
        Antwort::Konflikt { pfade } => Ok(Hochgeladen::Konflikt(pfade)),
        Antwort::Bericht { ok, text } => {
            // Übernommen? Dann den neuen Stand merken.
            if !text.starts_with("NICHT") {
                for (pfad, inhalt) in &aenderungen {
                    match inhalt {
                        Some(t) => {
                            kopie.dateien.insert(pfad.clone(), p::hash(t));
                        }
                        None => {
                            kopie.dateien.remove(pfad);
                        }
                    }
                }
                kopie.speichern(ordner)?;
            }
            let liste: Vec<String> = aenderungen
                .iter()
                .map(|(p, i)| format!("  {} {p}", if i.is_some() { "⬆" } else { "✖" }))
                .collect();
            Ok(Hochgeladen::Fertig { ok, text: format!("{text}\n\nDateien:\n{}", liste.join("\n")) })
        }
        Antwort::Fehler { text } => Err(VerbindungsFehler::Text(text)),
        _ => Err(VerbindungsFehler::Text("Unerwartete Antwort vom Server.".into())),
    }
}

/// Eine einfache Anfrage (Prüfen, Neuladen, Aktualisieren) – gibt (ok, Text) zurück.
pub fn einfach(kopie: &Arbeitskopie, schluessel: &str, a: Anfrage) -> Result<(bool, String), VerbindungsFehler> {
    let mut s = Sitzung::oeffnen(&kopie.adresse, Some(&kopie.fingerabdruck), schluessel)?;
    match s.anfrage(&a)? {
        Antwort::Bericht { ok, text } => Ok((ok, text)),
        Antwort::Fehler { text } => Ok((false, text)),
        _ => Ok((false, "Unerwartete Antwort vom Server.".into())),
    }
}

/// Die Fehlermeldung so, dass man versteht, was zu tun ist.
pub fn fehlertext(f: &VerbindungsFehler) -> String {
    match f {
        VerbindungsFehler::Text(t) => t.clone(),
        VerbindungsFehler::NeuerFingerabdruck(fp) => format!("Neuer Server mit Fingerabdruck {fp}"),
        VerbindungsFehler::FalscherFingerabdruck { erwartet, bekommen } => format!(
            "ACHTUNG: Der Server zeigt einen anderen Fingerabdruck als beim letzten Mal!\n\
             Erwartet: {erwartet}\nBekommen: {bekommen}\n\n\
             Das passiert, wenn auf dem Server das Zertifikat neu erzeugt wurde – oder wenn sich\n\
             jemand dazwischengeschaltet hat. Verbinde dich nur neu, wenn du sicher bist."
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adresse_bekommt_standardport() {
        assert_eq!(adresse_mit_port("meinserver.de"), "meinserver.de:4001");
        assert_eq!(adresse_mit_port("1.2.3.4:5000"), "1.2.3.4:5000");
        assert_eq!(adresse_mit_port("[::1]:4001"), "[::1]:4001");
    }

    /// Gegen einen echten Server: RMENGINE_TESTSERVER=127.0.0.1 RMENGINE_TESTSCHLUESSEL=… cargo test -- --ignored
    #[test]
    #[ignore]
    fn gegen_echten_server() {
        let adresse = std::env::var("RMENGINE_TESTSERVER").unwrap();
        let schluessel = std::env::var("RMENGINE_TESTSCHLUESSEL").unwrap();
        // 1. Erster Kontakt: nur Fingerabdruck, kein Schlüssel
        let fp = match Sitzung::oeffnen(&adresse, None, "") {
            Err(VerbindungsFehler::NeuerFingerabdruck(fp)) => fp,
            _ => panic!("erwartet: NeuerFingerabdruck"),
        };
        // 2. Falscher Fingerabdruck wird erkannt
        assert!(matches!(Sitzung::oeffnen(&adresse, Some("AA:BB"), &schluessel), Err(VerbindungsFehler::FalscherFingerabdruck { .. })));
        // 3. Falscher Schlüssel
        assert!(matches!(Sitzung::oeffnen(&adresse, Some(&fp), "FALSCH"), Err(VerbindungsFehler::Text(_))));
        // 4. Holen
        let ordner = std::env::temp_dir().join(format!("rmeditor_fw_{}", std::process::id()));
        let _ = fs::remove_dir_all(&ordner);
        holen(&ordner, &adresse, &fp, &schluessel, true).map_err(|f| fehlertext(&f)).unwrap();
        assert!(ordner.join("daten/welt.toml").exists());
        let kopie: Arbeitskopie = serde_json::from_str(&fs::read_to_string(Arbeitskopie::datei(&ordner)).unwrap()).unwrap();
        assert!(lokale_aenderungen(&ordner, &kopie).is_empty());
        // 5. Kaputte Änderung wird abgelehnt, gute übernommen
        let welt = ordner.join("daten/welt.toml");
        let original = fs::read_to_string(&welt).unwrap();
        fs::write(&welt, original.replace("startraum  =", "startraum  = \"nix\"\n#")).unwrap();
        match hochladen(&ordner, &schluessel, false, false, false).map_err(|f| fehlertext(&f)).unwrap() {
            Hochgeladen::Fertig { ok, text } => assert!(!ok && text.starts_with("NICHT"), "{text}"),
            _ => panic!(),
        }
        fs::write(&welt, original.replace("Hier beginnt dein Abenteuer.", "Hier beginnt dein Abenteuer!")).unwrap();
        fs::write(ordner.join("daten/raeume/extra.toml"), "# leer\n").unwrap();
        match hochladen(&ordner, &schluessel, true, false, false).map_err(|f| fehlertext(&f)).unwrap() {
            Hochgeladen::Fertig { ok, text } => assert!(ok, "{text}"),
            _ => panic!(),
        }
        let kopie: Arbeitskopie = serde_json::from_str(&fs::read_to_string(Arbeitskopie::datei(&ordner)).unwrap()).unwrap();
        assert!(lokale_aenderungen(&ordner, &kopie).is_empty());
        // 6. Löschen wird übertragen
        fs::remove_file(ordner.join("daten/raeume/extra.toml")).unwrap();
        assert!(matches!(hochladen(&ordner, &schluessel, false, false, false), Ok(Hochgeladen::Fertig { ok: true, .. })));
        let _ = fs::remove_dir_all(&ordner);
    }
}
