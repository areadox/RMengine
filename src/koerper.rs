//! Körperteile ([[koerper]] in daten/koerper/): Hände zum Halten, Teile zum
//! Angreifen (Maul, Pfoten) und zum Ansehen mit Details.

use crate::beschreibung::Detail;
use crate::daten::{GegenstandDef, KoerperDef, MonsterDef};
use crate::text::aufzaehlen;
use crate::welt::Welt;
use std::collections::BTreeMap;

/// Diese Plätze hält man in der Hand
pub const HALTE_PLAETZE: &[&str] = &["waffe", "schild", "licht"];

/// So viele Hände braucht ein Gegenstand
pub fn braucht_haende(g: &GegenstandDef) -> u32 {
    match g.haende {
        Some(n) => n,
        None if HALTE_PLAETZE.contains(&g.platz.as_str()) => 1,
        None => 0,
    }
}

/// Der Körperbau eines Spielers
pub fn von_spieler(w: &Welt, sid: u64) -> Option<&KoerperDef> {
    w.d.koerper_von(&w.spieler.get(&sid)?.sp.rasse)
}

/// Kann der Spieler den Gegenstand noch in die Hand nehmen? (ohne Körperbau: immer)
pub fn haende_pruefen(w: &Welt, sid: u64, neu: &GegenstandDef) -> Result<(), String> {
    let Some(k) = von_spieler(w, sid) else { return Ok(()) };
    let noetig = braucht_haende(neu);
    if noetig == 0 {
        return Ok(());
    }
    let haende: Vec<&str> = k.teile.iter().filter(|t| t.halten).map(|t| t.name.as_str()).collect();
    let gesamt = haende.len() as u32;
    if noetig > gesamt {
        return Err(format!("Dafür brauchst du {noetig} Hände – du hast nur {gesamt}."));
    }
    let sp = &w.spieler[&sid].sp;
    let gehalten: Vec<(&String, u32)> = sp
        .ausruestung
        .values()
        .filter_map(|id| w.d.gegenstaende.get(id).map(|g| (id, braucht_haende(g))))
        .filter(|(_, n)| *n > 0)
        .collect();
    let belegt: u32 = gehalten.iter().map(|(_, n)| n).sum();
    if belegt + noetig > gesamt {
        let namen: Vec<String> = gehalten.iter().map(|(id, _)| w.d.gegenstand_akk(id)).collect();
        let hand = if noetig == 1 { "keine Hand mehr" } else { "nicht genug Hände" };
        return Err(format!("Dafür hast du {hand} frei – du hältst schon {}.", aufzaehlen(&namen)));
    }
    Ok(())
}

/// Passt der Platz zum Körper? Gibt ein Körperbau irgendwo `plaetze` an, braucht
/// jeder Platz (außer Waffe, Schild, Licht – die hält man) ein Teil, das ihn trägt.
pub fn platz_pruefen(w: &Welt, sid: u64, g: &GegenstandDef) -> Result<(), String> {
    let Some(k) = von_spieler(w, sid) else { return Ok(()) };
    if g.platz.is_empty() || HALTE_PLAETZE.contains(&g.platz.as_str()) || k.teile.iter().all(|t| t.plaetze.is_empty()) {
        return Ok(());
    }
    if k.teile.iter().any(|t| t.plaetze.contains(&g.platz)) {
        Ok(())
    } else {
        Err(format!("{} passt nicht an deinen Körper.", crate::text::gross(&w.d.gegenstand_name(&g.id))))
    }
}

/// Angriff ohne Waffe mit einem Körperteil: (Text für "Du ...", Text für andere, Schaden)
pub fn spieler_angriff(w: &Welt, sid: u64, ziel_akk: &str, ziel_dativ: &str) -> Option<(String, String, i32)> {
    use rand::seq::SliceRandom;
    let s = w.spieler.get(&sid)?;
    if s.sp.ausruestung.contains_key("waffe") {
        return None;
    }
    let k = von_spieler(w, sid)?;
    let teile: Vec<_> = k.teile.iter().filter(|t| !t.angriff_du.trim().is_empty()).collect();
    let t = teile.choose(&mut rand::thread_rng())?;
    let fuellen = |a: &str| {
        let a = a.trim();
        if a.contains('{') {
            a.replace("{ziel_dativ}", ziel_dativ).replace("{ziel}", ziel_akk)
        } else {
            format!("{a} {ziel_akk}")
        }
    };
    let er = if t.angriff.trim().is_empty() { format!("greift {ziel_akk} an") } else { fuellen(&t.angriff) };
    Some((fuellen(&t.angriff_du), er, t.schaden))
}

/// Ein Angriff eines Monsters mit einem Körperteil: (Text für "dich", Text für andere, Schaden)
pub fn monster_angriff(w: &Welt, def: &MonsterDef, ziel_name: &str) -> Option<(String, String, i32)> {
    use rand::seq::SliceRandom;
    let k = w.d.koerper.get(&def.koerper)?;
    let teile: Vec<_> = k.teile.iter().filter(|t| !t.angriff.trim().is_empty()).collect();
    let t = teile.choose(&mut rand::thread_rng())?;
    let schaden = if t.schaden > 0 { t.schaden } else { def.kampf_schaden() };
    let a = t.angriff.trim();
    let (dich, andere) = if a.contains('{') {
        (
            a.replace("{ziel_dativ}", "dir").replace("{ziel}", "dich"),
            a.replace("{ziel_dativ}", ziel_name).replace("{ziel}", ziel_name),
        )
    } else {
        (format!("{a} dich"), format!("{a} {ziel_name}"))
    };
    Some((dich, andere, schaden))
}

/// Die Körperteile als Details (zum Ansehen, Riechen, Tasten)
pub fn details(w: &Welt, koerper: &str) -> BTreeMap<String, Detail> {
    w.d.koerper.get(koerper).map(|k| k.teile.iter().map(|t| t.als_detail()).collect()).unwrap_or_default()
}
