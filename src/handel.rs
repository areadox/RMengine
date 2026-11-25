//! Geld, Händler und Lehrmeister.
//!
//! Händler: NPCs mit `[npc.haendler]` verkaufen ihre `waren` und kaufen
//! Gegenstände an. Preise richten sich nach dem `wert` des Gegenstands.
//! Lehrmeister: NPCs mit `[[npc.lehrt]]` bringen Fertigkeiten (Stufe für Stufe)
//! und Zauber gegen Geld bei.

use crate::befehle::{finde_gegenstand, gruppieren, passt};
use crate::daten::{LehrDef, MonsterDef};
use crate::gemeinschaft::nummer;
use crate::text::{gross, normalisieren};
use crate::welt::Welt;

/// "2 Gold 5 Silber" – Beträge stehen immer in der kleinsten Einheit ([geld] in welt.toml).
pub fn betrag(w: &Welt, n: u64) -> String {
    crate::darstellung::betrag(n, &w.d.geld_einheiten, w.d.geld.anzeige == "kurz")
}

/// Gerahmte Ausgaben: bei rahmen = "ascii" nur einfache Zeichen.
pub fn rahmen(w: &Welt, t: String) -> String {
    if w.d.darstellung.rahmen == "ascii" {
        crate::darstellung::ascii(&t)
    } else {
        t
    }
}

/// Typen in der Reihenfolge der Anzeige (Stil "kategorien")
const KATEGORIEN: &[(&str, &str)] = &[
    ("nahrung", "Essen"),
    ("trank", "Tränke"),
    ("waffe", "Waffen"),
    ("ruestung", "Rüstung"),
    ("behaelter", "Taschen und Behälter"),
    ("werkzeug", "Werkzeug"),
    ("schmuck", "Schmuck"),
    ("schluessel", "Schlüssel"),
];

fn kategorie(typ: &str) -> (usize, &'static str) {
    KATEGORIEN.iter().position(|(t, _)| *t == typ).map(|i| (i, KATEGORIEN[i].1)).unwrap_or((KATEGORIEN.len(), "Sonstiges"))
}

/// Kurzinfo zu einer Ware, z.B. "+40 LP" oder "fasst 40, -25 %"
fn ware_info(g: &crate::daten::GegenstandDef) -> String {
    let mut v = Vec::new();
    let e = |k: &str| g.eigenschaften.get(k).copied().unwrap_or(0);
    if e("heilung") > 0 {
        v.push(format!("+{} LP", e("heilung")));
    }
    if e("mana") > 0 {
        v.push(format!("+{} MP", e("mana")));
    }
    if e("schaden") > 0 {
        v.push(format!("Schaden {}", e("schaden")));
    }
    if e("ruestung") > 0 {
        v.push(format!("Rüstung {}", e("ruestung")));
    }
    if g.fassungsvermoegen > 0 {
        v.push(format!("fasst {}", g.fassungsvermoegen));
        if g.gewicht_reduktion > 0 {
            v.push(format!("-{} %", g.gewicht_reduktion));
        }
    }
    v.join(", ")
}

fn nach_kategorien(w: &Welt) -> bool {
    w.d.darstellung.haendler == "kategorien"
}

/// Die Waren eines Händlers in der Reihenfolge der Anzeige.
fn waren_von(w: &Welt, d: &MonsterDef) -> Vec<crate::daten::GegenstandDef> {
    let mut v: Vec<_> =
        d.haendler.as_ref().map(|h| h.waren.iter().filter_map(|id| w.d.gegenstaende.get(id).cloned()).collect()).unwrap_or_default();
    if nach_kategorien(w) {
        v.sort_by_key(|g| kategorie(&g.typ).0);
    }
    v
}

/// NPCs im Raum des Spielers, auf die `pruefen` zutrifft.
fn npcs_im_raum<'a>(w: &'a Welt, sid: u64, pruefen: impl Fn(&MonsterDef) -> bool) -> Vec<(u64, &'a MonsterDef)> {
    let raum = &w.spieler[&sid].sp.raum;
    w.monster_im_raum(raum)
        .into_iter()
        .filter_map(|mid| w.monster.get(&mid).and_then(|m| w.d.monster.get(&m.def)).map(|d| (mid, d)))
        .filter(|(_, d)| pruefen(d))
        .collect()
}

fn verkaufspreis(d: &MonsterDef, wert: i32) -> u64 {
    let p = d.haendler.as_ref().map(|h| h.verkauf_prozent).unwrap_or(100) as u64;
    (wert.max(0) as u64 * p / 100).max(1)
}

