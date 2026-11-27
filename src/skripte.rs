//! Skripte in Lua (daten/skripte/*.lua).
//!
//! Ein Skript meldet Befehle und Ereignisse an, z.B.
//!
//! ```lua
//! befehl{ woerter = {"kriech", "krieche"}, raum = "taverne",
//!   funktion = function(s, rest, raum)
//!     if not rest:find("tisch") then return false end   -- nicht gemeint: normale Befehle
//!     s:sende("Du kriechst unter den Tisch.")
//!     s:andere(s.name .. " kriecht unter den Tisch.")
//!   end }
//! ```
//!
//! Ein Skript bekommt Spieler, Raum und Zeit als Daten. Was es tun will,
//! sammelt der Vorspann (skripte_vorspann.lua) in einer Liste; die Engine
//! führt sie danach aus. Skripte haben keinen Zugriff auf Dateien oder das
//! System, Endlosschleifen werden abgebrochen.

use crate::spieler::Merk;
use crate::text::normalisieren;
use crate::welt::Welt;
use mlua::{HookTriggers, Lua, LuaOptions, StdLib, Table, Value, VmState};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

const VORSPANN: &str = include_str!("skripte_vorspann.lua");
/// So viele Tausend Lua-Anweisungen darf ein Aufruf höchstens ausführen
const HOECHSTENS_TAUSEND: u64 = 5_000;

thread_local! {
    /// Schutz gegen Skripte, die über Befehle und Ereignisse sich selbst endlos auslösen
    static TIEFE: Cell<u32> = const { Cell::new(0) };
}

/// Ein Befehl aus einem Skript
#[derive(Debug, Clone)]
pub struct SkriptBefehl {
    pub woerter: Vec<String>,
    pub raeume: Vec<String>,
    pub gebiete: Vec<String>,
    pub hilfe: String,
    pub syntax: String,
}

#[derive(Default)]
pub struct Skripte {
    lua: Option<Lua>,
    pub befehle: Vec<SkriptBefehl>,
    /// Gibt es überhaupt Ereignisse dieser Art? (spart Aufrufe)
    hat: BTreeMap<&'static str, bool>,
    start: Option<Instant>,
    naechster_timer: Option<f64>,
    zaehler: Arc<AtomicU64>,
    /// Anzahl der geladenen Dateien
    pub dateien: usize,
}

/// Die Dateien mit Skripten (daten/skripte/**/*.lua)
pub fn dateien(ordner: &Path) -> Vec<PathBuf> {
    fn sammeln(o: &Path, aus: &mut Vec<PathBuf>) {
        let Ok(e) = std::fs::read_dir(o) else { return };
        for e in e.flatten() {
            let p = e.path();
            if p.is_dir() {
                sammeln(&p, aus);
            } else if p.extension().is_some_and(|x| x == "lua") {
                aus.push(p);
            }
        }
    }
    let mut v = Vec::new();
    sammeln(&ordner.join("skripte"), &mut v);
    v.sort();
    v
}

/// Ein neues, abgesichertes Lua: nur table, string, math und utf8 – kein io, os, require.
fn neues_lua(zaehler: Arc<AtomicU64>) -> mlua::Result<Lua> {
    let lua = Lua::new_with(StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::UTF8, LuaOptions::default())?;
    let g = lua.globals();
    for weg in ["dofile", "loadfile", "require", "collectgarbage", "load"] {
        g.set(weg, Value::Nil)?;
    }
    // print schreibt in die Server-Konsole
    g.set(
        "print",
        lua.create_function(|_, args: mlua::Variadic<Value>| {
            let teile: Vec<String> = args.iter().map(|v| v.to_string().unwrap_or_else(|_| "?".into())).collect();
            println!("[Skript] {}", teile.join(" "));
            Ok(())
        })?,
    )?;
    lua.set_memory_limit(64 * 1024 * 1024)?;
    lua.set_hook(HookTriggers::new().every_nth_instruction(1000), move |_, _| {
        if zaehler.fetch_add(1, Ordering::Relaxed) > HOECHSTENS_TAUSEND {
            Err(mlua::Error::RuntimeError("Das Skript läuft zu lange und wurde abgebrochen (Endlosschleife?).".into()))
        } else {
            Ok(VmState::Continue)
        }
    });
    lua.load(VORSPANN).set_name("vorspann").exec()?;
    Ok(lua)
}

