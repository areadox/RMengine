//! Rassen, Gilden und Berufe (Zünfte).
//!
//! - Rassen wählt man bei der Erschaffung der Figur. Sie geben Zuschläge auf
//!   Werte, Lebenspunkte, Mana und Ausdauer sowie Start-Fertigkeiten.
//! - Gilden und Berufe tritt man bei einem NPC bei, der dafür zuständig ist
//!   (`gilde = "..."` bzw. `beruf = "..."` beim NPC). Man kann in einer Gilde
//!   sein und mehrere Berufe haben (`max_berufe` in welt.toml).
//!
//! Was Gilden und Berufe genau können, baut man mit Lehrmeistern (`lehrt`),
//! Zaubern/Techniken mit `gilde = "..."` und Aufgaben (`gilde` bei Quests) aus.

use crate::daten::{Daten, GemeinschaftDef};
use crate::handel::betrag;
use crate::spieler::SpielerDaten;
use crate::text::{gross, normalisieren};
use crate::welt::Welt;

/// Die Vorteile einer Rasse, Gilde oder eines Berufs anwenden (oder mit `vorzeichen = -1` zurücknehmen).
fn vorteile(sp: &mut SpielerDaten, g: &GemeinschaftDef, vorzeichen: i32) {
    sp.max_lebenspunkte = (sp.max_lebenspunkte + g.lebenspunkte * vorzeichen).max(1);
    sp.max_mana = (sp.max_mana + g.mana * vorzeichen).max(0);
    sp.max_ausdauer = (sp.max_ausdauer + g.ausdauer * vorzeichen).max(1);
    sp.lebenspunkte = sp.lebenspunkte.min(sp.max_lebenspunkte);
    sp.mana = sp.mana.min(sp.max_mana);
    sp.ausdauer = sp.ausdauer.min(sp.max_ausdauer);
    if vorzeichen > 0 {
        sp.lebenspunkte = (sp.lebenspunkte + g.lebenspunkte).min(sp.max_lebenspunkte);
        sp.mana = (sp.mana + g.mana).min(sp.max_mana);
        sp.ausdauer = (sp.ausdauer + g.ausdauer).min(sp.max_ausdauer);
        for (f, stufe) in &g.fertigkeiten {
            let e = sp.fertigkeiten.entry(f.clone()).or_insert(0);
            *e = (*e).max(*stufe);
        }
        for z in &g.zauber {
            if !sp.zauber.contains(z) {
                sp.zauber.push(z.clone());
            }
        }
    }
}

/// Rassen, die man bei der Erschaffung wählen kann.
pub fn waehlbare_rassen(d: &Daten) -> Vec<&GemeinschaftDef> {
    d.rassen.values().filter(|r| r.waehlbar).collect()
}

/// Sucht eine Rasse nach Name oder ID.
pub fn rasse_finden<'a>(d: &'a Daten, such: &str) -> Option<&'a GemeinschaftDef> {
    // Auch über die Nummer aus der Liste
    if let Some(n) = nummer(such) {
        return waehlbare_rassen(d).into_iter().nth(n);
    }
    let s = normalisieren(such);
    waehlbare_rassen(d)
        .into_iter()
        .find(|r| normalisieren(&r.id) == s || normalisieren(&r.name) == s || normalisieren(&r.name).starts_with(&s) && !s.is_empty())
}

/// "2" oder "2." -> Some(1): eine Nummer aus einer Liste (ab 1)
pub fn nummer(t: &str) -> Option<usize> {
    t.trim().trim_end_matches(['.', ')']).parse::<usize>().ok().filter(|n| *n > 0).map(|n| n - 1)
}

/// Rasse festlegen (bei der Erschaffung).
pub fn rasse_anwenden(d: &Daten, sp: &mut SpielerDaten, id: &str) {
    if let Some(r) = d.rassen.get(id) {
        rasse_setzen(sp, r);
    }
}

fn rasse_setzen(sp: &mut SpielerDaten, r: &GemeinschaftDef) {
    sp.rasse = r.id.clone();
    vorteile(sp, r, 1);
    sp.lebenspunkte = sp.max_lebenspunkte;
    sp.mana = sp.max_mana;
    sp.ausdauer = sp.max_ausdauer;
}

