//! Behälter: Taschen, Beutel und Rucksäcke, in die man Dinge stecken kann.
//!
//! Ein Gegenstand mit `fassungsvermoegen > 0` ist ein Behälter. Was darin liegt,
//! wiegt um `gewicht_reduktion` Prozent weniger. Mit `nur_typen` kann man
//! festlegen, was hinein darf (z.B. ein Trankgürtel nur für Tränke).
//!
//! Der Inhalt steht in den Spielerdaten (`behaelter`). Einen gefüllten Behälter
//! kann man nicht ablegen, verkaufen oder weggeben – erst ausleeren.
//!
//! Tragkraft: tragkraft_basis + Stärke × tragkraft_pro_staerke (welt.toml).

use crate::befehle::{finde_gegenstand, gruppieren, passt};
use crate::daten::{Daten, GegenstandDef};
use crate::spieler::{Behaelter, SpielerDaten};
use crate::text::{gross, normalisieren};
use crate::welt::Welt;

pub fn ist_behaelter(d: &Daten, id: &str) -> bool {
    d.gegenstaende.get(id).is_some_and(|g| g.fassungsvermoegen > 0)
}

/// Wie viele Gegenstände mit dieser ID man hat (Gepäck und angelegt).
fn besitzt(sp: &SpielerDaten, gid: &str) -> usize {
    sp.inventar.iter().chain(sp.ausruestung.values()).filter(|g| *g == gid).count()
}

/// Gewicht des Inhalts, wie es auf der Waage wäre (ohne Reduktion).
fn inhalt_gewicht(d: &Daten, inhalt: &[String]) -> i32 {
    inhalt.iter().filter_map(|g| d.gegenstaende.get(g)).map(|g| g.gewicht.max(0)).sum()
}

/// Was man insgesamt trägt: Gepäck, Ausrüstung und Behälterinhalt (verringert).
pub fn tragegewicht(d: &Daten, sp: &SpielerDaten) -> i32 {
    let mut summe: i32 =
        sp.inventar.iter().chain(sp.ausruestung.values()).filter_map(|g| d.gegenstaende.get(g)).map(|g| g.gewicht.max(0)).sum();
    for b in &sp.behaelter {
        let reduktion = d.gegenstaende.get(&b.gegenstand).map(|g| g.gewicht_reduktion.min(100)).unwrap_or(0) as i32;
        let roh = inhalt_gewicht(d, &b.inhalt);
        summe += (roh * (100 - reduktion) + 99) / 100; // aufrunden
    }
    summe
}

/// Wie viel man tragen kann (None = unbegrenzt).
pub fn tragkraft(w: &Welt, sid: u64) -> Option<i32> {
    let (basis, pro) = (w.d.welt.tragkraft_basis, w.d.welt.tragkraft_pro_staerke);
    if basis == 0 && pro == 0 {
        return None;
    }
    Some(basis + crate::kampf::wert(w, sid, "staerke").max(0) * pro)
}

/// Kann man noch `zusatz` Gewicht tragen?
pub fn kann_tragen(w: &Welt, sid: u64, zusatz: i32) -> bool {
    match tragkraft(w, sid) {
        None => true,
        Some(max) => tragegewicht(&w.d, &w.spieler[&sid].sp) + zusatz <= max,
    }
}

/// "Gewicht 12/70" für die Anzeige
pub fn gewicht_text(w: &Welt, sid: u64) -> String {
    let g = tragegewicht(&w.d, &w.spieler[&sid].sp);
    match tragkraft(w, sid) {
        Some(max) if g > max => format!("{{hellrot}}Gewicht {g}/{max} – überladen!{{/}}"),
        Some(max) => format!("Gewicht {g}/{max}"),
        None => format!("Gewicht {g}"),
    }
}

/// Darf ein Gegenstand weg (ablegen, verkaufen ...)? Nicht, wenn es ein gefüllter
/// Behälter ist, von dem man sonst keinen leeren mehr hätte.
pub fn darf_weg(sp: &SpielerDaten, gid: &str) -> bool {
    let gefuellt = sp.behaelter.iter().filter(|b| b.gegenstand == gid && !b.inhalt.is_empty()).count();
    gefuellt < besitzt(sp, gid)
}

/// Leere Einträge wegräumen.
fn aufraeumen(sp: &mut SpielerDaten) {
    sp.behaelter.retain(|b| !b.inhalt.is_empty());
}

