//! Alle Befehle, die Spieler eintippen können.

use crate::daten::{Daten, Richtung};
use crate::karte;
use crate::text::{self, aufzaehlen, gross, normalisieren, umbrechen};
use crate::welt::Welt;
use rand::Rng;
use std::time::{Duration, Instant};

pub enum Folge {
    Weiter,
    Beenden,
}

/// Zeilenbreite des Spielers (Einstellung im Spielerbuch)
fn breite(w: &Welt, sid: u64) -> usize {
    w.spieler.get(&sid).map(|s| s.sp.einstellungen.breite).unwrap_or(78)
}

pub fn ausfuehren(w: &mut Welt, sid: u64, eingabe: &str) -> Folge {
    if !w.spieler.contains_key(&sid) {
        return Folge::Beenden;
    }
    let eingabe = eingabe.trim();
    let (befehl, rest) = if let Some(r) = eingabe.strip_prefix('\'') {
        ("sage".to_string(), r.trim())
    } else if let Some(r) = eingabe.strip_prefix(':') {
        ("emote".to_string(), r.trim())
    } else {
        match eingabe.split_once(char::is_whitespace) {
            Some((b, r)) => (normalisieren(b), r.trim()),
            None => (normalisieren(eingabe), ""),
        }
    };
    if befehl.is_empty() {
        prompt(w, sid);
        return Folge::Weiter;
    }
    let admin = w.spieler[&sid].admin;

    match befehl.as_str() {
        "ende" | "quit" | "exit" | "tschuess" => {
            w.sende(sid, "Bis bald! Deine Figur wurde gespeichert.");
            return Folge::Beenden;
        }
        "hilfe" | "help" | "?" => hilfe(w, sid, admin),
        "schau" | "schaue" | "l" | "look" | "umsehen" | "betrachte" | "untersuche" | "unt" | "x" => {
            if rest.is_empty() {
                raum_anzeigen(w, sid);
            } else {
                untersuchen(w, sid, rest);
            }
        }
        "gehe" | "geh" | "go" => match Richtung::aus_text(rest) {
            Some(r) => gehen(w, sid, r),
            None => w.sende(sid, "Wohin willst du gehen? (z.B. gehe norden)"),
        },
        "karte" | "map" | "k" => {
            let radius = rest.parse().unwrap_or(w.d.welt.karten_radius + 2);
            let k = karte::zeichnen(w, &w.spieler[&sid].sp, radius, true);
            w.sende(sid, &k);
        }
        "weltkarte" | "landkarte" | "worldmap" => weltkarte(w, sid, admin),
        "nimm" | "nehme" | "take" | "get" => nehmen(w, sid, rest),
        "lege" | "leg" | "ablegen" | "drop" | "fallenlassen" => fallenlassen(w, sid, rest),
        "inventar" | "i" | "inv" => inventar(w, sid),
        "anlegen" | "anziehen" | "ausruesten" | "zuecke" | "wear" | "wield" => anlegen(w, sid, rest),
        "ausziehen" | "abnehmen" | "remove" => ausziehen(w, sid, rest),
        "benutze" | "benutzen" | "trinke" | "trink" | "iss" | "esse" | "use" | "drink" | "eat" => {
            benutzen(w, sid, rest)
        }
        "sage" | "sag" | "say" => sagen(w, sid, rest),
        "emote" | "me" | "/me" => emote(w, sid, rest),
        "rufe" | "ruf" | "shout" => rufen(w, sid, rest),
        "wer" | "who" => wer(w, sid),
        "spielstand" | "sp" | "werte" | "score" | "status" => spielstand(w, sid),
        "spielerbuch" | "buch" | "sb" | "einstellungen" | "optionen" => spielerbuch(w, sid, rest),
        "lies" | "lese" | "read" => match normalisieren(rest).as_str() {
            "spielerbuch" | "buch" | "" => spielerbuch(w, sid, ""),
            _ => w.sende(sid, "Was willst du lesen?"),
        },
        "fertigkeiten" | "fert" | "skills" => fertigkeiten(w, sid),
        "zauber" | "spells" | "zaubersprueche" => zauber_liste(w, sid),
        "zaubere" | "zauber_sprechen" | "cast" | "z" => zaubern(w, sid, rest),
        "farbe" | "farben" | "color" => {
            let an = !matches!(normalisieren(rest).as_str(), "aus" | "off" | "nein");
            if let Some(s) = w.spieler.get_mut(&sid) {
                s.sp.farben = an;
            }
            w.sende(sid, if an { "{gruen}Farben sind an.{/}" } else { "Farben sind aus." });
        }
        "speichern" | "save" => {
            let s = &w.spieler[&sid];
            w.speichern(&s.sp);
            w.sende(sid, "Gespeichert.");
        }
        _ => {
            if let Some(r) = Richtung::aus_text(&befehl) {
                gehen(w, sid, r);
            } else if !(admin && admin_befehl(w, sid, &befehl, rest)) {
                w.sende(sid, "Wie bitte? Tippe {gelb}hilfe{/} für eine Liste der Befehle.");
            }
        }
    }
    prompt(w, sid);
    Folge::Weiter
}

pub fn prompt(w: &Welt, sid: u64) {
    if let Some(s) = w.spieler.get(&sid) {
        let p = &s.sp;
        if !p.einstellungen.prompt {
            s.sende_ohne_umbruch("\n> ");
            return;
        }
        s.sende_ohne_umbruch(&format!(
            "\n{{gruen}}LP {}/{}{{/}} {{cyan}}MP {}/{}{{/}} > ",
            p.lebenspunkte, p.max_lebenspunkte, p.mana, p.max_mana
        ));
    }
}

/// Die ganze Oberwelt. Dafür braucht man eine Landkarte (Gegenstand mit
/// der Eigenschaft `landkarte`) – Baumeister brauchen keine.
fn weltkarte(w: &Welt, sid: u64, admin: bool) {
    let Some(ow) = &w.d.oberwelt else {
        w.sende(sid, "Diese Welt hat keine Oberwelt.");
        return;
    };
    let sp = &w.spieler[&sid].sp;
    let hat_karte = sp
        .inventar
        .iter()
        .chain(sp.ausruestung.values())
        .any(|g| w.d.gegenstaende.get(g).is_some_and(|g| g.eigenschaften.get("landkarte").copied().unwrap_or(0) > 0));
    if !hat_karte && !admin {
        w.sende(sid, "Du hast keine Landkarte dabei.");
        return;
    }
    // Wo ist man? Auf der Oberwelt direkt, sonst am Eingang des Ortes, in dem man ist.
    let pos = crate::oberwelt::aus_id(&sp.raum).or_else(|| {
        let gebiet = w.d.gebiet(&sp.raum);
        ow.eingaenge
            .iter()
            .find(|e| e.raum == sp.raum || (!gebiet.is_empty() && w.d.gebiet(&e.raum) == gebiet))
            .map(|e| (e.x, e.y))
    });
    let (x, y) = pos.unwrap_or((-1, -1));
    w.sende(sid, &ow.weltkarte(x, y));
}

