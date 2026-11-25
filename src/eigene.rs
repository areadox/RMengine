//! Eigene Befehle aus daten/eigene_befehle/ ([[befehl]]).
//!
//! Ein eigener Befehl prüft Bedingungen (Raum, Gegenstände, Gilde, Kosten,
//! Abklingzeit ...), zeigt Texte wie eine Geste und hat Wirkungen
//! (Heilen, Geld, Gegenstände, Teleport, Zauberwirkung, weitere Befehle).

use crate::befehle::{finde_monster, finde_spieler_im_raum, wirkung_anwenden};
use crate::daten::EigenerBefehlDef;
use crate::spieler::Rolle;
use crate::text::gross;
use crate::welt::Welt;
use rand::Rng;
use std::time::{Duration, Instant};

thread_local! {
    /// Gerade laufende eigene Befehle – Schutz gegen Befehle, die sich (über Umwege) selbst aufrufen
    static LAEUFT: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Der Gegenstand eines Befehls wie "fuehre schwert": (ID, gefunden wo)
fn ding_finden(w: &Welt, sid: u64, b: &EigenerBefehlDef, such: &str) -> Result<String, String> {
    let s = &w.spieler[&sid];
    let ort = if b.gegenstand_ort.is_empty() { "dabei" } else { b.gegenstand_ort.as_str() };
    let angelegt: Vec<String> = s.sp.ausruestung.values().cloned().collect();
    let boden: Vec<String> = w.boden.get(&s.sp.raum).cloned().unwrap_or_default();
    let listen: Vec<&[String]> = match ort {
        "inventar" => vec![&s.sp.inventar],
        "angelegt" => vec![&angelegt],
        "raum" => vec![&boden],
        _ => vec![&s.sp.inventar, &angelegt],
    };
    // Erst unter den passenden Arten suchen, dann unter allen (für eine bessere Meldung)
    let passt_typ = |id: &String| b.gegenstand_typen.is_empty() || w.d.gegenstaende.get(id).is_some_and(|g| b.gegenstand_typen.contains(&g.typ));
    for l in &listen {
        let passende: Vec<String> = l.iter().filter(|id| passt_typ(id)).cloned().collect();
        if let Some(i) = crate::befehle::finde_gegenstand(&w.d, &passende, such) {
            return Ok(passende[i].clone());
        }
    }
    for l in &listen {
        if let Some(i) = crate::befehle::finde_gegenstand(&w.d, l, such) {
            let id = &l[i];
            let t = if b.falscher_gegenstand.trim().is_empty() {
                format!("Mit {} geht das nicht.", w.d.gegenstand_dativ(id))
            } else {
                b.falscher_gegenstand
                    .replace("{ding_akk}", &w.d.gegenstand_akk(id))
                    .replace("{ding_dativ}", &w.d.gegenstand_dativ(id))
                    .replace("{ding}", &w.d.gegenstand_name(id))
            };
            return Err(gross(&t));
        }
    }
    Err(match ort {
        "raum" => "So etwas siehst du hier nicht.".into(),
        "angelegt" => "So etwas hast du nicht angelegt.".into(),
        _ => "So etwas hast du nicht dabei.".into(),
    })
}

/// Ziel eines Befehls: (Name, Dativ, Spieler-ID)
type Ziel = (String, String, Option<u64>);

fn ziel_finden(w: &Welt, raum: &str, such: &str) -> Option<Ziel> {
    if let Some(pid) = finde_spieler_im_raum(w, raum, such) {
        let n = w.spieler[&pid].sp.name.clone();
        return Some((n.clone(), n, Some(pid)));
    }
    let mid = finde_monster(w, raum, such)?;
    let dativ = w
        .monster
        .get(&mid)
        .and_then(|m| w.d.monster.get(&m.def))
        .map(|d| d.dativ.clone())
        .filter(|d| !d.is_empty())
        .unwrap_or_else(|| w.monster_dativ(mid));
    Some((w.monster_akk(mid), dativ, None))
}

/// Prüft alle Bedingungen. Err = Meldung an den Spieler.
fn bedingungen(w: &Welt, sid: u64, b: &EigenerBefehlDef) -> Result<(), String> {
    let s = &w.spieler[&sid];
    let sp = &s.sp;
    let nicht_hier = || if b.nicht_hier.trim().is_empty() { "Das geht hier nicht.".to_string() } else { b.nicht_hier.clone() };
    if !b.raeume.is_empty() && !b.raeume.contains(&sp.raum) {
        return Err(nicht_hier());
    }
    if !b.gebiete.is_empty() && !b.gebiete.contains(&w.d.gebiet(&sp.raum)) {
        return Err(nicht_hier());
    }
    if b.nur_draussen && crate::zeit::drinnen(w, &sp.raum) {
        return Err(nicht_hier());
    }
    if !crate::zeit::passt(w, &b.tageszeiten, &b.jahreszeiten, &b.wetter) {
        return Err(if b.nicht_jetzt.trim().is_empty() { "Das geht jetzt nicht.".into() } else { b.nicht_jetzt.clone() });
    }
    if b.nicht_im_kampf && s.kampf_ziel.is_some() {
        return Err("Nicht mitten im Kampf!".into());
    }
    if sp.stufe < b.mindest_stufe {
        return Err(format!("Dafür musst du mindestens Stufe {} sein.", b.mindest_stufe));
    }
    if !b.gilde.is_empty() && sp.gilde != b.gilde {
        let name = w.d.gilden.get(&b.gilde).map(|g| g.name.clone()).unwrap_or_else(|| b.gilde.clone());
        return Err(format!("Das können nur Mitglieder der Gilde „{name}“."));
    }
    if !b.beruf.is_empty() && !sp.berufe.contains(&b.beruf) {
        let name = w.d.berufe.get(&b.beruf).map(|g| g.name.clone()).unwrap_or_else(|| b.beruf.clone());
        return Err(format!("Dafür brauchst du den Beruf „{name}“."));
    }
    if !b.fertigkeit.is_empty() && sp.fertigkeiten.get(&b.fertigkeit).copied().unwrap_or(0) < b.fertigkeit_stufe.max(1) {
        let name = w.d.fertigkeiten.get(&b.fertigkeit).map(|f| f.name.clone()).unwrap_or_else(|| b.fertigkeit.clone());
        return Err(format!("Dafür brauchst du {name} (mindestens Stufe {}).", b.fertigkeit_stufe.max(1)));
    }
    // Gegenstände: braucht (bleibt) und nimm (wird verbraucht)
    let mut vorrat: Vec<&String> = sp.inventar.iter().chain(sp.ausruestung.values()).collect();
    for g in b.braucht.iter().chain(&b.nimm) {
        match vorrat.iter().position(|x| *x == g) {
            Some(i) if b.nimm.contains(g) => {
                vorrat.remove(i);
            }
            Some(_) => {}
            None => return Err(format!("Dafür brauchst du {}.", w.d.gegenstand_akk(g))),
        }
    }
    if let Some(bis) = s.abklingzeiten.get(&format!("befehl:{}", b.id)) {
        let jetzt = Instant::now();
        if *bis > jetzt {
            return Err(format!("Das kannst du erst in {} Sekunden wieder.", (*bis - jetzt).as_secs() + 1));
        }
    }
    if sp.mana < b.mana_kosten {
        return Err(format!("Dafür reicht dein Mana nicht ({} nötig).", b.mana_kosten));
    }
    if sp.ausdauer < b.ausdauer_kosten {
        return Err(format!("Dafür bist du zu erschöpft ({} Ausdauer nötig).", b.ausdauer_kosten));
    }
    if sp.geld < b.kosten {
        return Err(format!("Das kostet {} – so viel hast du nicht.", crate::handel::betrag(w, b.kosten)));
    }
    Ok(())
}

/// Einen eigenen Befehl ausführen ("bete" oder "segne bert").
pub fn ausfuehren(w: &mut Welt, sid: u64, i: usize, rest: &str) {
    let Some(b) = w.d.befehle.eigene.get(i).and_then(|id| w.d.eigene_befehle.get(id)).cloned() else { return };
    if LAEUFT.with(|l| l.borrow().contains(&b.id)) {
        w.sende(sid, &format!("{{grau}}(„{}“ ruft sich selbst auf – übersprungen.){{/}}", b.woerter.first().cloned().unwrap_or_default()));
        return;
    }
    let noetig = Rolle::aus_text(&b.rolle).unwrap_or(Rolle::Spieler);
    if w.spieler.get(&sid).is_none_or(|s| s.rolle < noetig) {
        w.sende(sid, "Wie bitte? Tippe {gelb}hilfe{/} für eine Liste der Befehle.");
        return;
    }
    if let Err(m) = bedingungen(w, sid, &b) {
        w.sende(sid, &m);
        return;
    }
    let (name, raum) = {
        let s = &w.spieler[&sid];
        (s.sp.name.clone(), s.sp.raum.clone())
    };

    // Gegenstand? ("fuehre schwert")
    let ding = if b.mit_ding() {
        if rest.is_empty() {
            let t = if b.ohne_gegenstand.trim().is_empty() {
                let wort = b.woerter.first().cloned().unwrap_or_default();
                let art = b.gegenstand_typen.first().map(|t| t.as_str()).unwrap_or("ding");
                format!("Womit? (z.B. {wort} <{art}>)")
            } else {
                b.ohne_gegenstand.clone()
            };
            w.sende(sid, &t);
            return;
        }
        match ding_finden(w, sid, &b, rest) {
            Ok(id) => Some(id),
            Err(m) => {
                w.sende(sid, &m);
                return;
            }
        }
    } else {
        None
    };
    let ding_akk = ding.as_deref().map(|d| w.d.gegenstand_akk(d)).unwrap_or_default();
    let ding_name = ding.as_deref().map(|d| w.d.gegenstand_name(d)).unwrap_or_default();
    let ding_dativ = ding.as_deref().map(|d| w.d.gegenstand_dativ(d)).unwrap_or_default();
    // Geht das mit dem Gegenstand überhaupt? (vor Kosten und Texten)
    if let Some(id) = &ding {
        if let Err(m) = ding_pruefen(w, sid, &b.mit_gegenstand, id) {
            w.sende(sid, &m);
            return;
        }
    }

    // Ziel? Nur wenn es Texte mit Ziel gibt (und kein Gegenstand gemeint ist)
    let mit_ziel = ding.is_none() && !rest.is_empty() && !b.selbst_ziel.trim().is_empty();
    let ziel = if mit_ziel {
        match ziel_finden(w, &raum, rest) {
            Some(z) => Some(z),
            None => {
                w.sende(sid, "So jemanden siehst du hier nicht.");
                return;
            }
        }
    } else {
        if b.selbst.trim().is_empty() && !b.selbst_ziel.trim().is_empty() {
            w.sende(sid, "Bei wem denn? (Dieser Befehl braucht ein Ziel.)");
            return;
        }
        None
    };
    let (ziel_name, ziel_dativ, ziel_sid) = ziel.clone().unwrap_or_default();
    let fuellen = |t: &str| {
        gross(
            &t.replace("{name}", &name)
                .replace("{ziel_dativ}", &ziel_dativ)
                .replace("{ziel}", &ziel_name)
                .replace("{ding_akk}", &ding_akk)
                .replace("{ding_dativ}", &ding_dativ)
                .replace("{ding}", &ding_name)
                .replace("{rest}", rest),
        )
    };

    // Nur zum Essen/Trinken da, aber man ist satt? Dann nichts tun.
    let nur_ernaehrung = b.lp == 0 && b.mp == 0 && b.ausdauer == 0 && b.geld == 0 && b.erfahrung == 0 && b.gib.is_empty() && b.zauber.is_empty() && b.teleport.is_empty();
    if nur_ernaehrung && w.d.ernaehrung.aktiv && crate::ernaehrung::zu_satt(w, sid, b.essen, b.trinken) {
        w.sende(sid, if b.essen > 0 { "Du bist satt." } else { "Du hast keinen Durst." });
        return;
    }

    // Kosten werden immer fällig, auch bei Misserfolg
    if let Some(s) = w.spieler.get_mut(&sid) {
        s.sp.mana -= b.mana_kosten;
        s.sp.ausdauer -= b.ausdauer_kosten;
        s.sp.geld -= b.kosten;
        if b.abklingzeit > 0 {
            s.abklingzeiten.insert(format!("befehl:{}", b.id), Instant::now() + Duration::from_secs(b.abklingzeit));
        }
    }

    let erfolg = b.chance == 0 || b.chance >= 100 || rand::thread_rng().gen_range(0..100) < b.chance;
    if !erfolg {
        let t = if b.misserfolg.trim().is_empty() { "Es gelingt dir nicht.".to_string() } else { fuellen(&b.misserfolg) };
        w.sende(sid, &t);
        if !b.misserfolg_andere.trim().is_empty() {
            w.sende_raum(&raum, &fuellen(&b.misserfolg_andere), &[sid]);
        }
        return;
    }

    // Erst mit dem Gegenstand handeln – mit eigenem Text ohne die eingebaute Meldung
    if let Some(id) = &ding {
        let eigener_text = !b.selbst.trim().is_empty() && b.mit_gegenstand != "benutzen";
        if eigener_text {
            crate::welt::still(|| mit_ding(w, sid, &b.mit_gegenstand, id));
        } else {
            mit_ding(w, sid, &b.mit_gegenstand, id);
        }
    }

    // Texte
    if ziel.is_some() {
        w.sende(sid, &fuellen(&b.selbst_ziel));
        let mut ausser = vec![sid];
        if let Some(pid) = ziel_sid.filter(|p| *p != sid) {
            if !b.ziel.trim().is_empty() {
                w.sende(pid, &fuellen(&b.ziel));
            }
            ausser.push(pid);
        }
        if !b.andere_ziel.trim().is_empty() {
            w.sende_raum(&raum, &fuellen(&b.andere_ziel), &ausser);
        }
    } else {
        if !b.selbst.trim().is_empty() {
            w.sende(sid, &fuellen(&b.selbst));
        }
        if !b.andere.trim().is_empty() {
            w.sende_raum(&raum, &fuellen(&b.andere), &[sid]);
        }
    }

    wirkungen(w, sid, &b);
    crate::quests::fortschritt(w, sid, "befehl", &b.id, 1);

    // Weitere Befehle (mit Schutz gegen Endlosschleifen)
    if !b.befehle.is_empty() {
        LAEUFT.with(|l| l.borrow_mut().push(b.id.clone()));
        for c in &b.befehle {
            let c = c.replace("{rest}", rest).replace("{ziel}", rest);
            if !w.spieler.contains_key(&sid) {
                break;
            }
            crate::befehle::befehl_ohne_prompt(w, sid, &c);
        }
        LAEUFT.with(|l| l.borrow_mut().pop());
    }
}

/// Klappt die Handlung mit dem Gegenstand? Err = Meldung
fn ding_pruefen(w: &Welt, sid: u64, was: &str, id: &str) -> Result<(), String> {
    let s = &w.spieler[&sid];
    let Some(g) = w.d.gegenstaende.get(id) else { return Ok(()) };
    match was {
        "anlegen" => {
            if s.sp.ausruestung.values().any(|x| x == id) && !s.sp.inventar.iter().any(|x| x == id) {
                return Err(format!("Du hast {} schon angelegt.", w.d.gegenstand_akk(id)));
            }
            if g.platz.is_empty() {
                return Err(gross(&format!("{} kann man nicht anlegen.", w.d.gegenstand_akk(id))));
            }
            if let Some(alt) = s.sp.ausruestung.get(&g.platz) {
                return Err(format!("Du musst erst {} ausziehen.", w.d.gegenstand_akk(alt)));
            }
            crate::koerper::platz_pruefen(w, sid, g)?;
            crate::koerper::haende_pruefen(w, sid, g)?;
        }
        "ausziehen" if !s.sp.ausruestung.values().any(|x| x == id) => {
            return Err(format!("Du hast {} nicht angelegt.", w.d.gegenstand_akk(id)));
        }
        "nehmen" => {
            if !g.tragbar {
                return Err(gross(&format!("{} kann man nicht mitnehmen.", w.d.gegenstand_akk(id))));
            }
            if !crate::behaelter::kann_tragen(w, sid, g.gewicht.max(0)) {
                return Err("Das ist dir zu schwer.".into());
            }
        }
        _ => {}
    }
    Ok(())
}

/// Was mit dem Gegenstand passiert (die eingebauten Befehle melden selbst, was geschieht)
fn mit_ding(w: &mut Welt, sid: u64, was: &str, id: &str) {
    match was {
        "anlegen" => crate::befehle::anlegen(w, sid, id),
        "ausziehen" => crate::befehle::ausziehen(w, sid, id),
        "benutzen" => crate::befehle::benutzen(w, sid, id),
        "fallenlassen" => crate::befehle::fallenlassen(w, sid, id),
        "nehmen" => crate::befehle::nehmen(w, sid, id),
        "verbrauchen" => {
            if let Some(s) = w.spieler.get_mut(&sid) {
                if let Some(i) = s.sp.inventar.iter().position(|x| x == id) {
                    s.sp.inventar.remove(i);
                } else if let Some(p) = s.sp.ausruestung.iter().find(|(_, x)| *x == id).map(|(p, _)| p.clone()) {
                    s.sp.ausruestung.remove(&p);
                }
            }
        }
        _ => {}
    }
}

fn wirkungen(w: &mut Welt, sid: u64, b: &EigenerBefehlDef) {
    // Gegenstände nehmen und geben
    if let Some(s) = w.spieler.get_mut(&sid) {
        for g in &b.nimm {
            if let Some(i) = s.sp.inventar.iter().position(|x| x == g) {
                s.sp.inventar.remove(i);
            } else if let Some(platz) = s.sp.ausruestung.iter().find(|(_, x)| *x == g).map(|(p, _)| p.clone()) {
                s.sp.ausruestung.remove(&platz);
            }
        }
        s.sp.inventar.extend(b.gib.iter().cloned());
    }
    if !b.gib.is_empty() {
        let namen: Vec<String> = b.gib.iter().map(|g| w.d.gegenstand_akk(g)).collect();
        w.sende(sid, &format!("{{gruen}}Du erhältst {}.{{/}}", crate::text::aufzaehlen(&namen)));
    }
    if b.lp > 0 {
        wirkung_anwenden(w, sid, "heilen", b.lp, "");
    } else if b.lp < 0 {
        if let Some(s) = w.spieler.get_mut(&sid) {
            s.sp.lebenspunkte = (s.sp.lebenspunkte + b.lp).max(1);
            s.sende(&format!("{{rot}}Das tut weh! ({} LP){{/}}", b.lp));
        }
    }
    if b.mp != 0 {
        if b.mp > 0 {
            wirkung_anwenden(w, sid, "mana", b.mp, "");
        } else if let Some(s) = w.spieler.get_mut(&sid) {
            s.sp.mana = (s.sp.mana + b.mp).max(0);
        }
    }
    if b.ausdauer > 0 {
        wirkung_anwenden(w, sid, "ausdauer", b.ausdauer, "");
    }
    if b.essen != 0 || b.trinken != 0 {
        let t = crate::ernaehrung::auffuellen(w, sid, b.essen, b.trinken);
        if !t.is_empty() {
            w.sende(sid, &format!("{{gruen}}({}){{/}}", t.join(", ")));
        }
    }
    if b.geld > 0 {
        if let Some(s) = w.spieler.get_mut(&sid) {
            s.sp.geld += b.geld;
        }
        let t = crate::handel::betrag(w, b.geld);
        w.sende(sid, &format!("{{hellgelb}}Du erhältst {t}.{{/}}"));
    }
    if b.erfahrung > 0 {
        w.sende(sid, &format!("{{gelb}}Du erhältst {} Erfahrungspunkte.{{/}}", b.erfahrung));
        crate::kampf::erfahrung_geben(w, sid, b.erfahrung);
    }
    if let Some(z) = w.d.zauber.get(&b.zauber).cloned() {
        for wi in &z.wirkungen {
            if wi.dauer > 0.0 {
                crate::wirkung::auf_spieler(w, sid, &z, wi, sid);
            } else if matches!(wi.art.as_str(), "heilen" | "mana" | "ausdauer") {
                wirkung_anwenden(w, sid, &wi.art, wi.wert, "");
            }
        }
    }
    if !b.teleport.is_empty() {
        wirkung_anwenden(w, sid, "teleport", 0, &b.teleport);
    }
}

/// Für "hilfe <befehl>"
pub fn hilfe_text(b: &EigenerBefehlDef) -> String {
    let mut t = format!("{{fett}}{}{{/}}", b.woerter.first().cloned().unwrap_or_default());
    if !b.syntax.is_empty() {
        t.push_str(&format!("  {{gelb}}{}{{/}}", b.syntax));
    }
    t.push_str(&format!("\n  {}\n  {{grau}}Wörter:{{/}} {}", b.hilfe, b.woerter.join(", ")));
    if !b.selbst_ziel.is_empty() {
        t.push_str("\n  Auch mit Ziel: z.B. mit dem Namen eines Spielers");
    }
    t
}
