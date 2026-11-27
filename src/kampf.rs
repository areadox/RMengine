//! Das Kampfsystem: Echtzeit mit Erholungszeit – keine Runden.
//!
//! Jeder Angriff hat eine Erholungszeit (schnelle Waffen kurz, schwere lang).
//! Erst danach kann man wieder zuschlagen. Bewegen, Tränke trinken, Reden
//! und Zaubern geht jederzeit. Monster handeln nach ihrem eigenen Takt.
//!
//! Stirbt eine Figur, bleibt sie leblos liegen. Andere Spieler können sie
//! eine Weile lang wiederbeleben (`wiederbeleben <name>`), sonst – oder mit
//! `aufgeben` sofort – erwacht sie im Tempel. Verluste gibt es nur, wenn das
//! in welt.toml unter [kampf] eingestellt ist.

use crate::befehle;
use crate::text::gross;
use crate::welt::Welt;
use rand::seq::SliceRandom;
use rand::Rng;
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

/// Wie man kämpft – angriffslustig teilt mehr aus und steckt mehr ein.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Haltung {
    Angriffslustig,
    Ausgewogen,
    Defensiv,
}

impl Haltung {
    pub fn aus_text(t: &str) -> Option<Haltung> {
        match crate::text::normalisieren(t).as_str() {
            "angriffslustig" | "aggressiv" | "angriff" | "offensiv" => Some(Haltung::Angriffslustig),
            "ausgewogen" | "normal" | "neutral" => Some(Haltung::Ausgewogen),
            "defensiv" | "vorsichtig" | "verteidigung" => Some(Haltung::Defensiv),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Haltung::Angriffslustig => "angriffslustig",
            Haltung::Ausgewogen => "ausgewogen",
            Haltung::Defensiv => "defensiv",
        }
    }
    /// (eigener Schaden, erlittener Schaden, Trefferchance des Gegners)
    fn faktoren(self) -> (f64, f64, i32) {
        match self {
            Haltung::Angriffslustig => (1.25, 1.25, 10),
            Haltung::Ausgewogen => (1.0, 1.0, 0),
            Haltung::Defensiv => (0.75, 0.75, -10),
        }
    }
}

/// "schwer verletzt" usw.
pub fn zustand(lp: i32, max: i32) -> &'static str {
    let p = if max > 0 { lp * 100 / max } else { 0 };
    match p {
        100.. => "unverletzt",
        76..=99 => "leicht verletzt",
        51..=75 => "verletzt",
        26..=50 => "schwer verletzt",
        11..=25 => "sehr schwer verletzt",
        _ => "dem Tode nahe",
    }
}

/// Erfahrung, die man für die nächste Stufe braucht.
pub fn ep_fuer_naechste(w: &Welt, stufe: u32) -> u64 {
    let k = &w.d.kampf;
    (k.ep_basis * (stufe.max(1) as f64).powf(k.ep_faktor)).round().max(1.0) as u64
}

/// Ein Wert des Spielers inklusive Boni durch angelegte Ausrüstung.
pub fn wert(w: &Welt, sid: u64, name: &str) -> i32 {
    let Some(s) = w.spieler.get(&sid) else { return 0 };
    let grund = s.sp.werte.get(name).copied().unwrap_or(10);
    let bonus: i32 = s
        .sp
        .ausruestung
        .values()
        .filter_map(|g| w.d.gegenstaende.get(g))
        .filter_map(|g| g.eigenschaften.get(name))
        .sum();
    // Rasse, Stärkungen durch Zauber
    let rasse = w.d.rassen.get(&s.sp.rasse).and_then(|r| r.werte.get(name)).copied().unwrap_or(0);
    grund + bonus + rasse + crate::wirkung::summe(w, sid, "bonus", name)
}

fn ruestung_spieler(w: &Welt, sid: u64) -> i32 {
    w.spieler
        .get(&sid)
        .map(|s| {
            s.sp.ausruestung
                .values()
                .filter_map(|g| w.d.gegenstaende.get(g))
                .filter_map(|g| g.eigenschaften.get("ruestung"))
                .sum::<i32>()
        })
        .unwrap_or(0)
        + crate::wirkung::summe(w, sid, "ruestung", "")
}

/// (Schaden, Erholung in Sekunden, Fertigkeit) der angelegten Waffe.
fn waffe(w: &Welt, sid: u64) -> (i32, f64, String) {
    let k = &w.d.kampf;
    let waffe = w.spieler.get(&sid).and_then(|s| s.sp.ausruestung.get("waffe")).and_then(|g| w.d.gegenstaende.get(g));
    match waffe {
        Some(g) => (
            g.eigenschaften.get("schaden").copied().unwrap_or(k.waffenlos_schaden),
            g.erholung.unwrap_or(k.waffenlos_erholung),
            g.fertigkeit.clone(),
        ),
        None => (k.waffenlos_schaden, k.waffenlos_erholung, k.waffenlos_fertigkeit.clone()),
    }
}

pub fn ist_tot(w: &Welt, sid: u64) -> bool {
    w.spieler.get(&sid).is_some_and(|s| s.tot_bis.is_some())
}

/// Ist der Spieler gerade in einen Kampf verwickelt?
pub fn im_kampf(w: &Welt, sid: u64) -> bool {
    w.spieler.get(&sid).is_some_and(|s| s.kampf_ziel.is_some()) || w.monster.values().any(|m| m.ziele.contains(&sid))
}

