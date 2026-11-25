//! Hunger und Durst ([ernaehrung] in welt.toml).
//!
//! Essen und Trinken nehmen mit der Zeit ab. Ist man hungrig oder durstig,
//! bekommt man Meldungen; bei 0 erholt man sich nicht mehr und verliert
//! Lebenspunkte (sterben kann man daran nicht). Essen (Typ nahrung) und
//! Getränke (Typ getraenk) füllen auf – eigenschaften: essen = 30, trinken = 40.

use crate::daten::GegenstandDef;
use crate::welt::Welt;

/// 0 = gut, 1 = hungrig/durstig, 2 = leer
fn stufe(wert: f64, ab: i32) -> u8 {
    if wert <= 0.0 {
        2
    } else if wert <= ab as f64 {
        1
    } else {
        0
    }
}

/// Ist man so hungrig oder durstig, dass man sich nicht erholt?
pub fn erschoepft(w: &Welt, sp: &crate::spieler::SpielerDaten) -> bool {
    let e = &w.d.ernaehrung;
    e.aktiv && e.ohne_erholung && (sp.essen == 0.0 || sp.trinken == 0.0)
}

/// Einmal pro Takt
pub fn takt(w: &mut Welt, minute_um: bool) {
    let e = w.d.ernaehrung.clone();
    if !e.aktiv {
        return;
    }
    let minuten = w.d.welt.takt_sekunden as f64 / 60.0;
    for s in w.spieler.values_mut() {
        let sp = &mut s.sp;
        // Neue oder alte Figuren beginnen satt
        if sp.essen < 0.0 || sp.essen > e.max_essen as f64 {
            sp.essen = e.max_essen as f64;
        }
        if sp.trinken < 0.0 || sp.trinken > e.max_trinken as f64 {
            sp.trinken = e.max_trinken as f64;
        }
        // Baumeister und Admins bleiben satt
        if e.nur_spieler && s.rolle >= crate::spieler::Rolle::Baumeister {
            sp.essen = e.max_essen as f64;
            sp.trinken = e.max_trinken as f64;
            continue;
        }
        if s.tot_bis.is_some() {
            continue;
        }
        let (h0, d0) = (stufe(sp.essen, e.hungrig_ab), stufe(sp.trinken, e.durstig_ab));
        sp.essen = (sp.essen - e.essen_pro_minute * minuten).max(0.0);
        sp.trinken = (sp.trinken - e.trinken_pro_minute * minuten).max(0.0);
        let (h1, d1) = (stufe(sp.essen, e.hungrig_ab), stufe(sp.trinken, e.durstig_ab));
        let mut t = Vec::new();
        if h1 > h0 {
            t.push(if h1 == 1 { "{gelb}Dein Magen knurrt – du bist hungrig.{/}" } else { "{hellrot}Du bist am Verhungern! Iss etwas.{/}" });
        }
        if d1 > d0 {
            t.push(if d1 == 1 { "{gelb}Dein Mund wird trocken – du hast Durst.{/}" } else { "{hellrot}Du verdurstest! Trink etwas.{/}" });
        }
        if minute_um && (h1 == 2 || d1 == 2) && e.schaden_pro_minute > 0 && sp.lebenspunkte > 1 {
            let vorher = sp.lebenspunkte;
            sp.lebenspunkte = (sp.lebenspunkte - e.schaden_pro_minute).max(1);
            let was = match (h1 == 2, d1 == 2) {
                (true, true) => "Hunger und Durst zehren",
                (true, false) => "Der Hunger zehrt",
                _ => "Der Durst zehrt",
            };
            let msg = format!("{{rot}}{was} an deinen Kräften. (-{} LP){{/}}", vorher - sp.lebenspunkte);
            s.sende(&msg);
        }
        for m in t {
            s.sende(m);
        }
    }
}

/// "satt", "hungrig" ... für Spielstand und Prompt
pub fn zustand(w: &Welt, sp: &crate::spieler::SpielerDaten) -> Option<(String, String)> {
    let e = &w.d.ernaehrung;
    if !e.aktiv {
        return None;
    }
    let essen = if sp.essen < 0.0 { e.max_essen as f64 } else { sp.essen };
    let trinken = if sp.trinken < 0.0 { e.max_trinken as f64 } else { sp.trinken };
    let h = match stufe(essen, e.hungrig_ab) {
        0 if essen >= e.max_essen as f64 * 0.8 => "satt",
        0 => "nicht hungrig",
        1 => "hungrig",
        _ => "am Verhungern",
    };
    let d = match stufe(trinken, e.durstig_ab) {
        0 => "kein Durst",
        1 => "durstig",
        _ => "am Verdursten",
    };
    Some((h.to_string(), d.to_string()))
}

/// Wie viel ein Gegenstand sättigt und den Durst löscht
pub fn naehrwert(w: &Welt, g: &GegenstandDef) -> (i32, i32) {
    let e = &w.d.ernaehrung;
    if !e.aktiv {
        return (0, 0);
    }
    let eig = |k: &str| g.eigenschaften.get(k).copied();
    let essen = eig("essen").unwrap_or(if g.typ == "nahrung" { e.nahrung_standard } else { 0 });
    let trinken = eig("trinken").unwrap_or(if g.typ == "getraenk" { e.getraenk_standard } else { 0 });
    (essen, trinken)
}

/// Essen/Trinken gutschreiben. Ergebnis: Texte wie "satt" für die Meldung.
pub fn auffuellen(w: &mut Welt, sid: u64, essen: i32, trinken: i32) -> Vec<String> {
    let e = w.d.ernaehrung.clone();
    let mut t = Vec::new();
    if !e.aktiv {
        return t;
    }
    let Some(s) = w.spieler.get_mut(&sid) else { return t };
    let sp = &mut s.sp;
    if essen != 0 {
        let alt = if sp.essen < 0.0 { e.max_essen as f64 } else { sp.essen };
        sp.essen = (alt + essen as f64).clamp(0.0, e.max_essen as f64);
        t.push(if sp.essen >= e.max_essen as f64 { "satt".into() } else if essen > 0 { "weniger hungrig".into() } else { "hungriger".into() });
    }
    if trinken != 0 {
        let alt = if sp.trinken < 0.0 { e.max_trinken as f64 } else { sp.trinken };
        sp.trinken = (alt + trinken as f64).clamp(0.0, e.max_trinken as f64);
        t.push(if sp.trinken >= e.max_trinken as f64 { "Durst gelöscht".into() } else if trinken > 0 { "weniger durstig".into() } else { "durstiger".into() });
    }
    t
}

/// Ist man zu satt für noch mehr?
pub fn zu_satt(w: &Welt, sid: u64, essen: i32, trinken: i32) -> bool {
    let e = &w.d.ernaehrung;
    let sp = &w.spieler[&sid].sp;
    let voll_e = sp.essen < 0.0 || sp.essen >= e.max_essen as f64 - 0.5;
    let voll_t = sp.trinken < 0.0 || sp.trinken >= e.max_trinken as f64 - 0.5;
    (essen > 0 || trinken > 0) && (essen <= 0 || voll_e) && (trinken <= 0 || voll_t)
}
