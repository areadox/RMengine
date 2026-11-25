//! Tag und Nacht, Jahreszeiten und Wetter ([zeit] in welt.toml, [[wetter]] in daten/wetter/).
//!
//! Die Spielzeit wird aus der echten Uhrzeit berechnet – sie läuft also
//! auch weiter, wenn der Server aus ist, und alle sehen dieselbe Zeit.

use crate::daten::{JAHRESZEITEN, TAGESZEITEN};
use crate::welt::Welt;
use rand::Rng;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Ein Zeitpunkt in der Spielwelt
pub struct Zeitpunkt {
    pub stunde: u32,
    pub minute: u32,
    /// Tag in der Jahreszeit (ab 1)
    pub tag: u32,
    /// 0 = Frühling ... 3 = Winter
    pub jahreszeit: usize,
    pub jahr: u64,
    /// 0 = Morgen, 1 = Tag, 2 = Abend, 3 = Nacht
    pub tageszeit: usize,
}

impl Zeitpunkt {
    pub fn tageszeit_id(&self) -> &'static str {
        TAGESZEITEN[self.tageszeit]
    }
    pub fn jahreszeit_id(&self) -> &'static str {
        JAHRESZEITEN[self.jahreszeit]
    }
    pub fn tageszeit_name(&self) -> &'static str {
        ["Morgen", "Tag", "Abend", "Nacht"][self.tageszeit]
    }
}

/// Die aktuelle Spielzeit (None, wenn [zeit] aus ist)
pub fn jetzt(w: &Welt) -> Option<Zeitpunkt> {
    let z = &w.d.zeit;
    if !z.aktiv {
        return None;
    }
    let sekunden = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
    let spielminuten = (sekunden / (z.minuten_pro_stunde.max(0.01) * 60.0) * 60.0) as u64;
    let stunde = ((spielminuten / 60) % 24) as u32;
    let minute = (spielminuten % 60) as u32;
    let tage = spielminuten / (60 * 24);
    let pro = z.tage_pro_jahreszeit.max(1) as u64;
    let jahreszeit = ((tage / pro) % 4) as usize;
    let jahr = tage / (pro * 4) % 1000 + 1;
    let tageszeit = if stunde >= z.nacht || stunde < z.morgen {
        3
    } else if stunde >= z.abend {
        2
    } else if stunde >= z.tag {
        1
    } else {
        0
    };
    Some(Zeitpunkt { stunde, minute, tag: (tage % pro) as u32 + 1, jahreszeit, jahr, tageszeit })
}

/// Ist der Raum drinnen (kein Wetter, keine Sonne)?
pub fn drinnen(w: &Welt, raum: &str) -> bool {
    let Some(r) = w.d.raum(raum) else { return true };
    r.drinnen.unwrap_or_else(|| w.d.gelaende.get(&r.gelaende).is_some_and(|g| g.drinnen))
}

/// Ist gerade Nacht?
pub fn ist_nacht(w: &Welt) -> bool {
    jetzt(w).is_some_and(|z| z.tageszeit == 3)
}

/// Die aktuelle Wetterlage
pub fn wetter(w: &Welt) -> Option<&crate::daten::WetterDef> {
    w.d.wetter.get(&w.wetter)
}

/// Neues Wetter auswürfeln (passend zur Jahreszeit, möglichst ein anderes)
fn wetter_waehlen(w: &Welt, jahreszeit: &str) -> Option<String> {
    let moeglich: Vec<_> = w
        .d
        .wetter
        .values()
        .filter(|x| x.jahreszeiten.is_empty() || x.jahreszeiten.iter().any(|j| j == jahreszeit))
        .filter(|x| x.gewicht > 0)
        .collect();
    let summe: u32 = moeglich.iter().map(|x| x.gewicht).sum();
    if summe == 0 {
        return None;
    }
    let mut r = rand::thread_rng().gen_range(0..summe);
    for x in &moeglich {
        if r < x.gewicht {
            return Some(x.id.clone());
        }
        r -= x.gewicht;
    }
    None
}

/// An alle, die draußen sind
fn draussen_melden(w: &Welt, t: &str) {
    if t.trim().is_empty() {
        return;
    }
    for s in w.spieler.values() {
        if !drinnen(w, &s.sp.raum) {
            s.sende(&format!("{{hellcyan}}{t}{{/}}"));
        }
    }
}

