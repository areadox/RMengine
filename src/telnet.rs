//! Telnet und die gängigen MUD-Protokolle.
//!
//! Beim Verbinden bietet der Server an, was er kann; der Client sucht sich aus,
//! was er versteht. Ein einfaches Telnet-Programm lehnt einfach alles ab und
//! bekommt ganz normalen Text.
//!
//! | Protokoll | Wozu                                                        |
//! |-----------|-------------------------------------------------------------|
//! | NAWS      | Fenstergröße des Clients (für den Zeilenumbruch)            |
//! | TTYPE     | Name des Clients, mit MTTS auch seine Fähigkeiten           |
//! | EOR / GA  | markiert das Ende des Prompts                               |
//! | CHARSET   | Zeichensatz (UTF-8, für alte Clients auch ISO-8859-1)       |
//! | MCCP2     | Kompression aller Ausgaben                                  |
//! | GMCP      | Daten im Hintergrund (Lebensbalken, Automapper, Chat)       |
//! | MSDP      | dasselbe für Clients, die MSDP statt GMCP nutzen            |
//! | MSSP      | Serverinfos für MUD-Verzeichnisse                           |

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, MutexGuard};
use tokio::sync::mpsc::UnboundedSender;

// Telnet-Befehle
pub const IAC: u8 = 255;
pub const DONT: u8 = 254;
pub const DO: u8 = 253;
pub const WONT: u8 = 252;
pub const WILL: u8 = 251;
pub const SB: u8 = 250;
pub const GA: u8 = 249;
pub const SE: u8 = 240;
pub const EOR_BEFEHL: u8 = 239;

// Optionen
pub const ECHO: u8 = 1;
pub const TTYPE: u8 = 24;
pub const EOR: u8 = 25;
pub const NAWS: u8 = 31;
pub const CHARSET: u8 = 42;
pub const MSDP: u8 = 69;
pub const MSSP: u8 = 70;
pub const MCCP2: u8 = 86;
pub const GMCP: u8 = 201;

// MSDP-Bausteine
const MSDP_VAR: u8 = 1;
const MSDP_VAL: u8 = 2;
const MSDP_TABELLE_AUF: u8 = 3;
const MSDP_TABELLE_ZU: u8 = 4;
const MSDP_LISTE_AUF: u8 = 5;
const MSDP_LISTE_ZU: u8 = 6;

const MAX_ZEILE: usize = 4096;
const MAX_UNTERVERHANDLUNG: usize = 64 * 1024;

/// Was an den Client geht.
pub enum Ausgabe {
    Daten(Vec<u8>),
    /// Ab jetzt wird alles komprimiert (MCCP2)
    KompressionStarten,
}
pub type Sender = UnboundedSender<Ausgabe>;

pub fn senden(tx: &Sender, b: Vec<u8>) {
    let _ = tx.send(Ausgabe::Daten(b));
}

/// Was der Client kann und was mit ihm ausgehandelt wurde.
#[derive(Default)]
pub struct Protokolle {
    pub gmcp: bool,
    pub msdp: bool,
    pub eor: bool,
    pub mccp: bool,
    /// Der Client möchte ISO-8859-1 statt UTF-8
    pub latin1: bool,
    /// Fenstergröße (Breite, Höhe)
    pub naws: Option<(u16, u16)>,
    pub client: String,
    pub client_version: String,
    pub terminal: String,
    /// MTTS-Fähigkeiten (1 = ANSI, 4 = UTF-8, 8 = 256 Farben, ...)
    pub mtts: u32,
    /// Was der Client per GMCP Core.Supports.Set angemeldet hat
    pub gmcp_module: BTreeSet<String>,
    /// MSDP-Werte, die der Client laufend haben möchte
    pub msdp_gemeldet: BTreeSet<String>,
    /// Zuletzt gesendete Werte – damit nur Änderungen geschickt werden
    pub zuletzt: BTreeMap<String, String>,
    ttype_runde: u8,
    ttype_letzter: String,
    abgelehnt: BTreeSet<(u8, u8)>,
}

impl Protokolle {
    /// "Mudlet 4.17.2" oder "unbekannt"
    pub fn client_name(&self) -> String {
        match (self.client.is_empty(), self.client_version.is_empty()) {
            (true, _) => "unbekannt".into(),
            (false, true) => self.client.clone(),
            (false, false) => format!("{} {}", self.client, self.client_version),
        }
    }