fn woerter(t: &Table, k: &str) -> Vec<String> {
    t.get::<Table>(k).map(|l| l.sequence_values::<String>().flatten().collect()).unwrap_or_default()
}

/// Alle Skripte laden. Fehler (z.B. Tippfehler im Lua) kommen als Liste zurück.
pub fn laden(ordner: &Path) -> (Skripte, Vec<String>) {
    let mut fehler = Vec::new();
    let liste = dateien(ordner);
    let mut s = Skripte { start: Some(Instant::now()), ..Default::default() };
    if liste.is_empty() {
        return (s, fehler);
    }
    let lua = match neues_lua(s.zaehler.clone()) {
        Ok(l) => l,
        Err(e) => {
            fehler.push(format!("Lua konnte nicht gestartet werden: {e}"));
            return (s, fehler);
        }
    };
    for pfad in &liste {
        let name = pfad.strip_prefix(ordner).unwrap_or(pfad).display().to_string().replace('\\', "/");
        let code = match std::fs::read_to_string(pfad) {
            Ok(c) => c,
            Err(e) => {
                fehler.push(format!("Kann '{name}' nicht lesen: {e}"));
                continue;
            }
        };
        s.zaehler.store(0, Ordering::Relaxed);
        let rm: Table = lua.globals().get("_RM").expect("Vorspann");
        let _ = rm.set("datei", name.as_str());
        if let Err(e) = lua.load(&code).set_name(format!("={name}")).exec() {
            fehler.push(format!("Skript '{name}': {}", fehler_text(&e)));
        }
        s.dateien += 1;
    }
    // Was angemeldet wurde, für schnelles Nachschlagen merken
    let rm: Table = lua.globals().get("_RM").expect("Vorspann");
    if let Ok(befehle) = rm.get::<Table>("befehle") {
        for e in befehle.sequence_values::<Table>().flatten() {
            s.befehle.push(SkriptBefehl {
                woerter: woerter(&e, "woerter").iter().map(|w| normalisieren(w)).collect(),
                raeume: woerter(&e, "raeume"),
                gebiete: woerter(&e, "gebiete"),
                hilfe: e.get("hilfe").unwrap_or_default(),
                syntax: e.get("syntax").unwrap_or_default(),
            });
        }
    }
    for art in ["betreten", "verlassen", "sieg", "sagen", "takt"] {
        let n = rm.get::<Table>(art).map(|t| t.raw_len()).unwrap_or(0);
        s.hat.insert(art, n > 0);
    }
    s.lua = Some(lua);
    (s, fehler)
}

/// Nur prüfen (für --pruefen und die Fernwartung)
pub fn pruefen(ordner: &Path) -> Vec<String> {
    laden(ordner).1
}

/// Lua-Fehler kurz und lesbar (ohne die lange Rückverfolgung)
fn fehler_text(e: &mlua::Error) -> String {
    let t = match e {
        mlua::Error::CallbackError { cause, .. } => return fehler_text(cause),
        mlua::Error::SyntaxError { message, .. } => message.clone(),
        mlua::Error::RuntimeError(m) => m.clone(),
        andere => andere.to_string(),
    };
    t.lines().next().unwrap_or_default().to_string()
}

// ---------------------------------------------------------------------------
// Daten für das Skript
// ---------------------------------------------------------------------------

fn merk_wert(lua: &Lua, m: &Merk) -> Value {
    match m {
        Merk::Wahr(b) => Value::Boolean(*b),
        Merk::Zahl(z) if z.fract() == 0.0 && z.abs() < 9e15 => Value::Integer(*z as i64),
        Merk::Zahl(z) => Value::Number(*z),
        Merk::Text(t) => lua.create_string(t).map(Value::String).unwrap_or(Value::Nil),
    }
}