fn ankaufspreis(d: &MonsterDef, wert: i32) -> u64 {
    let p = d.haendler.as_ref().map(|h| h.ankauf_prozent).unwrap_or(50) as u64;
    wert.max(0) as u64 * p / 100
}

fn npc_sagt(w: &Welt, sid: u64, npc: &MonsterDef, text: &str) {
    let raum = w.spieler[&sid].sp.raum.clone();
    w.sende_raum(&raum, &format!("{{hellcyan}}{} sagt: „{text}“{{/}}", gross(&npc.name)), &[]);
}

/// Befehl "waren": was die Händler und Lehrmeister hier anbieten.
/// Alle Waren der Händler im Raum, in der Reihenfolge der Anzeige (für "kaufe 3").
fn alle_waren(w: &Welt, sid: u64) -> Vec<(MonsterDef, crate::daten::GegenstandDef)> {
    let mut v = Vec::new();
    for (_, d) in npcs_im_raum(w, sid, |d| d.haendler.is_some()) {
        for g in waren_von(w, d) {
            v.push((d.clone(), g));
        }
    }
    v
}

/// Alles, was Lehrmeister im Raum lehren, in der Reihenfolge der Anzeige (für "lerne 2").
fn alle_lehren(w: &Welt, sid: u64) -> Vec<(MonsterDef, LehrDef)> {
    let mut v = Vec::new();
    for (_, d) in npcs_im_raum(w, sid, |d| !d.lehrt.is_empty()) {
        for l in &d.lehrt {
            if w.d.fertigkeiten.contains_key(&l.id) || w.d.zauber.contains_key(&l.id) {
                v.push((d.clone(), l.clone()));
            }
        }
    }
    v
}

pub fn waren(w: &Welt, sid: u64, rest: &str) {
    use crate::darstellung::{Gruppe, Laden, Zeile};
    let liste = npcs_im_raum(w, sid, |d| d.haendler.is_some() || !d.lehrt.is_empty());
    if liste.iter().all(|(_, d)| !rest.is_empty() && !passt(rest, &d.name, &d.id, &d.schluesselwoerter)) {
        w.sende(sid, "Hier verkauft und lehrt niemand etwas.");
        return;
    }
    let s = &w.spieler[&sid];
    let sp = &s.sp;
    let stil = w.d.darstellung.haendler.as_str();
    let mut t = String::new();
    // Nummern laufen über alle Händler im Raum, damit "kaufe 3" eindeutig ist
    let (mut nr_ware, mut nr_lehre) = (0, 0);
    for (_, d) in liste {
        let zeigen = rest.is_empty() || passt(rest, &d.name, &d.id, &d.schluesselwoerter);
        let waren = waren_von(w, d);
        let lehren: Vec<&LehrDef> = d.lehrt.iter().filter(|l| w.d.fertigkeiten.contains_key(&l.id) || w.d.zauber.contains_key(&l.id)).collect();
        if !zeigen {
            nr_ware += waren.len();
            nr_lehre += lehren.len();
            continue;
        }
        let titel = d.haendler.as_ref().map(|h| h.laden.clone()).filter(|l| !l.is_empty()).unwrap_or_else(|| gross(&d.name));
        // Waren, nach Typen gruppiert (nur im Stil "kategorien" mit Überschriften)
        let mut gruppen: Vec<Gruppe> = Vec::new();
        for g in &waren {
            nr_ware += 1;
            let gtitel = if nach_kategorien(w) { kategorie(&g.typ).1.to_string() } else { String::new() };
            if gruppen.last().is_none_or(|gr| gr.titel != gtitel) {
                gruppen.push(Gruppe { titel: gtitel, zeilen: Vec::new() });
            }
            let z = Zeile { nr: nr_ware, name: gross(&g.name), preis: betrag(w, verkaufspreis(d, g.wert)), info: ware_info(g) };
            gruppen.last_mut().expect("Gruppe").zeilen.push(z);
        }
        let ankauf = match &d.haendler {
            Some(h) if h.ankauf => {
                let was = if h.ankauf_typen.is_empty() { "alles".to_string() } else { h.ankauf_typen.join(", ") };
                format!("Kauft an: {was} ({} % des Werts)", h.ankauf_prozent)
            }
            Some(_) if waren.is_empty() => "(verkauft gerade nichts)".into(),
            _ => String::new(),
        };
        let mut zeilen = Vec::new();
        for l in lehren {
            nr_lehre += 1;
            if let Some(f) = w.d.fertigkeiten.get(&l.id) {
                let bis = l.bis.unwrap_or(f.max_stufe);
                let hat = sp.fertigkeiten.get(&f.id).copied().unwrap_or(0);
                zeilen.push(Zeile {
                    nr: nr_lehre,
                    name: f.name.clone(),
                    preis: format!("{} je Stufe", betrag(w, l.preis)),
                    info: format!("bis {bis}, du: {hat}{}", bedingung(w, l)),
                });
            } else if let Some(z) = w.d.zauber.get(&l.id) {
                let art = if z.ist_technik() { "Technik" } else { "Zauber" };
                let kann = if sp.zauber.contains(&z.id) { ", kannst du schon" } else { "" };
                zeilen.push(Zeile { nr: nr_lehre, name: z.name.clone(), preis: betrag(w, l.preis), info: format!("{art}{kann}{}", bedingung(w, l)) });
            }
        }
        let laden = Laden { titel, waren: gruppen, ankauf, lehren: zeilen, lehren_titel: "lehrt".into() };
        t.push_str(&crate::darstellung::laden(stil, &laden, s.breite()));
    }
    t.push_str(&format!(
        "\nDu hast {{hellgelb}}{}{{/}}. {{gelb}}kaufe <nr>{{/}} · {{gelb}}verkaufe <ding>{{/}} · {{gelb}}lerne <nr>{{/}} · {{gelb}}schau <nr>{{/}} für Details",
        betrag(w, sp.geld)
    ));
    w.sende(sid, &rahmen(w, t));
}