    /// Kurze Liste der aktiven Protokolle, z.B. "GMCP, MCCP, NAWS"
    pub fn aktiv(&self) -> String {
        let mut v = Vec::new();
        for (an, name) in [
            (self.gmcp, "GMCP"),
            (self.msdp, "MSDP"),
            (self.mccp, "MCCP"),
            (self.eor, "EOR"),
            (self.naws.is_some(), "NAWS"),
            (self.mtts > 0, "MTTS"),
            (self.latin1, "Latin-1"),
        ] {
            if an {
                v.push(name);
            }
        }
        if v.is_empty() {
            "keine".into()
        } else {
            v.join(", ")
        }
    }

    /// Wandelt fertigen UTF-8-Text für den Client um.
    pub fn kodieren(&self, b: Vec<u8>) -> Vec<u8> {
        if !self.latin1 {
            return b; // UTF-8 enthält nie das Byte 255, muss also nicht maskiert werden
        }
        let mut aus = Vec::with_capacity(b.len());
        for c in String::from_utf8_lossy(&b).chars() {
            match c {
                'ÿ' => aus.extend([IAC, IAC]),
                '\0'..='þ' => aus.push(c as u8),
                // Zeichen, die es in ISO-8859-1 nicht gibt
                '„' | '“' | '”' => aus.push(b'"'),
                '‚' | '‘' | '’' => aus.push(b'\''),
                '–' | '—' => aus.push(b'-'),
                '…' => aus.extend(b"..."),
                '€' => aus.extend(b"EUR"),
                _ => aus.push(b'?'),
            }
        }
        aus
    }
}

pub type Geteilt = Arc<Mutex<Protokolle>>;

pub fn sperren(p: &Geteilt) -> MutexGuard<'_, Protokolle> {
    p.lock().unwrap_or_else(|e| e.into_inner())
}

/// Das schickt der Server gleich nach dem Verbinden: "Ich kann ..., kannst du ...?"
pub fn begruessung() -> Vec<u8> {
    let mut b = Vec::new();
    for opt in [MCCP2, GMCP, MSDP, MSSP, EOR, CHARSET] {
        b.extend([IAC, WILL, opt]);
    }
    for opt in [NAWS, TTYPE] {
        b.extend([IAC, DO, opt]);
    }
    b
}

/// Das Ende des Prompts markieren (EOR, wenn ausgehandelt, sonst GA).
pub fn prompt_ende(p: &Protokolle) -> Vec<u8> {
    if p.eor {
        vec![IAC, EOR_BEFEHL]
    } else {
        vec![IAC, GA]
    }
}

fn unterverhandlung(opt: u8, inhalt: &[u8]) -> Vec<u8> {
    let mut b = vec![IAC, SB, opt];
    for &c in inhalt {
        if c == IAC {
            b.push(IAC);
        }
        b.push(c);
    }
    b.extend([IAC, SE]);
    b
}

/// Ein GMCP-Paket, z.B. gmcp("Char.Vitals", &json!({"hp": 10}))
pub fn gmcp(paket: &str, daten: &serde_json::Value) -> Vec<u8> {
    let text = if daten.is_null() { paket.to_string() } else { format!("{paket} {daten}") };
    unterverhandlung(GMCP, text.as_bytes())
}

/// Ein MSDP-Wert: Text, Liste oder Tabelle.
#[derive(Clone, Debug, PartialEq)]
pub enum MsdpWert {
    Text(String),
    Liste(Vec<MsdpWert>),
    Tabelle(Vec<(String, MsdpWert)>),
}

impl MsdpWert {
    pub fn text(t: impl ToString) -> MsdpWert {
        MsdpWert::Text(t.to_string())
    }
    fn schreiben(&self, b: &mut Vec<u8>) {
        match self {
            MsdpWert::Text(t) => b.extend(t.bytes().filter(|c| *c > 6)),
            MsdpWert::Liste(l) => {
                b.push(MSDP_LISTE_AUF);
                for w in l {
                    b.push(MSDP_VAL);
                    w.schreiben(b);
                }
                b.push(MSDP_LISTE_ZU);
            }
            MsdpWert::Tabelle(t) => {
                b.push(MSDP_TABELLE_AUF);
                for (k, w) in t {
                    b.push(MSDP_VAR);
                    b.extend(k.bytes());
                    b.push(MSDP_VAL);
                    w.schreiben(b);
                }
                b.push(MSDP_TABELLE_ZU);
            }
        }
    }
}

/// MSDP-Variablen an den Client.
pub fn msdp(werte: &[(String, MsdpWert)]) -> Vec<u8> {
    let mut inhalt = Vec::new();
    for (k, w) in werte {
        inhalt.push(MSDP_VAR);
        inhalt.extend(k.bytes());
        inhalt.push(MSDP_VAL);
        w.schreiben(&mut inhalt);
    }
    unterverhandlung(MSDP, &inhalt)
}