fn aus_wert(v: &Value) -> Option<Merk> {
    match v {
        Value::Boolean(b) => Some(Merk::Wahr(*b)),
        Value::Integer(i) => Some(Merk::Zahl(*i as f64)),
        Value::Number(n) => Some(Merk::Zahl(*n)),
        Value::String(s) => Some(Merk::Text(s.to_string_lossy())),
        _ => None,
    }
}

fn liste_tabelle(lua: &Lua, v: &[String]) -> mlua::Result<Table> {
    lua.create_sequence_from(v.iter().cloned())
}

fn spieler_tabelle(lua: &Lua, w: &Welt, sid: u64) -> mlua::Result<Table> {
    let s = &w.spieler[&sid];
    let sp = &s.sp;
    let t = lua.create_table()?;
    t.set("sid", sid)?;
    t.set("name", sp.name.as_str())?;
    t.set("raum", sp.raum.as_str())?;
    t.set("stufe", sp.stufe)?;
    t.set("lp", sp.lebenspunkte)?;
    t.set("max_lp", sp.max_lebenspunkte)?;
    t.set("mp", sp.mana)?;
    t.set("max_mp", sp.max_mana)?;
    t.set("ausdauer", sp.ausdauer)?;
    t.set("geld", sp.geld)?;
    t.set("rasse", sp.rasse.as_str())?;
    t.set("gilde", sp.gilde.as_str())?;
    t.set("berufe", liste_tabelle(lua, &sp.berufe)?)?;
    t.set("rolle", format!("{:?}", s.rolle).to_lowercase())?;
    t.set("tot", s.tot_bis.is_some())?;
    t.set("im_kampf", s.kampf_ziel.is_some())?;
    t.set("inventar", liste_tabelle(lua, &sp.inventar)?)?;
    let angelegt = lua.create_table()?;
    for (p, g) in &sp.ausruestung {
        angelegt.set(p.as_str(), g.as_str())?;
    }
    t.set("angelegt", angelegt)?;
    let merker = lua.create_table()?;
    for (k, v) in &sp.merker {
        merker.set(k.as_str(), merk_wert(lua, v))?;
    }
    t.set("merker", merker)?;
    let fert = lua.create_table()?;
    for (k, v) in &sp.fertigkeiten {
        fert.set(k.as_str(), *v)?;
    }
    t.set("fertigkeiten", fert)?;
    Ok(t)
}

fn raum_tabelle(lua: &Lua, w: &Welt, raum: &str) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("id", raum)?;
    if let Some(r) = w.d.raum(raum) {
        t.set("name", r.name.as_str())?;
        t.set("gebiet", r.gebiet.as_str())?;
    }
    t.set("drinnen", crate::zeit::drinnen(w, raum))?;
    t.set("gegenstaende", liste_tabelle(lua, &w.boden.get(raum).cloned().unwrap_or_default())?)?;
    let monster: Vec<String> = w.monster_im_raum(raum).iter().filter_map(|m| w.monster.get(m)).map(|m| m.def.clone()).collect();
    t.set("monster", liste_tabelle(lua, &monster)?)?;
    let spieler: Vec<String> = w.spieler_im_raum(raum).iter().map(|s| w.spieler[s].sp.name.clone()).collect();
    t.set("spieler", liste_tabelle(lua, &spieler)?)?;
    Ok(t)
}

/// Vor jedem Aufruf: Aktionen leeren, Zeit und Welt-Merker setzen
fn vorbereiten(lua: &Lua, w: &Welt) -> mlua::Result<()> {
    w.skripte.zaehler.store(0, Ordering::Relaxed);
    let rm: Table = lua.globals().get("_RM")?;
    rm.set("aktionen", lua.create_table()?)?;
    rm.set("fehler", lua.create_table()?)?;
    rm.set("jetzt", w.skripte.start.map(|s| s.elapsed().as_secs_f64()).unwrap_or(0.0))?;
    let welt: Table = lua.globals().get("welt")?;
    let merker = lua.create_table()?;
    for (k, v) in &w.skript_merker {
        merker.set(k.as_str(), merk_wert(lua, v))?;
    }
    welt.set("merker", merker)?;
    match crate::zeit::jetzt(w) {
        Some(z) => {
            let zeit = lua.create_table()?;
            zeit.set("stunde", z.stunde)?;
            zeit.set("minute", z.minute)?;
            zeit.set("tageszeit", z.tageszeit_id())?;
            zeit.set("jahreszeit", z.jahreszeit_id())?;
            zeit.set("tag", z.tag)?;
            zeit.set("wetter", w.wetter.as_str())?;
            welt.set("zeit", zeit)?;
        }
        None => welt.set("zeit", Value::Nil)?,
    }
    Ok(())
}