/// "schau 3" bei einem Händler: Einzelheiten zur Ware Nummer 3. false = keine Ware.
pub fn ware_zeigen(w: &Welt, sid: u64, n: usize) -> bool {
    let Some((d, g)) = alle_waren(w, sid).into_iter().nth(n) else { return false };
    let mut t = format!("{{fett}}{}{{/}} – {{hellgelb}}{}{{/}}\n", gross(&g.name), betrag(w, verkaufspreis(&d, g.wert)));
    if !g.beschreibung.is_empty() {
        t.push_str(&crate::text::umbrechen(&g.beschreibung, w.spieler[&sid].breite()));
        t.push('\n');
    }
    let info = ware_info(&g);
    if !info.is_empty() {
        t.push_str(&format!("{{grau}}{info}{{/}}\n"));
    }
    t.push_str(&format!("{{grau}}Gewicht {}{}{{/}}", g.gewicht, if g.platz.is_empty() { String::new() } else { format!(", anlegen: {}", w.d.anzeigename(&g.platz)) }));
    w.sende(sid, &t);
    true
}

fn bedingung(w: &Welt, l: &LehrDef) -> String {
    let mut v = Vec::new();
    if l.stufe > 1 {
        v.push(format!("ab Stufe {}", l.stufe));
    }
    if let Some(g) = w.d.gilden.get(&l.gilde) {
        v.push(format!("nur {}", g.name));
    }
    if v.is_empty() {
        String::new()
    } else {
        format!(", {}", v.join(", "))
    }
}

/// "kaufe 3 brot" -> (3, "brot")
fn anzahl_und_name(rest: &str) -> (u32, &str) {
    match rest.split_once(char::is_whitespace) {
        Some((zahl, name)) if zahl.parse::<u32>().is_ok() => (zahl.parse::<u32>().unwrap_or(1).clamp(1, 50), name.trim()),
        _ => (1, rest),
    }
}

/// Befehl "kaufe <ware>"
pub fn kaufen(w: &mut Welt, sid: u64, rest: &str) {
    if rest.is_empty() {
        w.sende(sid, "Was willst du kaufen? ({gelb}waren{/} zeigt das Angebot)");
        return;
    }
    let (anzahl, such) = anzahl_und_name(rest);
    // "kaufe 3" = Ware Nummer 3 aus der Liste ("kaufe 2 3" = zweimal Nummer 3)
    let treffer = match nummer(such) {
        Some(n) => alle_waren(w, sid).into_iter().nth(n),
        None => npcs_im_raum(w, sid, |d| d.haendler.is_some()).into_iter().find_map(|(_, d)| {
            let h = d.haendler.as_ref()?;
            let i = finde_gegenstand(&w.d, &h.waren, such)?;
            let g = w.d.gegenstaende.get(&h.waren[i])?;
            Some((d.clone(), g.clone()))
        }),
    };
    let Some((npc, g)) = treffer else {
        if npcs_im_raum(w, sid, |d| d.haendler.is_some()).is_empty() {
            w.sende(sid, "Hier gibt es keinen Händler.");
        } else {
            w.sende(sid, "Das wird hier nicht verkauft. ({gelb}waren{/} zeigt das Angebot)");
        }
        return;
    };
    let preis = verkaufspreis(&npc, g.wert) * anzahl as u64;
    if !crate::behaelter::kann_tragen(w, sid, g.gewicht.max(0) * anzahl as i32) {
        w.sende(sid, "Das könntest du nicht mehr tragen. ({gelb}stecke{/} Dinge in eine Tasche oder verkaufe etwas.)");
        return;
    }
    let s = w.spieler.get_mut(&sid).expect("Spieler");
    if s.sp.geld < preis {
        let hat = s.sp.geld;
        w.sende(sid, &format!("Das kostet {}, du hast aber nur {}.", betrag(w, preis), betrag(w, hat)));
        return;
    }
    s.sp.geld -= preis;
    for _ in 0..anzahl {
        s.sp.inventar.push(g.id.clone());
    }
    let (name, raum) = (s.sp.name.clone(), s.sp.raum.clone());
    let akk = if g.akkusativ.is_empty() { g.name.clone() } else { g.akkusativ.clone() };
    let was = if anzahl > 1 { format!("{anzahl}x {}", g.name) } else { akk };
    w.sende(sid, &format!("Du kaufst {was} für {{hellgelb}}{}{{/}}.", betrag(w, preis)));
    w.sende_raum(&raum, &format!("{name} kauft etwas bei {}.", if npc.dativ.is_empty() { &npc.name } else { &npc.dativ }), &[sid]);
}

