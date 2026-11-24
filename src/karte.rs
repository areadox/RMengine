//! Zeichnet die Karte rund um einen Spieler.
//!
//! Stil "kasten" (Standard): Jeder Raum ist ein Kästchen, ( ) für draußen,
//! [ ] für Gebäude, mit einem Buchstaben für besondere Orte. X = du.
//!
//!     +-------------+
//!     |*(K)*(P)*( )*|
//!     |**|***|*****~|
//!     |-[ ]-[X]-(T)*|
//!     +-------------+
//!
//! Stil "linien": Jeder Raum ist ein Zeichen, verbunden mit - | / \

use crate::daten::{Daten, RaumDef, Richtung};
use crate::spieler::SpielerDaten;
use crate::welt::Welt;
use std::collections::BTreeSet;

#[derive(Clone)]
pub struct Zelle {
    pub zeichen: char,
    pub farbe: String,
}

/// Wie viele Felder die Karte durch die Karten-Fertigkeit größer ist
/// (welt.toml: karten_fertigkeit, karten_fertigkeit_stufen, karten_fertigkeit_max).
pub fn fertigkeits_bonus(w: &Welt, sp: &SpielerDaten) -> i32 {
    let k = &w.d.welt;
    if k.karten_fertigkeit.is_empty() || k.karten_fertigkeit_stufen == 0 {
        return 0;
    }
    let stufe = sp.fertigkeiten.get(&k.karten_fertigkeit).copied().unwrap_or(0);
    ((stufe / k.karten_fertigkeit_stufen) as i32).clamp(0, k.karten_fertigkeit_max.max(0))
}