/// MSSP-Serverinfos.
pub fn mssp(werte: &[(String, String)]) -> Vec<u8> {
    let mut inhalt = Vec::new();
    for (k, w) in werte {
        inhalt.push(1); // MSSP_VAR
        inhalt.extend(k.bytes().filter(|c| *c > 2));
        inhalt.push(2); // MSSP_VAL
        inhalt.extend(w.bytes().filter(|c| *c > 2));
    }
    unterverhandlung(MSSP, &inhalt)
}

// ---------------------------------------------------------------------------
// Eingaben vom Client zerlegen
// ---------------------------------------------------------------------------

/// Was aus dem Datenstrom herauskommt.
#[derive(Debug, PartialEq)]
pub enum Ereignis {
    Zeile(String),
    Befehl(u8, u8),
    Unter(u8, Vec<u8>),
}

#[derive(Default)]
enum Zustand {
    #[default]
    Text,
    Iac,
    Option(u8),
    Unter,
    UnterIac,
}

/// Zerlegt den Datenstrom in Zeilen und Telnet-Befehle.
#[derive(Default)]
pub struct Zerleger {
    zustand: Zustand,
    zeile: Vec<u8>,
    unter: Vec<u8>,
    nach_cr: bool,
}

impl Zerleger {
    pub fn fuettern(&mut self, daten: &[u8], aus: &mut Vec<Ereignis>) {
        for &b in daten {
            match self.zustand {
                Zustand::Text => {
                    if b == IAC {
                        self.zustand = Zustand::Iac;
                    } else {
                        self.zeichen(b, aus);
                    }
                }
                Zustand::Iac => {
                    self.zustand = Zustand::Text;
                    match b {
                        IAC => self.zeichen(IAC, aus),
                        WILL | WONT | DO | DONT => self.zustand = Zustand::Option(b),
                        SB => {
                            self.unter.clear();
                            self.zustand = Zustand::Unter;
                        }
                        _ => {} // NOP, GA, AYT ... – nicht wichtig
                    }
                }
                Zustand::Option(befehl) => {
                    aus.push(Ereignis::Befehl(befehl, b));
                    self.zustand = Zustand::Text;
                }
                Zustand::Unter => {
                    if b == IAC {
                        self.zustand = Zustand::UnterIac;
                    } else if self.unter.len() < MAX_UNTERVERHANDLUNG {
                        self.unter.push(b);
                    }
                }
                Zustand::UnterIac => match b {
                    IAC => {
                        if self.unter.len() < MAX_UNTERVERHANDLUNG {
                            self.unter.push(IAC);
                        }
                        self.zustand = Zustand::Unter;
                    }
                    _ => {
                        // SE (oder ein kaputter Client): Unterverhandlung ist fertig
                        if let Some((&opt, rest)) = self.unter.split_first() {
                            aus.push(Ereignis::Unter(opt, rest.to_vec()));
                        }
                        self.unter.clear();
                        self.zustand = Zustand::Text;
                    }
                },
            }
        }
    }

    fn zeichen(&mut self, b: u8, aus: &mut Vec<Ereignis>) {
        let nach_cr = std::mem::replace(&mut self.nach_cr, false);
        match b {
            b'\r' => {
                self.nach_cr = true;
                self.zeile_fertig(aus);
            }
            b'\n' | 0 if nach_cr => {} // CR LF bzw. CR NUL: schon erledigt
            b'\n' => self.zeile_fertig(aus),
            8 | 127 => {
                // Rücktaste: ein ganzes (UTF-8-)Zeichen löschen
                while let Some(c) = self.zeile.pop() {
                    if c & 0xC0 != 0x80 {
                        break;
                    }
                }
            }
            b'\t' => self.zeile.push(b' '),
            0..=31 => {}
            _ if self.zeile.len() < MAX_ZEILE => self.zeile.push(b),
            _ => {}
        }
    }

    fn zeile_fertig(&mut self, aus: &mut Vec<Ereignis>) {
        let roh = std::mem::take(&mut self.zeile);
        // UTF-8 – oder sonst ISO-8859-1 (alte Clients schicken Umlaute so)
        let text = String::from_utf8(roh).unwrap_or_else(|e| e.into_bytes().iter().map(|&b| b as char).collect());
        aus.push(Ereignis::Zeile(text.trim().to_string()));
    }
}

// ---------------------------------------------------------------------------
// Aushandeln
// ---------------------------------------------------------------------------

