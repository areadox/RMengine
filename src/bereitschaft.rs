//! Bereitschaft: Ein Zauber oder eine Kampftechnik, die der Autoangriff
//! von selbst einsetzt (Befehl: bereite <nr|name>).
//!
//! - Zauber wirken nur von selbst, wenn man einen Zauberstab angelegt hat
//!   (ein Gegenstand mit der Eigenschaft `zauberstab`).
//! - Techniken brauchen keinen Gegenstand.
//! - In beiden Fällen gelten die üblichen Regeln: Gilde, Fertigkeit, Mana,
//!   Ausdauer und Abklingzeit. Passt etwas nicht (z.B. das Mana ist alle),
//!   schlägt man ganz normal mit der Waffe zu.

use crate::daten::ZauberDef;
use crate::text::normalisieren;
use crate::welt::Welt;
use std::time::Instant;

/// Hat der Spieler einen Zauberstab angelegt?
pub fn hat_zauberstab(w: &Welt, sid: u64) -> bool {
    w.spieler[&sid]
        .sp
        .ausruestung
        .values()
        .filter_map(|g| w.d.gegenstaende.get(g))
        .any(|g| g.eigenschaften.get("zauberstab").is_some_and(|v| *v > 0))
}

/// Zauber und Techniken, die man bereithalten kann (gegen Gegner, Gilde passt).
fn moeglich(w: &Welt, sid: u64) -> Vec<ZauberDef> {
    let sp = &w.spieler[&sid].sp;
    sp.zauber
        .iter()
        .filter_map(|id| w.d.zauber.get(id))
        .filter(|z| z.ziel == "gegner" && gilde_passt(w, sp, z))
        .cloned()
        .collect()
}

/// Gilde passt (Techniken ohne Gilde gibt es nicht, wenn techniken_nur_gilden gilt).
fn gilde_passt(w: &Welt, sp: &crate::spieler::SpielerDaten, z: &ZauberDef) -> bool {
    if z.gilde.is_empty() {
        !(z.ist_technik() && w.d.welt.techniken_nur_gilden)
    } else {
        z.gilde == sp.gilde
    }
}

/// Kann der Zauber jetzt (still) gewirkt werden? Keine Meldungen – sonst schlägt man normal zu.
fn kann_wirken(w: &Welt, sid: u64, z: &ZauberDef) -> bool {
    let Some(s) = w.spieler.get(&sid) else { return false };
    let sp = &s.sp;
    let jetzt = Instant::now();
    sp.zauber.contains(&z.id)
        && gilde_passt(w, sp, z)
        && (z.ist_technik() || hat_zauberstab(w, sid))
        && (z.fertigkeit.is_empty() || sp.fertigkeiten.get(&z.fertigkeit).copied().unwrap_or(0) >= z.mindest_stufe)
        && s.abklingzeiten.get(&z.id).is_none_or(|bis| *bis <= jetzt)
        && sp.mana >= z.mana_kosten
        && sp.ausdauer >= z.ausdauer_kosten
        && s.kampf_ziel.and_then(|m| w.monster.get(&m)).is_some_and(|m| m.raum == sp.raum)
}

/// Der Autoangriff: bereitgehaltenen Zauber wirken – oder normal zuschlagen.
pub fn auto_angriff(w: &mut Welt, sid: u64) -> bool {
    let bereit = w.spieler.get(&sid).map(|s| s.sp.bereit.clone()).unwrap_or_default();
    if let Some(z) = w.d.zauber.get(&bereit).cloned() {
        if kann_wirken(w, sid, &z) {
            crate::befehle::zaubern(w, sid, &z.id, z.ist_technik());
            return true;
        }
    }
    crate::kampf::spieler_schlaegt(w, sid)
}

/// Kurz für den Prompt: "Feuerpfeil" (leer = nichts bereit)
pub fn anzeige(w: &Welt, sid: u64) -> String {
    let sp = &w.spieler[&sid].sp;
    w.d.zauber.get(&sp.bereit).map(|z| z.name.clone()).unwrap_or_default()
}

/// Befehl "bereite [nr|name|nichts]"
pub fn befehl(w: &mut Welt, sid: u64, rest: &str) {
    let liste = moeglich(w, sid);
    let such = normalisieren(rest);
    if such.is_empty() {
        let aktuell = anzeige(w, sid);
        let mut t = if aktuell.is_empty() {
            String::from("Du hältst nichts bereit – im Kampf schlägst du mit der Waffe zu.\n")
        } else {
            format!("Bereit: {{hellmagenta}}{aktuell}{{/}} – der Autoangriff setzt es ein, sobald es geht.\n")
        };
        if liste.is_empty() {
            t.push_str("Du kennst keine Zauber oder Techniken gegen Gegner, die du bereithalten könntest.");
        } else {
            t.push_str("{fett}Möglich:{/}\n");
            for (i, z) in liste.iter().enumerate() {
                let art = if z.ist_technik() { "Technik" } else { "Zauber, braucht Zauberstab" };
                t.push_str(&format!("  {{gelb}}{:>2}.{{/}} {} {{grau}}({art}){{/}}\n", i + 1, z.name));
            }
            t.push_str("Wählen: bereite <nr|name>, aufheben: bereite nichts");
        }
        w.sende(sid, t.trim_end());
        return;
    }
    if matches!(such.as_str(), "nichts" | "aus" | "keinen" | "keine") {
        if let Some(s) = w.spieler.get_mut(&sid) {
            s.sp.bereit.clear();
        }
        w.sende(sid, "Du hältst nichts mehr bereit.");
        return;
    }
    let gewaehlt = match crate::gemeinschaft::nummer(rest) {
        Some(n) => liste.get(n).cloned(),
        None => liste
            .iter()
            .find(|z| normalisieren(&z.name) == such || normalisieren(&z.id) == such || z.schluesselwoerter.iter().any(|k| normalisieren(k) == such))
            .or_else(|| liste.iter().find(|z| normalisieren(&z.name).starts_with(&such)))
            .cloned(),
    };
    let Some(z) = gewaehlt else {
        let kennt = w.spieler[&sid].sp.zauber.iter().filter_map(|id| w.d.zauber.get(id)).any(|z| normalisieren(&z.name).starts_with(&such));
        let t = if kennt {
            "Das kannst du nicht bereithalten: Es muss gegen Gegner wirken, und du musst in der richtigen Gilde sein."
        } else {
            "Das kennst du nicht. (bereite zeigt, was möglich ist)"
        };
        w.sende(sid, t);
        return;
    };
    if let Some(s) = w.spieler.get_mut(&sid) {
        s.sp.bereit = z.id.clone();
    }
    w.sende(sid, &format!("Du hältst {{hellmagenta}}{}{{/}} bereit.", z.name));
    if !z.ist_technik() && !hat_zauberstab(w, sid) {
        w.sende(sid, "{grau}Ohne angelegten Zauberstab wirkt der Zauber nicht von selbst.{/}");
    }
}