/// Text für die Rassenwahl beim Einloggen.
pub fn rassen_auswahl(d: &Daten) -> String {
    let mut t = String::from("\n{fett}Wähle die Rasse deiner Figur:{/}\n");
    for (i, r) in waehlbare_rassen(d).into_iter().enumerate() {
        t.push_str(&format!("  {{gelb}}{:>2}. {:<12}{{/}} {}\n", i + 1, r.name, kurz(d, r)));
    }
    t.push_str("Tippe die Nummer oder den Namen (oder 'info <nr|rasse>' für mehr): ");
    t
}

/// "Stärke +2, Intelligenz -1, +10 LP"
fn kurz(d: &Daten, g: &GemeinschaftDef) -> String {
    let mut v: Vec<String> = g.werte.iter().filter(|(_, w)| **w != 0).map(|(k, w)| format!("{} {w:+}", d.anzeigename(k))).collect();
    for (n, w) in [("max. LP", g.lebenspunkte), ("max. MP", g.mana), ("max. AP", g.ausdauer)] {
        if w != 0 {
            v.push(format!("{n} {w:+}"));
        }
    }
    if v.is_empty() {
        String::new()
    } else {
        format!("{{grau}}({}){{/}}", v.join(", "))
    }
}

/// Ausführliche Beschreibung einer Rasse, Gilde oder eines Berufs.
pub fn info(d: &Daten, g: &GemeinschaftDef) -> String {
    let mut t = format!("{{fett}}{}{{/}} {}\n", g.name, kurz(d, g));
    if !g.beschreibung.is_empty() {
        t.push_str(&crate::text::umbrechen(&g.beschreibung, 76));
        t.push('\n');
    }
    if !g.fertigkeiten.is_empty() {
        let f: Vec<String> = g
            .fertigkeiten
            .iter()
            .map(|(id, st)| format!("{} {st}", d.fertigkeiten.get(id).map(|f| f.name.clone()).unwrap_or_else(|| id.clone())))
            .collect();
        t.push_str(&format!("Fertigkeiten: {}\n", f.join(", ")));
    }
    if !g.zauber.is_empty() {
        let z: Vec<String> = g.zauber.iter().map(|id| d.zauber.get(id).map(|z| z.name.clone()).unwrap_or_else(|| id.clone())).collect();
        t.push_str(&format!("Zauber und Techniken: {}\n", z.join(", ")));
    }
    t
}

/// Befehl "rasse [name]"
pub fn rasse(w: &mut Welt, sid: u64, rest: &str) {
    if w.d.rassen.is_empty() {
        w.sende(sid, "In dieser Welt gibt es keine verschiedenen Rassen.");
        return;
    }
    let hat = w.spieler[&sid].sp.rasse.clone();
    if let Some(r) = w.d.rassen.get(&hat) {
        let t = format!("Deine Rasse:\n{}", info(&w.d, r));
        w.sende(sid, t.trim_end());
        return;
    }
    // Ältere Figuren ohne Rasse dürfen einmal wählen
    if rest.is_empty() {
        let t = rassen_auswahl(&w.d).replace("Tippe die Nummer oder den Namen (oder 'info <nr|rasse>' für mehr): ", "Wählen mit: rasse <nr|name>");
        w.sende(sid, t.trim());
        return;
    }
    let Some(r) = rasse_finden(&w.d, rest).cloned() else {
        w.sende(sid, "Diese Rasse gibt es nicht. Tippe 'rasse' für die Liste.");
        return;
    };
    let s = w.spieler.get_mut(&sid).expect("Spieler");
    rasse_setzen(&mut s.sp, &r);
    let sp = s.sp.clone();
    w.speichern(&sp);
    w.sende(sid, &format!("{{hellgruen}}Deine Rasse: {}.{{/}}", r.name));
}

/// NPCs im Raum, bei denen man einer Gilde oder einem Beruf beitreten kann.
fn aufnehmende(w: &Welt, sid: u64) -> Vec<(String, bool, String)> {
    // (ID, ist_gilde, NPC-Name)
    let raum = &w.spieler[&sid].sp.raum;
    let mut v = Vec::new();
    for mid in w.monster_im_raum(raum) {
        let Some(d) = w.monster.get(&mid).and_then(|m| w.d.monster.get(&m.def)) else { continue };
        if !d.gilde.is_empty() {
            v.push((d.gilde.clone(), true, d.name.clone()));
        }
        if !d.beruf.is_empty() {
            v.push((d.beruf.clone(), false, d.name.clone()));
        }
    }
    v
}