/// Alle Kämpfe des Spielers beenden (Flucht, Teleport, Tod, Abmelden).
pub fn kampf_beenden(w: &mut Welt, sid: u64) {
    if let Some(s) = w.spieler.get_mut(&sid) {
        s.kampf_ziel = None;
    }
    for m in w.monster.values_mut() {
        m.ziele.retain(|z| *z != sid);
    }
}

/// Spieler und Monster verwickeln sich in einen Kampf.
pub(crate) fn verwickeln(w: &mut Welt, sid: u64, mid: u64) {
    if let Some(m) = w.monster.get_mut(&mid) {
        if !m.ziele.contains(&sid) {
            if m.ziele.is_empty() {
                // Kurze Reaktionszeit, damit das Monster nicht sofort zuschlägt
                let erholung = w.d.monster.get(&m.def).map(|d| d.erholung).unwrap_or(3.0);
                m.bereit_ab = Instant::now() + Duration::from_secs_f64(erholung * 0.6);
            }
            m.ziele.push(sid);
        }
    }
    if let Some(s) = w.spieler.get_mut(&sid) {
        if s.kampf_ziel.is_none() {
            s.kampf_ziel = Some(mid);
        }
    }
}

// ---------------------------------------------------------------------------
// Befehle der Spieler
// ---------------------------------------------------------------------------

/// "toete ratte": Kampf beginnen (und sofort zuschlagen, wenn man bereit ist).
pub fn angreifen(w: &mut Welt, sid: u64, mid: u64) {
    let Some(def) = w.monster.get(&mid).and_then(|m| w.d.monster.get(&m.def)) else { return };
    if !def.ist_angreifbar() {
        let name = gross(&def.name);
        w.sende(sid, &format!("{name} ist friedlich. Hier wird nicht gekämpft."));
        return;
    }
    // Skripte können den Angriff verhindern ("Der Wächter weicht geschickt aus ...")
    if !crate::skripte::angriff(w, sid, mid) {
        return;
    }
    let akk = w.monster_akk(mid);
    let (name, raum) = {
        let s = &w.spieler[&sid];
        (s.sp.name.clone(), s.sp.raum.clone())
    };
    if let Some(s) = w.spieler.get_mut(&sid) {
        s.kampf_ziel = Some(mid);
    }
    verwickeln(w, sid, mid);
    w.sende(sid, &format!("{{hellrot}}Du greifst {akk} an!{{/}}"));
    w.sende_raum(&raum, &format!("{name} greift {akk} an!"), &[sid]);
    if w.spieler[&sid].bereit_ab <= Instant::now() {
        crate::bereitschaft::auto_angriff(w, sid);
    }
}

/// Ausweichen: Für kurze Zeit treffen Angriffe viel seltener.
pub fn ausweichen(w: &mut Welt, sid: u64) {
    let jetzt = Instant::now();
    let kosten = w.d.kampf.ausdauer_pro_angriff * 2;
    let Some(s) = w.spieler.get_mut(&sid) else { return };
    if s.bereit_ab > jetzt {
        let rest = (s.bereit_ab - jetzt).as_secs_f64();
        w.sende(sid, &format!("Du bist noch nicht bereit ({rest:.1} s)."));
        return;
    }
    if s.sp.ausdauer < kosten {
        w.sende(sid, "Dafür bist du zu erschöpft.");
        return;
    }
    s.sp.ausdauer -= kosten;
    s.ausweichen_bis = Some(jetzt + Duration::from_millis(2500));
    s.bereit_ab = jetzt + Duration::from_millis(1500);
    let (name, raum) = (s.sp.name.clone(), s.sp.raum.clone());
    w.sende(sid, "Du gehst in Deckung und achtest auf jede Bewegung.");
    w.sende_raum(&raum, &format!("{name} duckt sich und weicht aus."), &[sid]);
}

/// Fliehen: klappt nicht immer, dann verliert man Zeit.
pub fn fliehen(w: &mut Welt, sid: u64) {
    if !im_kampf(w, sid) {
        w.sende(sid, "Du kämpfst doch gar nicht.");
        return;
    }
    let jetzt = Instant::now();
    if w.spieler[&sid].bereit_ab > jetzt {
        let rest = (w.spieler[&sid].bereit_ab - jetzt).as_secs_f64();
        w.sende(sid, &format!("Du bist noch nicht bereit ({rest:.1} s)."));
        return;
    }
    let raum = w.spieler[&sid].sp.raum.clone();
    let name = w.spieler[&sid].sp.name.clone();
    // Fliehen geht nur durch offene Türen
    let ausgaenge: Vec<_> = w.offene_ausgaenge(&raum).into_iter().collect();
    let chance = (50 + (wert(w, sid, "geschick") - 10) * 3).clamp(20, 90);
    let mut rng = rand::thread_rng();
    let Some((richt, ziel)) = ausgaenge.choose(&mut rng).cloned() else {
        w.sende(sid, "Es gibt keinen Ausweg!");
        return;
    };
    if rng.gen_range(0..100) >= chance {
        if let Some(s) = w.spieler.get_mut(&sid) {
            s.bereit_ab = jetzt + Duration::from_secs(2);
        }
        w.sende(sid, "{hellrot}Du versuchst zu fliehen, aber dir wird der Weg abgeschnitten!{/}");
        w.sende_raum(&raum, &format!("{name} versucht zu fliehen, kommt aber nicht weg."), &[sid]);
        return;
    }
    kampf_beenden(w, sid);
    w.sende_raum(&raum, &format!("{name} flieht Hals über Kopf!"), &[sid]);
    if let Some(s) = w.spieler.get_mut(&sid) {
        s.sp.raum = ziel.clone();
        if !crate::oberwelt::ist_oberwelt(&ziel) {
            s.sp.besucht.insert(ziel.clone());
        }
    }
    w.sende(sid, &format!("{{hellgelb}}Du fliehst nach {}!{{/}}", richt.anzeige()));
    w.sende_raum(&ziel, &format!("{name} {} und ist völlig außer Atem.", richt.kommt()), &[sid]);
    befehle::raum_anzeigen(w, sid);
    crate::skripte::raumwechsel(w, sid, &raum, &ziel);
}

