//! Spieldaten "hinter den Kulissen" für MUD-Clients (GMCP und MSDP):
//! Lebenspunkte für Balken, Raumdaten für den Automapper, Gespräche für Chatfenster.

use crate::daten::Richtung;
use crate::kampf;
use crate::telnet::{self, MsdpWert};
use crate::welt::{Sitzung, Welt};
use serde_json::{json, Value};

/// Feste Raumnummer aus der Raum-ID (für Mapper, die Zahlen brauchen).
pub fn raumnummer(id: &str) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for b in id.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    (h & 0x7fff_ffff).max(1)
}

/// Kurzname einer Richtung, wie Mapper ihn erwarten.
fn richtung_kurz(r: Richtung) -> &'static str {
    match r {
        Richtung::Nord => "n",
        Richtung::Nordost => "ne",
        Richtung::Ost => "e",
        Richtung::Suedost => "se",
        Richtung::Sued => "s",
        Richtung::Suedwest => "sw",
        Richtung::West => "w",
        Richtung::Nordwest => "nw",
        Richtung::Hoch => "u",
        Richtung::Runter => "d",
        Richtung::Rein => "in",
        Richtung::Raus => "out",
    }
}

/// Entfernt Farbcodes wie {rot} ... {/}.
fn ohne_farben(t: &str) -> String {
    String::from_utf8_lossy(&crate::text::fuer_telnet(t, false, false)).replace("\r\n", "\n")
}

fn gmcp_senden(s: &Sitzung, paket: &str, daten: Value, nur_aenderung: bool) {
    let mut p = telnet::sperren(&s.proto);
    if !p.gmcp {
        return;
    }
    if nur_aenderung {
        let text = daten.to_string();
        if p.zuletzt.get(paket) == Some(&text) {
            return;
        }
        p.zuletzt.insert(paket.to_string(), text);
    }
    telnet::senden(&s.sender, telnet::gmcp(paket, &daten));
}

struct Gegner {
    name: String,
    prozent: i32,
}

fn gegner(w: &Welt, s: &Sitzung) -> Option<Gegner> {
    let m = s.kampf_ziel.and_then(|mid| w.monster.get(&mid))?;
    let d = w.d.monster.get(&m.def)?;
    Some(Gegner {
        name: crate::text::gross(&d.name),
        prozent: if d.lebenspunkte > 0 { (m.lebenspunkte * 100 / d.lebenspunkte).clamp(0, 100) } else { 0 },
    })
}

/// Beim Einloggen.
pub fn angemeldet(w: &Welt, sid: u64) {
    let Some(s) = w.spieler.get(&sid) else { return };
    gmcp_senden(s, "Char.Name", json!({ "name": s.sp.name, "fullname": s.sp.voller_name() }), false);
}

/// Vor jedem Prompt: Werte (nur, wenn sich etwas geändert hat).
pub fn werte(w: &Welt, sid: u64) {
    let Some(s) = w.spieler.get(&sid) else { return };
    let p = &s.sp;
    let mut vitals = json!({
        "hp": p.lebenspunkte, "maxhp": p.max_lebenspunkte,
        "mp": p.mana, "maxmp": p.max_mana,
        "ep": p.ausdauer, "maxep": p.max_ausdauer,
    });
    // Hunger und Durst (nur wenn eingeschaltet)
    let e = &w.d.ernaehrung;
    if e.aktiv {
        let voll = |v: f64, max: i32| if v < 0.0 { max } else { v.round() as i32 };
        vitals["food"] = json!(voll(p.essen, e.max_essen));
        vitals["maxfood"] = json!(e.max_essen);
        vitals["drink"] = json!(voll(p.trinken, e.max_trinken));
        vitals["maxdrink"] = json!(e.max_trinken);
    }
    gmcp_senden(s, "Char.Vitals", vitals, true);
    let g = gegner(w, s);
    gmcp_senden(
        s,
        "Char.Status",
        json!({
            "name": p.name,
            "fullname": p.voller_name(),
            "level": p.stufe,
            "xp": p.erfahrung,
            "xp_next": kampf::ep_fuer_naechste(w, p.stufe),
            "free_points": p.freie_punkte,
            "gold": p.geld,
            "race": p.rasse,
            "guild": p.gilde,
            "role": format!("{:?}", s.rolle).to_lowercase(),
            "dead": s.tot_bis.is_some(),
            "enemy": g.as_ref().map(|g| g.name.clone()).unwrap_or_default(),
            "enemy_health": g.as_ref().map(|g| g.prozent).unwrap_or(0),
            "time": crate::zeit::jetzt(w).map(|z| format!("{:02}:{:02}", z.stunde, z.minute)).unwrap_or_default(),
            "daytime": crate::zeit::jetzt(w).map(|z| z.tageszeit_id()).unwrap_or_default(),
            "season": crate::zeit::jetzt(w).map(|z| z.jahreszeit_id()).unwrap_or_default(),
            "weather": crate::zeit::wetter(w).map(|x| x.name.clone()).unwrap_or_default(),
        }),
        true,
    );
    msdp_melden(w, sid, s);
}