/// Befehl "verkaufe <ding>"
pub fn verkaufen(w: &mut Welt, sid: u64, rest: &str) {
    if rest.is_empty() {
        w.sende(sid, "Was willst du verkaufen?");
        return;
    }
    let haendler: Vec<MonsterDef> = npcs_im_raum(w, sid, |d| d.haendler.as_ref().is_some_and(|h| h.ankauf))
        .into_iter()
        .map(|(_, d)| d.clone())
        .collect();
    if haendler.is_empty() {
        w.sende(sid, "Hier gibt es niemanden, der etwas ankauft.");
        return;
    }
    let (anzahl, such) = anzahl_und_name(rest);
    let Some(i) = finde_gegenstand(&w.d, &w.spieler[&sid].sp.inventar, such) else {
        w.sende(sid, "So etwas hast du nicht dabei. (Angelegte Ausrüstung musst du erst ausziehen.)");
        return;
    };
    let gid = w.spieler[&sid].sp.inventar[i].clone();
    if !crate::behaelter::darf_weg(&w.spieler[&sid].sp, &gid) {
        w.sende(sid, "Da ist noch etwas drin. Nimm es erst heraus (nimm alles aus ...).");
        return;
    }
    let Some(g) = w.d.gegenstaende.get(&gid).cloned() else { return };
    let Some(npc) = haendler.iter().find(|d| {
        let h = d.haendler.as_ref().expect("Händler");
        h.ankauf_typen.is_empty() || h.ankauf_typen.contains(&g.typ)
    }) else {
        npc_sagt(w, sid, &haendler[0], "So etwas kaufe ich nicht.");
        return;
    };
    let stueck = ankaufspreis(npc, g.wert);
    if stueck == 0 {
        npc_sagt(w, sid, npc, "Das ist nichts wert, tut mir leid.");
        return;
    }
    let s = w.spieler.get_mut(&sid).expect("Spieler");
    let mut verkauft = 0;
    while verkauft < anzahl {
        let Some(pos) = s.sp.inventar.iter().position(|x| *x == gid) else { break };
        s.sp.inventar.remove(pos);
        verkauft += 1;
    }
    let preis = stueck * verkauft as u64;
    s.sp.geld += preis;
    let (name, raum) = (s.sp.name.clone(), s.sp.raum.clone());
    let akk = if g.akkusativ.is_empty() { g.name.clone() } else { g.akkusativ.clone() };
    let was = if verkauft > 1 { format!("{verkauft}x {}", g.name) } else { akk };
    w.sende(sid, &format!("Du verkaufst {was} für {{hellgelb}}{}{{/}}.", betrag(w, preis)));
    w.sende_raum(&raum, &format!("{name} verkauft etwas."), &[sid]);
}