/// "schaetze wolf": Wie gefährlich ist ein Gegner?
pub fn einschaetzen(w: &Welt, sid: u64, mid: u64) {
    let Some(m) = w.monster.get(&mid) else { return };
    let Some(def) = w.d.monster.get(&m.def) else { return };
    let name = gross(&def.name);
    let stufe = w.spieler[&sid].sp.stufe as i32;
    let unterschied = def.stufe as i32 - stufe;
    let urteil = match unterschied {
        i32::MIN..=-3 => "{hellgruen}ist kein Gegner für dich.{/}",
        -2..=-1 => "{gruen}sollte leicht zu besiegen sein.{/}",
        0 => "{gelb}ist dir ebenbürtig.{/}",
        1..=2 => "{hellrot}ist stärker als du. Sei vorsichtig!{/}",
        _ => "{rot}würde dich zerquetschen. Lauf!{/}",
    };
    let mut t = format!("{name} {urteil} Zustand: {}.", zustand(m.lebenspunkte, def.lebenspunkte));
    if !def.ist_angreifbar() {
        t = format!("{name} ist friedlich.");
    }
    w.sende(sid, &t);
}

/// "steigere staerke": freie Punkte aus Stufenaufstiegen verteilen.
pub fn steigern(w: &mut Welt, sid: u64, was: &str) {
    let id = crate::text::normalisieren(was);
    let Some(s) = w.spieler.get(&sid) else { return };
    if s.sp.freie_punkte == 0 {
        w.sende(sid, "Du hast keine Punkte zu verteilen. Die gibt es beim Aufstieg in eine neue Stufe.");
        return;
    }
    if id.is_empty() || !s.sp.werte.contains_key(&id) {
        let namen: Vec<String> = s.sp.werte.keys().map(|k| w.d.anzeigename(k)).collect();
        w.sende(
            sid,
            &format!("Was willst du steigern? Möglich: {}. Du hast {} Punkte.", namen.join(", "), s.sp.freie_punkte),
        );
        return;
    }
    let anzeige = w.d.anzeigename(&id);
    let Some(s) = w.spieler.get_mut(&sid) else { return };
    s.sp.freie_punkte -= 1;
    let neu = {
        let v = s.sp.werte.entry(id).or_insert(10);
        *v += 1;
        *v
    };
    let rest = s.sp.freie_punkte;
    let sp = s.sp.clone();
    w.speichern(&sp);
    let punkte = if rest == 1 { "Punkt" } else { "Punkte" };
    w.sende(sid, &format!("{{hellgruen}}{anzeige} steigt auf {neu}.{{/}} Noch {rest} {punkte}."));
}

/// Wiederbelebung durch einen anderen Spieler am selben Ort.
pub fn wiederbeleben(w: &mut Welt, sid: u64, wen: &str) {
    if ist_tot(w, sid) {
        w.sende(sid, "Du bist selbst tot.");
        return;
    }
    let raum = w.spieler[&sid].sp.raum.clone();
    let such = crate::text::normalisieren(wen);
    let ziel = w
        .spieler
        .iter()
        .find(|(id, s)| **id != sid && s.sp.raum == raum && s.tot_bis.is_some() && crate::text::normalisieren(&s.sp.name).starts_with(&such))
        .map(|(id, _)| *id);
    let Some(tid) = ziel else {
        w.sende(sid, "Hier liegt niemand, den du wiederbeleben könntest.");
        return;
    };
    let helfer = w.spieler[&sid].sp.name.clone();
    let name = w.spieler[&tid].sp.name.clone();
    erwecken(w, tid, false);
    w.sende(sid, &format!("{{hellgruen}}Du kniest neben {name} nieder und holst {name} ins Leben zurück.{{/}}"));
    w.sende(tid, &format!("{{hellgruen}}{helfer} hat dich ins Leben zurückgeholt!{{/}}"));
    w.sende_raum(&raum, &format!("{helfer} holt {name} ins Leben zurück."), &[sid, tid]);
    befehle::prompt(w, tid);
}

/// "aufgeben": sofort im Tempel erwachen.
pub fn aufgeben(w: &mut Welt, sid: u64) {
    if !ist_tot(w, sid) {
        w.sende(sid, "Du lebst doch noch!");
        return;
    }
    erwecken(w, sid, true);
}

// ---------------------------------------------------------------------------
// Angriffe
// ---------------------------------------------------------------------------

/// Der Spieler greift sein Ziel an. Gibt false zurück, wenn es kein gültiges Ziel gibt.
pub fn spieler_schlaegt(w: &mut Welt, sid: u64) -> bool {
    let Some(mid) = w.spieler.get(&sid).and_then(|s| s.kampf_ziel) else { return false };
    schlag(w, sid, mid, None)
}

/// Eine Kampftechnik: ein Waffenschlag mit Faktor und Zusatzschaden.
pub struct Technik<'a> {
    pub name: &'a str,
    pub faktor: f64,
    pub zusatz: i32,
    pub text_selbst: &'a str,
    pub text_raum: &'a str,
}

