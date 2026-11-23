//! Zeichnet die Karte rund um einen Spieler.
//!
//! Jeder Raum ist ein Zeichen. Verbindungen werden mit - | / \ gezeichnet:
//!
//!     T---T
//!     |  /
//!     #---@---~

use crate::daten::Richtung;
use crate::spieler::SpielerDaten;
use crate::welt::Welt;
use std::collections::BTreeSet;

#[derive(Clone)]
struct Zelle {
    zeichen: char,
    farbe: String,
}

pub fn zeichnen(w: &Welt, sp: &SpielerDaten, radius: i32, mit_legende: bool) -> String {
    let d = &w.d;
    // Auf der Oberwelt gibt es eine eigene Darstellung.
    if let (Some((x, y)), Some(ow)) = (crate::oberwelt::aus_id(&sp.raum), &d.oberwelt) {
        let andere: Vec<(i32, i32)> = w
            .spieler
            .values()
            .filter(|s| s.sp.name != sp.name)
            .filter_map(|s| crate::oberwelt::aus_id(&s.sp.raum))
            .collect();
        return ow.karte_zeichnen(x, y, &andere, mit_legende);
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
            let Some(id) = d.koordinaten.get(&(x, y, hier.z)) else { continue };
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
