//! Türen: öffnen, schließen, auf- und abschließen.
//!
//! Eine Tür steht in einem Raum an einem Ausgang ([raum.tueren.nord]) und gilt
//! auch für den Rückweg. Ihr Zustand liegt in `Welt::tuer_zustand` und wird beim
//! Auffüllen der Welt auf den Anfang zurückgesetzt.

use crate::daten::{Richtung, TuerDef};
use crate::text::{gross, normalisieren};
use crate::welt::Welt;

/// "Sie", "Er" oder "Es" – passend zum Artikel im Namen der Tür
fn pronomen(t: &TuerDef) -> &'static str {
    match t.name.split_whitespace().next().map(|a| a.to_lowercase()).as_deref() {
        Some("der" | "ein") => "Er",
        Some("das") => "Es",
        _ => "Sie",
    }
}

/// Die Tür, die der Spieler meint: Richtung, Name oder "tuer" (wenn es nur eine gibt)
pub fn finden(w: &Welt, raum: &str, such: &str) -> Option<Richtung> {
    let such = such.split(" mit ").next().unwrap_or("").trim();
    let tueren: Vec<Richtung> = w.sichtbare_ausgaenge(raum).keys().copied().filter(|r| w.tuer(raum, *r).is_some()).collect();
    if tueren.is_empty() {
        return None;
    }
    let woerter: Vec<String> = such.split_whitespace().map(normalisieren).filter(|w| !matches!(w.as_str(), "die" | "den" | "das" | "der")).collect();
    // Eine Richtung genannt? ("oeffne tuer norden", "oeffne nord")
    if let Some(r) = woerter.iter().find_map(|w| Richtung::aus_text(w)) {
        return tueren.contains(&r).then_some(r);
    }
    if woerter.is_empty() {
        return (tueren.len() == 1).then(|| tueren[0]);
    }
    tueren.into_iter().find(|r| {
        let Some((_, t, _, _)) = w.tuer(raum, *r) else { return false };
        let mut namen: Vec<String> = vec!["tuer".into(), "tor".into(), "pforte".into()];
        namen.extend(t.woerter.iter().map(|x| normalisieren(x)));
        namen.extend(t.name.split_whitespace().skip(1).map(normalisieren));
        woerter.iter().all(|w| namen.contains(w))
    })
}

fn setzen(w: &mut Welt, raum: &str, richt: Richtung, zu: bool, verschlossen: bool) {
    if let Some((k, _, _, _)) = w.tuer(raum, richt) {
        let k = k.clone();
        w.tuer_zustand.insert(k, (zu, verschlossen));
    }
}

/// Meldung an beide Seiten der Tür
fn melden(w: &Welt, sid: u64, raum: &str, richt: Richtung, hier: &str, drueben: &str) {
    w.sende_raum(raum, hier, &[sid]);
    if let Some(ziel) = w.ausgaenge(raum).get(&richt) {
        if ziel != raum {
            w.sende_raum(ziel, drueben, &[]);
        }
    }
}

fn hat_schluessel(w: &Welt, sid: u64, t: &TuerDef) -> bool {
    let sp = &w.spieler[&sid].sp;
    !t.schluessel.is_empty() && (sp.inventar.contains(&t.schluessel) || sp.ausruestung.values().any(|g| *g == t.schluessel))
}

