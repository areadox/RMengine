//! Wirkungen, die eine Zeit lang anhalten: Schilde, Stärkungen (Buffs),
//! Heilung über Zeit, Gift und Ähnliches.
//!
//! Jede Wirkung eines Zaubers oder einer Technik mit `dauer > 0` wird hier
//! verwaltet. Ein erneutes Sprechen frischt die Wirkung auf (statt sie zu stapeln).
//!
//! | art        | wert bedeutet                         |
//! |------------|---------------------------------------|
//! | schild     | so viel Schaden fängt der Schild ab   |
//! | ruestung   | zusätzliche Rüstung                   |
//! | bonus      | Zuschlag auf `attribut` (z.B. staerke)|
//! | kraft      | Prozent mehr Schaden                  |
//! | tempo      | Prozent schnellere Angriffe           |
//! | heilen     | LP alle `intervall` Sekunden          |
//! | mana       | Mana alle `intervall` Sekunden        |
//! | ausdauer   | Ausdauer alle `intervall` Sekunden    |
//! | schaden    | (auf Gegner) Schaden alle `intervall` Sekunden, z.B. Gift |

use crate::daten::{WirkungDef, ZauberDef};
use crate::welt::Welt;
use std::time::{Duration, Instant};

const STANDARD_INTERVALL: f64 = 3.0;

#[derive(Debug, Clone)]
pub struct Effekt {
    /// Zauber, von dem die Wirkung stammt (zum Auffrischen)
    pub quelle: String,
    /// Name des Zaubers für die Anzeige
    pub name: String,
    pub art: String,
    pub attribut: String,
    pub wert: i32,
    /// Schild: wie viel noch abgefangen wird
    pub rest: i32,
    pub bis: Instant,
    /// Für Wirkungen, die immer wieder wirken (heilen, schaden ...)
    pub intervall: Option<Duration>,
    pub naechster: Instant,
    pub ende_text: String,
    /// Wer die Wirkung ausgelöst hat (bei Gift auf Monstern: der Spieler)
    pub von: Option<u64>,
}

impl Effekt {
    fn neu(z: &ZauberDef, wi: &WirkungDef, von: Option<u64>) -> Effekt {
        let jetzt = Instant::now();
        let wiederholt = matches!(wi.art.as_str(), "heilen" | "mana" | "ausdauer" | "schaden");
        let intervall = wiederholt.then(|| Duration::from_secs_f64(wi.intervall.unwrap_or(STANDARD_INTERVALL).max(0.5)));
        Effekt {
            quelle: z.id.clone(),
            name: z.name.clone(),
            art: wi.art.clone(),
            attribut: wi.attribut.clone(),
            wert: wi.wert,
            rest: wi.wert,
            bis: jetzt + Duration::from_secs_f64(wi.dauer),
            intervall,
            naechster: jetzt + intervall.unwrap_or_default(),
            ende_text: z.text_ende.clone(),
            von,
        }
    }

    fn gleich(&self, anderer: &Effekt) -> bool {
        self.quelle == anderer.quelle && self.art == anderer.art && self.attribut == anderer.attribut
    }

    /// Kurzbeschreibung für die Liste, z.B. "Schild (fängt noch 12 ab)"
    pub fn beschreibung(&self, w: &Welt) -> String {
        match self.art.as_str() {
            "schild" => format!("Schild, fängt noch {} Schaden ab", self.rest.max(0)),
            "ruestung" => format!("Rüstung {:+}", self.wert),
            "bonus" => format!("{} {:+}", w.d.anzeigename(&self.attribut), self.wert),
            "kraft" => format!("Schaden {:+}%", self.wert),
            "tempo" => format!("Angriffstempo {:+}%", self.wert),
            "heilen" => format!("{:+} LP alle {:.0} s", self.wert, self.intervall.unwrap_or_default().as_secs_f64()),
            "mana" => format!("{:+} Mana alle {:.0} s", self.wert, self.intervall.unwrap_or_default().as_secs_f64()),
            "ausdauer" => format!("{:+} Ausdauer alle {:.0} s", self.wert, self.intervall.unwrap_or_default().as_secs_f64()),
            "schaden" => format!("{} Schaden alle {:.0} s", self.wert, self.intervall.unwrap_or_default().as_secs_f64()),
            a => a.to_string(),
        }
    }
}