/// Kampftechnik gegen ein Monster (Kosten sind schon bezahlt).
pub fn technik(w: &mut Welt, sid: u64, mid: u64, t: &Technik) -> bool {
    if let Some(s) = w.spieler.get_mut(&sid) {
        s.kampf_ziel = Some(mid);
    }
    verwickeln(w, sid, mid);
    schlag(w, sid, mid, Some(t))
}

/// Ein Schlag mit der Waffe – normal oder als Technik.
fn schlag(w: &mut Welt, sid: u64, mid: u64, technik: Option<&Technik>) -> bool {
    let jetzt = Instant::now();
    let Some(s) = w.spieler.get(&sid) else { return false };
    let Some(m) = w.monster.get(&mid) else { return false };
    if m.raum != s.sp.raum || s.tot_bis.is_some() {
        return false;
    }
    let Some(def) = w.d.monster.get(&m.def).cloned() else { return false };
    let (name, raum, haltung, zahlen) =
        (s.sp.name.clone(), s.sp.raum.clone(), s.haltung, s.sp.einstellungen.kampfzahlen);
    let spieler_stufe = s.sp.stufe as i32;
    let (mut waffen_schaden, erholung, fert_id) = waffe(w, sid);
    let fert = s.sp.fertigkeiten.get(&fert_id).copied().unwrap_or(0) as i32;
    let (staerke, geschick) = (wert(w, sid, "staerke"), wert(w, sid, "geschick"));
    let akk = w.monster_akk(mid);
    let monster_name = gross(&w.monster_name(mid));
    // Ohne Waffe: mit einem Körperteil (Faust, Tritt ...)
    let dativ = w.monster_dativ(mid);
    let koerper_angriff = if technik.is_none() { crate::koerper::spieler_angriff(w, sid, &akk, &dativ) } else { None };
    if let Some((_, _, sch)) = &koerper_angriff {
        if *sch > 0 {
            waffen_schaden = *sch;
        }
    }

    let mut rng = rand::thread_rng();
    let chance = (75 + (geschick - 10) * 2 + fert / 4 - (def.stufe as i32 - spieler_stufe) * 5).clamp(10, 95);
    let getroffen = rng.gen_range(0..100) < chance;
    let kraft = crate::wirkung::summe(w, sid, "kraft", "");
    let tempo_bonus = crate::wirkung::summe(w, sid, "tempo", "");

    // Ausdauer und Erholung
    let kosten = w.d.kampf.ausdauer_pro_angriff;
    let erschoepft;
    {
        let Some(s) = w.spieler.get_mut(&sid) else { return false };
        erschoepft = s.sp.ausdauer < kosten;
        s.sp.ausdauer = (s.sp.ausdauer - kosten).max(0);
        let tempo = (1.0 - (geschick - 10) as f64 * 0.02).clamp(0.6, 1.4);
        let mut sekunden = (erholung * tempo / (1.0 + tempo_bonus as f64 / 100.0).max(0.2)).max(0.6);
        if erschoepft {
            sekunden *= 1.5;
        }
        s.bereit_ab = jetzt + Duration::from_secs_f64(sekunden);
    }

    let ersetzen = |t: &str| t.replace("{name}", &name).replace("{ziel}", &akk);
    if let Some(t) = technik {
        let selbst = if t.text_selbst.is_empty() { format!("Du setzt {} ein!", t.name) } else { ersetzen(t.text_selbst) };
        let andere = if t.text_raum.is_empty() { format!("{name} setzt {} ein!", t.name) } else { ersetzen(t.text_raum) };
        w.sende(sid, &format!("{{hellmagenta}}{selbst}{{/}}"));
        w.sende_raum(&raum, &format!("{{hellmagenta}}{andere}{{/}}"), &[sid]);
    }
    if !getroffen {
        match &koerper_angriff {
            Some((du, er, _)) => {
                w.sende(sid, &format!("Du {du}, verfehlst aber."));
                w.sende_raum(&raum, &format!("{name} {er}, verfehlt aber."), &[sid]);
            }
            None => {
                w.sende(sid, &format!("Du verfehlst {akk}."));
                w.sende_raum(&raum, &format!("{name} verfehlt {akk}."), &[sid]);
            }
        }
        return true;
    }

    let mut schaden = (waffen_schaden + staerke / 4 + fert / 10) as f64;
    if let Some(t) = technik {
        schaden = schaden * t.faktor + t.zusatz as f64;
    }
    schaden *= 1.0 + kraft as f64 / 100.0;
    schaden *= rng.gen_range(0.75..1.25) * haltung.faktoren().0;
    if erschoepft {
        schaden *= 0.5;
    }
    let schaden = ((schaden.round() as i32) - def.ruestung).max(1);

    let (lp_neu, tot) = {
        let Some(m) = w.monster.get_mut(&mid) else { return false };
        m.lebenspunkte -= schaden;
        (m.lebenspunkte, m.lebenspunkte <= 0)
    };
    let zahl = if zahlen { format!(" {{grau}}({schaden}){{/}}") } else { String::new() };
    let wie = if erschoepft { " erschöpft" } else { "" };
    match &koerper_angriff {
        Some((du, er, _)) => {
            w.sende(sid, &format!("Du {du}{wie}.{zahl}"));
            w.sende_raum(&raum, &format!("{name} {er}."), &[sid]);
        }
        None => {
            w.sende(sid, &format!("Du triffst {akk}{wie}.{zahl}"));
            w.sende_raum(&raum, &format!("{name} trifft {akk}."), &[sid]);
        }
    }

    // Übung macht den Meister
    if let Some(f) = w.d.fertigkeiten.get(&fert_id).cloned() {
        if rng.gen_range(0..100) < f.lernchance {
            if let Some(s) = w.spieler.get_mut(&sid) {
                let st = s.sp.fertigkeiten.entry(f.id.clone()).or_insert(0);
                if *st < f.max_stufe {
                    *st += 1;
                    let neu = *st;
                    w.sende(sid, &format!("{{hellgruen}}Deine Fertigkeit {} steigt auf {neu}!{{/}}", f.name));
                }
            }
        }
    }

    if tot {
        monster_stirbt(w, mid);
    } else {
        w.sende(sid, &format!("{monster_name} ist {}.", zustand(lp_neu, def.lebenspunkte)));
    }
    true
}

