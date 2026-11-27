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
    /// Nur, wenn man einen dieser Gegenstände dabeihat (oder er im Raum liegt)
    pub gegenstaende: Vec<String>,
    /// Nur, wenn einer dieser NPCs/Monster im Raum ist
    pub npcs: Vec<String>,
    pub hilfe: String,
    pub syntax: String,
}

#[derive(Default)]
pub struct Skripte {
    lua: Option<Lua>,
    pub befehle: Vec<SkriptBefehl>,
    /// Angemeldete Ereignisse: (Art, wo) – spart Aufrufe, wenn es keine gibt
    orte: std::collections::HashSet<(String, String)>,
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

/// Skripte, die direkt an Räumen, Gegenständen und NPCs stehen: (Art, ID, Datei, Code)
fn eingebettet(d: &crate::daten::Daten) -> Vec<(&'static str, String, String, String)> {
    let mut v = Vec::new();
    for r in d.raeume.values().filter(|r| !r.skript.trim().is_empty()) {
        v.push(("raum", r.id.clone(), r.quelle.clone(), r.skript.clone()));
    }
    for g in d.gegenstaende.values().filter(|g| !g.skript.trim().is_empty()) {
        v.push(("gegenstand", g.id.clone(), g.quelle.clone(), g.skript.clone()));
    }
    for m in d.monster.values().filter(|m| !m.skript.trim().is_empty()) {
        v.push((if m.ist_npc { "npc" } else { "monster" }, m.id.clone(), m.quelle.clone(), m.skript.clone()));
    }
    v
}

/// Alle Skripte laden. Fehler (z.B. Tippfehler im Lua) kommen als Liste zurück.
pub fn laden(ordner: &Path, d: &crate::daten::Daten) -> (Skripte, Vec<String>) {
    let mut fehler = Vec::new();
    let liste = dateien(ordner);
    let inline = eingebettet(d);
    let mut s = Skripte { start: Some(Instant::now()), ..Default::default() };
    if liste.is_empty() && inline.is_empty() {
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
    // Skripte an Räumen, Gegenständen und NPCs: "hier" ist der Besitzer
    for (art, id, datei, code) in &inline {
        s.zaehler.store(0, Ordering::Relaxed);
        let rm: Table = lua.globals().get("_RM").expect("Vorspann");
        let name = format!("{datei} ({art} {id})");
        let _ = rm.set("datei", name.as_str());
        if let Ok(hier) = lua.create_table() {
            let _ = hier.set("art", *art);
            let _ = hier.set("id", id.as_str());
            let _ = rm.set("hier", hier);
        }
        if let Err(e) = lua.load(code).set_name(format!("={name}")).exec() {
            fehler.push(format!("Skript in {name}: {}", fehler_text(&e)));
        }
        let _ = rm.set("hier", Value::Nil);
    }
    // Was angemeldet wurde, für schnelles Nachschlagen merken
    let rm: Table = lua.globals().get("_RM").expect("Vorspann");
    if let Ok(befehle) = rm.get::<Table>("befehle") {
        for e in befehle.sequence_values::<Table>().flatten() {
            s.befehle.push(SkriptBefehl {
                woerter: woerter(&e, "woerter").iter().map(|w| normalisieren(w)).collect(),
                raeume: woerter(&e, "raeume"),
                gebiete: woerter(&e, "gebiete"),
                gegenstaende: woerter(&e, "gegenstaende"),
                npcs: woerter(&e, "npcs"),
                hilfe: e.get("hilfe").unwrap_or_default(),
                syntax: e.get("syntax").unwrap_or_default(),
            });
        }
    }
    if let Ok(ereignisse) = rm.get::<Table>("ereignisse") {
        for (art, liste) in ereignisse.pairs::<String, Table>().flatten() {
            for e in liste.sequence_values::<Table>().flatten() {
                let wo: String = e.get("wo").unwrap_or_else(|_| "*".into());
                s.orte.insert((art.clone(), wo));
            }
        }
    }
    s.lua = Some(lua);
    (s, fehler)
}

/// Nur prüfen (für --pruefen und die Fernwartung)
pub fn pruefen(ordner: &Path, d: &crate::daten::Daten) -> Vec<String> {
    laden(ordner, d).1
}

impl Skripte {
    /// Gibt es für dieses Ereignis hier eine Funktion?
    fn hat(&self, art: &str, wo: &str) -> bool {
        self.lua.is_some() && (self.orte.contains(&(art.to_string(), "*".to_string())) || self.orte.contains(&(art.to_string(), wo.to_string())))
    }
    /// Gibt es Ereignisse dieser Art überhaupt?
    fn hat_art(&self, art: &str) -> bool {
        self.lua.is_some() && self.orte.iter().any(|(a, _)| a == art)
    }
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

fn wesen_tabelle(lua: &Lua, w: &Welt, uid: u64) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("uid", uid)?;
    if let Some(m) = w.monster.get(&uid) {
        t.set("id", m.def.as_str())?;
        t.set("raum", m.raum.as_str())?;
        t.set("lp", m.lebenspunkte)?;
        t.set("im_kampf", !m.ziele.is_empty())?;
        if let Some(d) = w.d.monster.get(&m.def) {
            t.set("name", d.name.as_str())?;
            t.set("max_lp", d.lebenspunkte)?;
            t.set("npc", d.ist_npc)?;
        }
    }
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
    let uids = w.monster_im_raum(raum);
    let monster: Vec<String> = uids.iter().filter_map(|m| w.monster.get(m)).map(|m| m.def.clone()).collect();
    t.set("monster", liste_tabelle(lua, &monster)?)?;
    let wesen = lua.create_table()?;
    for uid in &uids {
        wesen.push(wesen_tabelle(lua, w, *uid)?)?;
    }
    t.set("wesen", wesen)?;
    let spieler: Vec<String> = w.spieler_im_raum(raum).iter().map(|s| w.spieler[s].sp.name.clone()).collect();
    t.set("spieler", liste_tabelle(lua, &spieler)?)?;
    let ausgaenge = lua.create_table()?;
    let tueren = lua.create_table()?;
    for (richt, ziel) in w.sichtbare_ausgaenge(raum) {
        ausgaenge.set(richt.schluessel(), ziel.as_str())?;
        if let Some((_, _, zu, verschlossen)) = w.tuer(raum, richt) {
            tueren.set(richt.schluessel(), if verschlossen { "verschlossen" } else if zu { "zu" } else { "offen" })?;
        }
    }
    t.set("ausgaenge", ausgaenge)?;
    t.set("tueren", tueren)?;
    if let Some(z) = w.raum_zusatz.get(raum) {
        t.set("zusatz", z.as_str())?;
    }
    Ok(t)
}

/// Während eines Aufrufs: welt.raum(id), welt.spieler(name), welt.gegenstand(id) zum Nachschlagen
fn mit_lesern<R>(lua: &Lua, w: &Welt, f: impl FnOnce() -> mlua::Result<R>) -> mlua::Result<R> {
    lua.scope(|scope| {
        let welt: Table = lua.globals().get("welt")?;
        welt.set(
            "raum",
            scope.create_function(|lua, id: String| {
                if w.d.raum_existiert(&id) {
                    raum_tabelle(lua, w, &id).map(Value::Table)
                } else {
                    Ok(Value::Nil)
                }
            })?,
        )?;
        welt.set(
            "spieler",
            scope.create_function(|lua, name: String| {
                match w.spieler.iter().find(|(_, s)| s.sp.name.eq_ignore_ascii_case(&name)) {
                    Some((sid, _)) => {
                        let t = spieler_tabelle(lua, w, *sid)?;
                        let mt: Table = lua.globals().get("Spieler")?;
                        t.set_metatable(Some(mt));
                        Ok(Value::Table(t))
                    }
                    None => Ok(Value::Nil),
                }
            })?,
        )?;
        welt.set(
            "gegenstand",
            scope.create_function(|lua, id: String| {
                let Some(g) = w.d.gegenstaende.get(&id) else { return Ok(Value::Nil) };
                let t = lua.create_table()?;
                t.set("id", g.id.as_str())?;
                t.set("name", g.name.as_str())?;
                t.set("typ", g.typ.as_str())?;
                t.set("gewicht", g.gewicht)?;
                t.set("wert", g.wert)?;
                Ok(Value::Table(t))
            })?,
        )?;
        f()
    })
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
    let hat_ding = |id: &String| {
        let sp = &w.spieler[&sid].sp;
        sp.inventar.contains(id) || sp.ausruestung.values().any(|g| g == id) || w.boden.get(&raum).is_some_and(|b| b.contains(id))
    };
    let npc_da = |id: &String| w.monster_im_raum(&raum).iter().any(|m| w.monster.get(m).is_some_and(|m| m.def == *id));
    let kandidaten: Vec<usize> = w
        .skripte
        .befehle
        .iter()
        .enumerate()
        .filter(|(_, b)| b.woerter.contains(&wort))
        .filter(|(_, b)| b.raeume.is_empty() || b.raeume.contains(&raum))
        .filter(|(_, b)| b.gebiete.is_empty() || b.gebiete.contains(&gebiet))
        .filter(|(_, b)| b.gegenstaende.is_empty() || b.gegenstaende.iter().any(hat_ding))
        .filter(|(_, b)| b.npcs.is_empty() || b.npcs.iter().any(npc_da))
        .map(|(i, _)| i)
        .collect();
    for i in kandidaten {
        let Some(lua) = w.skripte.lua.as_ref() else { return false };
        let ergebnis = mit_lesern(lua, w, || {
            vorbereiten(lua, w)?;
            let f: mlua::Function = lua.globals().get::<Table>("_RM")?.get("befehl")?;
            f.call::<bool>((i + 1, spieler_tabelle(lua, w, sid)?, rest, raum_tabelle(lua, w, &raum)?))
        });
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

/// Ein Wert für die Mitte der Argumente eines Ereignisses
enum Arg {
    Text(String),
    Wesen(u64),
}

/// Ein Ereignis an die Skripte geben. Ergebnis: false, wenn ein Skript es verhindert.
fn ereignis(w: &mut Welt, art: &str, wo: &str, sid: Option<u64>, raum: &str, mitte: Vec<Arg>) -> bool {
    if !w.skripte.hat(art, wo) {
        return true;
    }
    let tiefe = TIEFE.with(|t| t.get());
    if tiefe > 8 {
        return true;
    }
    let Some(lua) = w.skripte.lua.as_ref() else { return true };
    let ergebnis = mit_lesern(lua, w, || {
        vorbereiten(lua, w)?;
        let f: mlua::Function = lua.globals().get::<Table>("_RM")?.get("ereignis")?;
        let s = match sid {
            Some(sid) if w.spieler.contains_key(&sid) => Value::Table(spieler_tabelle(lua, w, sid)?),
            _ => Value::Nil,
        };
        let raum_t = if raum.is_empty() { Value::Nil } else { Value::Table(raum_tabelle(lua, w, raum)?) };
        let mut args = mlua::MultiValue::new();
        args.push_back(Value::String(lua.create_string(art)?));
        args.push_back(Value::String(lua.create_string(wo)?));
        args.push_back(s);
        args.push_back(raum_t);
        for a in &mitte {
            args.push_back(match a {
                Arg::Text(t) => Value::String(lua.create_string(t)?),
                Arg::Wesen(uid) => Value::Table(wesen_tabelle(lua, w, *uid)?),
            });
        }
        f.call::<bool>(args)
    });
    let (aktionen, mut fehler) = abholen(lua);
    let erlaubt = match ergebnis {
        Ok(e) => e,
        Err(e) => {
            fehler.push(fehler_text(&e));
            true
        }
    };
    fehler_melden(w, sid, &fehler);
    TIEFE.with(|t| t.set(tiefe + 1));
    anwenden(w, aktionen);
    TIEFE.with(|t| t.set(tiefe));
    erlaubt
}

fn raum_von(w: &Welt, sid: u64) -> String {
    w.spieler.get(&sid).map(|s| s.sp.raum.clone()).unwrap_or_default()
}

/// Darf der Spieler das? (nehmen, ablegen, benutzen, anlegen, ausziehen: wo = Gegenstand;
/// gehen, tuer, ansehen: wo = Raum). false = ein Skript hat es verhindert.
pub fn erlaubt(w: &mut Welt, sid: u64, art: &str, wo: &str, mitte: &[&str]) -> bool {
    let raum = raum_von(w, sid);
    ereignis(w, art, wo, Some(sid), &raum, mitte.iter().map(|t| Arg::Text(t.to_string())).collect())
}

/// Ein Spieler hat den Raum gewechselt (Gehen, Flucht, Teleport ...)
pub fn raumwechsel(w: &mut Welt, sid: u64, alt: &str, neu: &str) {
    if alt != neu {
        ereignis(w, "verlassen", alt, Some(sid), alt, Vec::new());
        ereignis(w, "betreten", neu, Some(sid), neu, Vec::new());
    }
}

pub fn sieg(w: &mut Welt, sid: u64, monster: &str) {
    let raum = raum_von(w, sid);
    ereignis(w, "sieg", monster, Some(sid), &raum, vec![Arg::Text(monster.to_string())]);
}

/// "sage": erst der Raum (bei_sagen), dann die NPCs im Raum (bei_gespraech).
/// Ergebnis: NPCs, deren normale Antwort ein Skript übernommen hat.
pub fn sagen(w: &mut Welt, sid: u64, text: &str) -> Vec<u64> {
    let raum = raum_von(w, sid);
    ereignis(w, "sagen", &raum, Some(sid), &raum, vec![Arg::Text(text.to_string())]);
    let mut stumm = Vec::new();
    if w.skripte.hat_art("gespraech") {
        for uid in w.monster_im_raum(&raum) {
            if !gespraech(w, sid, uid, text) {
                stumm.push(uid);
            }
        }
    }
    stumm
}

/// Ein Spieler redet mit einem NPC (sage, frage, rede). false = Skript antwortet selbst
pub fn gespraech(w: &mut Welt, sid: u64, uid: u64, text: &str) -> bool {
    let Some(def) = w.monster.get(&uid).map(|m| m.def.clone()) else { return true };
    let raum = raum_von(w, sid);
    ereignis(w, "gespraech", &def, Some(sid), &raum, vec![Arg::Text(text.to_string()), Arg::Wesen(uid)])
}

/// Ein Spieler gibt einem NPC etwas. false = der NPC nimmt es nicht an
pub fn bekommen(w: &mut Welt, sid: u64, uid: u64, gegenstand: &str) -> Option<bool> {
    let def = w.monster.get(&uid).map(|m| m.def.clone())?;
    if !w.skripte.hat("bekommen", &def) {
        return None;
    }
    let raum = raum_von(w, sid);
    Some(ereignis(w, "bekommen", &def, Some(sid), &raum, vec![Arg::Text(gegenstand.to_string()), Arg::Wesen(uid)]))
}

/// Ein Spieler will ein Monster angreifen. false = verhindert
pub fn angriff(w: &mut Welt, sid: u64, uid: u64) -> bool {
    let Some(def) = w.monster.get(&uid).map(|m| m.def.clone()) else { return true };
    let raum = raum_von(w, sid);
    ereignis(w, "angriff", &def, Some(sid), &raum, vec![Arg::Wesen(uid)])
}

/// Takt der Welt: bei_takt-Funktionen
pub fn takt(w: &mut Welt) {
    ereignis(w, "takt", "*", None, "", Vec::new());
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
    let ergebnis = mit_lesern(lua, w, || {
        vorbereiten(lua, w)?;
        let f: mlua::Function = rm.get("timer_pruefen")?;
        f.call::<Option<f64>>(())
    });
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
            "ausgang" => {
                let raum = text("raum");
                let Some(richt) = crate::daten::Richtung::aus_text(&text("richtung")) else {
                    fehler_melden(w, None, &[format!("welt.ausgang: '{}' ist keine Richtung.", text("richtung"))]);
                    continue;
                };
                if a.get::<bool>("normal").unwrap_or(false) {
                    w.extra_ausgaenge.remove(&(raum, richt));
                } else if a.get::<bool>("sperren").unwrap_or(false) {
                    w.extra_ausgaenge.insert((raum, richt), None);
                } else {
                    let ziel = text("ziel");
                    if w.d.raum_existiert(&ziel) {
                        w.extra_ausgaenge.insert((raum, richt), Some(ziel));
                    } else {
                        fehler_melden(w, None, &[format!("welt.ausgang: Raum '{ziel}' gibt es nicht.")]);
                    }
                }
            }
            "tuer" => {
                let raum = text("raum");
                let richt = crate::daten::Richtung::aus_text(&text("richtung"));
                let schluessel = richt.and_then(|r| w.tuer(&raum, r)).map(|(k, _, _, _)| k.clone());
                let zustand = match text("zustand").as_str() {
                    "offen" | "auf" => Some((false, false)),
                    "zu" | "geschlossen" => Some((true, false)),
                    "verschlossen" | "abgeschlossen" => Some((true, true)),
                    _ => None,
                };
                match (schluessel, zustand) {
                    (Some(k), Some(z)) => {
                        w.tuer_zustand.insert(k, z);
                    }
                    (None, _) => fehler_melden(w, None, &[format!("welt.tuer: In '{raum}' gibt es dort keine Tür.")]),
                    (_, None) => fehler_melden(w, None, &["welt.tuer: Zustand muss \"offen\", \"zu\" oder \"verschlossen\" sein.".into()]),
                }
            }
            "zusatz" => {
                let raum = text("raum");
                match a.get::<Option<String>>("text").ok().flatten() {
                    Some(t) => w.raum_zusatz.insert(raum, t),
                    None => w.raum_zusatz.remove(&raum),
                };
            }
            "wesen_sagt" | "wesen_tut" => {
                let uid: u64 = a.get("uid").unwrap_or(0);
                if let Some(m) = w.monster.get(&uid) {
                    let raum = m.raum.clone();
                    let name = crate::text::gross(&w.monster_name(uid));
                    let t = if art == "wesen_sagt" {
                        format!("{{cyan}}{name} sagt: „{}“{{/}}", text("text"))
                    } else {
                        format!("{name} {}", text("text"))
                    };
                    w.sende_raum(&raum, &t, &[]);
                }
            }
            "wesen_lp" => {
                let uid: u64 = a.get("uid").unwrap_or(0);
                let tot = w.monster.get_mut(&uid).is_some_and(|m| {
                    let max = m.lebenspunkte.max(1);
                    m.lebenspunkte = (m.lebenspunkte + zahl("n") as i32).min(max.max(m.lebenspunkte));
                    m.lebenspunkte <= 0
                });
                if let Some(max) = w.monster.get(&uid).and_then(|m| w.d.monster.get(&m.def)).map(|d| d.lebenspunkte) {
                    if let Some(m) = w.monster.get_mut(&uid) {
                        m.lebenspunkte = m.lebenspunkte.min(max);
                    }
                }
                if tot {
                    crate::kampf::monster_stirbt(w, uid);
                }
            }
            "wesen_gehe" => {
                let uid: u64 = a.get("uid").unwrap_or(0);
                let ziel = text("raum");
                if !w.d.raum_existiert(&ziel) {
                    fehler_melden(w, None, &[format!("wesen:gehe: Raum '{ziel}' gibt es nicht.")]);
                    continue;
                }
                let name = crate::text::gross(&w.monster_name(uid));
                if let Some(m) = w.monster.get_mut(&uid) {
                    let alt = std::mem::replace(&mut m.raum, ziel.clone());
                    m.ziele.clear();
                    w.sende_raum(&alt, &format!("{name} geht davon."), &[]);
                    w.sende_raum(&ziel, &format!("{name} kommt herbei."), &[]);
                }
            }
            "wesen_weg" => {
                let uid: u64 = a.get("uid").unwrap_or(0);
                w.monster.remove(&uid);
                for s in w.spieler.values_mut() {
                    if s.kampf_ziel == Some(uid) {
                        s.kampf_ziel = None;
                    }
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
        let d = crate::daten::laden(&ordner).expect("Beispielwelt");
        let (s, fehler) = laden(&ordner, &d);
        assert!(fehler.is_empty(), "{fehler:?}");
        let kriech = s.befehle.iter().find(|b| b.woerter.contains(&"kriech".to_string())).expect("kriech");
        assert_eq!(kriech.raeume, vec!["taverne".to_string()]);
        assert!(s.hat("betreten", "taverne"));
        // Skripte direkt an Raum, Gegenstand und NPC
        assert!(s.befehle.iter().any(|b| b.raeume == vec!["keller".to_string()] && b.woerter.contains(&"schieb".to_string())));
        assert!(s.befehle.iter().any(|b| b.gegenstaende == vec!["zauberstab".to_string()]));
        assert!(s.hat("bekommen", "wirt") && s.hat("anlegen", "zauberstab") && s.hat("betreten", "geheimkammer"));
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