fn einfuegen(liste: &mut Vec<Effekt>, e: Effekt) -> bool {
    match liste.iter_mut().find(|x| x.gleich(&e)) {
        Some(alt) => {
            *alt = e;
            true
        }
        None => {
            liste.push(e);
            false
        }
    }
}

/// Wirkung mit Dauer auf einen Spieler legen.
pub fn auf_spieler(w: &mut Welt, sid: u64, z: &ZauberDef, wi: &WirkungDef, von: u64) {
    let e = Effekt::neu(z, wi, Some(von));
    let text = e.beschreibung(w);
    let sekunden = wi.dauer.round() as u64;
    let Some(s) = w.spieler.get_mut(&sid) else { return };
    let aufgefrischt = einfuegen(&mut s.effekte, e);
    let wie = if aufgefrischt { "aufgefrischt" } else { "wirkt" };
    s.sende(&format!("{{hellcyan}}{}: {text} ({wie}, {sekunden} s){{/}}", z.name));
}

/// Wirkung mit Dauer auf ein Monster legen (z.B. Gift).
pub fn auf_monster(w: &mut Welt, mid: u64, z: &ZauberDef, wi: &WirkungDef, von: u64) {
    let e = Effekt::neu(z, wi, Some(von));
    if let Some(m) = w.monster.get_mut(&mid) {
        einfuegen(&mut m.effekte, e);
    }
}

/// Summe einer Wirkung (z.B. alle "bonus" auf "staerke") bei einem Spieler.
pub fn summe(w: &Welt, sid: u64, art: &str, attribut: &str) -> i32 {
    let jetzt = Instant::now();
    w.spieler
        .get(&sid)
        .map(|s| {
            s.effekte
                .iter()
                .filter(|e| e.bis > jetzt && e.art == art && (attribut.is_empty() || e.attribut == attribut))
                .map(|e| e.wert)
                .sum()
        })
        .unwrap_or(0)
}

/// Schilde fangen Schaden ab. Gibt den Schaden zurück, der noch durchkommt.
pub fn schild_abfangen(w: &mut Welt, sid: u64, mut schaden: i32) -> (i32, i32) {
    let Some(s) = w.spieler.get_mut(&sid) else { return (schaden, 0) };
    let mut abgefangen = 0;
    for e in s.effekte.iter_mut().filter(|e| e.art == "schild" && e.rest > 0) {
        let weg = e.rest.min(schaden);
        e.rest -= weg;
        schaden -= weg;
        abgefangen += weg;
        if schaden == 0 {
            break;
        }
    }
    // Verbrauchte Schilde enden sofort
    let mut enden = Vec::new();
    s.effekte.retain(|e| {
        let weg = e.art == "schild" && e.rest <= 0;
        if weg {
            enden.push(e.clone());
        }
        !weg
    });
    for e in enden {
        ende_melden(s, &e, true);
    }
    (schaden, abgefangen)
}

fn ende_melden(s: &crate::welt::Sitzung, e: &Effekt, zerbrochen: bool) {
    let t = if !e.ende_text.is_empty() {
        e.ende_text.clone()
    } else if zerbrochen {
        format!("Dein Schild ({}) zerbricht.", e.name)
    } else {
        format!("Die Wirkung von {} lässt nach.", e.name)
    };
    s.sende(&format!("{{grau}}{t}{{/}}"));
}