/// Ein Zauber mit der Wirkung "schaden" trifft ein Monster.
pub fn zauber_trifft(w: &mut Welt, sid: u64, mid: u64, grundwert: i32, fertigkeit: &str) {
    let Some(def) = w.monster.get(&mid).and_then(|m| w.d.monster.get(&m.def)).cloned() else { return };
    if !def.ist_angreifbar() {
        w.sende(sid, &format!("{} ist friedlich. Der Zauber verpufft.", gross(&def.name)));
        return;
    }
    let intelligenz = wert(w, sid, "intelligenz");
    let fert = w.spieler[&sid].sp.fertigkeiten.get(fertigkeit).copied().unwrap_or(0) as i32;
    let mut rng = rand::thread_rng();
    let schaden =
        (((grundwert + intelligenz / 3 + fert / 10) as f64) * rng.gen_range(0.8..1.2)).round().max(1.0) as i32;
    let zahlen = w.spieler[&sid].sp.einstellungen.kampfzahlen;
    let zauber_erholung = w.d.kampf.zauber_erholung;
    verwickeln(w, sid, mid);
    if let Some(s) = w.spieler.get_mut(&sid) {
        s.bereit_ab = s.bereit_ab.max(Instant::now() + Duration::from_secs_f64(zauber_erholung));
    }
    let (lp, tot) = {
        let Some(m) = w.monster.get_mut(&mid) else { return };
        m.lebenspunkte -= schaden;
        (m.lebenspunkte, m.lebenspunkte <= 0)
    };
    let zahl = if zahlen { format!(" {{grau}}({schaden}){{/}}") } else { String::new() };
    let name = gross(&def.name);
    if tot {
        w.sende(sid, &format!("Der Zauber trifft!{zahl}"));
        monster_stirbt(w, mid);
    } else {
        w.sende(sid, &format!("Der Zauber trifft!{zahl} {name} ist {}.", zustand(lp, def.lebenspunkte)));
    }
}

/// Das Monster greift einen seiner Gegner an.
fn monster_schlaegt(w: &mut Welt, mid: u64) {
    let jetzt = Instant::now();
    let Some(m) = w.monster.get(&mid) else { return };
    let Some(def) = w.d.monster.get(&m.def).cloned() else { return };
    let Some(&sid) = m.ziele.first() else { return };
    let monster_mana = m.mana;
    let Some(s) = w.spieler.get(&sid) else { return };
    let (name, raum, haltung, zahlen) =
        (s.sp.name.clone(), s.sp.raum.clone(), s.haltung, s.sp.einstellungen.kampfzahlen);
    let ausweichen = s.ausweichen_bis.is_some_and(|b| b > jetzt);
    let spieler_stufe = s.sp.stufe as i32;
    let monster_name = gross(&def.name);
    let mut rng = rand::thread_rng();

    if let Some(m) = w.monster.get_mut(&mid) {
        m.bereit_ab = jetzt + Duration::from_secs_f64(def.erholung * rng.gen_range(0.85..1.15));
    }

    // Manchmal zaubert das Monster
    let zauber = def
        .zauber
        .iter()
        .filter_map(|z| w.d.zauber.get(z))
        .filter(|z| z.mana_kosten <= monster_mana && z.wirkungen.iter().any(|wi| wi.art == "schaden"))
        .collect::<Vec<_>>()
        .choose(&mut rng)
        .map(|z| (*z).clone());
    let teil_angriff = crate::koerper::monster_angriff(w, &def, &name);
    let (grundschaden, verb_dich, verb_andere, magisch) = match zauber {
        Some(z) if rng.gen_range(0..100) < 30 => {
            if let Some(m) = w.monster.get_mut(&mid) {
                m.mana -= z.mana_kosten;
            }
            let wert = z.wirkungen.iter().filter(|wi| wi.art == "schaden").map(|wi| wi.wert).sum::<i32>();
            (wert, format!("schleudert {} auf dich", z.name), format!("schleudert {} auf {name}", z.name), true)
        }
        // Mit einem Körperteil (Maul, Pfote ...)
        _ if teil_angriff.is_some() => {
            let (dich, andere, schaden) = teil_angriff.expect("geprüft");
            (schaden, dich, andere, false)
        }
        _ => {
            let verb = def.angriffe.choose(&mut rng).cloned().unwrap_or_else(|| "greift".into());
            let (dich, andere) = if verb == "greift" {
                ("greift dich an".to_string(), format!("greift {name} an"))
            } else {
                (format!("{verb} dich"), format!("{verb} {name}"))
            };
            (def.kampf_schaden(), dich, andere, false)
        }
    };

    let geschick = wert(w, sid, "geschick");
    let mut chance = (60 + def.stufe as i32 * 3 + (def.stufe as i32 - spieler_stufe) * 3 - (geschick - 10) * 2
        + haltung.faktoren().2)
        .clamp(5, 95);
    if ausweichen {
        chance /= 3;
    }
    if !magisch && rng.gen_range(0..100) >= chance {
        let wie = if ausweichen { "Du weichst geschickt aus!" } else { "Du kannst ausweichen." };
        w.sende(sid, &format!("{monster_name} {verb_dich}, aber verfehlt. {wie}"));
        w.sende_raum(&raum, &format!("{monster_name} {verb_andere}, verfehlt aber."), &[sid]);
        return;
    }
    let mut schaden = grundschaden as f64 * rng.gen_range(0.75..1.25) * haltung.faktoren().1;
    if !magisch {
        schaden -= ruestung_spieler(w, sid) as f64;
    }
    let schaden = (schaden.round() as i32).max(1);
    // Schilde fangen zuerst ab
    let (schaden, abgefangen) = crate::wirkung::schild_abfangen(w, sid, schaden);
    if schaden == 0 {
        let zahl = if zahlen { format!(" {{grau}}({abgefangen} abgefangen){{/}}") } else { String::new() };
        w.sende(sid, &format!("{monster_name} {verb_dich}, aber dein Schild fängt alles ab.{zahl}"));
        w.sende_raum(&raum, &format!("{monster_name} {verb_andere}, prallt aber an einem Schild ab."), &[sid]);
        return;
    }
    let (lp, max, tot) = {
        let Some(s) = w.spieler.get_mut(&sid) else { return };
        s.sp.lebenspunkte -= schaden;
        (s.sp.lebenspunkte, s.sp.max_lebenspunkte, s.sp.lebenspunkte <= 0)
    };
    let zahl = match (zahlen, abgefangen) {
        (false, _) => String::new(),
        (true, 0) => format!(" {{grau}}({schaden}){{/}}"),
        (true, a) => format!(" {{grau}}({schaden}, Schild fängt {a} ab){{/}}"),
    };
    w.sende(sid, &format!("{{hellrot}}{monster_name} {verb_dich}!{{/}}{zahl}"));
    w.sende_raum(&raum, &format!("{monster_name} {verb_andere}."), &[sid]);
    if tot {
        spieler_stirbt(w, sid);
    } else if lp * 100 / max.max(1) <= 25 {
        w.sende(sid, "{rot}Du bist schwer verletzt! Denk an Flucht oder einen Heiltrank.{/}");
    }
}