/// Befehl "beitreten [gilde|beruf]"
pub fn beitreten(w: &mut Welt, sid: u64, rest: &str) {
    let moeglich = aufnehmende(w, sid);
    if moeglich.is_empty() {
        w.sende(sid, "Hier kannst du keiner Gilde und keinem Beruf beitreten.");
        return;
    }
    let such = normalisieren(rest);
    let gewaehlt = match nummer(rest) {
        Some(n) => moeglich.get(n),
        None => moeglich.iter().find(|(id, gilde, _)| {
            let g = if *gilde { w.d.gilden.get(id) } else { w.d.berufe.get(id) };
            such.is_empty() || g.is_some_and(|g| normalisieren(&g.name).contains(&such) || normalisieren(&g.id) == such)
        }),
    };
    let Some((id, ist_gilde, npc)) = gewaehlt.cloned() else {
        w.sende(sid, "Dem kannst du hier nicht beitreten.");
        return;
    };
    if such.is_empty() && moeglich.len() > 1 {
        let mut t = String::from("Wem willst du beitreten?\n");
        for (i, (id, gilde, _)) in moeglich.iter().enumerate() {
            let g = if *gilde { w.d.gilden.get(id) } else { w.d.berufe.get(id) };
            if let Some(g) = g {
                let art = if *gilde { "Gilde" } else { "Beruf" };
                t.push_str(&format!("  {{gelb}}{:>2}.{{/}} {} {{grau}}({art}){{/}}\n", i + 1, g.name));
            }
        }
        t.push_str("Tippe: beitreten <nr>");
        w.sende(sid, &t);
        return;
    }
    let Some(g) = (if ist_gilde { w.d.gilden.get(&id) } else { w.d.berufe.get(&id) }).cloned() else { return };
    let sp = &w.spieler[&sid].sp;
    let npc = gross(&npc);
    let antwort = |t: &str| format!("{{hellcyan}}{npc} sagt: „{t}“{{/}}");
    if ist_gilde && sp.gilde == g.id || !ist_gilde && sp.berufe.contains(&g.id) {
        w.sende(sid, &antwort("Du gehörst doch schon zu uns!"));
        return;
    }
    if ist_gilde && !sp.gilde.is_empty() {
        let alt = w.d.gilden.get(&sp.gilde).map(|g| g.name.clone()).unwrap_or_default();
        w.sende(sid, &antwort(&format!("Du bist schon bei „{alt}“. Tritt dort erst aus (austreten gilde).")));
        return;
    }
    if !ist_gilde && sp.berufe.len() as u32 >= w.d.welt.max_berufe {
        w.sende(sid, &antwort(&format!("Du hast schon {} Berufe – mehr schafft niemand.", sp.berufe.len())));
        return;
    }
    if sp.stufe < g.aufnahme_stufe {
        w.sende(sid, &antwort(&format!("Komm wieder, wenn du Stufe {} erreicht hast.", g.aufnahme_stufe)));
        return;
    }
    if !g.rassen.is_empty() && !g.rassen.contains(&sp.rasse) {
        w.sende(sid, &antwort("Deinesgleichen nehmen wir nicht auf."));
        return;
    }
    if sp.geld < g.preis {
        w.sende(sid, &antwort(&format!("Die Aufnahme kostet {}.", betrag(w, g.preis))));
        return;
    }
    let s = w.spieler.get_mut(&sid).expect("Spieler");
    s.sp.geld -= g.preis;
    if ist_gilde {
        s.sp.gilde = g.id.clone();
        if !g.titel.is_empty() {
            s.sp.titel = g.titel.clone();
        }
    } else {
        s.sp.berufe.push(g.id.clone());
    }
    vorteile(&mut s.sp, &g, 1);
    let (name, raum) = (s.sp.name.clone(), s.sp.raum.clone());
    let sp = s.sp.clone();
    w.speichern(&sp);
    let text = if g.aufnahme_text.is_empty() { format!("Willkommen bei „{}“!", g.name) } else { g.aufnahme_text.clone() };
    w.sende(sid, &antwort(&text));
    w.sende(sid, &format!("{{hellgruen}}Du gehörst jetzt zu: {}.{{/}}", g.name));
    w.sende_raum(&raum, &format!("{name} wird bei „{}“ aufgenommen.", g.name), &[sid]);
}