/// Welcher Behälter ist gemeint? Gibt die Gegenstand-ID zurück.
fn behaelter_finden(w: &Welt, sp: &SpielerDaten, such: &str) -> Option<GegenstandDef> {
    let mut alle: Vec<String> = sp.inventar.iter().chain(sp.ausruestung.values()).cloned().collect();
    alle.retain(|g| ist_behaelter(&w.d, g));
    let i = finde_gegenstand(&w.d, &alle, such)?;
    w.d.gegenstaende.get(&alle[i]).cloned()
}

/// Passt der Gegenstand `g` noch in einen Behälter vom Typ `b`? Gibt den Index
/// des Eintrags zurück (oder None für einen neuen Eintrag) – oder einen Grund.
fn platz_finden(w: &Welt, sp: &SpielerDaten, b: &GegenstandDef, g: &GegenstandDef) -> Result<Option<usize>, String> {
    if !b.nur_typen.is_empty() && !b.nur_typen.contains(&g.typ) {
        return Err(format!("In {} passt nur: {}.", akkusativ(b), b.nur_typen.join(", ")));
    }
    if g.fassungsvermoegen > 0 {
        return Err("Behälter kann man nicht ineinander stecken.".into());
    }
    let passt_in = |inhalt: &[String]| {
        inhalt_gewicht(&w.d, inhalt) + g.gewicht.max(0) <= b.fassungsvermoegen && (b.max_dinge == 0 || (inhalt.len() as u32) < b.max_dinge)
    };
    for (i, e) in sp.behaelter.iter().enumerate() {
        if e.gegenstand == b.id && passt_in(&e.inhalt) {
            return Ok(Some(i));
        }
    }
    let vorhanden = sp.behaelter.iter().filter(|e| e.gegenstand == b.id).count();
    if vorhanden < besitzt(sp, &b.id) && passt_in(&[]) {
        return Ok(None);
    }
    Err(format!("{} ist voll.", gross(&b.name)))
}

/// Befehl "stecke <ding|alles> in <behälter>"
pub fn stecken(w: &mut Welt, sid: u64, rest: &str) {
    let Some((ding, behaelter)) = rest.rsplit_once(" in ") else {
        w.sende(sid, "Benutzung: stecke <ding> in <behälter>, z.B. stecke brot in rucksack");
        return;
    };
    let sp = w.spieler[&sid].sp.clone();
    let Some(b) = behaelter_finden(w, &sp, behaelter.trim()) else {
        w.sende(sid, "So einen Behälter hast du nicht.");
        return;
    };
    // Welche Dinge? Den Behälter selbst und andere Behälter nie.
    let kandidaten: Vec<usize> = if normalisieren(ding) == "alles" {
        (0..sp.inventar.len()).filter(|i| !ist_behaelter(&w.d, &sp.inventar[*i])).collect()
    } else {
        match finde_gegenstand(&w.d, &sp.inventar, ding.trim()) {
            Some(i) => vec![i],
            None => {
                w.sende(sid, "So etwas hast du nicht im Gepäck.");
                return;
            }
        }
    };
    let mut gesteckt: Vec<String> = Vec::new();
    let mut grund = None;
    for i in kandidaten.into_iter().rev() {
        let gid = w.spieler[&sid].sp.inventar[i].clone();
        let Some(g) = w.d.gegenstaende.get(&gid).cloned() else { continue };
        match platz_finden(w, &w.spieler[&sid].sp, &b, &g) {
            Ok(platz) => {
                let s = w.spieler.get_mut(&sid).expect("Spieler");
                s.sp.inventar.remove(i);
                match platz {
                    Some(e) => s.sp.behaelter[e].inhalt.push(gid.clone()),
                    None => s.sp.behaelter.push(Behaelter { gegenstand: b.id.clone(), inhalt: vec![gid.clone()] }),
                }
                gesteckt.push(gid);
            }
            Err(e) => grund = Some(e),
        }
    }
    let ziel = akkusativ(&b);
    if gesteckt.is_empty() {
        w.sende(sid, &grund.unwrap_or_else(|| "Da ist nichts, was du hineinstecken könntest.".into()));
        return;
    }
    gesteckt.reverse();
    let teile: Vec<String> = gruppieren(&gesteckt)
        .into_iter()
        .map(|(id, n)| if n > 1 { format!("{n}x {}", w.d.gegenstand_name(&id)) } else { w.d.gegenstand_akk(&id) })
        .collect();
    w.sende(sid, &format!("Du steckst {} in {ziel}.", crate::text::aufzaehlen(&teile)));
    if let Some(e) = grund {
        w.sende(sid, &format!("{{grau}}{e}{{/}}"));
    }
}