// ---------------------------------------------------------------------------
// Sieg, Tod und Stufen
// ---------------------------------------------------------------------------

pub(crate) fn monster_stirbt(w: &mut Welt, mid: u64) {
    let Some(m) = w.monster.remove(&mid) else { return };
    let Some(def) = w.d.monster.get(&m.def).cloned() else { return };
    let name = gross(&def.name);
    let text = match def.todestext.trim().strip_prefix('*') {
        Some(t) => format!("{name} {}", t.trim()),
        None if !def.todestext.trim().is_empty() => def.todestext.clone(),
        None => format!("{name} bricht zusammen und stirbt."),
    };
    w.sende_raum(&m.raum, &format!("{{hellgelb}}{text}{{/}}"), &[]);

    // Beute: in die Leiche oder (ohne Leiche) auf den Boden
    let mut rng = rand::thread_rng();
    let beute: Vec<String> = def.beute.iter().filter(|b| rng.gen_range(0..100) < b.chance).map(|b| b.gegenstand.clone()).collect();
    let sieger_namen: Vec<String> =
        m.ziele.iter().filter_map(|sid| w.spieler.get(sid)).filter(|s| s.sp.raum == m.raum).map(|s| s.sp.name.clone()).collect();
    for g in crate::leichen::hinlegen(w, &def.id, &m.raum, beute, sieger_namen) {
        w.boden.entry(m.raum.clone()).or_default().push(g.clone());
        let g = gross(&w.d.gegenstand_name(&g));
        w.sende_raum(&m.raum, &format!("{g} fällt zu Boden."), &[]);
    }

    // Erfahrung (und Geld) für alle, die mitgekämpft haben und noch da sind
    let ep = def.kampf_erfahrung();
    let sieger: Vec<u64> = m
        .ziele
        .iter()
        .copied()
        .filter(|sid| w.spieler.get(sid).is_some_and(|s| s.sp.raum == m.raum && s.tot_bis.is_none()))
        .collect();
    let geld = def.geld.wuerfeln();
    if geld > 0 && !sieger.is_empty() {
        let anteil = geld / sieger.len() as u64;
        let rest = geld % sieger.len() as u64;
        for (i, sid) in sieger.iter().enumerate() {
            let betrag = anteil + if i == 0 { rest } else { 0 };
            if betrag == 0 {
                continue;
            }
            if let Some(s) = w.spieler.get_mut(sid) {
                s.sp.geld += betrag;
            }
            let bei = if def.dativ.is_empty() { String::new() } else { format!(" bei {}", def.dativ) };
            let summe = crate::handel::betrag(w, betrag);
            w.sende(*sid, &format!("{{hellgelb}}Du findest {summe}{bei}.{{/}}"));
        }
    }
    for sid in &sieger {
        crate::quests::fortschritt(w, *sid, "toeten", &def.id, 1);
        crate::skripte::sieg(w, *sid, &def.id);
    }
    for sid in m.ziele {
        let Some(s) = w.spieler.get_mut(&sid) else { continue };
        if s.kampf_ziel == Some(mid) {
            s.kampf_ziel = None;
        }
        if s.sp.raum != m.raum || s.tot_bis.is_some() {
            continue;
        }
        s.sp.siege += 1;
        w.sende(sid, &format!("{{hellgruen}}Du erhältst {ep} Erfahrungspunkte.{{/}}"));
        erfahrung_geben(w, sid, ep);
    }
    // Wer dieses Monster noch als Ziel hatte, sucht sich das nächste
    for s in w.spieler.values_mut() {
        if s.kampf_ziel == Some(mid) {
            s.kampf_ziel = None;
        }
    }
}

