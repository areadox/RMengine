//! Leichen von Gegnern: Sie liegen eine Weile im Raum, man kann sie ansehen
//! (Beschreibung, Details, Sinne aus [monster.leiche]) und die Beute aus
//! ihnen nehmen ("nimm alles aus leiche"). Danach verwesen sie.

use crate::befehle::{finde_gegenstand, passt};
use crate::text::{aufzaehlen, gross, normalisieren};
use crate::welt::Welt;
use std::time::{Duration, Instant};

/// Eine Leiche im Raum. Der Gegenstand "leiche:<monster>" liegt dazu am Boden.
pub struct Leiche {
    pub raum: String,
    pub id: String,
    pub inhalt: Vec<String>,
    pub bis: Instant,
    pub verwest: String,
}

/// Ein Monster ist gestorben: Leiche hinlegen. Gibt die Beute zurück, die
/// NICHT in die Leiche kam (die fällt wie bisher zu Boden).
pub fn hinlegen(w: &mut Welt, monster: &str, raum: &str, beute: Vec<String>) -> Vec<String> {
    let id = crate::daten::leichen_id(monster);
    if !w.d.gegenstaende.contains_key(&id) {
        return beute;
    }
    let Some(def) = w.d.monster.get(monster) else { return beute };
    let minuten = def.leiche.minuten.unwrap_or(w.d.kampf.leichen_minuten);
    let verwest = def.leiche.verwest.clone();
    let in_leiche = w.d.kampf.beute_in_leiche;
    w.boden.entry(raum.to_string()).or_default().push(id.clone());
    w.leichen.push(Leiche {
        raum: raum.to_string(),
        id,
        inhalt: if in_leiche { beute.clone() } else { Vec::new() },
        bis: Instant::now() + Duration::from_secs(minuten * 60),
        verwest,
    });
    if in_leiche {
        Vec::new()
    } else {
        beute
    }
}

/// Verweste Leichen entfernen; was noch darin war, fällt zu Boden.
pub fn takt(w: &mut Welt) {
    let jetzt = Instant::now();
    let mut i = 0;
    while i < w.leichen.len() {
        if w.leichen[i].bis > jetzt {
            i += 1;
            continue;
        }
        let l = w.leichen.remove(i);
        let boden = w.boden.entry(l.raum.clone()).or_default();
        let Some(pos) = boden.iter().position(|g| *g == l.id) else { continue };
        boden.remove(pos);
        boden.extend(l.inhalt.iter().cloned());
        let name = gross(&w.d.gegenstand_name(&l.id));
        let t = if l.verwest.trim().is_empty() { format!("{name} zerfällt zu Staub.") } else { l.verwest.clone() };
        w.sende_raum(&l.raum, &format!("{{grau}}{t}{{/}}"), &[]);
    }
}

/// Index der Leiche im Raum, die zum Suchwort passt
fn finden(w: &Welt, raum: &str, such: &str) -> Option<usize> {
    w.leichen.iter().position(|l| {
        l.raum == raum && w.d.gegenstaende.get(&l.id).is_some_and(|g| passt(such, &g.name, &g.id, &g.schluesselwoerter))
    })
}

/// "Darin: ..." für schau leiche
pub fn inhalt_text(w: &Welt, raum: &str, id: &str) -> Option<String> {
    let l = w.leichen.iter().find(|l| l.raum == raum && l.id == id)?;
    let bei = w.d.gegenstand_dativ(id);
    if l.inhalt.is_empty() {
        return Some(format!("{{grau}}Bei {bei} ist nichts mehr zu holen.{{/}}"));
    }
    let namen: Vec<String> = crate::befehle::gruppieren(&l.inhalt)
        .into_iter()
        .map(|(g, n)| if n > 1 { format!("{n}x {}", w.d.gegenstand_name(&g)) } else { w.d.gegenstand_name(&g) })
        .collect();
    Some(format!("Bei {bei} findest du: {}. {{grau}}(nimm alles aus leiche){{/}}", aufzaehlen(&namen)))
}

/// "nimm X aus leiche" / "nimm alles aus leiche". false = keine Leiche gemeint.
pub fn nehmen(w: &mut Welt, sid: u64, rest: &str) -> bool {
    let Some((ding, ziel)) = rest.rsplit_once(" aus ") else { return false };
    let raum = w.spieler[&sid].sp.raum.clone();
    let Some(li) = finden(w, &raum, ziel.trim()) else { return false };
    let alles = matches!(normalisieren(ding).as_str(), "alles" | "all");
    let mut genommen = Vec::new();
    let mut zu_schwer = false;
    loop {
        let inhalt = &w.leichen[li].inhalt;
        let i = if alles { (!inhalt.is_empty()).then_some(0) } else { finde_gegenstand(&w.d, inhalt, ding.trim()) };
        let Some(i) = i else { break };
        let gid = inhalt[i].clone();
        let gewicht = w.d.gegenstaende.get(&gid).map(|g| g.gewicht.max(0)).unwrap_or(0);
        if !crate::behaelter::kann_tragen(w, sid, gewicht) {
            zu_schwer = true;
            break;
        }
        w.leichen[li].inhalt.remove(i);
        if let Some(s) = w.spieler.get_mut(&sid) {
            s.sp.inventar.push(gid.clone());
        }
        genommen.push(gid);
        if !alles {
            break;
        }
    }
    let leiche = w.d.gegenstand_dativ(&w.leichen[li].id);
    if genommen.is_empty() {
        let t = if zu_schwer {
            "Das kannst du nicht mehr tragen.".to_string()
        } else if alles {
            format!("Bei {leiche} ist nichts zu holen.")
        } else {
            format!("So etwas findest du bei {leiche} nicht.")
        };
        w.sende(sid, &t);
        return true;
    }
    let namen: Vec<String> = genommen.iter().map(|g| w.d.gegenstand_akk(g)).collect();
    let liste = aufzaehlen(&namen);
    w.sende(sid, &format!("Du nimmst {liste} von {leiche}."));
    if zu_schwer {
        w.sende(sid, "Mehr kannst du nicht tragen.");
    }
    let name = w.spieler[&sid].sp.name.clone();
    w.sende_raum(&raum, &format!("{name} nimmt {liste} von {leiche}."), &[sid]);
    true
}