/// Was der Server nach einer Nachricht des Clients tun soll
/// (dafür braucht er die Welt, deshalb erledigt das netz.rs).
#[derive(Debug, PartialEq)]
pub enum Aktion {
    /// Serverinfos schicken
    Mssp,
    /// MSDP-Befehle des Clients: (Befehl, Werte), z.B. ("REPORT", ["HEALTH", "MANA"])
    Msdp(Vec<(String, Vec<String>)>),
    /// Ein GMCP-Paket, das nicht schon hier erledigt wurde
    Gmcp(String, serde_json::Value),
}

/// Antwortet einmal mit WONT/DONT auf etwas, das wir nicht können.
fn ablehnen(p: &mut Protokolle, tx: &Sender, befehl: u8, opt: u8) {
    if p.abgelehnt.insert((befehl, opt)) {
        senden(tx, vec![IAC, befehl, opt]);
    }
}

/// Ein DO/DONT/WILL/WONT vom Client.
pub fn befehl(p: &mut Protokolle, tx: &Sender, befehl: u8, opt: u8) -> Option<Aktion> {
    match (befehl, opt) {
        (DO, MCCP2) => {
            if !p.mccp {
                p.mccp = true;
                senden(tx, vec![IAC, SB, MCCP2, IAC, SE]);
                let _ = tx.send(Ausgabe::KompressionStarten);
            }
        }
        (DO, GMCP) => p.gmcp = true,
        (DO, MSDP) => p.msdp = true,
        (DO, EOR) => p.eor = true,
        (DO, MSSP) => return Some(Aktion::Mssp),
        (DO, CHARSET) => senden(tx, unterverhandlung(CHARSET, b"\x01;UTF-8;ISO-8859-1")),
        (DONT, GMCP) => p.gmcp = false,
        (DONT, MSDP) => p.msdp = false,
        (DONT, EOR) => p.eor = false,
        // Antworten auf unser Passwort-Echo: nichts zu tun
        (DO | DONT, ECHO) => {}
        (DO, _) => ablehnen(p, tx, WONT, opt),
        (WILL, NAWS) => {}
        (WILL, TTYPE) => {
            if p.ttype_runde == 0 {
                p.ttype_runde = 1;
                senden(tx, unterverhandlung(TTYPE, &[1]));
            }
        }
        (WILL, _) => ablehnen(p, tx, DONT, opt),
        _ => {}
    }
    None
}

/// Eine Unterverhandlung (IAC SB ... IAC SE) vom Client.
pub fn unter(p: &mut Protokolle, tx: &Sender, opt: u8, d: &[u8]) -> Option<Aktion> {
    match opt {
        NAWS if d.len() >= 4 => {
            let breite = u16::from_be_bytes([d[0], d[1]]);
            let hoehe = u16::from_be_bytes([d[2], d[3]]);
            p.naws = (breite > 0).then_some((breite, hoehe));
        }
        TTYPE if d.first() == Some(&0) => {
            let name = String::from_utf8_lossy(&d[1..]).trim().to_string();
            let wiederholt = name.eq_ignore_ascii_case(&p.ttype_letzter);
            match p.ttype_runde {
                1 => {
                    if p.client.is_empty() {
                        p.client = name.clone();
                    }
                }
                2 => p.terminal = name.clone(),
                _ => {}
            }
            if let Some(zahl) = name.strip_prefix("MTTS ") {
                p.mtts = zahl.trim().parse().unwrap_or(0);
            }
            // MTTS: bis zu dreimal nachfragen, bis sich die Antwort wiederholt
            if !wiederholt && p.ttype_runde < 3 {
                p.ttype_runde += 1;
                senden(tx, unterverhandlung(TTYPE, &[1]));
            }
            p.ttype_letzter = name;
        }
        CHARSET => match d.first() {
            Some(2) => {
                let gewaehlt = String::from_utf8_lossy(&d[1..]).to_uppercase();
                p.latin1 = gewaehlt.contains("8859-1") || gewaehlt.contains("LATIN");
            }
            _ => {}
        },
        GMCP => {
            let text = String::from_utf8_lossy(d);
            let (paket, rest) = text.split_once(' ').unwrap_or((&text, ""));
            let wert: serde_json::Value = serde_json::from_str(rest.trim()).unwrap_or(serde_json::Value::Null);
            match paket.to_lowercase().as_str() {
                "core.hello" => {
                    if let Some(c) = wert["client"].as_str() {
                        p.client = c.to_string();
                    }
                    if let Some(v) = wert["version"].as_str() {
                        p.client_version = v.to_string();
                    }
                }
                "core.supports.set" | "core.supports.add" | "core.supports.remove" => {
                    if paket.eq_ignore_ascii_case("core.supports.set") {
                        p.gmcp_module.clear();
                    }
                    for m in wert.as_array().into_iter().flatten().filter_map(|m| m.as_str()) {
                        let name = m.split_whitespace().next().unwrap_or_default().to_string();
                        if paket.eq_ignore_ascii_case("core.supports.remove") {
                            p.gmcp_module.remove(&name);
                        } else {
                            p.gmcp_module.insert(name);
                        }
                    }
                }
                "core.ping" => senden(tx, gmcp("Core.Ping", &serde_json::Value::Null)),
                _ => return Some(Aktion::Gmcp(paket.to_string(), wert)),
            }
        }
        MSDP => return Some(Aktion::Msdp(msdp_lesen(d))),
        _ => {}
    }
    None
}