/// Nach dem Aufruf: was das Skript tun will, und welche Fehler es gab
fn abholen(lua: &Lua) -> (Vec<Table>, Vec<String>) {
    let Ok(rm) = lua.globals().get::<Table>("_RM") else { return (Vec::new(), Vec::new()) };
    let aktionen = rm.get::<Table>("aktionen").map(|t| t.sequence_values::<Table>().flatten().collect()).unwrap_or_default();
    let fehler = rm.get::<Table>("fehler").map(|t| t.sequence_values::<String>().flatten().collect()).unwrap_or_default();
    (aktionen, fehler)
}

// ---------------------------------------------------------------------------
// Aufrufe aus der Engine
// ---------------------------------------------------------------------------

/// Fehler melden: in der Konsole und an Baumeister/Admins im Spiel
fn fehler_melden(w: &Welt, sid: Option<u64>, fehler: &[String]) {
    for f in fehler {
        eprintln!("[Skriptfehler] {f}");
        let erste = f.lines().next().unwrap_or_default().replace("runtime error: ", "");
        let zeile = format!("{{rot}}[Skriptfehler] {erste}{{/}}");
        match sid.and_then(|s| w.spieler.get(&s)) {
            Some(s) if s.baut() => s.sende(&zeile),
            _ => {
                for s in w.spieler.values().filter(|s| s.baut()) {
                    s.sende(&zeile);
                }
            }
        }
    }
}

/// Ein eingetippter Befehl: gibt es dafür ein Skript (hier im Raum)? true = erledigt
pub fn befehl(w: &mut Welt, sid: u64, wort: &str, rest: &str) -> bool {
    if w.skripte.befehle.is_empty() {
        return false;
    }
    let wort = normalisieren(wort);
    let Some(raum) = w.spieler.get(&sid).map(|s| s.sp.raum.clone()) else { return false };
    let gebiet = w.d.gebiet(&raum);
    let kandidaten: Vec<usize> = w
        .skripte
        .befehle
        .iter()
        .enumerate()
        .filter(|(_, b)| b.woerter.contains(&wort))
        .filter(|(_, b)| b.raeume.is_empty() || b.raeume.contains(&raum))
        .filter(|(_, b)| b.gebiete.is_empty() || b.gebiete.contains(&gebiet))
        .map(|(i, _)| i)
        .collect();
    for i in kandidaten {
        let Some(lua) = w.skripte.lua.as_ref() else { return false };
        let ergebnis = (|| -> mlua::Result<bool> {
            vorbereiten(lua, w)?;
            let f: mlua::Function = lua.globals().get::<Table>("_RM")?.get("befehl")?;
            f.call::<bool>((i + 1, spieler_tabelle(lua, w, sid)?, rest, raum_tabelle(lua, w, &raum)?))
        })();
        let (aktionen, mut fehler) = abholen(lua);
        let erledigt = match ergebnis {
            Ok(e) => e,
            Err(e) => {
                fehler.push(fehler_text(&e));
                true
            }
        };
        if !fehler.is_empty() {
            fehler_melden(w, Some(sid), &fehler);
            if !w.spieler.get(&sid).is_some_and(|s| s.baut()) {
                w.sende(sid, "{grau}(Da ist etwas schiefgegangen.){/}");
            }
        }
        anwenden(w, aktionen);
        if erledigt {
            return true;
        }
    }
    false
}