pub fn zeichnen(w: &Welt, sp: &SpielerDaten, radius: i32, mit_legende: bool) -> String {
    let d = &w.d;
    let bonus = fertigkeits_bonus(w, sp);
    let radius = radius + bonus;
    // Auf der Oberwelt gibt es eine eigene Darstellung.
    if let (Some((x, y)), Some(ow)) = (crate::oberwelt::aus_id(&sp.raum), &d.oberwelt) {
        let andere: Vec<(i32, i32)> = w
            .spieler
            .values()
            .filter(|s| s.sp.name != sp.name)
            .filter_map(|s| crate::oberwelt::aus_id(&s.sp.raum))
            .collect();
        return ow.karte_zeichnen(x, y, &andere, mit_legende, sp.einstellungen.kartengroesse, bonus);
    }
    if d.welt.stadtkarte_stil == "kasten" {
        return zeichnen_kasten(w, sp, radius, mit_legende);
    }
    let Some(hier) = d.raeume.get(&sp.raum) else { return String::new() };
    let radius = radius.clamp(1, 15);
    let breite = (radius * 2 * 4 + 1) as usize;
    let hoehe = (radius * 2 * 2 + 1) as usize;
    let leer = Zelle { zeichen: ' ', farbe: String::new() };
    let mut gitter = vec![vec![leer; breite]; hoehe];
    let mut gelaende_gesehen = BTreeSet::new();

    let sichtbar = |id: &str| !d.welt.nur_besuchte_raeume_auf_karte || sp.besucht.contains(id) || id == sp.raum;

    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let (x, y) = (hier.x + dx, hier.y + dy);
            let Some(id) = d.koordinaten.get(&(hier.karte.clone(), x, y, hier.z)) else { continue };
            let Some(raum) = d.raeume.get(id) else { continue };
            if !sichtbar(id) {
                continue;
            }
            let spalte = ((dx + radius) * 4) as usize;
            let zeile = ((radius - dy) * 2) as usize;
            let (zeichen, farbe) = d.raum_symbol(raum);
            gitter[zeile][spalte] = Zelle { zeichen, farbe };
            if !raum.gelaende.is_empty() {
                gelaende_gesehen.insert(raum.gelaende.clone());
            }

            // Verbindungen zeichnen
            for (richt, ziel) in &raum.ausgaenge {
                let (ox, oy, oz) = richt.versatz();
                if oz != 0 {
                    continue;
                }
                let Some(z) = d.raeume.get(ziel) else { continue };
                if (z.x, z.y, z.z) != (x + ox, y + oy, hier.z) {
                    continue;
                }
                if (dx + ox).abs() > radius || (dy + oy).abs() > radius {
                    continue;
                }
                let s = spalte as i32;
                let zl = zeile as i32;
                let (felder, zeichen): (Vec<(i32, i32)>, char) = match richt {
                    Richtung::Ost => (vec![(zl, s + 1), (zl, s + 2), (zl, s + 3)], '-'),
                    Richtung::West => (vec![(zl, s - 1), (zl, s - 2), (zl, s - 3)], '-'),
                    Richtung::Nord => (vec![(zl - 1, s)], '|'),
                    Richtung::Sued => (vec![(zl + 1, s)], '|'),
                    Richtung::Nordost => (vec![(zl - 1, s + 2)], '/'),
                    Richtung::Suedwest => (vec![(zl + 1, s - 2)], '/'),
                    Richtung::Nordwest => (vec![(zl - 1, s - 2)], '\\'),
                    Richtung::Suedost => (vec![(zl + 1, s + 2)], '\\'),
                    _ => continue,
                };
                for (r, c) in felder {
                    let zelle = &mut gitter[r as usize][c as usize];
                    zelle.zeichen = match (zelle.zeichen, zeichen) {
                        ('/', '\\') | ('\\', '/') | ('X', _) => 'X',
                        _ => zeichen,
                    };
                    zelle.farbe = "grau".into();
                }
            }
        }
    }

    // Andere Spieler und man selbst
    for s in w.spieler.values() {
        if s.sp.name == sp.name {
            continue;
        }
        if let Some(r) = d.raeume.get(&s.sp.raum) {
            let (dx, dy) = (r.x - hier.x, r.y - hier.y);
            if r.z == hier.z && dx.abs() <= radius && dy.abs() <= radius && sichtbar(&r.id) {
                gitter[((radius - dy) * 2) as usize][((dx + radius) * 4) as usize] =
                    Zelle { zeichen: '*', farbe: "hellmagenta".into() };
            }
        }
    }
    let mitte = Zelle { zeichen: '@', farbe: "hellgelb".into() };
    gitter[(radius * 2) as usize][(radius * 4) as usize] = mitte;

    // In Text verwandeln; leere Zeilen oben und unten weglassen
    let zeilen: Vec<String> = gitter
        .iter()
        .map(|zeile| {
            let mut t = String::new();
            for z in zeile {
                if z.farbe.is_empty() || z.zeichen == ' ' {
                    t.push(z.zeichen);
                } else {
                    t.push_str(&format!("{{{}}}{}{{/}}", z.farbe, z.zeichen));
                }
            }
            t.trim_end().to_string()
        })
        .collect();
    let erste = zeilen.iter().position(|z| !z.is_empty()).unwrap_or(0);
    let letzte = zeilen.iter().rposition(|z| !z.is_empty()).unwrap_or(0);

    let mut aus = String::new();
    let mut kopf = String::new();
    if !hier.gebiet.is_empty() {
        kopf.push_str(&crate::text::gross(&hier.gebiet.replace('_', " ")));
    }
    if hier.z != 0 {
        if !kopf.is_empty() {
            kopf.push_str(", ");
        }
        kopf.push_str(&format!("Ebene {}", hier.z));
    }
    if !kopf.is_empty() {
        aus.push_str(&format!("{{grau}}[{kopf}]{{/}}\n"));
    }
    for z in &zeilen[erste..=letzte] {
        aus.push_str("  ");
        aus.push_str(z);
        aus.push('\n');
    }

    // Hinweis auf Treppen
    let mut ebenen = Vec::new();
    if hier.ausgaenge.contains_key(&Richtung::Hoch) {
        ebenen.push("nach oben");
    }
    if hier.ausgaenge.contains_key(&Richtung::Runter) {
        ebenen.push("nach unten");
    }
    if hier.ausgaenge.contains_key(&Richtung::Raus) {
        ebenen.push("hinaus");
    }
    if !ebenen.is_empty() {
        aus.push_str(&format!("  {{grau}}(Hier geht es auch {}.){{/}}\n", ebenen.join(" und ")));
    }

    if mit_legende {
        let mut teile = vec!["{hellgelb}@{/} du".to_string(), "{hellmagenta}*{/} andere Spieler".to_string()];
        for g in gelaende_gesehen {
            if let Some(gd) = d.gelaende.get(&g) {
                let name = if gd.name.is_empty() { g.clone() } else { gd.name.clone() };
                let sym = if gd.farbe.is_empty() {
                    gd.symbol.clone()
                } else {
                    format!("{{{}}}{}{{/}}", gd.farbe, gd.symbol)
                };
                teile.push(format!("{sym} {name}"));
            }
        }
        aus.push_str(&format!("\n  {}\n", teile.join("   ")));
    }
    aus
}