/// Erfahrung gutschreiben und ggf. aufsteigen.
pub fn erfahrung_geben(w: &mut Welt, sid: u64, ep: u64) {
    let k = w.d.kampf.clone();
    let Some(s) = w.spieler.get(&sid) else { return };
    let mut stufe = s.sp.stufe;
    let mut erfahrung = s.sp.erfahrung + ep;
    let mut aufstiege = 0;
    while erfahrung >= ep_fuer_naechste(w, stufe) {
        erfahrung -= ep_fuer_naechste(w, stufe);
        stufe += 1;
        aufstiege += 1;
    }
    let Some(s) = w.spieler.get_mut(&sid) else { return };
    s.sp.erfahrung = erfahrung;
    if aufstiege == 0 {
        return;
    }
    s.sp.stufe = stufe;
    s.sp.max_lebenspunkte += k.lp_pro_stufe * aufstiege;
    s.sp.max_mana += k.mp_pro_stufe * aufstiege;
    s.sp.max_ausdauer += k.ausdauer_pro_stufe * aufstiege;
    s.sp.freie_punkte += k.punkte_pro_stufe * aufstiege as u32;
    s.sp.lebenspunkte = s.sp.max_lebenspunkte;
    s.sp.mana = s.sp.max_mana;
    s.sp.ausdauer = s.sp.max_ausdauer;
    let name = s.sp.name.clone();
    let punkte = s.sp.freie_punkte;
    let sp = s.sp.clone();
    w.speichern(&sp);
    w.sende(
        sid,
        &format!(
            "\n{{fett}}{{hellgelb}}*** Du steigst auf Stufe {stufe} auf! ***{{/}}\n\
             Mehr Lebenspunkte, Mana und Ausdauer – und du bist vollständig geheilt.\n\
             Du hast {punkte} Punkte zu verteilen: {{gelb}}steigere <wert>{{/}}, z.B. steigere staerke"
        ),
    );
    w.sende_ankuendigung(&format!("{{grau}}[{name} hat Stufe {stufe} erreicht!]{{/}}"), &[sid]);
}

pub(crate) fn spieler_stirbt(w: &mut Welt, sid: u64) {
    kampf_beenden(w, sid);
    let k = w.d.kampf.clone();
    let jetzt = Instant::now();
    let Some(s) = w.spieler.get_mut(&sid) else { return };
    s.sp.lebenspunkte = 0;
    s.sp.ist_tot = true;
    s.sp.tode += 1;
    s.tot_bis = Some(jetzt + Duration::from_secs(k.tod_wartezeit));
    s.ausweichen_bis = None;
    // Verluste nur, wenn sie eingestellt sind
    if k.tod_ep_verlust_prozent > 0 {
        s.sp.erfahrung -= s.sp.erfahrung * k.tod_ep_verlust_prozent.min(100) as u64 / 100;
    }
    let fallen: Vec<String> =
        if k.tod_gegenstaende == "fallen_lassen" {
            let mut alles: Vec<String> = s.sp.inventar.drain(..).collect();
            alles.extend(crate::behaelter::alles_ausleeren(&mut s.sp));
            alles
        } else {
            Vec::new()
        };
    let (name, raum) = (s.sp.name.clone(), s.sp.raum.clone());
    let ort = w.d.raum(&raum).map(|r| r.name.clone()).unwrap_or_default();
    if !fallen.is_empty() {
        w.boden.entry(raum.clone()).or_default().extend(fallen);
    }
    let minuten = (k.tod_wartezeit as f64 / 60.0).ceil() as u64;
    w.sende(
        sid,
        &format!(
            "\n{{fett}}{{rot}}*** Du bist gestorben. ***{{/}}\n\
             Alles wird dunkel und still ...\n\n\
             Andere Spieler können dich in den nächsten {minuten} Minuten zurückholen\n\
             ({{gelb}}wiederbeleben {name}{{/}}). Danach erwachst du im Tempel.\n\
             Mit {{gelb}}aufgeben{{/}} erwachst du sofort dort."
        ),
    );
    w.sende_raum(&raum, &format!("{{rot}}{name} bricht leblos zusammen!{{/}}"), &[sid]);
    w.sende_ankuendigung(
        &format!("{{hellrot}}[{name} ist gestorben ({ort}) und braucht Hilfe: wiederbeleben {name}]{{/}}"),
        &[sid],
    );
    let sp = w.spieler[&sid].sp.clone();
    w.speichern(&sp);
}