/// Ein Ereignis an die Skripte geben: art = betreten, verlassen, sieg, sagen
fn ereignis(w: &mut Welt, art: &'static str, wo: &str, sid: Option<u64>, a: Option<String>) {
    if !w.skripte.hat.get(art).copied().unwrap_or(false) {
        return;
    }
    let tiefe = TIEFE.with(|t| t.get());
    if tiefe > 8 {
        return;
    }
    let Some(lua) = w.skripte.lua.as_ref() else { return };
    let ergebnis = (|| -> mlua::Result<()> {
        vorbereiten(lua, w)?;
        let f: mlua::Function = lua.globals().get::<Table>("_RM")?.get("ereignis")?;
        let s = match sid {
            Some(sid) if w.spieler.contains_key(&sid) => Value::Table(spieler_tabelle(lua, w, sid)?),
            _ => Value::Nil,
        };
        let raum_id = sid.and_then(|s| w.spieler.get(&s)).map(|s| s.sp.raum.clone()).unwrap_or_else(|| wo.to_string());
        let raum = Value::Table(raum_tabelle(lua, w, &raum_id)?);
        match art {
            // (s, raum)
            "betreten" | "verlassen" => f.call::<()>((art, wo, s, raum_tabelle(lua, w, wo)?)),
            // (s, monster_id, raum) bzw. (s, text, raum)
            _ => f.call::<()>((art, wo, s, a.unwrap_or_default(), raum)),
        }
    })();
    let (aktionen, mut fehler) = abholen(lua);
    if let Err(e) = ergebnis {
        fehler.push(fehler_text(&e));
    }
    fehler_melden(w, sid, &fehler);
    TIEFE.with(|t| t.set(tiefe + 1));
    anwenden(w, aktionen);
    TIEFE.with(|t| t.set(tiefe));
}

/// Ein Spieler hat den Raum gewechselt (Gehen, Flucht, Teleport ...)
pub fn raumwechsel(w: &mut Welt, sid: u64, alt: &str, neu: &str) {
    if alt != neu {
        ereignis(w, "verlassen", alt, Some(sid), None);
        ereignis(w, "betreten", neu, Some(sid), None);
    }
}

pub fn sieg(w: &mut Welt, sid: u64, monster: &str) {
    ereignis(w, "sieg", monster, Some(sid), Some(monster.to_string()));
}

pub fn sagen(w: &mut Welt, sid: u64, text: &str) {
    let raum = w.spieler.get(&sid).map(|s| s.sp.raum.clone()).unwrap_or_default();
    ereignis(w, "sagen", &raum, Some(sid), Some(text.to_string()));
}

/// Takt der Welt: bei_takt-Funktionen
pub fn takt(w: &mut Welt) {
    ereignis(w, "takt", "*", None, None);
}

/// Oft aufgerufen (Kampftakt): fällige spaeter(...)-Funktionen ausführen
pub fn timer(w: &mut Welt) {
    let Some(lua) = w.skripte.lua.as_ref() else { return };
    let jetzt = w.skripte.start.map(|s| s.elapsed().as_secs_f64()).unwrap_or(0.0);
    // Gibt es überhaupt Timer? (ohne Lua-Aufruf prüfen, wenn möglich)
    let rm: Table = match lua.globals().get("_RM") {
        Ok(t) => t,
        Err(_) => return,
    };
    let anzahl = rm.get::<Table>("timer").map(|t| t.raw_len()).unwrap_or(0);
    if anzahl == 0 || w.skripte.naechster_timer.is_some_and(|n| n > jetzt) {
        return;
    }
    let ergebnis = (|| -> mlua::Result<Option<f64>> {
        vorbereiten(lua, w)?;
        let f: mlua::Function = rm.get("timer_pruefen")?;
        f.call::<Option<f64>>(())
    })();
    let (aktionen, mut fehler) = abholen(lua);
    let naechster = match ergebnis {
        Ok(n) => n,
        Err(e) => {
            fehler.push(fehler_text(&e));
            None
        }
    };
    w.skripte.naechster_timer = naechster;
    fehler_melden(w, None, &fehler);
    anwenden(w, aktionen);
}

/// Neue Timer: beim nächsten Kampftakt prüfen
fn timer_neu_pruefen(w: &mut Welt) {
    w.skripte.naechster_timer = None;
}