/// Die Stadtkarte im Kästchen-Stil.
fn zeichnen_kasten(w: &Welt, sp: &SpielerDaten, radius: i32, mit_legende: bool) -> String {
    let d = &w.d;
    let Some(hier) = d.raeume.get(&sp.raum) else { return String::new() };
    let r = radius.clamp(1, 10);
    let n = 2 * r + 1;
    let breite = (4 * n + 1) as usize;
    let hoehe = (2 * n + 1) as usize;
    let hg = d.welt.karten_hintergrund.chars().next().unwrap_or(' ');
    let mut gitter = vec![vec![Zelle { zeichen: hg, farbe: "grau".into() }; breite]; hoehe];
    let sichtbar = |id: &str| !d.welt.nur_besuchte_raeume_auf_karte || sp.besucht.contains(id) || id == sp.raum;
    let mut legende: Vec<(char, String)> = Vec::new();
    // Welche Stellen sind noch Hintergrund (dürfen verziert werden)?
    let mut frei = vec![vec![true; breite]; hoehe];
    let mut gezeichnet: Vec<(&RaumDef, usize, usize)> = Vec::new();

    // Wer ist wo?
    let andere: Vec<&str> =
        w.spieler.values().filter(|s| s.sp.name != sp.name).map(|s| s.sp.raum.as_str()).collect();

    for dy in -r..=r {
        for dx in -r..=r {
            let (x, y) = (hier.x + dx, hier.y + dy);
            let Some(id) = d.koordinaten.get(&(hier.karte.clone(), x, y, hier.z)) else { continue };
            let Some(raum) = d.raeume.get(id) else { continue };
            if !sichtbar(id) {
                continue;
            }
            let zeile = (2 * (r - dy) + 1) as usize;
            let mitte = (4 * (dx + r) + 2) as usize;

            // Klammern und Farbe aus dem Raum oder seinem Gelände
            let g = d.gelaende.get(&raum.gelaende);
            let klammern = if raum.klammern.chars().count() == 2 {
                raum.klammern.clone()
            } else {
                g.map(|g| g.klammern.clone()).unwrap_or_else(|| "[]".into())
            };
            let mut k = klammern.chars();
            let (auf, zu) = (k.next().unwrap_or('['), k.next().unwrap_or(']'));
            let farbe = if raum.farbe.is_empty() { g.map(|g| g.farbe.clone()).unwrap_or_default() } else { raum.farbe.clone() };
            gitter[zeile][mitte - 1] = Zelle { zeichen: auf, farbe: farbe.clone() };
            gitter[zeile][mitte + 1] = Zelle { zeichen: zu, farbe };
            for s in mitte - 1..=mitte + 1 {
                frei[zeile][s] = false;
            }
            gezeichnet.push((raum, zeile, mitte));

            let innen = raum.symbol.chars().next();
            let (zeichen, farbe) = if id == &sp.raum {
                ('X', "hellgelb".to_string())
            } else if andere.contains(&id.as_str()) {
                ('@', "hellmagenta".to_string())
            } else {
                (innen.unwrap_or(' '), "hellweiss".to_string())
            };
            gitter[zeile][mitte] = Zelle { zeichen, farbe };
            if let Some(c) = innen {
                if !legende.iter().any(|(z, _)| *z == c) {
                    legende.push((c, raum.name.clone()));
                }
            }

            // Wege: auch solche, die aus dem Ausschnitt hinausführen
            for (richt, ziel) in &raum.ausgaenge {
                let (ox, oy, oz) = richt.versatz();
                if oz != 0 || (ox, oy) == (0, 0) {
                    continue;
                }
                let Some(z) = d.raeume.get(ziel) else { continue };
                if (z.x, z.y, z.z) != (x + ox, y + oy, hier.z) {
                    continue;
                }
                let (zl, sp_) = match richt {
                    Richtung::Ost => (zeile, mitte + 2),
                    Richtung::West => (zeile, mitte - 2),
                    Richtung::Nord => (zeile - 1, mitte),
                    Richtung::Sued => (zeile + 1, mitte),
                    Richtung::Nordost => (zeile - 1, mitte + 2),
                    Richtung::Suedwest => (zeile + 1, mitte - 2),
                    Richtung::Nordwest => (zeile - 1, mitte - 2),
                    Richtung::Suedost => (zeile + 1, mitte + 2),
                    _ => continue,
                };
                let zeichen = match richt {
                    Richtung::Ost | Richtung::West => '-',
                    Richtung::Nord | Richtung::Sued => '|',
                    Richtung::Nordost | Richtung::Suedwest => '/',
                    _ => '\\',
                };
                frei[zl][sp_] = false;
                let zelle = &mut gitter[zl][sp_];
                zelle.zeichen = match (zelle.zeichen, zeichen) {
                    ('/', '\\') | ('\\', '/') | ('x', _) => 'x',
                    _ => zeichen,
                };
                zelle.farbe = String::new();
            }
        }
    }

    verzieren(d, &gezeichnet, &mut gitter, &frei);

    let mut aus = String::new();
    if !hier.gebiet.is_empty() || hier.z != 0 {
        let mut kopf = crate::text::gross(&hier.gebiet.replace('_', " "));
        if hier.z != 0 {
            if !kopf.is_empty() {
                kopf.push_str(", ");
            }
            kopf.push_str(&format!("Ebene {}", hier.z));
        }
        aus.push_str(&format!("{{grau}}[{kopf}]{{/}}\n"));
    }
    aus.push_str(&rahmen(&gitter));

    let mut ebenen = Vec::new();
    for (ri, t) in [(Richtung::Hoch, "nach oben"), (Richtung::Runter, "nach unten"), (Richtung::Raus, "hinaus")] {
        if hier.ausgaenge.contains_key(&ri) {
            ebenen.push(t);
        }
    }
    if !ebenen.is_empty() {
        aus.push_str(&format!("{{grau}}(Hier geht es auch {}.){{/}}\n", ebenen.join(" und ")));
    }
    if mit_legende {
        let mut teile = vec!["{hellgelb}X{/} du".to_string(), "{hellmagenta}@{/} andere".to_string()];
        let mut klammern_gesehen = BTreeSet::new();
        for g in d.gelaende.values() {
            if g.klammern.chars().count() == 2 && klammern_gesehen.insert(g.klammern.clone()) {
                let k: Vec<char> = g.klammern.chars().collect();
                let namen: Vec<String> = d
                    .gelaende
                    .values()
                    .filter(|x| x.klammern == g.klammern)
                    .map(|x| x.name.clone())
                    .collect();
                teile.push(format!("{} {} {}", k[0], k[1], namen.join("/")));
            }
        }
        for (c, name) in legende {
            teile.push(format!("{c} {name}"));
        }
        for t in teile.chunks(4) {
            aus.push_str(&format!("{}\n", t.join("   ")));
        }
    }
    aus
}