/// Zerlegt MSDP vom Client: VAR "REPORT" VAL "HEALTH" VAL "MANA" (oder als Liste).
fn msdp_lesen(d: &[u8]) -> Vec<(String, Vec<String>)> {
    let mut aus: Vec<(String, Vec<String>)> = Vec::new();
    let mut aktuell = Vec::new();
    let mut ziel = 0; // 0 = nichts, 1 = Name, 2 = Wert
    let fertig = |aus: &mut Vec<(String, Vec<String>)>, ziel: u8, aktuell: &mut Vec<u8>| {
        let t = String::from_utf8_lossy(aktuell).trim().to_string();
        aktuell.clear();
        match ziel {
            1 => aus.push((t.to_uppercase(), Vec::new())),
            2 if !t.is_empty() => {
                if let Some(letzter) = aus.last_mut() {
                    letzter.1.push(t.to_uppercase());
                }
            }
            _ => {}
        }
    };
    for &b in d {
        match b {
            MSDP_VAR => {
                fertig(&mut aus, ziel, &mut aktuell);
                ziel = 1;
            }
            MSDP_VAL => {
                fertig(&mut aus, ziel, &mut aktuell);
                ziel = 2;
            }
            MSDP_LISTE_AUF | MSDP_LISTE_ZU | MSDP_TABELLE_AUF | MSDP_TABELLE_ZU => {
                fertig(&mut aus, ziel, &mut aktuell);
                ziel = 2;
            }
            _ => aktuell.push(b),
        }
    }
    fertig(&mut aus, ziel, &mut aktuell);
    aus
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zeilen_und_befehle() {
        let mut z = Zerleger::default();
        let mut ev = Vec::new();
        z.fuettern(b"hal\xff\xfb\x1flo\r\n", &mut ev);
        z.fuettern(b"\xff\xfa\x1f\x00\x50\x00\x18\xff\xf0gr\xc3\xbc\xc3\x9fe\r\0x\x08y\n", &mut ev);
        assert_eq!(
            ev,
            vec![
                Ereignis::Befehl(WILL, NAWS),
                Ereignis::Zeile("hallo".into()),
                Ereignis::Unter(NAWS, vec![0, 80, 0, 24]),
                Ereignis::Zeile("grüße".into()),
                Ereignis::Zeile("y".into()),
            ]
        );
    }

    #[test]
    fn latin1_eingabe() {
        let mut z = Zerleger::default();
        let mut ev = Vec::new();
        z.fuettern(b"gr\xfc\xdfe\n", &mut ev);
        assert_eq!(ev, vec![Ereignis::Zeile("grüße".into())]);
    }

    #[test]
    fn msdp_befehle() {
        let d = [MSDP_VAR, b'R', b'E', b'P', b'O', b'R', b'T', MSDP_VAL, b'H', b'P', MSDP_VAL, b'M', b'P'];
        assert_eq!(msdp_lesen(&d), vec![("REPORT".into(), vec!["HP".into(), "MP".into()])]);
        let mut d = vec![MSDP_VAR];
        d.extend(b"SEND");
        d.extend([MSDP_VAL, MSDP_LISTE_AUF, MSDP_VAL]);
        d.extend(b"MANA");
        d.push(MSDP_LISTE_ZU);
        assert_eq!(msdp_lesen(&d), vec![("SEND".into(), vec!["MANA".into()])]);
    }

    #[test]
    fn iac_wird_maskiert() {
        assert_eq!(unterverhandlung(GMCP, &[1, 255, 2]), vec![IAC, SB, GMCP, 1, 255, 255, 2, IAC, SE]);
    }
}