/// Befehle: oeffne, schliesse (auch "schliesse tuer auf/ab"), aufschliessen, abschliessen
pub fn befehl(w: &mut Welt, sid: u64, was: &str, rest: &str) {
    let rest = rest.trim();
    let (was, rest) = match was {
        "schliessen" if rest.ends_with(" auf") => ("aufschliessen", rest.trim_end_matches(" auf")),
        "schliessen" if rest.ends_with(" ab") => ("abschliessen", rest.trim_end_matches(" ab")),
        _ => (was, rest),
    };
    let raum = w.spieler[&sid].sp.raum.clone();
    let Some(richt) = finden(w, &raum, rest) else {
        let t = if w.sichtbare_ausgaenge(&raum).keys().any(|r| w.tuer(&raum, *r).is_some()) {
            "Welche Tür meinst du? (z.B. mit der Richtung: oeffne tuer norden)"
        } else {
            "Hier gibt es keine Tür."
        };
        w.sende(sid, t);
        return;
    };
    let Some((_, t, zu, verschlossen)) = w.tuer(&raum, richt) else { return };
    let t = t.clone();
    if !crate::skripte::erlaubt(w, sid, "tuer", &raum, &[was, richt.schluessel()]) {
        return;
    }
    let name = w.spieler[&sid].sp.name.clone();
    let tuer = t.name.clone();
    let er = pronomen(&t);
    match was {
        "oeffnen" => {
            if !zu {
                w.sende(sid, &format!("{er} ist schon offen."));
                return;
            }
            if verschlossen {
                if !hat_schluessel(w, sid, &t) {
                    w.sende(sid, &format!("{} ist verschlossen.", gross(&tuer)));
                    return;
                }
                let s = w.d.gegenstand_dativ(&t.schluessel);
                w.sende(sid, &format!("Du schließt {tuer} mit {s} auf."));
            }
            setzen(w, &raum, richt, false, false);
            w.sende(sid, &format!("Du öffnest {tuer}."));
            melden(w, sid, &raum, richt, &format!("{name} öffnet {tuer}."), &format!("{} wird von der anderen Seite geöffnet.", gross(&tuer)));
        }
        "schliessen" => {
            if zu {
                w.sende(sid, &format!("{er} ist schon geschlossen."));
                return;
            }
            setzen(w, &raum, richt, true, false);
            w.sende(sid, &format!("Du schließt {tuer}."));
            melden(w, sid, &raum, richt, &format!("{name} schließt {tuer}."), &format!("{} wird von der anderen Seite geschlossen.", gross(&tuer)));
        }
        "aufschliessen" | "abschliessen" => {
            let auf = was == "aufschliessen";
            if t.schluessel.is_empty() {
                w.sende(sid, &format!("{} hat kein Schloss.", gross(&tuer)));
                return;
            }
            if auf && !verschlossen {
                w.sende(sid, &format!("{er} ist gar nicht verschlossen."));
                return;
            }
            if !auf && verschlossen {
                w.sende(sid, &format!("{er} ist schon verschlossen."));
                return;
            }
            if !auf && !zu {
                let ihn = match er {
                    "Er" => "ihn",
                    "Es" => "es",
                    _ => "sie",
                };
                w.sende(sid, &format!("{er} ist offen – du musst {ihn} erst schließen."));
                return;
            }
            if !hat_schluessel(w, sid, &t) {
                w.sende(sid, "Du hast keinen passenden Schlüssel.");
                return;
            }
            let s = w.d.gegenstand_dativ(&t.schluessel);
            setzen(w, &raum, richt, zu, !auf);
            let wie = if auf { "auf" } else { "ab" };
            w.sende(sid, &format!("Du schließt {tuer} mit {s} {wie}."));
            melden(w, sid, &raum, richt, &format!("{name} schließt {tuer} {wie}."), "Auf der anderen Seite dreht sich ein Schlüssel im Schloss.");
        }
        _ => {}
    }
}

/// "schau tuer": Beschreibung und Zustand. false = keine Tür gemeint
pub fn ansehen(w: &Welt, sid: u64, such: &str) -> bool {
    let raum = &w.spieler[&sid].sp.raum;
    let woerter: Vec<String> = such.split_whitespace().map(normalisieren).collect();
    // Nur, wenn wirklich eine Tür gemeint ist (nicht "schau norden")
    let gemeint = woerter.iter().any(|x| {
        matches!(x.as_str(), "tuer" | "tor" | "pforte")
            || w.sichtbare_ausgaenge(raum).keys().any(|r| {
                w.tuer(raum, *r).is_some_and(|(_, t, _, _)| {
                    t.woerter.iter().any(|y| normalisieren(y) == *x) || t.name.split_whitespace().skip(1).any(|y| normalisieren(y) == *x)
                })
            })
    });
    if !gemeint {
        return false;
    }
    let Some(richt) = finden(w, raum, such) else { return false };
    let Some((_, t, zu, verschlossen)) = w.tuer(raum, richt) else { return false };
    let mut text = if t.beschreibung.is_empty() { format!("{} führt nach {}.", gross(&t.name), richt.anzeige()) } else { t.beschreibung.clone() };
    let zustand = match (zu, verschlossen) {
        (false, _) => "offen",
        (true, false) => "geschlossen",
        (true, true) => "verschlossen",
    };
    text.push_str(&format!("\n{} ist {zustand}.", pronomen(t)));
    w.sende(sid, &crate::text::umbrechen(&text, w.spieler[&sid].breite()));
    true
}