/// Wieder lebendig: am Ort (Hilfe) oder im Tempel.
fn erwecken(w: &mut Welt, sid: u64, im_tempel: bool) {
    let k = w.d.kampf.clone();
    let ziel = w.wiederbelebungsraum();
    let Some(s) = w.spieler.get_mut(&sid) else { return };
    s.tot_bis = None;
    s.sp.ist_tot = false;
    s.sp.lebenspunkte = (s.sp.max_lebenspunkte * k.wiederbelebung_lp_prozent / 100).max(1);
    s.sp.ausdauer = s.sp.max_ausdauer;
    s.bereit_ab = Instant::now();
    let name = s.sp.name.clone();
    if im_tempel {
        let alt = std::mem::replace(&mut s.sp.raum, ziel.clone());
        s.sp.besucht.insert(ziel.clone());
        w.sende_raum(&alt, &format!("Der leblose Körper von {name} löst sich in Licht auf."), &[sid]);
        w.sende(sid, "{hellcyan}Ein warmes Licht umfängt dich. Du schlägst die Augen auf ...{/}\n");
        w.sende_raum(&ziel, &format!("{name} erscheint in einem Lichtschein, bleich und zitternd."), &[sid]);
        befehle::raum_anzeigen(w, sid);
        crate::skripte::raumwechsel(w, sid, &alt, &ziel);
        befehle::prompt(w, sid);
    }
    let sp = w.spieler[&sid].sp.clone();
    w.speichern(&sp);
}

// ---------------------------------------------------------------------------
// Der Kampftakt (mehrmals pro Sekunde)
// ---------------------------------------------------------------------------

pub fn takt(w: &mut Welt) {
    // Fällige spaeter(...)-Funktionen der Skripte
    crate::skripte::timer(w);
    let jetzt = Instant::now();
    let mut beteiligt: BTreeSet<u64> = BTreeSet::new();

    // 0. Wirkungen mit Dauer (Schilde, Gift, Heilung über Zeit ...)
    beteiligt.extend(crate::wirkung::takt(w));

    // 1. Aufräumen: Wer nicht mehr im selben Raum ist oder tot ist, kämpft nicht mehr.
    let orte: std::collections::HashMap<u64, (String, bool)> =
        w.spieler.iter().map(|(id, s)| (*id, (s.sp.raum.clone(), s.tot_bis.is_some()))).collect();
    for m in w.monster.values_mut() {
        m.ziele.retain(|sid| orte.get(sid).is_some_and(|(raum, tot)| *raum == m.raum && !tot));
    }
    let monster_orte: std::collections::HashMap<u64, String> =
        w.monster.iter().map(|(id, m)| (*id, m.raum.clone())).collect();
    for s in w.spieler.values_mut() {
        if let Some(mid) = s.kampf_ziel {
            if monster_orte.get(&mid) != Some(&s.sp.raum) {
                s.kampf_ziel = None;
            }
        }
    }
    // Wer kein Ziel (mehr) hat, aber angegriffen wird, wehrt sich gegen den Angreifer
    let angreifer_von: Vec<(u64, u64)> = w
        .spieler
        .iter()
        .filter(|(_, s)| s.kampf_ziel.is_none() && s.tot_bis.is_none())
        .filter_map(|(sid, _)| w.monster.iter().find(|(_, m)| m.ziele.contains(sid)).map(|(mid, _)| (*sid, *mid)))
        .collect();
    for (sid, mid) in angreifer_von {
        if let Some(s) = w.spieler.get_mut(&sid) {
            s.kampf_ziel = Some(mid);
        }
    }

    // 2. Tote, deren Wartezeit abgelaufen ist, erwachen im Tempel.
    let erwachen: Vec<u64> =
        w.spieler.iter().filter(|(_, s)| s.tot_bis.is_some_and(|b| b <= jetzt)).map(|(id, _)| *id).collect();
    for sid in erwachen {
        w.sende(sid, "Niemand ist gekommen, um dir zu helfen.");
        erwecken(w, sid, true);
    }

    // 3. Aggressive Monster stürzen sich auf Spieler im Raum.
    let mut neue: Vec<(u64, u64)> = Vec::new();
    for (mid, m) in &w.monster {
        if !m.ziele.is_empty() {
            continue;
        }
        let Some(def) = w.d.monster.get(&m.def) else { continue };
        if !def.aggressiv || !def.ist_angreifbar() {
            continue;
        }
        if let Some((sid, _)) = w.spieler.iter().find(|(_, s)| s.sp.raum == m.raum && s.tot_bis.is_none()) {
            neue.push((*mid, *sid));
        }
    }
    for (mid, sid) in neue {
        let name = gross(&w.monster_name(mid));
        verwickeln(w, sid, mid);
        w.sende(sid, &format!("{{hellrot}}{name} greift dich an!{{/}}"));
        beteiligt.insert(sid);
    }

    // 4. Spieler, die bereit sind, schlagen automatisch zu.
    let auto = w.d.kampf.auto_angriff;
    let angreifer: Vec<u64> = w
        .spieler
        .iter()
        .filter(|(_, s)| {
            auto && s.sp.einstellungen.autoangriff && s.kampf_ziel.is_some() && s.bereit_ab <= jetzt && s.tot_bis.is_none()
        })
        .map(|(id, _)| *id)
        .collect();
    for sid in angreifer {
        if crate::bereitschaft::auto_angriff(w, sid) {
            beteiligt.insert(sid);
        }
    }

    // 5. Monster, die bereit sind, schlagen zu.
    let bereit: Vec<u64> =
        w.monster.iter().filter(|(_, m)| !m.ziele.is_empty() && m.bereit_ab <= jetzt).map(|(id, _)| *id).collect();
    for mid in bereit {
        if let Some(&sid) = w.monster.get(&mid).and_then(|m| m.ziele.first()) {
            monster_schlaegt(w, mid);
            beteiligt.insert(sid);
        }
    }

    for sid in beteiligt {
        if w.spieler.contains_key(&sid) && !ist_tot(w, sid) {
            befehle::prompt(w, sid);
        }
    }
}