/// Befehl "austreten gilde|<beruf>"
pub fn austreten(w: &mut Welt, sid: u64, rest: &str) {
    let such = normalisieren(rest);
    let sp = &w.spieler[&sid].sp;
    let gilde = w.d.gilden.get(&sp.gilde);
    // Mitgliedschaften durchnummeriert: zuerst die Gilde, dann die Berufe
    let mut liste: Vec<(String, bool)> = Vec::new();
    if gilde.is_some() {
        liste.push((sp.gilde.clone(), true));
    }
    liste.extend(sp.berufe.iter().filter(|b| w.d.berufe.contains_key(*b)).map(|b| (b.clone(), false)));
    let (id, ist_gilde) = if let Some(n) = nummer(rest) {
        match liste.get(n) {
            Some(e) => e.clone(),
            None => {
                w.sende(sid, "Diese Nummer gibt es nicht.");
                return;
            }
        }
    } else if gilde.is_some_and(|g| such == "gilde" || normalisieren(&g.name).contains(&such) && !such.is_empty()) {
        (sp.gilde.clone(), true)
    } else if let Some(b) = sp
        .berufe
        .iter()
        .find(|b| w.d.berufe.get(*b).is_some_and(|g| normalisieren(&g.name).contains(&such) || normalisieren(&g.id) == such) && !such.is_empty())
    {
        (b.clone(), false)
    } else {
        if liste.is_empty() {
            w.sende(sid, "Du bist in keiner Gilde und hast keinen Beruf.");
            return;
        }
        let mut t = String::from("Woraus willst du austreten?\n");
        for (i, (id, ist_gilde)) in liste.iter().enumerate() {
            let g = if *ist_gilde { w.d.gilden.get(id) } else { w.d.berufe.get(id) };
            if let Some(g) = g {
                let art = if *ist_gilde { "Gilde" } else { "Beruf" };
                t.push_str(&format!("  {{gelb}}{:>2}.{{/}} {} {{grau}}({art}){{/}}\n", i + 1, g.name));
            }
        }
        t.push_str("Tippe: austreten <nr>");
        w.sende(sid, &t);
        return;
    };
    let Some(g) = (if ist_gilde { w.d.gilden.get(&id) } else { w.d.berufe.get(&id) }).cloned() else { return };
    let s = w.spieler.get_mut(&sid).expect("Spieler");
    if ist_gilde {
        s.sp.gilde.clear();
        if !g.titel.is_empty() && s.sp.titel == g.titel {
            s.sp.titel.clear();
        }
    } else {
        s.sp.berufe.retain(|b| *b != id);
    }
    vorteile(&mut s.sp, &g, -1);
    let sp = s.sp.clone();
    w.speichern(&sp);
    w.sende(sid, &format!("Du gehörst nicht mehr zu: {}.", g.name));
}

/// Befehl "gilde": eigene Gilde und wer von ihr gerade da ist.
pub fn gilde(w: &Welt, sid: u64) {
    let sp = &w.spieler[&sid].sp;
    let Some(g) = w.d.gilden.get(&sp.gilde) else {
        let anzahl = w.d.gilden.len();
        if anzahl == 0 {
            w.sende(sid, "In dieser Welt gibt es keine Gilden.");
        } else {
            let namen: Vec<String> = w.d.gilden.values().map(|g| g.name.clone()).collect();
            w.sende(sid, &format!("Du bist in keiner Gilde. Es gibt: {}.", namen.join(", ")));
        }
        return;
    };
    let mitglieder: Vec<String> = w.spieler.values().filter(|s| s.sp.gilde == g.id).map(|s| s.sp.name.clone()).collect();
    let t = format!("{}Mitglieder in der Welt: {}", info(&w.d, g), mitglieder.join(", "));
    w.sende(sid, &t);
}

/// Befehl "beruf": eigene Berufe.
pub fn berufe(w: &Welt, sid: u64) {
    let sp = &w.spieler[&sid].sp;
    if sp.berufe.is_empty() {
        let namen: Vec<String> = w.d.berufe.values().map(|g| g.name.clone()).collect();
        if namen.is_empty() {
            w.sende(sid, "In dieser Welt gibt es keine Berufe.");
        } else {
            w.sende(sid, &format!("Du hast noch keinen Beruf. Es gibt: {}.", namen.join(", ")));
        }
        return;
    }
    let mut t = String::new();
    for b in &sp.berufe {
        if let Some(g) = w.d.berufe.get(b) {
            t.push_str(&info(&w.d, g));
        }
    }
    w.sende(sid, t.trim_end());
}