/// "nimm <ding|alles> aus <behälter>" – gibt false zurück, wenn es kein "aus" gibt.
pub fn herausnehmen(w: &mut Welt, sid: u64, rest: &str) -> bool {
    let Some((ding, behaelter)) = rest.rsplit_once(" aus ") else { return false };
    let sp = w.spieler[&sid].sp.clone();
    let Some(b) = behaelter_finden(w, &sp, behaelter.trim()) else {
        // Vielleicht meint er etwas anderes ("aus" im Namen) – dann normal nehmen
        return false;
    };
    let alles = normalisieren(ding) == "alles";
    let mut genommen = Vec::new();
    let mut zu_schwer = false;
    loop {
        let sp = &w.spieler[&sid].sp;
        let treffer = sp.behaelter.iter().enumerate().filter(|(_, e)| e.gegenstand == b.id).find_map(|(e, eintrag)| {
            let i = if alles { (!eintrag.inhalt.is_empty()).then_some(0) } else { finde_gegenstand(&w.d, &eintrag.inhalt, ding.trim()) };
            i.map(|i| (e, i))
        });
        let Some((e, i)) = treffer else { break };
        let gid = sp.behaelter[e].inhalt[i].clone();
        // Herausnehmen macht es schwerer (die Reduktion fällt weg)
        let g = w.d.gegenstaende.get(&gid).map(|g| g.gewicht.max(0)).unwrap_or(0);
        let ersparnis = g * b.gewicht_reduktion.min(100) as i32 / 100;
        if !kann_tragen(w, sid, ersparnis) {
            zu_schwer = true;
            break;
        }
        let s = w.spieler.get_mut(&sid).expect("Spieler");
        s.sp.behaelter[e].inhalt.remove(i);
        s.sp.inventar.push(gid.clone());
        genommen.push(gid);
        if !alles {
            break;
        }
    }
    if let Some(s) = w.spieler.get_mut(&sid) {
        aufraeumen(&mut s.sp);
    }
    if genommen.is_empty() {
        let t = if zu_schwer { "Das wäre dir zu schwer.".to_string() } else { format!("Das ist nicht in {}.", dativ(w, &b)) };
        w.sende(sid, &t);
        return true;
    }
    let teile: Vec<String> = gruppieren(&genommen)
        .into_iter()
        .map(|(id, n)| if n > 1 { format!("{n}x {}", w.d.gegenstand_name(&id)) } else { w.d.gegenstand_akk(&id) })
        .collect();
    w.sende(sid, &format!("Du nimmst {} aus {}.", crate::text::aufzaehlen(&teile), dativ(w, &b)));
    if zu_schwer {
        w.sende(sid, "{grau}Mehr kannst du nicht tragen.{/}");
    }
    true
}

/// Unbestimmten Artikel durch den bestimmten ersetzen.
fn artikel_tauschen(t: &str, tausch: &[(&str, &str)]) -> String {
    let klein = t.to_lowercase();
    for (von, nach) in tausch {
        if klein.starts_with(von) {
            return format!("{nach}{}", &t[von.len()..]);
        }
    }
    t.to_string()
}

/// "dem Rucksack", "der Zaubertasche"
fn dativ(_w: &Welt, b: &GegenstandDef) -> String {
    if !b.dativ.is_empty() {
        return artikel_tauschen(&b.dativ, &[("einem ", "dem "), ("einer ", "der ")]);
    }
    artikel_tauschen(&b.name, &[("ein ", "dem "), ("eine ", "der ")])
}

/// "den Rucksack", "die Zaubertasche", "das Kästchen"
fn akkusativ(b: &GegenstandDef) -> String {
    if b.akkusativ.is_empty() {
        artikel_tauschen(&b.name, &[("eine ", "die "), ("ein ", "das ")])
    } else {
        artikel_tauschen(&b.akkusativ, &[("einen ", "den "), ("eine ", "die "), ("ein ", "das ")])
    }
}