/// Einmal pro Takt: Tageszeit, Jahreszeit und Wetter weiterschalten
pub fn takt(w: &mut Welt) {
    let Some(z) = jetzt(w) else { return };
    let cfg = w.d.zeit.clone();
    let stand = (z.tageszeit, z.jahreszeit);
    let vorher = w.zeit_stand;
    w.zeit_stand = Some(stand);
    if let Some((t0, j0)) = vorher {
        if j0 != z.jahreszeit {
            let t = [&cfg.text_fruehling, &cfg.text_sommer, &cfg.text_herbst, &cfg.text_winter][z.jahreszeit];
            if !t.trim().is_empty() {
                w.sende_alle(&format!("{{hellgruen}}{t}{{/}}"), &[]);
            }
        }
        if t0 != z.tageszeit {
            let t = [&cfg.text_morgen, &cfg.text_tag, &cfg.text_abend, &cfg.text_nacht][z.tageszeit];
            draussen_melden(w, t);
        }
    }
    // Wetter: nach Ablauf oder wenn es nicht mehr zur Jahreszeit passt
    let passt = wetter(w).is_some_and(|x| x.jahreszeiten.is_empty() || x.jahreszeiten.iter().any(|j| j == z.jahreszeit_id()));
    if w.d.wetter.is_empty() || (passt && w.wetter_bis > Instant::now()) {
        return;
    }
    let minuten = cfg.wetter_minuten.max(1);
    w.wetter_bis = Instant::now() + Duration::from_secs(rand::thread_rng().gen_range(minuten * 30..=minuten * 90));
    if let Some(neu) = wetter_waehlen(w, z.jahreszeit_id()) {
        let erstes_mal = w.wetter.is_empty();
        if neu != w.wetter {
            w.wetter = neu;
            if !erstes_mal {
                let t = wetter(w).map(|x| x.beginn.clone()).unwrap_or_default();
                draussen_melden(w, &t);
            }
        }
    }
}

/// Zeile unter der Raumbeschreibung, z.B. "Es ist Nacht. Es regnet." (nur draußen)
pub fn raum_zusatz(w: &Welt, raum: &str) -> Option<String> {
    let z = jetzt(w)?;
    if drinnen(w, raum) {
        return None;
    }
    let c = &w.d.zeit;
    let mut teile = Vec::new();
    let t = [&c.zusatz_morgen, &c.zusatz_tag, &c.zusatz_abend, &c.zusatz_nacht][z.tageszeit];
    if !t.trim().is_empty() {
        teile.push(t.trim().to_string());
    }
    if let Some(x) = wetter(w) {
        if !x.text.trim().is_empty() {
            teile.push(x.text.trim().to_string());
        }
    }
    (!teile.is_empty()).then(|| teile.join(" "))
}

/// Befehl "zeit": Uhrzeit, Datum, Wetter
pub fn befehl(w: &Welt, sid: u64) {
    let Some(z) = jetzt(w) else {
        w.sende(sid, "In dieser Welt gibt es keine Uhrzeit.");
        return;
    };
    let mut t = format!(
        "Es ist {:02}:{:02} Uhr – {}. Heute ist der {}. Tag des {} im Jahr {}.",
        z.stunde,
        z.minute,
        z.tageszeit_name(),
        z.tag,
        ["Frühlings", "Sommers", "Herbstes", "Winters"][z.jahreszeit],
        z.jahr
    );
    if let Some(x) = wetter(w) {
        let wo = if w.spieler.get(&sid).is_some_and(|s| drinnen(w, &s.sp.raum)) { " (draußen)" } else { "" };
        t.push_str(&format!("\nWetter{wo}: {}", x.name));
    }
    w.sende(sid, &t);
}

/// Für eigene Befehle: passen Tageszeit, Jahreszeit und Wetter?
pub fn passt(w: &Welt, tageszeiten: &[String], jahreszeiten: &[String], wetter_ids: &[String]) -> bool {
    if tageszeiten.is_empty() && jahreszeiten.is_empty() && wetter_ids.is_empty() {
        return true;
    }
    let Some(z) = jetzt(w) else { return true };
    (tageszeiten.is_empty() || tageszeiten.iter().any(|t| t == z.tageszeit_id()))
        && (jahreszeiten.is_empty() || jahreszeiten.iter().any(|j| j == z.jahreszeit_id()))
        && (wetter_ids.is_empty() || wetter_ids.contains(&w.wetter))
}