/// Bäume, Wasser und andere Verzierungen in die freien Stellen rund um die Räume.
///
/// Rund um jedes Kästchen gibt es 12 Stellen (5 darüber, 5 darunter, je eine
/// links und rechts). `deko_richtung` füllt eine ganze Seite, `deko` verstreut
/// Zeichen zufällig – aber immer gleich, damit die Karte nicht flackert.
fn verzieren(d: &Daten, raeume: &[(&RaumDef, usize, usize)], gitter: &mut [Vec<Zelle>], frei: &[Vec<bool>]) {
    let mut belegt = vec![vec![false; frei.first().map(|z| z.len()).unwrap_or(0)]; frei.len()];
    for (raum, zeile, mitte) in raeume {
        let g = d.gelaende.get(&raum.gelaende);
        let deko: Vec<char> =
            if raum.deko.is_empty() { g.map(|g| g.deko.chars().collect()).unwrap_or_default() } else { raum.deko.chars().collect() };
        let dichte = raum.deko_dichte.unwrap_or_else(|| g.map(|g| g.deko_dichte).unwrap_or(30));
        let seiten: Vec<((i32, i32), Vec<char>)> = raum
            .deko_richtung
            .iter()
            .filter_map(|(r, z)| Richtung::aus_text(r).map(|r| ((r.versatz().0, r.versatz().1), z.chars().collect())))
            .collect();

        for dy in -1i32..=1 {
            for dx in -2i32..=2 {
                if dy == 0 && dx.abs() < 2 {
                    continue; // das Kästchen selbst
                }
                let (zl, sp) = (*zeile as i32 + dy, *mitte as i32 + dx);
                if zl < 0 || sp < 0 || zl as usize >= gitter.len() || sp as usize >= gitter[0].len() {
                    continue;
                }
                let (zl, sp) = (zl as usize, sp as usize);
                if !frei[zl][sp] || belegt[zl][sp] {
                    continue;
                }
                // Stelle in der ganzen Welt -> gleiche Stelle, gleiches Zeichen
                let welt_sp = 4 * raum.x as i64 + dx as i64;
                let welt_zl = -2 * raum.y as i64 + dy as i64;
                let h = streuwert(welt_sp, welt_zl, raum.z as i64);

                let seite = seiten.iter().find(|((ox, oy), _)| {
                    (*oy == 0 || dy == -oy) && (*ox == 0 || dx == 2 * ox)
                });
                let zeichen = match seite {
                    Some((_, z)) if !z.is_empty() => Some(z[(h as usize) % z.len()]),
                    _ if !deko.is_empty() && (h % 100) < dichte as u64 => Some(deko[((h / 100) as usize) % deko.len()]),
                    _ => None,
                };
                if let Some(c) = zeichen {
                    let farbe = d.deko_farben.get(&c.to_string()).cloned().unwrap_or_default();
                    gitter[zl][sp] = Zelle { zeichen: c, farbe };
                    belegt[zl][sp] = true;
                }
            }
        }
    }
}

fn streuwert(x: i64, y: i64, z: i64) -> u64 {
    let mut h = (x.wrapping_mul(73_856_093) ^ y.wrapping_mul(19_349_663) ^ z.wrapping_mul(83_492_791)) as u64;
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^ (h >> 15)
}

/// Setzt ein Zeichengitter in einen Rahmen aus + - |.
pub fn rahmen(gitter: &[Vec<Zelle>]) -> String {
    let breite = gitter.first().map(|z| z.len()).unwrap_or(0);
    let linie = format!("+{}+\n", "-".repeat(breite));
    let mut aus = linie.clone();
    for zeile in gitter {
        aus.push('|');
        for z in zeile {
            if z.farbe.is_empty() || z.zeichen == ' ' {
                aus.push(z.zeichen);
            } else {
                aus.push_str(&format!("{{{}}}{}{{/}}", z.farbe, z.zeichen));
            }
        }
        aus.push_str("|\n");
    }
    aus.push_str(&linie);
    aus
}