// ---------------------------------------------------------------------------
// Die Aktionen ausführen
// ---------------------------------------------------------------------------

fn anwenden(w: &mut Welt, aktionen: Vec<Table>) {
    if aktionen.is_empty() {
        return;
    }
    timer_neu_pruefen(w);
    for a in aktionen {
        let art: String = a.get("art").unwrap_or_default();
        let sid: Option<u64> = a.get("sid").ok();
        let text = |k: &str| a.get::<String>(k).unwrap_or_default();
        let zahl = |k: &str| a.get::<i64>(k).unwrap_or(0);
        // Aktionen für Spieler, die nicht (mehr) da sind, entfallen
        if let Some(sid) = sid {
            if !w.spieler.contains_key(&sid) {
                continue;
            }
        }
        let sid = sid.unwrap_or(0);
        match art.as_str() {
            "sende" => w.sende(sid, &text("text")),
            "andere" => {
                let raum = w.spieler[&sid].sp.raum.clone();
                w.sende_raum(&raum, &text("text"), &[sid]);
            }
            "raum" => w.sende_raum(&text("raum"), &text("text"), &[]),
            "alle" => w.sende_alle(&text("text"), &[]),
            "gehe" => gehe(w, sid, &text("raum"), &text("weg"), &text("an")),
            "gib" => {
                let id = text("id");
                if w.d.gegenstaende.contains_key(&id) {
                    if let Some(s) = w.spieler.get_mut(&sid) {
                        s.sp.inventar.push(id);
                    }
                } else {
                    fehler_melden(w, Some(sid), &[format!("s:gib: Gegenstand '{id}' gibt es nicht.")]);
                }
            }
            "nimm" => {
                let id = text("id");
                if let Some(s) = w.spieler.get_mut(&sid) {
                    if let Some(i) = s.sp.inventar.iter().position(|g| *g == id) {
                        s.sp.inventar.remove(i);
                    }
                }
            }
            "geld" => {
                if let Some(s) = w.spieler.get_mut(&sid) {
                    s.sp.geld = (s.sp.geld as i64 + zahl("n")).max(0) as u64;
                }
            }
            "lp" => {
                let n = zahl("n") as i32;
                let tot = w.spieler.get_mut(&sid).is_some_and(|s| {
                    s.sp.lebenspunkte = (s.sp.lebenspunkte + n).min(s.sp.max_lebenspunkte);
                    s.sp.lebenspunkte <= 0 && s.tot_bis.is_none()
                });
                if tot {
                    crate::kampf::spieler_stirbt(w, sid);
                }
            }
            "mp" => {
                if let Some(s) = w.spieler.get_mut(&sid) {
                    s.sp.mana = (s.sp.mana + zahl("n") as i32).clamp(0, s.sp.max_mana);
                }
            }
            "erfahrung" => crate::kampf::erfahrung_geben(w, sid, zahl("n").max(0) as u64),
            "merke" => {
                let name = text("name");
                let wert = a.get::<Value>("wert").ok().and_then(|v| aus_wert(&v));
                if let Some(s) = w.spieler.get_mut(&sid) {
                    match wert {
                        Some(v) => s.sp.merker.insert(name, v),
                        None => s.sp.merker.remove(&name),
                    };
                }
            }
            "welt_merke" => {
                let name = text("name");
                match a.get::<Value>("wert").ok().and_then(|v| aus_wert(&v)) {
                    Some(v) => w.skript_merker.insert(name, v),
                    None => w.skript_merker.remove(&name),
                };
            }
            "befehl" => {
                let tiefe = TIEFE.with(|t| t.get());
                if tiefe <= 8 {
                    TIEFE.with(|t| t.set(tiefe + 1));
                    crate::befehle::befehl_ohne_prompt(w, sid, &text("text"));
                    TIEFE.with(|t| t.set(tiefe));
                }
            }
            "fortschritt" => crate::quests::fortschritt(w, sid, "skript", &text("ziel"), 1),
            "zauber" => {
                if let Some(z) = w.d.zauber.get(&text("id")).cloned() {
                    for wi in &z.wirkungen {
                        if wi.dauer > 0.0 {
                            crate::wirkung::auf_spieler(w, sid, &z, wi, sid);
                        } else if matches!(wi.art.as_str(), "heilen" | "mana" | "ausdauer") {
                            crate::befehle::wirkung_anwenden(w, sid, &wi.art, wi.wert, "");
                        }
                    }
                }
            }
            "lege" => {
                let (raum, id) = (text("raum"), text("id"));
                if w.d.gegenstaende.contains_key(&id) {
                    w.boden.entry(raum).or_default().push(id);
                }
            }
            "entferne" => {
                let (raum, id) = (text("raum"), text("id"));
                if let Some(b) = w.boden.get_mut(&raum) {
                    if let Some(i) = b.iter().position(|g| *g == id) {
                        b.remove(i);
                    }
                }
            }
            "erschaffe" => {
                let (raum, id) = (text("raum"), text("id"));
                if w.d.raum_existiert(&raum) {
                    w.monster_erschaffen(&id, &raum);
                }
            }
            andere => fehler_melden(w, None, &[format!("Unbekannte Aktion '{andere}'.")]),
        }
    }
}