/// Wenn der Spieler einen Raum sieht: Daten für den Automapper.
pub fn raum(w: &Welt, sid: u64) {
    let Some(s) = w.spieler.get(&sid) else { return };
    let Some(r) = w.d.raum(&s.sp.raum) else { return };
    let ausgaenge: serde_json::Map<String, Value> = w
        .d
        .ausgaenge(&r.id)
        .iter()
        .map(|(richt, ziel)| (richtung_kurz(*richt).to_string(), json!(raumnummer(ziel))))
        .collect();
    gmcp_senden(
        s,
        "Room.Info",
        json!({
            "num": raumnummer(&r.id),
            "id": r.id,
            "name": ohne_farben(&r.name),
            "area": gebiet(w, &r.id, &r.gebiet),
            "map": r.karte,
            "environment": r.gelaende,
            "coords": format!("{},{},{}", r.x, r.y, r.z),
            "x": r.x, "y": r.y, "z": r.z,
            "exits": ausgaenge,
        }),
        false,
    );
}

fn gebiet(w: &Welt, id: &str, gebiet: &str) -> String {
    if !gebiet.is_empty() {
        gebiet.to_string()
    } else if crate::oberwelt::ist_oberwelt(id) {
        "Oberwelt".into()
    } else {
        w.d.welt.name.clone()
    }
}

/// Gespräche für Chatfenster: kanal = "say", "shout", "emote", ...
pub fn gespraech(w: &Welt, empfaenger: &[u64], kanal: &str, sprecher: &str, text: &str) {
    let daten = json!({ "channel": kanal, "talker": sprecher, "text": ohne_farben(text) });
    for sid in empfaenger {
        if let Some(s) = w.spieler.get(sid) {
            gmcp_senden(s, "Comm.Channel.Text", daten.clone(), false);
        }
    }
}

// ---------------------------------------------------------------------------
// MSDP
// ---------------------------------------------------------------------------

pub const MSDP_VARIABLEN: &[&str] = &[
    "CHARACTER_NAME",
    "SERVER_ID",
    "SERVER_TIME",
    "HEALTH",
    "HEALTH_MAX",
    "MANA",
    "MANA_MAX",
    "MOVEMENT",
    "MOVEMENT_MAX",
    "LEVEL",
    "MONEY",
    "EXPERIENCE",
    "EXPERIENCE_MAX",
    "EXPERIENCE_TNL",
    "OPPONENT_NAME",
    "OPPONENT_HEALTH",
    "OPPONENT_HEALTH_MAX",
    "ROOM",
    "ROOM_NAME",
    "ROOM_VNUM",
    "ROOM_AREA",
    "ROOM_TERRAIN",
    "ROOM_EXITS",
];