fn hilfe(w: &Welt, sid: u64, admin: bool) {
    let mut t = String::from(
        "{fett}Befehle{/}
  {gelb}schau{/} [etwas]        Umsehen oder etwas genauer ansehen (kurz: l)
  {gelb}n o s w no nw so sw{/}  In eine Richtung gehen; {gelb}h{/}och / {gelb}r{/}unter
  {gelb}karte{/} [größe]        Die Karte der Umgebung zeigen
  {gelb}weltkarte{/}            Die ganze Oberwelt (mit einer Landkarte)
  {gelb}rein{/} / {gelb}raus{/}           Eine Stadt betreten oder verlassen
  {gelb}nimm{/} <ding|alles>    Etwas aufheben
  {gelb}lege{/} <ding|alles>    Etwas fallen lassen
  {gelb}inventar{/}             Zeigen, was du bei dir hast (kurz: i)
  {gelb}anlegen{/} <ding>       Waffe oder Rüstung anlegen
  {gelb}ausziehen{/} <ding>     Ausrüstung wieder abnehmen
  {gelb}benutze{/} <ding>       Trank trinken, Essen essen ...
  {gelb}sage{/} <text>          Etwas sagen (kurz: 'text)
  {gelb}emote{/} <text>         Eine Handlung zeigen (kurz: :text)
  {gelb}rufe{/} <text>          Etwas rufen, das alle hören
  {gelb}wer{/}                  Wer ist gerade da?
  {gelb}spielstand{/} (sp)      Deine Werte, Ausrüstung und Fortschritte
  {gelb}spielerbuch{/}          Einstellungen: Karte, Farben, Titel, Passwort ...
  {gelb}fertigkeiten{/}         Deine Fertigkeiten
  {gelb}zauber{/}               Deine Zaubersprüche
  {gelb}zaubere{/} <zauber> [ziel]  Einen Zauber sprechen
  {gelb}farbe{/} an|aus         Farben ein- oder ausschalten
  {gelb}speichern{/}, {gelb}ende{/}",
    );
    if admin {
        t.push_str(
            "

{fett}{rot}Baumeister-Befehle{/}
  {gelb}neuladen{/}                 Alle Weltdaten neu laden (ohne Neustart)
  {gelb}pruefen{/}                  Weltdaten prüfen, ohne sie zu laden
  {gelb}wo{/}                       ID, Koordinaten und Datei des Raums
  {gelb}springe{/} <raum-id>        Zu einem Raum springen
  {gelb}erschaffe{/} <gegenstand-id> Einen Gegenstand erschaffen
  {gelb}beschwoere{/} <monster-id>  Ein Monster herbeirufen
  {gelb}lehre{/} <spieler> <fertigkeit-/zauber-id> [stufe]
  {gelb}liste{/} raeume|gegenstaende|monster|fertigkeiten|zauber|gelaende
  {gelb}zuruecksetzen{/}            Gegenstände und Monster sofort auffüllen",
        );
    }
    w.sende(sid, &t);
}

// ---------------------------------------------------------------------------
// Suchen von Dingen anhand dessen, was der Spieler tippt
// ---------------------------------------------------------------------------

/// Passt ein getipptes Wort zu einem Ding?
fn passt(such: &str, name: &str, id: &str, schluessel: &[String]) -> bool {
    let s = normalisieren(such);
    if s.is_empty() {
        return false;
    }
    let n = normalisieren(&text::farben_umwandeln(name, false));
    n == s
        || normalisieren(id) == s
        || schluessel.iter().any(|k| normalisieren(k) == s)
        || n.split(|c: char| !c.is_alphanumeric()).any(|wort| wort == s)
}

fn finde_gegenstand(d: &Daten, liste: &[String], such: &str) -> Option<usize> {
    liste.iter().position(|id| {
        d.gegenstaende.get(id).is_some_and(|g| passt(such, &g.name, &g.id, &g.schluesselwoerter))
    })
}

fn finde_monster(w: &Welt, raum: &str, such: &str) -> Option<u64> {
    w.monster_im_raum(raum).into_iter().find(|id| {
        w.monster
            .get(id)
            .and_then(|m| w.d.monster.get(&m.def))
            .is_some_and(|d| passt(such, &d.name, &d.id, &d.schluesselwoerter))
    })
}

fn finde_spieler_im_raum(w: &Welt, raum: &str, such: &str) -> Option<u64> {
    let s = normalisieren(such);
    w.spieler_im_raum(raum)
        .into_iter()
        .find(|id| normalisieren(&w.spieler[id].sp.name).starts_with(&s))
}

/// Fasst gleiche Gegenstände zusammen: [(id, anzahl)]
fn gruppieren(ids: &[String]) -> Vec<(String, usize)> {
    let mut aus: Vec<(String, usize)> = Vec::new();
    for id in ids {
        match aus.iter_mut().find(|(i, _)| i == id) {
            Some((_, n)) => *n += 1,
            None => aus.push((id.clone(), 1)),
        }
    }
    aus
}

// ---------------------------------------------------------------------------
// Umsehen und Bewegen
// ---------------------------------------------------------------------------

/// Raum mit Beschreibung zeigen (beim Umsehen).
pub fn raum_anzeigen(w: &Welt, sid: u64) {
    raum_zeigen(w, sid, true);
}

/// `voll = false` beim Gehen: Mit der Einstellung "kurz" fehlt dann die Beschreibung.
fn raum_zeigen(w: &Welt, sid: u64, voll: bool) {
    let Some(s) = w.spieler.get(&sid) else { return };
    let Some(raum) = w.d.raum(&s.sp.raum) else { return };
    let e = &s.sp.einstellungen;
    let mut aus = String::new();

    if w.d.welt.karte_beim_umsehen && e.karte {
        let radius = (w.d.welt.karten_radius + e.kartengroesse).max(1);
        aus.push_str(&karte::zeichnen(w, &s.sp, radius, false));
        aus.push('\n');
    }
    aus.push_str(&format!("{{fett}}{{gelb}}{}{{/}}", raum.name));
    if crate::oberwelt::ist_oberwelt(&raum.id) {
        aus.push_str(&format!(" {{grau}}({}, {}){{/}}", raum.x, raum.y));
    } else if s.admin {
        aus.push_str(&format!(" {{grau}}[{}]{{/}}", raum.id));
    }
    aus.push('\n');
    if voll || !e.kurz {
        aus.push_str(&umbrechen(&raum.beschreibung, e.breite));
        aus.push('\n');
    }

    let mut ausgaenge: Vec<String> = raum.ausgaenge.keys().map(|r| r.anzeige().to_string()).collect();
    let himmelsrichtungen = raum.ausgaenge.keys().filter(|r| r.versatz().2 == 0 && r.versatz() != (0, 0, 0)).count();
    if himmelsrichtungen == 8 {
        ausgaenge.retain(|a| a == "hinein" || a == "hinaus");
        ausgaenge.insert(0, "in alle Richtungen".into());
    }
    if ausgaenge.is_empty() {
        aus.push_str("{cyan}Es gibt keine sichtbaren Ausgänge.{/}\n");
    } else {
        aus.push_str(&format!("{{cyan}}Ausgänge: {}{{/}}\n", aufzaehlen(&ausgaenge)));
    }

    let boden = w.boden.get(&raum.id).cloned().unwrap_or_default();
    for (id, anzahl) in gruppieren(&boden) {
        let Some(g) = w.d.gegenstaende.get(&id) else { continue };
        let t = if g.bodentext.is_empty() { format!("{} liegt hier.", gross(&g.name)) } else { g.bodentext.clone() };
        if anzahl > 1 {
            aus.push_str(&format!("[{anzahl}x] {t}\n"));
        } else {
            aus.push_str(&format!("{t}\n"));
        }
    }
    for mid in w.monster_im_raum(&raum.id) {
        let Some(def) = w.monster.get(&mid).and_then(|m| w.d.monster.get(&m.def)) else { continue };
        let t = if def.raumtext.is_empty() { format!("{} ist hier.", gross(&def.name)) } else { def.raumtext.clone() };
        aus.push_str(&format!("{{hellrot}}{t}{{/}}\n"));
    }
    for pid in w.spieler_im_raum(&raum.id) {
        if pid != sid {
            aus.push_str(&format!("{{hellgruen}}{} ist hier.{{/}}\n", w.spieler[&pid].sp.voller_name()));
        }
    }
    w.sende(sid, aus.trim_end());
}

fn gehen(w: &mut Welt, sid: u64, richt: Richtung) {
    let s = &w.spieler[&sid];
    let alt = s.sp.raum.clone();
    let name = s.sp.name.clone();
    let Some(ziel) = w.d.ausgaenge(&alt).get(&richt).cloned() else {
        let t = match richt {
            Richtung::Rein => "Hier gibt es nichts, das du betreten könntest.",
            Richtung::Raus => "Von hier aus kommst du nicht hinaus.",
            _ => "Dorthin kannst du nicht gehen.",
        };
        w.sende(sid, t);
        return;
    };
    w.sende_raum(&alt, &format!("{name} {}.", richt.geht()), &[sid]);
    if let Some(s) = w.spieler.get_mut(&sid) {
        s.sp.raum = ziel.clone();
        if !crate::oberwelt::ist_oberwelt(&ziel) {
            s.sp.besucht.insert(ziel.clone());
        }
    }
    w.sende_raum(&ziel, &format!("{name} {}.", richt.kommt()), &[sid]);
    raum_zeigen(w, sid, false);
}

fn untersuchen(w: &Welt, sid: u64, such: &str) {
    let s = &w.spieler[&sid];
    let raum_id = &s.sp.raum;
    let such = such.strip_prefix("an ").unwrap_or(such);
    let br = breite(w, sid);

    // 1. Details im Raum (z.B. "brunnen")
    if let Some(r) = w.d.raum(raum_id) {
        if let Some(t) = r.details.get(&normalisieren(such)) {
            w.sende(sid, &umbrechen(t, br));
            return;
        }
    }
    // 2. Gegenstände bei sich, angelegt oder am Boden
    let angelegt: Vec<String> = s.sp.ausruestung.values().cloned().collect();
    let boden = w.boden.get(raum_id).cloned().unwrap_or_default();
    for liste in [&s.sp.inventar, &angelegt, &boden] {
        if let Some(i) = finde_gegenstand(&w.d, liste, such) {
            let g = &w.d.gegenstaende[&liste[i]];
            let mut t = format!("{{fett}}{}{{/}}\n", gross(&g.name));
            t.push_str(&umbrechen(if g.beschreibung.is_empty() { "Nichts Besonderes." } else { &g.beschreibung }, br));
            if !g.platz.is_empty() {
                t.push_str(&format!("\n{{grau}}(Kann angelegt werden: {}){{/}}", g.platz));
            }
            if s.admin {
                t.push_str(&format!(
                    "\n{{grau}}[id: {}, typ: {}, gewicht: {}, wert: {}, eigenschaften: {:?}, datei: {}]{{/}}",
                    g.id, g.typ, g.gewicht, g.wert, g.eigenschaften, g.quelle
                ));
            }
            w.sende(sid, &t);
            return;
        }
    }
    // 3. Monster
    if let Some(mid) = finde_monster(w, raum_id, such) {
        let m = &w.monster[&mid];
        if let Some(def) = w.d.monster.get(&m.def) {
            let mut t = format!("{{fett}}{}{{/}}\n", gross(&def.name));
            t.push_str(&umbrechen(if def.beschreibung.is_empty() { "Nichts Besonderes." } else { &def.beschreibung }, br));
            if s.admin {
                t.push_str(&format!(
                    "\n{{grau}}[id: {}, stufe: {}, LP: {}/{}, MP: {}, heimat: {}, datei: {}]{{/}}",
                    def.id, def.stufe, m.lebenspunkte, def.lebenspunkte, m.mana, m.heimat, def.quelle
                ));
            }
            w.sende(sid, &t);
            return;
        }
    }
    // 4. Andere Spieler
    if let Some(pid) = finde_spieler_im_raum(w, raum_id, such) {
        let o = &w.spieler[&pid].sp;
        let teile: Vec<String> = o.ausruestung.values().map(|g| w.d.gegenstand_akk(g)).collect();
        let mut t = format!("{{fett}}{}{{/}}\n", o.voller_name());
        if !o.beschreibung.is_empty() {
            t.push_str(&umbrechen(&o.beschreibung, br));
            t.push('\n');
        }
        if teile.is_empty() {
            t.push_str(&format!("{} trägt nichts Besonderes.", o.name));
        } else {
            t.push_str(&format!("{} trägt {}.", o.name, aufzaehlen(&teile)));
        }
        w.sende(sid, &t);
        return;
    }
    w.sende(sid, "So etwas siehst du hier nicht.");
}

// ---------------------------------------------------------------------------
// Gegenstände
// ---------------------------------------------------------------------------

fn nehmen(w: &mut Welt, sid: u64, such: &str) {
    if such.is_empty() {
        w.sende(sid, "Was willst du nehmen?");
        return;
    }
    let raum = w.spieler[&sid].sp.raum.clone();
    let name = w.spieler[&sid].sp.name.clone();
    let boden = w.boden.entry(raum.clone()).or_default();

    let mut genommen = Vec::new();
    if normalisieren(such) == "alles" {
        let d = &w.d;
        let (nehmbar, bleibt): (Vec<String>, Vec<String>) =
            boden.drain(..).partition(|id| d.gegenstaende.get(id).is_some_and(|g| g.tragbar));
        *boden = bleibt;
        genommen = nehmbar;
    } else if let Some(i) = finde_gegenstand(&w.d, boden, such) {
        if w.d.gegenstaende.get(&boden[i]).is_some_and(|g| !g.tragbar) {
            w.sende(sid, "Das kannst du nicht mitnehmen.");
            return;
        }
        genommen.push(boden.remove(i));
    }
    if genommen.is_empty() {
        w.sende(sid, "Hier gibt es nichts dergleichen zum Mitnehmen.");
        return;
    }
    for id in &genommen {
        let n = w.d.gegenstand_akk(id);
        w.sende(sid, &format!("Du nimmst {n}."));
        w.sende_raum(&raum, &format!("{name} nimmt {n}."), &[sid]);
    }
    if let Some(s) = w.spieler.get_mut(&sid) {
        s.sp.inventar.extend(genommen);
    }
}

fn fallenlassen(w: &mut Welt, sid: u64, such: &str) {
    if such.is_empty() {
        w.sende(sid, "Was willst du hinlegen?");
        return;
    }
    let Some(s) = w.spieler.get_mut(&sid) else { return };
    let raum = s.sp.raum.clone();
    let name = s.sp.name.clone();
    let weg: Vec<String> = if normalisieren(such) == "alles" {
        s.sp.inventar.drain(..).collect()
    } else {
        match finde_gegenstand(&w.d, &s.sp.inventar, such) {
            Some(i) => vec![s.sp.inventar.remove(i)],
            None => Vec::new(),
        }
    };
    if weg.is_empty() {
        w.sende(sid, "So etwas hast du nicht.");
        return;
    }
    for id in &weg {
        let n = w.d.gegenstand_akk(id);
        w.sende(sid, &format!("Du legst {n} ab."));
        w.sende_raum(&raum, &format!("{name} legt {n} ab."), &[sid]);
    }
    w.boden.entry(raum).or_default().extend(weg);
}

fn inventar(w: &Welt, sid: u64) {
    let sp = &w.spieler[&sid].sp;
    let mut t = String::from("{fett}Du trägst bei dir:{/}\n");
    if sp.inventar.is_empty() {
        t.push_str("  nichts\n");
    }
    let mut gewicht = 0;
    for (id, n) in gruppieren(&sp.inventar) {
        let name = w.d.gegenstand_name(&id);
        gewicht += w.d.gegenstaende.get(&id).map(|g| g.gewicht).unwrap_or(0) * n as i32;
        if n > 1 {
            t.push_str(&format!("  [{n}x] {name}\n"));
        } else {
            t.push_str(&format!("  {name}\n"));
        }
    }
    if !sp.ausruestung.is_empty() {
        t.push_str("{fett}Angelegt:{/}\n");
        for (platz, id) in &sp.ausruestung {
            t.push_str(&format!("  {:<10} {}\n", format!("<{platz}>"), w.d.gegenstand_name(id)));
        }
    }
    t.push_str(&format!("{{grau}}Gewicht: {gewicht}{{/}}"));
    w.sende(sid, &t);
}

fn anlegen(w: &mut Welt, sid: u64, such: &str) {
    let Some(s) = w.spieler.get_mut(&sid) else { return };
    let Some(i) = finde_gegenstand(&w.d, &s.sp.inventar, such) else {
        w.sende(sid, "So etwas hast du nicht dabei.");
        return;
    };
    let id = s.sp.inventar[i].clone();
    let Some(g) = w.d.gegenstaende.get(&id) else { return };
    if g.platz.is_empty() {
        w.sende(sid, "Das kann man nicht anlegen.");
        return;
    }
    if let Some(alt) = s.sp.ausruestung.get(&g.platz) {
        let alt = w.d.gegenstand_akk(alt);
        w.sende(sid, &format!("Du musst erst {alt} ausziehen."));
        return;
    }
    s.sp.inventar.remove(i);
    s.sp.ausruestung.insert(g.platz.clone(), id);
    let (raum, name) = (s.sp.raum.clone(), s.sp.name.clone());
    let n = w.d.gegenstand_akk(&g.id);
    w.sende(sid, &format!("Du legst {n} an."));
    w.sende_raum(&raum, &format!("{name} legt {n} an."), &[sid]);
}

fn ausziehen(w: &mut Welt, sid: u64, such: &str) {
    let Some(s) = w.spieler.get_mut(&sid) else { return };
    let platz = s.sp.ausruestung.iter().find_map(|(p, id)| {
        let g = w.d.gegenstaende.get(id)?;
        (passt(such, &g.name, &g.id, &g.schluesselwoerter) || normalisieren(such) == *p).then(|| p.clone())
    });
    let Some(platz) = platz else {
        w.sende(sid, "So etwas hast du nicht angelegt.");
        return;
    };
    let Some(id) = s.sp.ausruestung.remove(&platz) else { return };
    let n = w.d.gegenstand_akk(&id);
    s.sp.inventar.push(id);
    let (raum, name) = (s.sp.raum.clone(), s.sp.name.clone());
    w.sende(sid, &format!("Du nimmst {n} ab."));
    w.sende_raum(&raum, &format!("{name} nimmt {n} ab."), &[sid]);
}

fn benutzen(w: &mut Welt, sid: u64, such: &str) {
    let Some(s) = w.spieler.get_mut(&sid) else { return };
    let Some(i) = finde_gegenstand(&w.d, &s.sp.inventar, such) else {
        w.sende(sid, "So etwas hast du nicht dabei.");
        return;
    };
    let Some(g) = w.d.gegenstaende.get(&s.sp.inventar[i]) else { return };
    let heilung = g.eigenschaften.get("heilung").copied().unwrap_or(0);
    let mana = g.eigenschaften.get("mana").copied().unwrap_or(0);
    if heilung == 0 && mana == 0 {
        w.sende(sid, "Damit kannst du nichts anfangen.");
        return;
    }
    s.sp.inventar.remove(i);
    let vorher = (s.sp.lebenspunkte, s.sp.mana);
    s.sp.lebenspunkte = (s.sp.lebenspunkte + heilung).min(s.sp.max_lebenspunkte);
    s.sp.mana = (s.sp.mana + mana).min(s.sp.max_mana);
    let plus_lp = s.sp.lebenspunkte - vorher.0;
    let plus_mp = s.sp.mana - vorher.1;
    let (raum, name) = (s.sp.raum.clone(), s.sp.name.clone());

    let akk = if g.akkusativ.is_empty() { g.name.clone() } else { g.akkusativ.clone() };
    let t = if g.benutzen_text.is_empty() { format!("Du benutzt {akk}.") } else { g.benutzen_text.clone() };
    let mut extra = Vec::new();
    if plus_lp > 0 {
        extra.push(format!("+{plus_lp} LP"));
    }
    if plus_mp > 0 {
        extra.push(format!("+{plus_mp} MP"));
    }
    if extra.is_empty() {
        w.sende(sid, &t);
    } else {
        w.sende(sid, &format!("{t} {{gruen}}({}){{/}}", extra.join(", ")));
    }
    w.sende_raum(&raum, &format!("{name} benutzt {akk}."), &[sid]);
}

// ---------------------------------------------------------------------------
// Unterhaltung
// ---------------------------------------------------------------------------

fn sagen(w: &Welt, sid: u64, t: &str) {
    if t.is_empty() {
        w.sende(sid, "Was willst du sagen?");
        return;
    }
    let s = &w.spieler[&sid].sp;
    w.sende(sid, &format!("{{cyan}}Du sagst: „{t}“{{/}}"));
    w.sende_raum(&s.raum, &format!("{{cyan}}{} sagt: „{t}“{{/}}", s.name), &[sid]);
}

fn emote(w: &Welt, sid: u64, t: &str) {
    if t.is_empty() {
        w.sende(sid, "Was willst du tun? (z.B. :lächelt)");
        return;
    }
    let s = &w.spieler[&sid].sp;
    w.sende_raum(&s.raum, &format!("{} {t}", s.name), &[]);
}

fn rufen(w: &Welt, sid: u64, t: &str) {
    if t.is_empty() {
        w.sende(sid, "Was willst du rufen?");
        return;
    }
    let s = &w.spieler[&sid].sp;
    w.sende(sid, &format!("{{hellgelb}}Du rufst: „{t}“{{/}}"));
    w.sende_alle(&format!("{{hellgelb}}{} ruft: „{t}“{{/}}", s.name), &[sid]);
}

fn wer(w: &Welt, sid: u64) {
    let mut t = format!("{{fett}}In {} sind gerade unterwegs:{{/}}\n", w.d.welt.name);
    for s in w.spieler.values() {
        let marke = if s.admin { " {rot}[Baumeister]{/}" } else { "" };
        t.push_str(&format!("  {}{marke}\n", s.sp.voller_name()));
    }
    t.push_str(&format!("{} Spieler", w.spieler.len()));
    w.sende(sid, &t);
}

// ---------------------------------------------------------------------------
// Werte, Fertigkeiten, Zauber
// ---------------------------------------------------------------------------

fn fertigkeiten(w: &Welt, sid: u64) {
    let sp = &w.spieler[&sid].sp;
    if sp.fertigkeiten.is_empty() {
        w.sende(sid, "Du hast noch keine Fertigkeiten gelernt.");
        return;
    }
    let mut nach_kategorie: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for (id, stufe) in &sp.fertigkeiten {
        let Some(f) = w.d.fertigkeiten.get(id) else { continue };
        nach_kategorie
            .entry(f.kategorie.clone())
            .or_default()
            .push(format!("  {:<20} {:>3}/{:<3} {{grau}}{}{{/}}", f.name, stufe, f.max_stufe, f.beschreibung));
    }
    let mut t = String::from("{fett}Deine Fertigkeiten{/}\n");
    for (k, zeilen) in nach_kategorie {
        t.push_str(&format!("{{gelb}}{}{{/}}\n{}\n", gross(&k), zeilen.join("\n")));
    }
    w.sende(sid, t.trim_end());
}

fn zauber_liste(w: &Welt, sid: u64) {
    let sp = &w.spieler[&sid].sp;
    if sp.zauber.is_empty() {
        w.sende(sid, "Du kennst noch keine Zaubersprüche.");
        return;
    }
    let mut t = String::from("{fett}Deine Zaubersprüche{/}\n");
    for id in &sp.zauber {
        let Some(z) = w.d.zauber.get(id) else { continue };
        t.push_str(&format!("  {{magenta}}{:<18}{{/}} {:>3} MP  {{grau}}{}{{/}}\n", z.name, z.mana_kosten, z.beschreibung));
    }
    t.push_str("Benutzung: zaubere <name> [ziel]");
    w.sende(sid, &t);
}

fn zaubern(w: &mut Welt, sid: u64, rest: &str) {
    if rest.is_empty() {
        w.sende(sid, "Was willst du zaubern? ({gelb}zauber{/} zeigt deine Sprüche)");
        return;
    }
    // Welcher Zauber ist gemeint? Der längste passende Name gewinnt.
    let such = normalisieren(rest);
    let mut bester: Option<(String, usize)> = None;
    for zid in &w.spieler[&sid].sp.zauber {
        let Some(z) = w.d.zauber.get(zid) else { continue };
        let mut namen = vec![normalisieren(&z.name), normalisieren(&z.id)];
        namen.extend(z.schluesselwoerter.iter().map(|k| normalisieren(k)));
        for n in namen {
            let trifft = !n.is_empty() && (such == n || such.starts_with(&format!("{n} ")));
            if trifft && bester.as_ref().is_none_or(|(_, l)| n.len() > *l) {
                bester = Some((zid.clone(), n.len()));
            }
        }
    }
    let Some((zid, laenge)) = bester else {
        w.sende(sid, "Einen solchen Zauber kennst du nicht.");
        return;
    };
    let zielwort = such[laenge..].trim().to_string();
    let Some(z) = w.d.zauber.get(&zid).cloned() else { return };

    // Kann der Spieler den Zauber sprechen?
    let jetzt = Instant::now();
    {
        let s = &w.spieler[&sid];
        if !z.fertigkeit.is_empty() {
            let stufe = s.sp.fertigkeiten.get(&z.fertigkeit).copied().unwrap_or(0);
            if stufe < z.mindest_stufe {
                let f = w.d.fertigkeiten.get(&z.fertigkeit).map(|f| f.name.clone()).unwrap_or_default();
                w.sende(sid, &format!("Dafür brauchst du {f} auf Stufe {}.", z.mindest_stufe));
                return;
            }
        }
        if let Some(bis) = s.abklingzeiten.get(&zid) {
            if *bis > jetzt {
                let sek = (*bis - jetzt).as_secs() + 1;
                w.sende(sid, &format!("Du musst noch {sek} Sekunden warten."));
                return;
            }
        }
        if s.sp.mana < z.mana_kosten {
            w.sende(sid, "Dein Mana reicht dafür nicht aus.");
            return;
        }
    }

    let raum = w.spieler[&sid].sp.raum.clone();
    let name = w.spieler[&sid].sp.name.clone();

    // Ziel bestimmen
    let mut ziel_spieler: Vec<u64> = Vec::new();
    let mut ziel_monster: Option<u64> = None;
    match z.ziel.as_str() {
        "spieler" if !zielwort.is_empty() => match finde_spieler_im_raum(w, &raum, &zielwort) {
            Some(p) => ziel_spieler.push(p),
            None => {
                w.sende(sid, "Diese Person ist nicht hier.");
                return;
            }
        },
        "raum" => ziel_spieler = w.spieler_im_raum(&raum),
        "gegner" => {
            if zielwort.is_empty() {
                w.sende(sid, "Auf wen willst du den Zauber sprechen?");
                return;
            }
            match finde_monster(w, &raum, &zielwort) {
                Some(m) => ziel_monster = Some(m),
                None => {
                    w.sende(sid, "Das ist nicht hier.");
                    return;
                }
            }
        }
        _ => ziel_spieler.push(sid),
    }
    let zielname = match (ziel_monster, ziel_spieler.as_slice()) {
        (Some(m), _) => w.monster_akk(m),
        (None, [p]) if *p == sid => "dich".to_string(),
        (None, [p]) => w.spieler[p].sp.name.clone(),
        _ => "alle".to_string(),
    };
    let ersetzen = |t: &str| t.replace("{name}", &name).replace("{ziel}", &zielname).replace("{zauber}", &z.name);

    // Bezahlen
    if let Some(s) = w.spieler.get_mut(&sid) {
        s.sp.mana -= z.mana_kosten;
        if z.abklingzeit > 0 {
            s.abklingzeiten.insert(zid.clone(), jetzt + Duration::from_secs(z.abklingzeit));
        }
    }

    // Texte
    let selbst = if z.text_selbst.is_empty() { "Du sprichst {zauber}." } else { &z.text_selbst };
    let im_raum = if z.text_raum.is_empty() { "{name} spricht {zauber}." } else { &z.text_raum };
    let am_ziel = if z.text_ziel.is_empty() { "{name} spricht {zauber} auf dich." } else { &z.text_ziel };
    w.sende(sid, &format!("{{magenta}}{}{{/}}", ersetzen(selbst)));
    let mut ausser = vec![sid];
    for p in &ziel_spieler {
        if *p != sid {
            w.sende(*p, &format!("{{magenta}}{}{{/}}", ersetzen(am_ziel)));
            ausser.push(*p);
        }
    }
    w.sende_raum(&raum, &format!("{{magenta}}{}{{/}}", ersetzen(im_raum)), &ausser);

    // Wirkungen
    if ziel_monster.is_some() {
        if z.wirkungen.iter().any(|w| w.art == "schaden") {
            w.sende(sid, "{grau}(Das Kampfsystem ist noch nicht eingebaut – der Schaden wird später verrechnet.){/}");
        }
    } else {
        for p in ziel_spieler {
            for wirkung in &z.wirkungen {
                wirkung_anwenden(w, p, &wirkung.art, wirkung.wert, &wirkung.raum);
            }
        }
    }

    // Übung macht den Meister
    if let Some(f) = w.d.fertigkeiten.get(&z.fertigkeit).cloned() {
        let mut rng = rand::thread_rng();
        if let Some(s) = w.spieler.get_mut(&sid) {
            let stufe = s.sp.fertigkeiten.entry(f.id.clone()).or_insert(0);
            if *stufe < f.max_stufe && rng.gen_range(0..100) < f.lernchance {
                *stufe += 1;
                let neu = *stufe;
                w.sende(sid, &format!("{{hellgruen}}Deine Fertigkeit {} steigt auf {neu}!{{/}}", f.name));
            }
        }
    }
}

fn wirkung_anwenden(w: &mut Welt, sid: u64, art: &str, wert: i32, raum: &str) {
    let Some(s) = w.spieler.get_mut(&sid) else { return };
    match art {
        "heilen" => {
            let vorher = s.sp.lebenspunkte;
            s.sp.lebenspunkte = (vorher + wert).min(s.sp.max_lebenspunkte);
            let plus = s.sp.lebenspunkte - vorher;
            if plus > 0 {
                s.sende(&format!("{{gruen}}Du fühlst dich besser. (+{plus} LP){{/}}"));
            } else {
                s.sende("{gruen}Du bist bereits völlig unverletzt.{/}");
            }
        }
        "mana" => {
            let vorher = s.sp.mana;
            s.sp.mana = (vorher + wert).min(s.sp.max_mana);
            let plus = s.sp.mana - vorher;
            if plus > 0 {
                s.sende(&format!("{{cyan}}Magische Kraft durchströmt dich. (+{plus} MP){{/}}"));
            } else {
                s.sende("{cyan}Dein Mana ist bereits voll.{/}");
            }
        }
        "teleport" => {
            let alt = std::mem::replace(&mut s.sp.raum, raum.to_string());
            if !crate::oberwelt::ist_oberwelt(raum) {
                s.sp.besucht.insert(raum.to_string());
            }
            let name = s.sp.name.clone();
            w.sende_raum(&alt, &format!("{name} verschwindet in einem Lichtblitz."), &[sid]);
            w.sende_raum(raum, &format!("{name} erscheint in einem Lichtblitz."), &[sid]);
            raum_anzeigen(w, sid);
        }
        "schaden" => {
            s.sende("{grau}(Schaden gegen Spieler gibt es erst mit dem Kampfsystem.){/}");
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Baumeister-Befehle
// ---------------------------------------------------------------------------

fn admin_befehl(w: &mut Welt, sid: u64, befehl: &str, rest: &str) -> bool {
    match befehl {
        "neuladen" | "reload" => match w.neuladen() {
            Ok(warnungen) => {
                let mut t = format!(
                    "{{gruen}}Welt neu geladen:{{/}} {} Räume, {} Gegenstände, {} Monster, {} Fertigkeiten, {} Zauber.",
                    w.d.raeume.len(),
                    w.d.gegenstaende.len(),
                    w.d.monster.len(),
                    w.d.fertigkeiten.len(),
                    w.d.zauber.len()
                );
                for warn in warnungen {
                    t.push_str(&format!("\n{{gelb}}Warnung:{{/}} {warn}"));
                }
                w.sende(sid, &t);
                w.sende_alle("{grau}[Die Welt verändert sich ...]{/}", &[sid]);
                for id in w.spieler.keys().copied().collect::<Vec<_>>() {
                    raum_anzeigen(w, id);
                }
            }
            Err(fehler) => {
                let mut t = String::from("{rot}Neuladen abgebrochen, die alte Welt bleibt aktiv. Fehler:{/}");
                for f in fehler {
                    t.push_str(&format!("\n- {f}"));
                }
                w.sende(sid, &t);
            }
        },
        "pruefen" | "check" => match crate::daten::laden(&w.daten_ordner) {
            Ok(d) => {
                let mut t = String::from("{gruen}Keine Fehler gefunden.{/}");
                for warn in d.warnungen {
                    t.push_str(&format!("\n{{gelb}}Warnung:{{/}} {warn}"));
                }
                w.sende(sid, &t);
            }
            Err(fehler) => {
                let mut t = String::from("{rot}Fehler:{/}");
                for f in fehler {
                    t.push_str(&format!("\n- {f}"));
                }
                w.sende(sid, &t);
            }
        },
        "wo" => {
            let sp = &w.spieler[&sid].sp;
            if let (Some((x, y)), Some(ow)) = (crate::oberwelt::aus_id(&sp.raum), &w.d.oberwelt) {
                let zeichen = ow.zeichen(x, y).unwrap_or(' ');
                let feld = ow.feld(x, y).map(|f| f.name.clone()).unwrap_or_default();
                w.sende(
                    sid,
                    &format!(
                        "Oberwelt, Spalte x={x}, Zeile y={y} (so auch in karte.txt)\nZeichen '{zeichen}' = {feld}, Sichtweite {}\nRaum-ID für Ausgänge/Teleport: {}",
                        ow.sichtweite(x, y),
                        sp.raum
                    ),
                );
            } else if let Some(r) = w.d.raeume.get(&sp.raum) {
                let aus: Vec<String> =
                    r.ausgaenge.iter().map(|(ri, z)| format!("{}→{}", ri.schluessel(), z)).collect();
                w.sende(
                    sid,
                    &format!(
                        "Raum '{}' in Datei {}\nKoordinaten: x={} y={} z={}  Gebiet: '{}'  Gelände: '{}'\nAusgänge: {}",
                        r.id, r.quelle, r.x, r.y, r.z, r.gebiet, r.gelaende, aus.join(", ")
                    ),
                );
            }
        }
        "springe" | "goto" => {
            if !w.d.raum_existiert(rest) {
                let tipp = text::vorschlag(rest, w.d.raeume.keys()).map(|v| format!(" Meintest du '{v}'?")).unwrap_or_default();
                w.sende(sid, &format!("Raum '{rest}' gibt es nicht.{tipp}"));
                return true;
            }
            wirkung_anwenden(w, sid, "teleport", 0, rest);
        }
        "erschaffe" | "create" => {
            if !w.d.gegenstaende.contains_key(rest) {
                let tipp = text::vorschlag(rest, w.d.gegenstaende.keys()).map(|v| format!(" Meintest du '{v}'?")).unwrap_or_default();
                w.sende(sid, &format!("Gegenstand '{rest}' gibt es nicht.{tipp}"));
                return true;
            }
            let n = w.d.gegenstand_name(rest);
            if let Some(s) = w.spieler.get_mut(&sid) {
                s.sp.inventar.push(rest.to_string());
            }
            w.sende(sid, &format!("Du erschaffst {n}."));
        }
        "beschwoere" | "summon" => {
            let raum = w.spieler[&sid].sp.raum.clone();
            match w.monster_erschaffen(rest, &raum) {
                Some(id) => {
                    let n = gross(&w.monster_name(id));
                    w.sende_raum(&raum, &format!("{n} erscheint aus dem Nichts."), &[]);
                }
                None => {
                    let tipp = text::vorschlag(rest, w.d.monster.keys()).map(|v| format!(" Meintest du '{v}'?")).unwrap_or_default();
                    w.sende(sid, &format!("Monster '{rest}' gibt es nicht.{tipp}"));
                }
            }
        }
        "lehre" | "teach" => {
            let teile: Vec<&str> = rest.split_whitespace().collect();
            if teile.len() < 2 {
                w.sende(sid, "Benutzung: lehre <spieler> <fertigkeit-/zauber-id> [stufe]");
                return true;
            }
            let Some(pid) = w.spieler.iter().find(|(_, s)| s.sp.name.eq_ignore_ascii_case(teile[0])).map(|(id, _)| *id) else {
                w.sende(sid, "Dieser Spieler ist nicht online.");
                return true;
            };
            let id = teile[1].to_string();
            let stufe: u32 = teile.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
            let d = &w.d;
            let Some(s) = w.spieler.get_mut(&pid) else { return true };
            let meldung = if let Some(f) = d.fertigkeiten.get(&id) {
                s.sp.fertigkeiten.insert(id.clone(), stufe.min(f.max_stufe));
                format!("{} beherrscht nun {} auf Stufe {}.", s.sp.name, f.name, stufe.min(f.max_stufe))
            } else if let Some(z) = d.zauber.get(&id) {
                if !s.sp.zauber.contains(&id) {
                    s.sp.zauber.push(id.clone());
                }
                format!("{} kennt nun den Zauber {}.", s.sp.name, z.name)
            } else {
                format!("'{id}' ist weder eine Fertigkeit noch ein Zauber.")
            };
            w.sende(sid, &meldung);
            if pid != sid {
                w.sende(pid, &format!("{{hellgruen}}Du hast etwas Neues gelernt! ({meldung}){{/}}"));
            }
        }
        "liste" | "list" => {
            let d = &w.d;
            let zeilen: Vec<String> = match normalisieren(rest).as_str() {
                "raeume" | "raum" => d.raeume.values().map(|r| format!("{:<20} {} ({},{},{}) [{}]", r.id, r.name, r.x, r.y, r.z, r.quelle)).collect(),
                "gegenstaende" | "gegenstand" => d.gegenstaende.values().map(|g| format!("{:<20} {} ({}) [{}]", g.id, g.name, g.typ, g.quelle)).collect(),
                "monster" => d.monster.values().map(|m| format!("{:<20} {} (Stufe {}) [{}]", m.id, m.name, m.stufe, m.quelle)).collect(),
                "fertigkeiten" | "fertigkeit" => d.fertigkeiten.values().map(|f| format!("{:<20} {} ({}) [{}]", f.id, f.name, f.kategorie, f.quelle)).collect(),
                "zauber" => d.zauber.values().map(|z| format!("{:<20} {} ({} MP, Ziel: {}) [{}]", z.id, z.name, z.mana_kosten, z.ziel, z.quelle)).collect(),
                "gelaende" => d.gelaende.iter().map(|(id, g)| format!("{:<20} {} '{}'", id, g.name, g.symbol)).collect(),
                _ => vec!["Benutzung: liste raeume|gegenstaende|monster|fertigkeiten|zauber|gelaende".into()],
            };
            w.sende(sid, &zeilen.join("\n"));
        }
        "zuruecksetzen" | "reset" => {
            w.zuruecksetzen();
            w.sende(sid, "Gegenstände und Monster wurden aufgefüllt.");
        }
        _ => return false,
    }
    true
}

// ---------------------------------------------------------------------------
// Spielstand (sp): Werte und Fortschritte auf einen Blick
// ---------------------------------------------------------------------------

/// [##########----------]
fn balken(wert: i32, max: i32, breite: usize, farbe: &str) -> String {
    let anteil = if max > 0 { (wert.max(0) as f64 / max as f64).min(1.0) } else { 0.0 };
    let voll = (anteil * breite as f64).round() as usize;
    format!("[{{{farbe}}}{}{{/}}{{grau}}{}{{/}}]", "#".repeat(voll), "-".repeat(breite - voll))
}

fn dauer(sekunden: u64) -> String {
    let (std, min) = (sekunden / 3600, (sekunden % 3600) / 60);
    match (std, min) {
        (0, m) => format!("{m} Min"),
        (s, m) => format!("{s} Std {m} Min"),
    }
}

/// Sekunden seit 1970 -> "23.11.2026"
fn datum(sekunden: u64) -> String {
    let tage = (sekunden / 86_400) as i64 + 719_468;
    let era = tage.div_euclid(146_097);
    let doe = tage - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let tag = doy - (153 * mp + 2) / 5 + 1;
    let monat = if mp < 10 { mp + 3 } else { mp - 9 };
    let jahr = yoe + era * 400 + if monat <= 2 { 1 } else { 0 };
    format!("{tag:02}.{monat:02}.{jahr}")
}

fn spielstand(w: &Welt, sid: u64) {
    let sp = &w.spieler[&sid].sp;
    let mut t = format!("{{fett}}Spielstand von {}{{/}}\n\n", sp.voller_name());

    t.push_str(&format!(
        "  Lebenspunkte {} {:>4}/{}\n",
        balken(sp.lebenspunkte, sp.max_lebenspunkte, 20, "gruen"),
        sp.lebenspunkte,
        sp.max_lebenspunkte
    ));
    t.push_str(&format!(
        "  Mana         {} {:>4}/{}\n",
        balken(sp.mana, sp.max_mana, 20, "cyan"),
        sp.mana,
        sp.max_mana
    ));

    // Boni durch angelegte Ausrüstung
    let mut boni: std::collections::BTreeMap<String, i32> = Default::default();
    for id in sp.ausruestung.values() {
        if let Some(g) = w.d.gegenstaende.get(id) {
            for (k, v) in &g.eigenschaften {
                *boni.entry(k.clone()).or_default() += v;
            }
        }
    }
    if !sp.werte.is_empty() {
        t.push_str("\n{gelb}Werte{/}\n");
        for (k, v) in &sp.werte {
            let bonus = boni.remove(k).unwrap_or(0);
            let extra = match bonus {
                0 => String::new(),
                b if b > 0 => format!(" {{hellgruen}}(+{b}){{/}}"),
                b => format!(" {{hellrot}}({b}){{/}}"),
            };
            t.push_str(&format!("  {:<14}{:>4}{extra}\n", w.d.anzeigename(k), v + bonus));
        }
    }

    t.push_str("\n{gelb}Ausrüstung{/}\n");
    if sp.ausruestung.is_empty() {
        t.push_str("  nichts angelegt\n");
    }
    for (platz, id) in &sp.ausruestung {
        t.push_str(&format!("  {:<14}{}\n", w.d.anzeigename(platz), w.d.gegenstand_name(id)));
    }
    if !boni.is_empty() {
        let teile: Vec<String> = boni.iter().map(|(k, v)| format!("{} {v}", w.d.anzeigename(k))).collect();
        t.push_str(&format!("  {{grau}}Zusammen: {}{{/}}\n", teile.join(", ")));
    }

    t.push_str("\n{gelb}Können{/}\n");
    let mut fert: Vec<(&String, &u32)> = sp.fertigkeiten.iter().collect();
    fert.sort_by(|a, b| b.1.cmp(a.1));
    let beste: Vec<String> = fert
        .iter()
        .take(3)
        .filter_map(|(id, stufe)| w.d.fertigkeiten.get(*id).map(|f| format!("{} {stufe}", f.name)))
        .collect();
    t.push_str(&format!(
        "  Fertigkeiten  {}{}\n",
        sp.fertigkeiten.len(),
        if beste.is_empty() { String::new() } else { format!("  {{grau}}(beste: {}){{/}}", beste.join(", ")) }
    ));
    let zauber: Vec<String> = sp.zauber.iter().filter_map(|z| w.d.zauber.get(z).map(|z| z.name.clone())).collect();
    t.push_str(&format!(
        "  Zauber        {}{}\n",
        zauber.len(),
        if zauber.is_empty() { String::new() } else { format!("  {{grau}}({}){{/}}", zauber.join(", ")) }
    ));

    let gewicht: i32 = sp.inventar.iter().filter_map(|g| w.d.gegenstaende.get(g)).map(|g| g.gewicht).sum();
    let raeume = w.d.raeume.len().max(1);
    let besucht = sp.besucht.iter().filter(|r| w.d.raeume.contains_key(*r)).count();
    let ort = w.d.raum(&sp.raum).map(|r| r.name.clone()).unwrap_or_default();
    t.push_str("\n{gelb}Unterwegs{/}\n");
    t.push_str(&format!("  Aufenthalt    {ort}\n"));
    t.push_str(&format!("  Gepäck        {} Gegenstände, Gewicht {gewicht}\n", sp.inventar.len()));
    t.push_str(&format!(
        "  Erkundet      {besucht} von {} Räumen ({}%)\n",
        w.d.raeume.len(),
        besucht * 100 / raeume
    ));
    t.push_str(&format!("  Spielzeit     {}", dauer(sp.spielzeit)));
    if sp.erstellt > 0 {
        t.push_str(&format!(", dabei seit {}", datum(sp.erstellt)));
    }
    w.sende(sid, &t);
}

// ---------------------------------------------------------------------------
// Spielerbuch: persönliche Einstellungen
// ---------------------------------------------------------------------------

fn an_aus(b: bool) -> &'static str {
    if b {
        "{hellgruen}an{/}"
    } else {
        "{grau}aus{/}"
    }
}

fn als_schalter(wert: &str) -> Option<bool> {
    match normalisieren(wert).as_str() {
        "an" | "ein" | "ja" | "1" | "on" => Some(true),
        "aus" | "nein" | "0" | "off" => Some(false),
        _ => None,
    }
}

const EINSTELLUNGEN: &[&str] = &[
    "farben", "karte", "kartengroesse", "kurz", "breite", "prompt", "ankuendigungen", "npc", "titel", "beschreibung", "passwort",
];

fn spielerbuch_zeigen(w: &Welt, sid: u64) {
    let sp = &w.spieler[&sid].sp;
    let e = &sp.einstellungen;
    let groesse = match e.kartengroesse {
        i32::MIN..=-1 => "klein",
        0 => "normal",
        _ => "gross",
    };
    let zeile = |name: &str, wert: &str, beschreibung: &str| {
        format!("  {{gelb}}{name:<15}{{/}} {} {{grau}}{beschreibung}{{/}}\n", text::auffuellen(wert, 6))
    };
    let mut t = format!("{{fett}}Spielerbuch von {}{{/}}\n\n", sp.voller_name());
    t.push_str("{fett}Anzeige{/}\n");
    t.push_str(&zeile("farben", an_aus(sp.farben), "farbige Texte"));
    t.push_str(&zeile("karte", an_aus(e.karte), "Karte beim Umsehen zeigen"));
    t.push_str(&zeile("kartengroesse", groesse, "klein, normal oder gross"));
    t.push_str(&zeile("kurz", an_aus(e.kurz), "beim Gehen keine Raumbeschreibung (schau zeigt sie)"));
    t.push_str(&zeile("breite", &e.breite.to_string(), "Zeilenbreite für Texte, 40 bis 200"));
    t.push_str(&zeile("prompt", an_aus(e.prompt), "Eingabezeile mit Lebenspunkten und Mana"));
    t.push_str("\n{fett}Mitteilungen{/}\n");
    t.push_str(&zeile("ankuendigungen", an_aus(e.ankuendigungen), "melden, wenn Spieler kommen und gehen"));
    t.push_str(&zeile("npc", an_aus(e.npc_sprueche), "Sprüche von Bewohnern und Tieren"));
    t.push_str("\n{fett}Deine Figur{/}\n");
    let titel = if sp.titel.is_empty() { "(keiner)".to_string() } else { sp.titel.clone() };
    t.push_str(&format!("  {{gelb}}{:<15}{{/}} {titel}\n", "titel"));
    let besch = if sp.beschreibung.is_empty() { "(keine)".to_string() } else { sp.beschreibung.clone() };
    t.push_str(&format!("  {{gelb}}{:<15}{{/}} {besch}\n", "beschreibung"));
    t.push_str(&format!("  {{gelb}}{:<15}{{/}} {{grau}}ändern mit: spielerbuch passwort <alt> <neu>{{/}}\n", "passwort"));
    t.push_str("\nÄndern: {gelb}spielerbuch <einstellung> <wert>{/}, z.B. {gelb}spielerbuch karte aus{/}");
    w.sende(sid, &t);
}

fn spielerbuch(w: &mut Welt, sid: u64, rest: &str) {
    if rest.is_empty() {
        spielerbuch_zeigen(w, sid);
        return;
    }
    let (was, wert) = match rest.split_once(char::is_whitespace) {
        Some((a, b)) => (normalisieren(a), b.trim()),
        None => (normalisieren(rest), ""),
    };
    let Some(s) = w.spieler.get_mut(&sid) else { return };
    let e = &mut s.sp.einstellungen;

    // Ein-/Aus-Schalter
    let schalter: Option<&mut bool> = match was.as_str() {
        "farben" | "farbe" => Some(&mut s.sp.farben),
        "karte" => Some(&mut e.karte),
        "kurz" | "kurzbeschreibung" => Some(&mut e.kurz),
        "prompt" => Some(&mut e.prompt),
        "ankuendigungen" | "ankuendigung" => Some(&mut e.ankuendigungen),
        "npc" | "npcs" | "sprueche" | "npc_sprueche" => Some(&mut e.npc_sprueche),
        _ => None,
    };
    let meldung = if let Some(feld) = schalter {
        match als_schalter(wert) {
            Some(b) => {
                *feld = b;
                format!("{} ist jetzt {}.", gross(&was), an_aus(b))
            }
            None => format!("Bitte {{gelb}}an{{/}} oder {{gelb}}aus{{/}} angeben, z.B. spielerbuch {was} aus"),
        }
    } else {
        match was.as_str() {
            "kartengroesse" | "kartengrosse" | "groesse" => match normalisieren(wert).as_str() {
                "klein" => {
                    e.kartengroesse = -1;
                    "Die Karte ist jetzt klein.".into()
                }
                "normal" | "mittel" => {
                    e.kartengroesse = 0;
                    "Die Karte ist jetzt normal groß.".into()
                }
                "gross" | "groß" => {
                    e.kartengroesse = 1;
                    "Die Karte ist jetzt groß.".into()
                }
                _ => "Bitte klein, normal oder gross angeben.".into(),
            },
            "breite" => match wert.parse::<usize>() {
                Ok(b) if (40..=200).contains(&b) => {
                    e.breite = b;
                    format!("Texte werden jetzt auf {b} Zeichen umbrochen.")
                }
                _ => "Bitte eine Zahl von 40 bis 200 angeben.".into(),
            },
            "titel" => {
                if wert.is_empty() || matches!(normalisieren(wert).as_str(), "loeschen" | "-" | "keiner") {
                    s.sp.titel.clear();
                    "Dein Titel wurde entfernt.".into()
                } else if text::sichtbare_laenge(wert) > 40 {
                    "Der Titel darf höchstens 40 Zeichen lang sein.".into()
                } else {
                    s.sp.titel = wert.to_string();
                    format!("Du heißt jetzt: {}", s.sp.voller_name())
                }
            }
            "beschreibung" => {
                if wert.is_empty() || matches!(normalisieren(wert).as_str(), "loeschen" | "-" | "keine") {
                    s.sp.beschreibung.clear();
                    "Deine Beschreibung wurde entfernt.".into()
                } else if text::sichtbare_laenge(wert) > 400 {
                    "Die Beschreibung darf höchstens 400 Zeichen lang sein.".into()
                } else {
                    s.sp.beschreibung = wert.to_string();
                    "So sehen dich andere jetzt.".into()
                }
            }
            "passwort" => {
                let teile: Vec<&str> = wert.split_whitespace().collect();
                match teile.as_slice() {
                    [alt, neu] if s.sp.passwort_stimmt(alt) => {
                        if neu.chars().count() < 4 {
                            "Das neue Passwort ist zu kurz (mindestens 4 Zeichen).".into()
                        } else {
                            s.sp.passwort_setzen(neu);
                            "Dein Passwort wurde geändert.".into()
                        }
                    }
                    [_, _] => "Das alte Passwort stimmt nicht.".into(),
                    _ => "Benutzung: spielerbuch passwort <altes> <neues>".into(),
                }
            }
            _ => {
                let namen: Vec<String> = EINSTELLUNGEN.iter().map(|s| s.to_string()).collect();
                let tipp = text::vorschlag(&was, namen.iter()).map(|v| format!(" Meintest du '{v}'?")).unwrap_or_default();
                format!("Diese Einstellung gibt es nicht.{tipp} Tippe {{gelb}}spielerbuch{{/}} für eine Übersicht.")
            }
        }
    };
    // Sofort speichern, damit nichts verloren geht
    if let Some(s) = w.spieler.get(&sid) {
        w.speichern(&s.sp);
    }
    w.sende(sid, &meldung);
}