/// s:gehe(raum): den Spieler versetzen
fn gehe(w: &mut Welt, sid: u64, ziel: &str, weg: &str, an: &str) {
    if !w.d.raum_existiert(ziel) {
        fehler_melden(w, Some(sid), &[format!("s:gehe: Raum '{ziel}' gibt es nicht.")]);
        return;
    }
    crate::kampf::kampf_beenden(w, sid);
    let Some(s) = w.spieler.get_mut(&sid) else { return };
    let alt = std::mem::replace(&mut s.sp.raum, ziel.to_string());
    if !crate::oberwelt::ist_oberwelt(ziel) {
        s.sp.besucht.insert(ziel.to_string());
    }
    let name = s.sp.name.clone();
    let weg = if weg.is_empty() { format!("{name} ist plötzlich verschwunden.") } else { weg.to_string() };
    let an = if an.is_empty() { format!("{name} ist plötzlich da.") } else { an.to_string() };
    w.sende_raum(&alt, &weg, &[sid]);
    w.sende_raum(ziel, &an, &[sid]);
    crate::befehle::raum_anzeigen(w, sid);
    raumwechsel(w, sid, &alt, ziel);
}

// ---------------------------------------------------------------------------
// Welt-Merker speichern
// ---------------------------------------------------------------------------

fn merker_datei(w: &Welt) -> PathBuf {
    w.spieler_verzeichnis().join("welt_merker.json")
}

pub fn merker_speichern(w: &Welt) {
    if w.skript_merker.is_empty() {
        return;
    }
    if let Ok(t) = serde_json::to_string_pretty(&w.skript_merker) {
        let _ = std::fs::create_dir_all(w.spieler_verzeichnis());
        let _ = std::fs::write(merker_datei(w), t);
    }
}

pub fn merker_laden(w: &mut Welt) {
    if let Ok(t) = std::fs::read_to_string(merker_datei(w)) {
        if let Ok(m) = serde_json::from_str(&t) {
            w.skript_merker = m;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beispielskripte_laden() {
        let ordner = Path::new(env!("CARGO_MANIFEST_DIR")).join("daten");
        let (s, fehler) = laden(&ordner);
        assert!(fehler.is_empty(), "{fehler:?}");
        let kriech = s.befehle.iter().find(|b| b.woerter.contains(&"kriech".to_string())).expect("kriech");
        assert_eq!(kriech.raeume, vec!["taverne".to_string()]);
        assert_eq!(s.hat.get("betreten"), Some(&true));
    }

    #[test]
    fn endlosschleife_wird_abgebrochen() {
        let zaehler = Arc::new(AtomicU64::new(0));
        let lua = neues_lua(zaehler).expect("Lua");
        let e = lua.load("while true do end").exec().expect_err("muss abbrechen");
        assert!(fehler_text(&e).contains("zu lange"), "{e}");
        // Kein Zugriff auf das System
        assert!(lua.load("return io == nil and os == nil and require == nil").eval::<bool>().unwrap());
    }
}