/// "Im Rucksack", "In der Zaubertasche"
fn darin(w: &Welt, b: &GegenstandDef) -> String {
    let d = dativ(w, b);
    match d.strip_prefix("dem ") {
        Some(rest) => format!("Im {rest}"),
        None => format!("In {d}"),
    }
}

/// Inhalt aller Behälter für die Anzeige im Inventar.
pub fn inventar_zeilen(w: &Welt, sid: u64) -> String {
    let sp = &w.spieler[&sid].sp;
    let mut t = String::new();
    let mut gezeigt: Vec<String> = Vec::new();
    for gid in sp.inventar.iter().chain(sp.ausruestung.values()) {
        if !ist_behaelter(&w.d, gid) || gezeigt.contains(gid) {
            continue;
        }
        gezeigt.push(gid.clone());
        let Some(b) = w.d.gegenstaende.get(gid) else { continue };
        for e in sp.behaelter.iter().filter(|e| e.gegenstand == *gid) {
            let roh = inhalt_gewicht(&w.d, &e.inhalt);
            t.push_str(&format!("{{fett}}{}{{/}} {{grau}}({roh}/{}){{/}}:\n", darin(w, b), b.fassungsvermoegen));
            for (id, n) in gruppieren(&e.inhalt) {
                let name = w.d.gegenstand_name(&id);
                if n > 1 {
                    t.push_str(&format!("  [{n}x] {name}\n"));
                } else {
                    t.push_str(&format!("  {name}\n"));
                }
            }
        }
    }
    t
}

/// "schau rucksack": Inhalt eines Behälters (true = es war ein Behälter).
pub fn ansehen(w: &Welt, sid: u64, such: &str) -> bool {
    let such = such.strip_prefix("in ").unwrap_or(such).trim();
    let sp = &w.spieler[&sid].sp;
    let Some(b) = behaelter_finden(w, sp, such) else { return false };
    if !passt(such, &b.name, &b.id, &b.schluesselwoerter) {
        return false;
    }
    let mut t = String::new();
    if !b.beschreibung.is_empty() {
        t.push_str(&b.beschreibung);
        t.push('\n');
    }
    let eintraege: Vec<&Behaelter> = sp.behaelter.iter().filter(|e| e.gegenstand == b.id).collect();
    if eintraege.is_empty() {
        t.push_str(&format!("{} ist leer.", gross(&b.name)));
    }
    for e in eintraege {
        let roh = inhalt_gewicht(&w.d, &e.inhalt);
        t.push_str(&format!("Darin {{grau}}({roh}/{}){{/}}:\n", b.fassungsvermoegen));
        for (id, n) in gruppieren(&e.inhalt) {
            t.push_str(&format!("  {}{}\n", if n > 1 { format!("[{n}x] ") } else { String::new() }, w.d.gegenstand_name(&id)));
        }
    }
    let mut info = vec![format!("fasst {}", b.fassungsvermoegen)];
    if b.gewicht_reduktion > 0 {
        info.push(format!("Inhalt {} % leichter", b.gewicht_reduktion));
    }
    if !b.nur_typen.is_empty() {
        info.push(format!("nur {}", b.nur_typen.join(", ")));
    }
    t.push_str(&format!("{{grau}}({}){{/}}", info.join(", ")));
    w.sende(sid, t.trim_end());
    true
}

/// Beim Tod mit "fallen_lassen": auch der Inhalt der Behälter fällt zu Boden.
pub fn alles_ausleeren(sp: &mut SpielerDaten) -> Vec<String> {
    sp.behaelter.drain(..).flat_map(|b| b.inhalt).collect()
}

/// Nach dem Neuladen: Inhalt gelöschter Gegenstände entfernen,
/// Inhalt von Behältern, die keine mehr sind, ins Gepäck.
pub fn nach_neuladen(d: &Daten, sp: &mut SpielerDaten) {
    let mut zurueck = Vec::new();
    for b in &mut sp.behaelter {
        b.inhalt.retain(|g| d.gegenstaende.contains_key(g));
        if !ist_behaelter(d, &b.gegenstand) || besitzt_roh(&sp.inventar, &sp.ausruestung, &b.gegenstand) == 0 {
            zurueck.append(&mut b.inhalt);
        }
    }
    sp.inventar.extend(zurueck);
    aufraeumen(sp);
}

fn besitzt_roh(inv: &[String], aus: &std::collections::BTreeMap<String, String>, gid: &str) -> usize {
    inv.iter().chain(aus.values()).filter(|g| *g == gid).count()
}