/// Den Wert einer MSDP-Variable berechnen (None = unbekannt oder noch nicht eingeloggt).
pub fn msdp_wert(w: &Welt, sid: Option<u64>, name: &str) -> Option<MsdpWert> {
    match name {
        "SERVER_ID" => return Some(MsdpWert::text(&w.d.welt.name)),
        "SERVER_TIME" => {
            let jetzt = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            return Some(MsdpWert::text(jetzt));
        }
        _ => {}
    }
    let s = w.spieler.get(&sid?)?;
    let p = &s.sp;
    let raum = || w.d.raum(&p.raum);
    let ausgaenge = || {
        MsdpWert::Tabelle(
            w.d.ausgaenge(&p.raum)
                .iter()
                .map(|(r, z)| (richtung_kurz(*r).to_string(), MsdpWert::text(raumnummer(z))))
                .collect(),
        )
    };
    Some(match name {
        "CHARACTER_NAME" => MsdpWert::text(&p.name),
        "HEALTH" => MsdpWert::text(p.lebenspunkte),
        "HEALTH_MAX" => MsdpWert::text(p.max_lebenspunkte),
        "MANA" => MsdpWert::text(p.mana),
        "MANA_MAX" => MsdpWert::text(p.max_mana),
        "MOVEMENT" => MsdpWert::text(p.ausdauer),
        "MOVEMENT_MAX" => MsdpWert::text(p.max_ausdauer),
        "LEVEL" => MsdpWert::text(p.stufe),
        "MONEY" => MsdpWert::text(p.geld),
        "EXPERIENCE" => MsdpWert::text(p.erfahrung),
        "EXPERIENCE_MAX" => MsdpWert::text(kampf::ep_fuer_naechste(w, p.stufe)),
        "EXPERIENCE_TNL" => MsdpWert::text(kampf::ep_fuer_naechste(w, p.stufe).saturating_sub(p.erfahrung)),
        "OPPONENT_NAME" => MsdpWert::text(gegner(w, s).map(|g| g.name).unwrap_or_default()),
        "OPPONENT_HEALTH" => MsdpWert::text(gegner(w, s).map(|g| g.prozent).unwrap_or(0)),
        "OPPONENT_HEALTH_MAX" => MsdpWert::text(100),
        "ROOM_NAME" => MsdpWert::text(ohne_farben(&raum()?.name)),
        "ROOM_VNUM" => MsdpWert::text(raumnummer(&p.raum)),
        "ROOM_AREA" => {
            let r = raum()?;
            MsdpWert::text(gebiet(w, &r.id, &r.gebiet))
        }
        "ROOM_TERRAIN" => MsdpWert::text(&raum()?.gelaende),
        "ROOM_EXITS" => ausgaenge(),
        "ROOM" => {
            let r = raum()?;
            MsdpWert::Tabelle(vec![
                ("VNUM".into(), MsdpWert::text(raumnummer(&r.id))),
                ("NAME".into(), MsdpWert::text(ohne_farben(&r.name))),
                ("AREA".into(), MsdpWert::text(gebiet(w, &r.id, &r.gebiet))),
                ("TERRAIN".into(), MsdpWert::text(&r.gelaende)),
                (
                    "COORDS".into(),
                    MsdpWert::Tabelle(vec![("X".into(), MsdpWert::text(r.x)), ("Y".into(), MsdpWert::text(r.y)), ("Z".into(), MsdpWert::text(r.z))]),
                ),
                ("EXITS".into(), ausgaenge()),
            ])
        }
        _ => return None,
    })
}

/// Schickt die Werte, die der Client mit REPORT bestellt hat – wenn sie sich geändert haben.
fn msdp_melden(w: &Welt, sid: u64, s: &Sitzung) {
    let namen: Vec<String> = {
        let p = telnet::sperren(&s.proto);
        if !p.msdp || p.msdp_gemeldet.is_empty() {
            return;
        }
        p.msdp_gemeldet.iter().cloned().collect()
    };
    let mut neu = Vec::new();
    let mut p = telnet::sperren(&s.proto);
    for n in namen {
        if n == "SERVER_TIME" {
            continue; // ändert sich jede Sekunde – nur auf Nachfrage
        }
        let Some(wert) = msdp_wert(w, Some(sid), &n) else { continue };
        let schluessel = format!("msdp:{n}");
        let text = format!("{wert:?}");
        if p.zuletzt.get(&schluessel) != Some(&text) {
            p.zuletzt.insert(schluessel, text);
            neu.push((n, wert));
        }
    }
    if !neu.is_empty() {
        telnet::senden(&s.sender, telnet::msdp(&neu));
    }
}