/// Alle Wirkungen fortschreiben: wiederkehrende auslösen, abgelaufene beenden.
/// Gibt die Spieler zurück, deren Werte sich geändert haben (für den Prompt).
pub fn takt(w: &mut Welt) -> Vec<u64> {
    let jetzt = Instant::now();
    let mut geaendert = Vec::new();

    // Spieler
    let ids: Vec<u64> = w.spieler.iter().filter(|(_, s)| !s.effekte.is_empty()).map(|(id, _)| *id).collect();
    for sid in ids {
        let Some(s) = w.spieler.get_mut(&sid) else { continue };
        if s.tot_bis.is_some() {
            s.effekte.clear();
            continue;
        }
        let mut veraendert = false;
        for e in s.effekte.iter_mut() {
            let Some(iv) = e.intervall else { continue };
            while e.naechster <= jetzt && e.naechster <= e.bis {
                e.naechster += iv;
                let p = &mut s.sp;
                match e.art.as_str() {
                    "heilen" => p.lebenspunkte = (p.lebenspunkte + e.wert).clamp(1, p.max_lebenspunkte),
                    "mana" => p.mana = (p.mana + e.wert).clamp(0, p.max_mana),
                    "ausdauer" => p.ausdauer = (p.ausdauer + e.wert).clamp(0, p.max_ausdauer),
                    _ => {}
                }
                veraendert = true;
            }
        }
        let (bleiben, vorbei): (Vec<Effekt>, Vec<Effekt>) = std::mem::take(&mut s.effekte).into_iter().partition(|e| e.bis > jetzt);
        s.effekte = bleiben;
        for e in &vorbei {
            ende_melden(s, e, false);
        }
        if veraendert || !vorbei.is_empty() {
            geaendert.push(sid);
        }
    }

    // Monster (Gift und Ähnliches)
    let mids: Vec<u64> = w.monster.iter().filter(|(_, m)| !m.effekte.is_empty()).map(|(id, _)| *id).collect();
    for mid in mids {
        let mut treffer: Vec<(i32, Option<u64>, String)> = Vec::new();
        if let Some(m) = w.monster.get_mut(&mid) {
            for e in m.effekte.iter_mut() {
                let Some(iv) = e.intervall else { continue };
                while e.naechster <= jetzt && e.naechster <= e.bis {
                    e.naechster += iv;
                    if e.art == "schaden" {
                        treffer.push((e.wert, e.von, e.name.clone()));
                    }
                }
            }
            m.effekte.retain(|e| e.bis > jetzt);
        }
        for (schaden, von, name) in treffer {
            let tot = {
                let Some(m) = w.monster.get_mut(&mid) else { break };
                m.lebenspunkte -= schaden.max(1);
                m.lebenspunkte <= 0
            };
            let monster = crate::text::gross(&w.monster_name(mid));
            if let Some(sid) = von {
                let zahl = if w.spieler.get(&sid).is_some_and(|s| s.sp.einstellungen.kampfzahlen) {
                    format!(" {{grau}}({schaden}){{/}}")
                } else {
                    String::new()
                };
                w.sende(sid, &format!("{monster} leidet unter {name}.{zahl}"));
                geaendert.push(sid);
            }
            if tot {
                crate::kampf::monster_stirbt(w, mid);
                break;
            }
        }
    }
    geaendert
}

/// Befehl "wirkungen": was gerade auf einem wirkt.
pub fn anzeigen(w: &Welt, sid: u64) {
    let Some(s) = w.spieler.get(&sid) else { return };
    let jetzt = Instant::now();
    if s.effekte.is_empty() {
        w.sende(sid, "Gerade wirkt nichts auf dich.");
        return;
    }
    let mut t = String::from("{fett}Was gerade auf dich wirkt:{/}\n");
    for e in &s.effekte {
        let rest = e.bis.saturating_duration_since(jetzt).as_secs();
        t.push_str(&format!("  {{hellcyan}}{:<20}{{/}} {} {{grau}}(noch {rest} s){{/}}\n", e.name, e.beschreibung(w)));
    }
    w.sende(sid, t.trim_end());
}