/// Befehl "lerne <fertigkeit|zauber>"
pub fn lernen(w: &mut Welt, sid: u64, rest: &str) {
    let lehrer = npcs_im_raum(w, sid, |d| !d.lehrt.is_empty());
    if lehrer.is_empty() {
        w.sende(sid, "Hier ist niemand, der dir etwas beibringen kann.");
        return;
    }
    if rest.is_empty() {
        waren(w, sid, "");
        return;
    }
    let such = normalisieren(rest);
    let gefunden = if let Some(n) = nummer(rest) {
        alle_lehren(w, sid).into_iter().nth(n)
    } else {
        lehrer.iter().find_map(|(_, d)| {
        d.lehrt.iter().find(|l| {
            let (name, woerter) = match (w.d.fertigkeiten.get(&l.id), w.d.zauber.get(&l.id)) {
                (Some(f), _) => (f.name.clone(), Vec::new()),
                (_, Some(z)) => (z.name.clone(), z.schluesselwoerter.clone()),
                _ => return false,
            };
            passt(&such, &name, &l.id, &woerter) || normalisieren(&name) == such
        })
        .map(|l| ((*d).clone(), l.clone()))
        })
    };
    let Some((npc, l)) = gefunden else {
        w.sende(sid, "Das kann dir hier niemand beibringen. ({gelb}waren{/} zeigt, was gelehrt wird)");
        return;
    };
    let sp = &w.spieler[&sid].sp;
    if sp.stufe < l.stufe {
        npc_sagt(w, sid, &npc, &format!("Komm wieder, wenn du Stufe {} erreicht hast.", l.stufe));
        return;
    }
    // Zauber und Techniken einer Gilde lernen nur deren Mitglieder
    let gilde = w.d.zauber.get(&l.id).map(|z| z.gilde.clone()).filter(|g| !g.is_empty()).unwrap_or_else(|| l.gilde.clone());
    let l = LehrDef { gilde, ..l };
    if !l.gilde.is_empty() && sp.gilde != l.gilde {
        let g = w.d.gilden.get(&l.gilde).map(|g| g.name.clone()).unwrap_or_default();
        npc_sagt(w, sid, &npc, &format!("Das lehre ich nur Mitgliedern von „{g}“."));
        return;
    }
    if sp.geld < l.preis {
        npc_sagt(w, sid, &npc, &format!("Das kostet {}. Du hast nicht genug.", betrag(w, l.preis)));
        return;
    }
    let meldung = if let Some(f) = w.d.fertigkeiten.get(&l.id).cloned() {
        let hat = sp.fertigkeiten.get(&f.id).copied().unwrap_or(0);
        let bis = l.bis.unwrap_or(f.max_stufe).min(f.max_stufe);
        if hat >= bis {
            npc_sagt(w, sid, &npc, &format!("Mehr kann ich dir in {} nicht beibringen.", f.name));
            return;
        }
        if let Some(fehlt) = f.voraussetzungen.iter().find(|v| sp.fertigkeiten.get(*v).copied().unwrap_or(0) == 0) {
            let n = w.d.fertigkeiten.get(fehlt).map(|f| f.name.clone()).unwrap_or_else(|| fehlt.clone());
            npc_sagt(w, sid, &npc, &format!("Dafür musst du zuerst {n} lernen."));
            return;
        }
        let s = w.spieler.get_mut(&sid).expect("Spieler");
        s.sp.fertigkeiten.insert(f.id.clone(), hat + 1);
        format!("{} bringt dir {} bei. {{hellgruen}}{} steigt auf {}.{{/}}", gross(&npc.name), f.name, f.name, hat + 1)
    } else if let Some(z) = w.d.zauber.get(&l.id).cloned() {
        if sp.zauber.contains(&z.id) {
            npc_sagt(w, sid, &npc, &format!("{} beherrschst du doch schon.", z.name));
            return;
        }
        let s = w.spieler.get_mut(&sid).expect("Spieler");
        s.sp.zauber.push(z.id.clone());
        let befehl = if z.ist_technik() { "spezial" } else { "zaubere" };
        format!(
            "{} lehrt dich {}. {{hellgruen}}Du beherrschst jetzt {}!{{/}} {{grau}}({befehl} {}){{/}}",
            gross(&npc.name),
            z.name,
            z.name,
            normalisieren(&z.name)
        )
    } else {
        return;
    };
    let s = w.spieler.get_mut(&sid).expect("Spieler");
    s.sp.geld -= l.preis;
    let sp = s.sp.clone();
    w.speichern(&sp);
    w.sende(sid, &format!("{meldung} {{grau}}(-{}){{/}}", betrag(w, l.preis)));
}

/// Befehl "geld"
pub fn geld(w: &Welt, sid: u64) {
    w.sende(sid, &format!("Du hast {{hellgelb}}{}{{/}}.", betrag(w, w.spieler[&sid].sp.geld)));
}

/// Wie viele Gegenstände man zum Bringen dabei hat.
pub fn im_inventar(w: &Welt, sid: u64, gid: &str) -> u32 {
    gruppieren(&w.spieler[&sid].sp.inventar).into_iter().find(|(id, _)| id == gid).map(|(_, n)| n as u32).unwrap_or(0)
}