/// Beantwortet MSDP-Befehle des Clients (LIST, REPORT, UNREPORT, SEND, RESET).
pub fn msdp_befehle(w: &Welt, sid: Option<u64>, proto: &telnet::Geteilt, tx: &telnet::Sender, befehle: Vec<(String, Vec<String>)>) {
    let liste = |v: &[&str]| MsdpWert::Liste(v.iter().map(MsdpWert::text).collect());
    let mut antwort: Vec<(String, MsdpWert)> = Vec::new();
    let mut p = telnet::sperren(proto);
    for (befehl, werte) in befehle {
        match befehl.as_str() {
            "LIST" => {
                for was in werte {
                    let w = match was.as_str() {
                        "COMMANDS" => liste(&["LIST", "REPORT", "RESET", "SEND", "UNREPORT"]),
                        "LISTS" => liste(&[
                            "COMMANDS",
                            "LISTS",
                            "CONFIGURABLE_VARIABLES",
                            "REPORTABLE_VARIABLES",
                            "REPORTED_VARIABLES",
                            "SENDABLE_VARIABLES",
                        ]),
                        "REPORTABLE_VARIABLES" | "SENDABLE_VARIABLES" => liste(MSDP_VARIABLEN),
                        "REPORTED_VARIABLES" => MsdpWert::Liste(p.msdp_gemeldet.iter().map(MsdpWert::text).collect()),
                        "CONFIGURABLE_VARIABLES" => MsdpWert::Liste(Vec::new()),
                        _ => continue,
                    };
                    antwort.push((was, w));
                }
            }
            "REPORT" | "SEND" => {
                for name in werte {
                    if !MSDP_VARIABLEN.contains(&name.as_str()) {
                        continue;
                    }
                    if befehl == "REPORT" {
                        p.msdp_gemeldet.insert(name.clone());
                    }
                    if let Some(wert) = msdp_wert(w, sid, &name) {
                        p.zuletzt.insert(format!("msdp:{name}"), format!("{wert:?}"));
                        antwort.push((name, wert));
                    }
                }
            }
            "UNREPORT" => {
                for name in werte {
                    p.msdp_gemeldet.remove(&name);
                    p.zuletzt.remove(&format!("msdp:{name}"));
                }
            }
            "RESET" => {
                p.msdp_gemeldet.clear();
                p.zuletzt.retain(|k, _| !k.starts_with("msdp:"));
            }
            _ => {}
        }
    }
    if !antwort.is_empty() {
        telnet::senden(tx, telnet::msdp(&antwort));
    }
}

// ---------------------------------------------------------------------------
// MSSP
// ---------------------------------------------------------------------------

/// Serverinfos für MUD-Verzeichnisse. Eigene Angaben kommen aus [mssp] in welt.toml.
pub fn mssp(w: &Welt, startzeit: u64) -> Vec<(String, String)> {
    let gebiete: std::collections::BTreeSet<&str> =
        w.d.raeume.values().map(|r| r.gebiet.as_str()).filter(|g| !g.is_empty()).collect();
    let mut v: Vec<(String, String)> = [
        ("NAME", w.d.welt.name.clone()),
        ("PLAYERS", w.spieler.len().to_string()),
        ("UPTIME", startzeit.to_string()),
        ("CODEBASE", format!("RMengine {}", crate::update::VERSION)),
        ("FAMILY", "Custom".into()),
        ("PORT", w.d.welt.port.to_string()),
        ("LANGUAGE", "German".into()),
        ("CRAWL DELAY", "-1".into()),
        ("AREAS", gebiete.len().to_string()),
        ("ROOMS", w.d.raeume.len().to_string()),
        ("OBJECTS", w.d.gegenstaende.keys().filter(|k| !k.starts_with("leiche:")).count().to_string()),
        ("MOBILES", w.d.monster.len().to_string()),
        ("PLAYER KILLING", "None".into()),
        ("ANSI", "1".into()),
        ("UTF-8", "1".into()),
        ("GMCP", "1".into()),
        ("MSDP", "1".into()),
        ("MCCP", "1".into()),
        ("MSSP", "1".into()),
        ("MXP", "0".into()),
        ("MSP", "0".into()),
        ("VT100", "0".into()),
        ("XTERM 256 COLORS", "0".into()),
        ("PUEBLO", "0".into()),
    ]
    .into_iter()
    .map(|(k, w)| (k.to_string(), w))
    .collect();
    for (k, wert) in &w.d.mssp {
        let k = k.to_uppercase();
        match v.iter_mut().find(|(n, _)| *n == k) {
            Some(e) => e.1 = wert.clone(),
            None => v.push((k, wert.clone())),
        }
    }
    v
}
