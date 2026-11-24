//! Alle Befehle, die Spieler eintippen können.

use crate::beschreibung::{self, Detail, Sinn};
use crate::daten::{Daten, Richtung};
use crate::gespraech;
use crate::kampf::{self, Haltung};
use crate::befehlskonfig::Treffer;
use crate::befehlsliste;
use crate::karte;
use crate::spieler::{self, Rolle};
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
    w.spieler.get(&sid).map(|s| s.breite()).unwrap_or(78)
}

pub fn ausfuehren(w: &mut Welt, sid: u64, eingabe: &str) -> Folge {
    if !w.spieler.contains_key(&sid) {
        return Folge::Beenden;
    }
    let eingabe = eingabe.trim();
    // 'text und :text sind immer "sage" und "emote" – egal wie die Befehle heißen
    let (befehl, rest, fest) = if let Some(r) = eingabe.strip_prefix('\'') {
        (String::new(), r.trim(), Some(Treffer::Befehl("sage")))
    } else if let Some(r) = eingabe.strip_prefix(':') {
        (String::new(), r.trim(), Some(Treffer::Befehl("emote")))
    } else {
        match eingabe.split_once(char::is_whitespace) {
            Some((b, r)) => (normalisieren(b), r.trim(), None),
            None => (normalisieren(eingabe), "", None),
        }
    };
    if befehl.is_empty() && fest.is_none() {
        prompt(w, sid);
        return Folge::Weiter;
    }
    let rolle = w.spieler[&sid].rolle;
    let baut = rolle >= Rolle::Baumeister;
    let tot = kampf::ist_tot(w, sid);
    const TOT: &str = "{grau}Du bist tot. Warte, bis dich jemand wiederbelebt, oder tippe 'aufgeben'.{/}";
    const UNBEKANNT: &str = "Wie bitte? Tippe {gelb}hilfe{/} für eine Liste der Befehle.";

    // Was ist gemeint? (Wörter, Rollen und Abschalten stehen in befehle.toml)
    let treffer = fest.or_else(|| w.d.befehle.aufloesen(&befehl));
    let id: &str = match treffer {
        Some(Treffer::Befehl(id)) => {
            let noetig = Rolle::aus_text(&w.d.befehle.rolle(id)).unwrap_or(Rolle::Spieler);
            if rolle < noetig {
                if rolle == Rolle::Spieler {
                    w.sende(sid, UNBEKANNT);
                } else {
                    w.sende(sid, &format!("Dafür brauchst du die Rolle {}.", noetig.name()));
                }
                prompt(w, sid);
                return Folge::Weiter;
            }
            if tot && !befehlsliste::finde(id).is_some_and(|e| e.als_toter) {
                w.sende(sid, TOT);
                prompt(w, sid);
                return Folge::Weiter;
            }
            id
        }
        Some(Treffer::Geste(i)) => {
            if tot {
                w.sende(sid, TOT);
            } else {
                geste(w, sid, i, rest);
            }
            prompt(w, sid);
            return Folge::Weiter;
        }
        None => {
            match Richtung::aus_text(&befehl) {
                Some(_) if tot => w.sende(sid, TOT),
                Some(r) => gehen(w, sid, r),
                None => w.sende(sid, UNBEKANNT),
            }
            prompt(w, sid);
            return Folge::Weiter;
        }
    };

    match id {
        "ende" | "quit" | "exit" | "tschuess" => {
            w.sende(sid, "Bis bald! Deine Figur wurde gespeichert.");
            return Folge::Beenden;
        }
        "hilfe" | "help" | "?" => hilfe(w, sid, rolle, rest),
        "schau" | "schaue" | "l" | "look" | "umsehen" | "betrachte" | "untersuche" | "unt" | "x" => {
            if rest.is_empty() {
                raum_anzeigen(w, sid);
            } else {
                untersuchen(w, sid, rest);
            }
        }
        "riech" | "rieche" | "riechen" | "schnupper" | "schnuppere" | "smell" => wahrnehmen(w, sid, rest, Sinn::Riechen),
        "hoer" | "hoere" | "hoeren" | "horch" | "horche" | "lausche" | "listen" => wahrnehmen(w, sid, rest, Sinn::Hoeren),
        "taste" | "tasten" | "fuehl" | "fuehle" | "beruehre" | "fass" | "fasse" | "touch" | "feel" => {
            wahrnehmen(w, sid, rest, Sinn::Tasten)
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
        "weltkarte" | "landkarte" | "worldmap" => weltkarte(w, sid, baut),
        "nimm" | "nehme" | "take" | "get" => nehmen(w, sid, rest),
        "lege" | "leg" | "ablegen" | "drop" | "fallenlassen" => fallenlassen(w, sid, rest),
        "inventar" | "i" | "inv" => inventar(w, sid),
        "anlegen" | "anziehen" | "ausruesten" | "zuecke" | "wear" | "wield" => anlegen(w, sid, rest),
        "ausziehen" | "abnehmen" | "remove" => ausziehen(w, sid, rest),
        "benutze" | "benutzen" | "trinke" | "trink" | "iss" | "esse" | "use" | "drink" | "eat" => {
            benutzen(w, sid, rest)
        }
        "toete" | "tote" | "angriff" | "angreifen" | "greif" | "greife" | "attackiere" | "kill" | "kaempfe"
        | "schlag" | "schlage" => toeten(w, sid, rest),
        "ausweichen" | "weiche" | "duck" | "ducke" | "dodge" => kampf::ausweichen(w, sid),
        "flieh" | "fliehe" | "flucht" | "flee" => kampf::fliehen(w, sid),
        "stopp" | "stop" | "aufhoeren" => {
            if let Some(s) = w.spieler.get_mut(&sid) {
                s.kampf_ziel = None;
            }
            w.sende(sid, "Du hörst auf anzugreifen. (Wer dich angreift, macht vielleicht trotzdem weiter.)");
        }
        "haltung" => match Haltung::aus_text(rest) {
            Some(h) => {
                if let Some(s) = w.spieler.get_mut(&sid) {
                    s.haltung = h;
                }
                let wirkung = match h {
                    Haltung::Angriffslustig => "Du teilst mehr aus, steckst aber auch mehr ein.",
                    Haltung::Ausgewogen => "Angriff und Verteidigung halten sich die Waage.",
                    Haltung::Defensiv => "Du wirst seltener und schwächer getroffen, teilst aber weniger aus.",
                };
                w.sende(sid, &format!("Deine Haltung ist jetzt {}. {wirkung}", h.name()));
            }
            None => {
                let jetzt = w.spieler[&sid].haltung.name();
                w.sende(sid, &format!("Deine Haltung: {jetzt}. Möglich: angriffslustig, ausgewogen, defensiv."));
            }
        },
        "steigere" | "verbessere" | "trainiere" => kampf::steigern(w, sid, rest),
        "schaetze" | "einschaetzen" | "consider" | "con" => {
            let raum = w.spieler[&sid].sp.raum.clone();
            match finde_monster(w, &raum, rest) {
                Some(mid) => kampf::einschaetzen(w, sid, mid),
                None => w.sende(sid, "Wen willst du einschätzen?"),
            }
        }
        "wiederbeleben" | "wiederbelebe" | "belebe" | "rette" | "revive" => kampf::wiederbeleben(w, sid, rest),
        "aufgeben" => kampf::aufgeben(w, sid),
        "sage" | "sag" | "say" => {
            sagen(w, sid, rest);
            if let Some(mid) = npc_fuer_satz(w, sid, rest) {
                crate::quests::angesprochen(w, sid, mid, gespraech::fragt_nach_aufgabe(rest));
            }
        }
        "frage" | "frag" | "ask" => {
            fragen(w, sid, rest);
            let (wer, thema) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
            if let Some(mid) = finde_monster(w, &w.spieler[&sid].sp.raum, wer).filter(|m| kann_reden(w, *m)) {
                crate::quests::angesprochen(w, sid, mid, gespraech::fragt_nach_aufgabe(thema));
            }
        }
        "rede" | "red" | "sprich" | "sprech" | "talk" => {
            reden(w, sid, rest);
            let wer = rest.strip_prefix("mit ").unwrap_or(rest).trim();
            let raum = w.spieler[&sid].sp.raum.clone();
            let npcs: Vec<u64> = w.monster_im_raum(&raum).into_iter().filter(|m| kann_reden(w, *m)).collect();
            let mid = if wer.is_empty() { (npcs.len() == 1).then(|| npcs[0]) } else { finde_monster(w, &raum, wer) };
            if let Some(mid) = mid.filter(|m| kann_reden(w, *m)) {
                crate::quests::angesprochen(w, sid, mid, true);
            }
        }
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
        "zaubere" | "zauber_sprechen" | "cast" | "z" => zaubern(w, sid, rest, false),
        "spezial" => zaubern(w, sid, rest, true),
        "wirkungen" => crate::wirkung::anzeigen(w, sid),
        "waren" => crate::handel::waren(w, sid, rest),
        "kaufe" => crate::handel::kaufen(w, sid, rest),
        "verkaufe" => crate::handel::verkaufen(w, sid, rest),
        "lerne" => crate::handel::lernen(w, sid, rest),
        "geld" => crate::handel::geld(w, sid),
        "aufgaben" => crate::quests::anzeigen(w, sid, rest),
        "annehmen" => crate::quests::annehmen(w, sid, rest),
        "abgeben" => crate::quests::abgeben(w, sid),
        "abbrechen" => crate::quests::abbrechen(w, sid, rest),
        "rasse" => crate::gemeinschaft::rasse(w, sid, rest),
        "gilde" => crate::gemeinschaft::gilde(w, sid),
        "beruf" => crate::gemeinschaft::berufe(w, sid),
        "beitreten" => crate::gemeinschaft::beitreten(w, sid, rest),
        "austreten" => crate::gemeinschaft::austreten(w, sid, rest),
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
        // Baumeister- und Admin-Befehle (die Rolle wurde oben schon geprüft)
        _ => {
            if !admin_befehl(w, sid, id, rest) {
                w.sende(sid, UNBEKANNT);
            }
        }
    }
    // Aufgaben: Ist man in einem Raum, den man besuchen sollte?
    if w.spieler.contains_key(&sid) {
        crate::quests::raum_pruefen(w, sid);
    }
    prompt(w, sid);
    Folge::Weiter
}

/// Welcher NPC ist mit einem gesagten Satz gemeint? (mit Namen angesprochen oder der einzige im Raum)
fn npc_fuer_satz(w: &Welt, sid: u64, t: &str) -> Option<u64> {
    let raum = &w.spieler.get(&sid)?.sp.raum;
    let npcs: Vec<u64> = w.monster_im_raum(raum).into_iter().filter(|m| kann_reden(w, *m)).collect();
    let satz = gespraech::satz_vorbereiten(t);
    npcs.iter()
        .copied()
        .find(|mid| {
            w.monster.get(mid).and_then(|m| w.d.monster.get(&m.def)).is_some_and(|d| {
                satz.split_whitespace().any(|wort| passt(wort, &d.name, &d.id, &d.schluesselwoerter))
            })
        })
        .or_else(|| (npcs.len() == 1).then(|| npcs[0]))
}

/// Eine Geste aus befehle.toml ausführen, z.B. "winke" oder "winke bert".
fn geste(w: &Welt, sid: u64, i: usize, rest: &str) {
    let Some(g) = w.d.befehle.gesten.get(i).cloned() else { return };
    let noetig = Rolle::aus_text(&g.rolle).unwrap_or(Rolle::Spieler);
    if w.spieler[&sid].rolle < noetig {
        w.sende(sid, "Wie bitte? Tippe {gelb}hilfe{/} für eine Liste der Befehle.");
        return;
    }
    let s = &w.spieler[&sid];
    let (name, raum) = (s.sp.name.clone(), s.sp.raum.clone());
    let fuellen = |t: &str, ziel: &str, dativ: &str| {
        gross(&t.replace("{name}", &name).replace("{ziel_dativ}", dativ).replace("{ziel}", ziel))
    };

    if rest.is_empty() || g.selbst_ziel.trim().is_empty() {
        if g.selbst.trim().is_empty() {
            w.sende(sid, "Wem denn? (Diese Geste braucht ein Ziel.)");
            return;
        }
        w.sende(sid, &fuellen(&g.selbst, "", ""));
        if !g.andere.trim().is_empty() {
            w.sende_raum(&raum, &fuellen(&g.andere, "", ""), &[sid]);
        }
        return;
    }
    // Ziel: ein Spieler oder ein Monster/NPC im Raum
    let (ziel_name, ziel_dativ, ziel_sid) = if let Some(pid) = finde_spieler_im_raum(w, &raum, rest) {
        let n = w.spieler[&pid].sp.name.clone();
        (n.clone(), n, Some(pid))
    } else if let Some(mid) = finde_monster(w, &raum, rest) {
        let dativ = w
            .monster
            .get(&mid)
            .and_then(|m| w.d.monster.get(&m.def))
            .map(|d| d.dativ.clone())
            .filter(|d| !d.is_empty())
            .unwrap_or_else(|| w.monster_name(mid));
        (w.monster_akk(mid), dativ, None)
    } else {
        w.sende(sid, "So jemanden siehst du hier nicht.");
        return;
    };
    w.sende(sid, &fuellen(&g.selbst_ziel, &ziel_name, &ziel_dativ));
    let mut ausser = vec![sid];
    if let Some(pid) = ziel_sid.filter(|p| *p != sid) {
        if !g.ziel.trim().is_empty() {
            w.sende(pid, &fuellen(&g.ziel, &ziel_name, &ziel_dativ));
        }
        ausser.push(pid);
    }
    if !g.andere_ziel.trim().is_empty() {
        w.sende_raum(&raum, &fuellen(&g.andere_ziel, &ziel_name, &ziel_dativ), &ausser);
    }
}

pub fn prompt(w: &Welt, sid: u64) {
    crate::clientdaten::werte(w, sid);
    prompt_text(w, sid);
    // Ende des Prompts markieren (EOR oder GA), damit Clients ihn erkennen
    if let Some(s) = w.spieler.get(&sid) {
        let ende = crate::telnet::prompt_ende(&crate::telnet::sperren(&s.proto));
        crate::telnet::senden(&s.sender, ende);
    }
}

fn prompt_text(w: &Welt, sid: u64) {
    if let Some(s) = w.spieler.get(&sid) {
        let p = &s.sp;
        if !p.einstellungen.prompt {
            s.sende_ohne_umbruch("\n> ");
            return;
        }
        if s.tot_bis.is_some() {
            s.sende_ohne_umbruch("\n{rot}[tot]{/} > ");
            return;
        }
        // Im Kampf: Zustand des Gegners anzeigen
        let gegner = s
            .kampf_ziel
            .and_then(|mid| w.monster.get(&mid))
            .and_then(|m| w.d.monster.get(&m.def).map(|d| (d, m.lebenspunkte)))
            .map(|(d, lp)| format!(" {{hellrot}}[{}: {}]{{/}}", gross(&d.name), kampf::zustand(lp, d.lebenspunkte)))
            .unwrap_or_default();
        s.sende_ohne_umbruch(&format!(
            "\n{{gruen}}LP {}/{}{{/}} {{cyan}}MP {}/{}{{/}} {{gelb}}AP {}/{}{{/}}{gegner} > ",
            p.lebenspunkte, p.max_lebenspunkte, p.mana, p.max_mana, p.ausdauer, p.max_ausdauer
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

/// Die Hilfe entsteht aus der Befehlsliste und befehle.toml:
/// umbenannte Befehle, eigene Hilfetexte und Gesten erscheinen automatisch.
fn hilfe(w: &Welt, sid: u64, rolle: Rolle, rest: &str) {
    let b = &w.d.befehle;
    let darf = |r: &str| Rolle::aus_text(r).unwrap_or(Rolle::Spieler) <= rolle;

    // hilfe <befehl>: alles über einen Befehl
    if !rest.is_empty() {
        let t = match b.aufloesen(rest) {
            Some(Treffer::Befehl(id)) if darf(&b.rolle(id)) => format!(
                "{{fett}}{}{{/}}\n  {}\n  {{grau}}Aufruf:{{/}} {}\n  {{grau}}Wörter:{{/}} {}",
                b.hauptname(id),
                b.hilfe(id),
                b.syntax(id),
                b.woerter(id).join(", ")
            ),
            Some(Treffer::Geste(i)) => {
                let g = &b.gesten[i];
                format!(
                    "{{fett}}{}{{/}} (Geste)\n  {}\n  {{grau}}Wörter:{{/}} {}{}",
                    g.woerter.first().cloned().unwrap_or_default(),
                    g.hilfe,
                    g.woerter.join(", "),
                    if g.selbst_ziel.is_empty() { "" } else { "\n  Auch mit Ziel: z.B. mit dem Namen eines Spielers" }
                )
            }
            _ => "Diesen Befehl kenne ich nicht.".to_string(),
        };
        w.sende(sid, &t);
        return;
    }

    let mut t = String::new();
    for kat in befehlsliste::KATEGORIEN {
        let zeilen: Vec<String> = befehlsliste::EINGEBAUT
            .iter()
            .filter(|e| e.kategorie == *kat && b.aktiv(e.id) && darf(&b.rolle(e.id)))
            .map(|e| {
                let syntax = b.syntax(e.id);
                let kopf = format!("{{gelb}}{}{{/}}", if syntax.is_empty() { b.hauptname(e.id) } else { syntax });
                format!("  {} {}", text::auffuellen(&kopf, 30), b.hilfe(e.id))
            })
            .collect();
        if zeilen.is_empty() {
            continue;
        }
        let farbe = match *kat {
            "Baumeister" => "{hellgelb}",
            "Admin" => "{hellrot}",
            _ => "",
        };
        t.push_str(&format!("\n{{fett}}{farbe}{kat}{{/}}\n{}\n", zeilen.join("\n")));
    }
    let gesten: Vec<String> = b
        .gesten
        .iter()
        .filter(|g| g.aktiv && darf(&g.rolle))
        .filter_map(|g| g.woerter.first().cloned())
        .collect();
    if !gesten.is_empty() {
        t.push_str(&format!("\n{{fett}}Gesten{{/}}\n  {}\n", gesten.join(", ")));
    }
    t.push_str("\n{grau}Mehr zu einem Befehl: hilfe <befehl>{/}");
    w.sende(sid, t.trim_start());
}

// ---------------------------------------------------------------------------
// Suchen von Dingen anhand dessen, was der Spieler tippt
// ---------------------------------------------------------------------------

/// Passt ein getipptes Wort zu einem Ding?
pub(crate) fn passt(such: &str, name: &str, id: &str, schluessel: &[String]) -> bool {
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

pub(crate) fn finde_gegenstand(d: &Daten, liste: &[String], such: &str) -> Option<usize> {
    liste.iter().position(|id| {
        d.gegenstaende.get(id).is_some_and(|g| passt(such, &g.name, &g.id, &g.schluesselwoerter))
    })
}

pub(crate) fn finde_monster(w: &Welt, raum: &str, such: &str) -> Option<u64> {
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
pub(crate) fn gruppieren(ids: &[String]) -> Vec<(String, usize)> {
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

    let karte = if w.d.welt.karte_beim_umsehen && e.karte {
        let radius = (w.d.welt.karten_radius + e.kartengroesse).max(1);
        karte::zeichnen(w, &s.sp, radius, false)
    } else {
        String::new()
    };

    // Name, Beschreibung und Ausgänge
    let mut kopf = format!("{{fett}}{{gelb}}{}{{/}}", raum.name);
    if crate::oberwelt::ist_oberwelt(&raum.id) {
        kopf.push_str(&format!(" {{grau}}({}, {}){{/}}", raum.x, raum.y));
    } else if s.baut() {
        kopf.push_str(&format!(" {{grau}}[{}]{{/}}", raum.id));
    }
    let mut ausgaenge: Vec<String> = raum.ausgaenge.keys().map(|r| r.anzeige().to_string()).collect();
    let himmelsrichtungen = raum.ausgaenge.keys().filter(|r| r.versatz().2 == 0 && r.versatz() != (0, 0, 0)).count();
    if himmelsrichtungen == 8 {
        ausgaenge.retain(|a| a == "hinein" || a == "hinaus");
        ausgaenge.insert(0, "in alle Richtungen".into());
    }
    let ausgaenge = if ausgaenge.is_empty() {
        "{cyan}Es gibt keine sichtbaren Ausgänge.{/}".to_string()
    } else {
        format!("{{cyan}}Ausgänge: {}{{/}}", aufzaehlen(&ausgaenge))
    };
    let mut absaetze: Vec<(String, bool)> = vec![(kopf, false)];
    if voll || !e.kurz {
        for (i, a) in raum.beschreibung.trim().split("\n\n").enumerate() {
            if !a.trim().is_empty() {
                absaetze.push((a.to_string(), i > 0));
            }
        }
    }
    absaetze.push((ausgaenge, false));

    let breite = s.breite();
    let mut karten_zeilen: Vec<String> = karte.trim_end().lines().map(str::to_string).collect();
    // Hinweise unter der Karte, die breiter sind als die Karte selbst (z.B. "Hier geht es
    // auch nach oben"), kommen beim Nebeneinander mit in die Textspalte
    let mut hinweise: Vec<String> = Vec::new();
    while karten_zeilen.len() > 1 {
        let letzte = text::sichtbare_laenge(karten_zeilen.last().expect("da"));
        let sonst = karten_zeilen[..karten_zeilen.len() - 1].iter().map(|z| text::sichtbare_laenge(z)).max().unwrap_or(0);
        if letzte <= sonst {
            break;
        }
        hinweise.insert(0, karten_zeilen.pop().expect("da").trim().to_string());
    }
    // Eine Kopfzeile, die breiter ist als die Karte, steht beim Nebeneinander allein darüber
    let mut kopfzeile = None;
    if karten_zeilen.len() > 1 {
        let erste = text::sichtbare_laenge(&karten_zeilen[0]);
        let sonst = karten_zeilen[1..].iter().map(|z| text::sichtbare_laenge(z)).max().unwrap_or(0);
        if erste > sonst {
            kopfzeile = Some(karten_zeilen.remove(0));
        }
    }
    let kartenbreite = karten_zeilen.iter().map(|z| text::sichtbare_laenge(z)).max().unwrap_or(0);
    let daneben = e.karte_daneben.unwrap_or(w.d.welt.karte_daneben);
    let rechts = breite.saturating_sub(kartenbreite + 3);
    if !karten_zeilen.is_empty() && daneben && rechts >= 25 {
        // Text rechts neben der Karte, darunter in voller Breite weiter
        absaetze.extend(hinweise.into_iter().map(|h| (h, false)));
        if let Some(k) = &kopfzeile {
            aus.push_str(k);
            aus.push('\n');
        }
        let zeilen = text::umbrechen_fliessend(&absaetze, rechts, karten_zeilen.len(), breite);
        aus.push_str(&text::nebeneinander(&karten_zeilen, &zeilen, 3));
    } else {
        if !karten_zeilen.is_empty() {
            aus.push_str(&karte);
            aus.push('\n');
        }
        for z in text::umbrechen_fliessend(&absaetze, breite, 0, breite) {
            aus.push_str(&z);
            aus.push('\n');
        }
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
        let gegner: Vec<String> = w.monster[&mid].ziele.iter().filter_map(|p| w.spieler.get(p)).map(|s| s.sp.name.clone()).collect();
        let t = if !gegner.is_empty() {
            format!("{} kämpft hier mit {}!", gross(&def.name), aufzaehlen(&gegner))
        } else if def.raumtext.is_empty() {
            format!("{} ist hier.", gross(&def.name))
        } else {
            def.raumtext.clone()
        };
        aus.push_str(&format!("{{hellrot}}{t}{{/}}\n"));
    }
    for pid in w.spieler_im_raum(&raum.id) {
        if pid != sid {
            let anderer = &w.spieler[&pid];
            if anderer.tot_bis.is_some() {
                aus.push_str(&format!("{{rot}}{} liegt leblos am Boden. (wiederbeleben {}){{/}}\n", anderer.sp.name, anderer.sp.name.to_lowercase()));
            } else {
                aus.push_str(&format!("{{hellgruen}}{} ist hier.{{/}}\n", anderer.sp.voller_name()));
            }
        }
    }
    w.sende(sid, aus.trim_end());
    crate::clientdaten::raum(w, sid);
}

fn gehen(w: &mut Welt, sid: u64, richt: Richtung) {
    if kampf::im_kampf(w, sid) {
        w.sende(sid, "{hellrot}Du bist mitten im Kampf! Mit 'flieh' kannst du versuchen zu entkommen.{/}");
        return;
    }
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

/// Was hat der Spieler gemeint?
enum Ziel {
    Detail(String, Detail),
    Gegenstand(String),
    Monster(u64),
    Spieler(u64),
}

/// "rinde der alten eiche" -> ["rinde der alten eiche", "rinde", "alten", "eiche"]
fn suchwoerter(such: &str) -> Vec<String> {
    const FUELL: &[&str] = &[
        "der", "die", "das", "den", "dem", "des", "ein", "eine", "einen", "am", "an", "im", "in", "vom", "von",
        "auf", "unter", "mit", "zum", "zur",
    ];
    let mut v = vec![such.trim().to_string()];
    for wort in such.split_whitespace() {
        if !FUELL.contains(&normalisieren(wort).as_str()) && !v.iter().any(|x| x == wort) {
            v.push(wort.to_string());
        }
    }
    v
}

/// Details eines Gegenstands oder Monsters plus deren Bibliothek.
fn ding_kontext(d: &Daten, details: &std::collections::BTreeMap<String, Detail>, nutzt: &[String]) -> std::collections::BTreeMap<String, Detail> {
    let mut ids = nutzt.to_vec();
    beschreibung::alle_verweise(details, &mut ids);
    let mut alles = details.clone();
    alles.extend(d.bibliothek_fuer(&ids));
    alles
}

fn finde(w: &Welt, sid: u64, such: &str) -> Option<Ziel> {
    let s = &w.spieler[&sid];
    let raum = w.d.raum(&s.sp.raum)?;
    let angelegt: Vec<String> = s.sp.ausruestung.values().cloned().collect();
    let boden = w.boden.get(&raum.id).cloned().unwrap_or_default();
    let alle_dinge: Vec<&String> = s.sp.inventar.iter().chain(angelegt.iter()).chain(boden.iter()).collect();
    let monster = w.monster_im_raum(&raum.id);
    let kontext = w.d.raum_kontext(&raum);

    for wort in suchwoerter(such) {
        // 1. Eigene Details des Raums
        if let Some((n, d)) = beschreibung::suchen(&raum.details, &wort) {
            return Some(Ziel::Detail(n.to_string(), d.clone()));
        }
        // 2. Gegenstände: bei sich, angelegt, am Boden
        for liste in [&s.sp.inventar, &angelegt, &boden] {
            if let Some(i) = finde_gegenstand(&w.d, liste, &wort) {
                return Some(Ziel::Gegenstand(liste[i].clone()));
            }
        }
        // 3. Monster und Spieler
        if let Some(m) = finde_monster(w, &raum.id, &wort) {
            return Some(Ziel::Monster(m));
        }
        if let Some(p) = finde_spieler_im_raum(w, &raum.id, &wort) {
            return Some(Ziel::Spieler(p));
        }
        // 4. Details an Gegenständen und Monstern ("griff", "fell")
        for id in &alle_dinge {
            if let Some(g) = w.d.gegenstaende.get(*id) {
                let k = ding_kontext(&w.d, &g.details, &g.nutzt);
                if let Some((n, d)) = beschreibung::suchen(&k, &wort) {
                    return Some(Ziel::Detail(n.to_string(), d.clone()));
                }
            }
        }
        for mid in &monster {
            if let Some(def) = w.monster.get(mid).and_then(|m| w.d.monster.get(&m.def)) {
                let k = ding_kontext(&w.d, &def.details, &def.nutzt);
                if let Some((n, d)) = beschreibung::suchen(&k, &wort) {
                    return Some(Ziel::Detail(n.to_string(), d.clone()));
                }
            }
        }
        // 5. Allgemeine Beschreibungen aus der Bibliothek (Bäume, Himmel ...)
        if let Some((n, d)) = beschreibung::suchen(&kontext, &wort) {
            return Some(Ziel::Detail(n.to_string(), d.clone()));
        }
    }
    None
}

fn untersuchen(w: &Welt, sid: u64, such: &str) {
    let s = &w.spieler[&sid];
    let br = breite(w, sid);
    let such = such.strip_prefix("an ").unwrap_or(such);

    match finde(w, sid, such) {
        Some(Ziel::Detail(name, d)) => {
            let text = if d.text().is_empty() { "Daran ist nichts Besonderes." } else { d.text() };
            let mut t = umbrechen(text, br);
            if s.baut() {
                let unter = beschreibung::unterdetails(&d);
                let sinne: Vec<&str> = [(Sinn::Riechen, "geruch"), (Sinn::Hoeren, "geraeusch"), (Sinn::Tasten, "gefuehl")]
                    .iter()
                    .filter(|(sinn, _)| !d.sinn(*sinn).is_empty())
                    .map(|(_, n)| *n)
                    .collect();
                t.push_str(&format!("\n{{grau}}[Detail '{name}'"));
                if !unter.is_empty() {
                    t.push_str(&format!(", Unterdetails: {}", unter.join(", ")));
                }
                if !sinne.is_empty() {
                    t.push_str(&format!(", Sinne: {}", sinne.join(", ")));
                }
                t.push_str("]{/}");
            }
            w.sende(sid, &t);
        }
        Some(Ziel::Gegenstand(id)) => {
            let Some(g) = w.d.gegenstaende.get(&id) else { return };
            let mut t = format!("{{fett}}{}{{/}}\n", gross(&g.name));
            t.push_str(&umbrechen(if g.beschreibung.is_empty() { "Nichts Besonderes." } else { &g.beschreibung }, br));
            if !g.platz.is_empty() {
                t.push_str(&format!("\n{{grau}}(Kann angelegt werden: {}){{/}}", w.d.anzeigename(&g.platz)));
            }
            if s.baut() {
                t.push_str(&format!(
                    "\n{{grau}}[id: {}, typ: {}, gewicht: {}, wert: {}, eigenschaften: {:?}, datei: {}]{{/}}",
                    g.id, g.typ, g.gewicht, g.wert, g.eigenschaften, g.quelle
                ));
            }
            w.sende(sid, &t);
        }
        Some(Ziel::Monster(mid)) => {
            let Some(m) = w.monster.get(&mid) else { return };
            let Some(def) = w.d.monster.get(&m.def) else { return };
            let mut t = format!("{{fett}}{}{{/}}\n", gross(&def.name));
            t.push_str(&umbrechen(if def.beschreibung.is_empty() { "Nichts Besonderes." } else { &def.beschreibung }, br));
            if s.baut() {
                t.push_str(&format!(
                    "\n{{grau}}[id: {}, stufe: {}, LP: {}/{}, MP: {}, heimat: {}, datei: {}]{{/}}",
                    def.id, def.stufe, m.lebenspunkte, def.lebenspunkte, m.mana, m.heimat, def.quelle
                ));
            }
            w.sende(sid, &t);
        }
        Some(Ziel::Spieler(pid)) => {
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
        }
        None => w.sende(sid, "So etwas siehst du hier nicht."),
    }
}

/// riech / hoer / taste – mit oder ohne Ziel.
fn wahrnehmen(w: &Welt, sid: u64, such: &str, sinn: Sinn) {
    let s = &w.spieler[&sid];
    let br = breite(w, sid);
    let name = s.sp.name.clone();
    let raum_id = s.sp.raum.clone();
    let such = such.strip_prefix("an ").unwrap_or(such).trim();
    let waehle = |geruch: &str, geraeusch: &str, gefuehl: &str| -> String {
        match sinn {
            Sinn::Riechen => geruch,
            Sinn::Hoeren => geraeusch,
            Sinn::Tasten => gefuehl,
        }
        .to_string()
    };

    let text = if such.is_empty() {
        w.d.raum(&raum_id).map(|r| w.d.raum_sinn(&r, sinn)).unwrap_or_default()
    } else {
        match finde(w, sid, such) {
            Some(Ziel::Detail(name, d)) => {
                // Ein einfaches Detail ohne Sinne? Dann hilft die Bibliothek aus.
                let eigen = d.sinn(sinn).to_string();
                if eigen.is_empty() {
                    w.d.raum(&raum_id)
                        .and_then(|r| {
                            let k = w.d.raum_kontext(&r);
                            beschreibung::suchen(&k, &name).map(|(_, b)| b.sinn(sinn).to_string())
                        })
                        .unwrap_or_default()
                } else {
                    eigen
                }
            }
            Some(Ziel::Gegenstand(id)) => {
                w.d.gegenstaende.get(&id).map(|g| waehle(&g.geruch, &g.geraeusch, &g.gefuehl)).unwrap_or_default()
            }
            Some(Ziel::Monster(mid)) => w
                .monster
                .get(&mid)
                .and_then(|m| w.d.monster.get(&m.def))
                .map(|d| waehle(&d.geruch, &d.geraeusch, &d.gefuehl))
                .unwrap_or_default(),
            Some(Ziel::Spieler(pid)) => {
                let anderer = w.spieler[&pid].sp.name.clone();
                if sinn == Sinn::Tasten && pid != sid {
                    w.sende(pid, &format!("{name} stupst dich an."));
                    format!("Du stupst {anderer} an.")
                } else {
                    String::new()
                }
            }
            None => {
                w.sende(sid, "So etwas findest du hier nicht.");
                return;
            }
        }
    };
    let text = if text.is_empty() { sinn.nichts().to_string() } else { text };
    w.sende(sid, &umbrechen(&text, br));
    w.sende_raum(&raum_id, &format!("{name} {}.", sinn.handlung()), &[sid]);
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
    t.push_str(&format!("{{grau}}Gewicht: {gewicht}{{/}}   Geld: {{hellgelb}}{}{{/}}", crate::handel::betrag(w, sp.geld)));
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
    crate::clientdaten::gespraech(w, &w.spieler_im_raum(&s.raum), "say", &s.name, &format!("{} sagt: „{t}“", s.name));

    // Hören NPCs zu? Wer mit Namen angesprochen wird, antwortet immer.
    // Ist nur ein NPC da, antwortet er, wenn er etwas zum Thema weiß.
    let npcs: Vec<u64> = w.monster_im_raum(&s.raum).into_iter().filter(|m| kann_reden(w, *m)).collect();
    let satz = gespraech::satz_vorbereiten(t);
    let angesprochen = npcs.iter().copied().find(|mid| {
        w.monster.get(mid).and_then(|m| w.d.monster.get(&m.def)).is_some_and(|d| {
            satz.split_whitespace().any(|wort| passt(wort, &d.name, &d.id, &d.schluesselwoerter))
        })
    });
    match (angesprochen, npcs.as_slice()) {
        (Some(mid), _) => {
            npc_antwortet(w, &s.raum, mid, &s.name, t, true);
        }
        (None, [mid]) => {
            npc_antwortet(w, &s.raum, *mid, &s.name, t, false);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Gespräche mit NPCs
// ---------------------------------------------------------------------------

pub(crate) fn kann_reden(w: &Welt, mid: u64) -> bool {
    w.monster.get(&mid).and_then(|m| w.d.monster.get(&m.def)).is_some_and(|d| {
        let g = &d.gespraech;
        !d.thema.is_empty()
            || [&g.begruessung, &g.abschied, &g.beruf, &g.aufgabe, &g.name, &g.neuigkeiten, &g.ort, &g.unbekannt]
                .iter()
                .any(|t| !t.ist_leer())
    })
}

/// Der NPC antwortet für alle hörbar. `notfalls`: auch antworten, wenn er die Frage nicht versteht.
pub(crate) fn npc_antwortet(w: &Welt, raum: &str, mid: u64, spieler: &str, frage: &str, notfalls: bool) -> bool {
    let Some(def) = w.monster.get(&mid).and_then(|m| w.d.monster.get(&m.def)) else { return false };
    let text = match gespraech::antwort(&def.gespraech, &def.thema, frage) {
        Some(t) => t,
        None if notfalls => def.gespraech.unbekannt.waehle().unwrap_or_else(|| "*zuckt mit den Schultern.".into()),
        None => return false,
    };
    let text = text.replace("{spieler}", spieler).replace("{name}", &def.name);
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let npc = gross(&def.name);
    let zeile = match text.trim().strip_prefix('*') {
        Some(handlung) => format!("{npc} {}", handlung.trim()),
        None => format!("{{hellcyan}}{npc} sagt: „{}“{{/}}", text.trim()),
    };
    w.sende_raum(raum, &zeile, &[]);
    true
}

/// frage <wer> [nach] <thema>
fn fragen(w: &Welt, sid: u64, rest: &str) {
    let s = &w.spieler[&sid].sp;
    let (wer, thema) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    if wer.is_empty() {
        w.sende(sid, "Wen willst du fragen? (z.B. frage wirt nach arbeit)");
        return;
    }
    let Some(mid) = finde_monster(w, &s.raum, wer) else {
        w.sende(sid, "So jemanden siehst du hier nicht.");
        return;
    };
    let npc_akk = w.monster_akk(mid);
    if !kann_reden(w, mid) {
        w.sende(sid, &format!("{} kann dir nicht antworten.", gross(&w.monster_name(mid))));
        return;
    }
    let thema = thema.trim();
    let thema = ["nach ", "ob ", "wegen ", "ueber ", "über ", "zu "]
        .iter()
        .find_map(|p| thema.strip_prefix(p))
        .unwrap_or(thema)
        .trim();
    if thema.is_empty() {
        w.sende(sid, &format!("Du sprichst {npc_akk} an."));
        w.sende_raum(&s.raum, &format!("{} spricht {npc_akk} an.", s.name), &[sid]);
    } else {
        w.sende(sid, &format!("Du fragst {npc_akk} nach {thema}."));
        w.sende_raum(&s.raum, &format!("{} fragt {npc_akk} nach {thema}.", s.name), &[sid]);
    }
    npc_antwortet(w, &s.raum, mid, &s.name, thema, true);
}

/// rede [mit] <wer>: Begrüßung und eine Liste der Themen.
fn reden(w: &Welt, sid: u64, rest: &str) {
    let s = &w.spieler[&sid].sp;
    let wer = rest.strip_prefix("mit ").unwrap_or(rest).trim();
    let npcs: Vec<u64> = w.monster_im_raum(&s.raum).into_iter().filter(|m| kann_reden(w, *m)).collect();
    let mid = if wer.is_empty() {
        match npcs.as_slice() {
            [m] => *m,
            [] => {
                w.sende(sid, "Hier ist niemand, mit dem du reden könntest.");
                return;
            }
            _ => {
                w.sende(sid, "Mit wem willst du reden? (rede mit <name>)");
                return;
            }
        }
    } else {
        match finde_monster(w, &s.raum, wer) {
            Some(m) => m,
            None => {
                w.sende(sid, "So jemanden siehst du hier nicht.");
                return;
            }
        }
    };
    if !kann_reden(w, mid) {
        w.sende(sid, &format!("{} kann nicht sprechen.", gross(&w.monster_name(mid))));
        return;
    }
    let npc_akk = w.monster_akk(mid);
    w.sende(sid, &format!("Du sprichst {npc_akk} an."));
    w.sende_raum(&s.raum, &format!("{} spricht {npc_akk} an.", s.name), &[sid]);
    npc_antwortet(w, &s.raum, mid, &s.name, "hallo", true);
    if let Some(d) = w.monster.get(&mid).and_then(|m| w.d.monster.get(&m.def)) {
        let themen = gespraech::themenliste(&d.gespraech, &d.thema);
        if !themen.is_empty() {
            let kurz = d.schluesselwoerter.first().cloned().unwrap_or_else(|| d.id.clone());
            w.sende(
                sid,
                &format!(
                    "{{grau}}Du könntest fragen nach: {}.  (z.B. frage {kurz} nach {}){{/}}",
                    themen.join(", "),
                    themen[0].to_lowercase()
                ),
            );
        }
    }
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
    let alle: Vec<u64> = w.spieler.keys().copied().collect();
    crate::clientdaten::gespraech(w, &alle, "shout", &s.name, &format!("{} ruft: „{t}“", s.name));
}

fn wer(w: &Welt, sid: u64) {
    let admin = w.spieler.get(&sid).is_some_and(|s| s.rolle >= crate::spieler::Rolle::Admin);
    let mut t = format!("{{fett}}In {} sind gerade unterwegs:{{/}}\n", w.d.welt.name);
    for s in w.spieler.values() {
        let marke = s.rolle.marke();
        // Admins sehen auch, mit welchem Programm jemand spielt
        let client = if admin {
            format!(" {{grau}}({}){{/}}", crate::telnet::sperren(&s.proto).client_name())
        } else {
            String::new()
        };
        t.push_str(&format!("  {}{marke}{client}\n", s.sp.voller_name()));
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
        w.sende(sid, "Du kennst noch keine Zaubersprüche und Kampftechniken.");
        return;
    }
    let kosten = |z: &crate::daten::ZauberDef| {
        let mut k = Vec::new();
        if z.mana_kosten > 0 {
            k.push(format!("{} MP", z.mana_kosten));
        }
        if z.ausdauer_kosten > 0 {
            k.push(format!("{} AP", z.ausdauer_kosten));
        }
        text::auffuellen(&k.join(" "), 12)
    };
    let mut t = String::new();
    let mut nr = 0;
    for (technik, titel, befehl) in [(false, "Deine Zaubersprüche", "zaubere <nr|name> [ziel]"), (true, "Deine Kampftechniken", "spezial <nr|name> [ziel]")] {
        let liste: Vec<_> = sp.zauber.iter().filter_map(|id| w.d.zauber.get(id)).filter(|z| z.ist_technik() == technik).collect();
        if liste.is_empty() {
            continue;
        }
        t.push_str(&format!("{{fett}}{titel}{{/}}  {{grau}}({befehl}){{/}}\n"));
        for z in liste {
            nr += 1;
            t.push_str(&format!("  {{gelb}}{nr:>2}.{{/}} {{magenta}}{:<18}{{/}} {} {{grau}}{}{{/}}\n", z.name, kosten(z), z.beschreibung));
        }
    }
    w.sende(sid, t.trim_end());
}

/// "zaubere ..." oder (mit `technik`) "spezial ...".
fn zaubern(w: &mut Welt, sid: u64, rest: &str, technik: bool) {
    let was = if technik { "Welche Technik willst du einsetzen?" } else { "Was willst du zaubern?" };
    if rest.is_empty() {
        w.sende(sid, &format!("{was} ({{gelb}}zauber{{/}} zeigt, was du kannst)"));
        return;
    }
    // Welcher Zauber ist gemeint? Die Nummer aus "zauber" – oder der längste passende Name.
    let such = normalisieren(rest);
    let mut bester: Option<(String, usize)> = None;
    let (erstes, _) = such.split_once(' ').unwrap_or((&such, ""));
    if let Some(n) = crate::gemeinschaft::nummer(erstes) {
        let welt: &Welt = w;
        let sp = &welt.spieler[&sid].sp;
        let mut reihenfolge: Vec<String> = Vec::new();
        for t in [false, true] {
            reihenfolge.extend(sp.zauber.iter().filter(|id| welt.d.zauber.get(*id).is_some_and(|z| z.ist_technik() == t)).cloned());
        }
        match reihenfolge.get(n) {
            Some(id) => bester = Some((id.clone(), erstes.len())),
            None => {
                w.sende(sid, "Diese Nummer gibt es nicht. ({gelb}zauber{/} zeigt die Liste)");
                return;
            }
        }
    }
    let per_nummer = bester.is_some();
    for zid in w.spieler[&sid].sp.zauber.iter().filter(|_| !per_nummer) {
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
        let t = if technik { "Eine solche Technik beherrschst du nicht." } else { "Einen solchen Zauber kennst du nicht." };
        w.sende(sid, t);
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
        if !z.gilde.is_empty() && s.sp.gilde != z.gilde {
            let g = w.d.gilden.get(&z.gilde).map(|g| g.name.clone()).unwrap_or_default();
            w.sende(sid, &format!("Das können nur Mitglieder der {g}."));
            return;
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
        if s.sp.ausdauer < z.ausdauer_kosten {
            w.sende(sid, "Dafür bist du zu erschöpft.");
            return;
        }
        // Ein Waffenschlag braucht, dass man bereit ist
        if z.waffen_faktor > 0.0 && s.bereit_ab > jetzt {
            let rest = (s.bereit_ab - jetzt).as_secs_f64();
            w.sende(sid, &format!("Du bist noch nicht bereit ({rest:.1} s)."));
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
            // Ohne Ziel: der aktuelle Gegner
            let aktuell = w.spieler[&sid].kampf_ziel.filter(|m| w.monster.get(m).is_some_and(|m| m.raum == raum));
            let gefunden = if zielwort.is_empty() { aktuell } else { finde_monster(w, &raum, &zielwort) };
            match gefunden {
                Some(m) => ziel_monster = Some(m),
                None if zielwort.is_empty() => {
                    w.sende(sid, "Auf wen? (Gib ein Ziel an, z.B. ratte)");
                    return;
                }
                None => {
                    w.sende(sid, "Das ist nicht hier.");
                    return;
                }
            }
            let angreifbar =
                ziel_monster.and_then(|m| w.monster.get(&m)).and_then(|m| w.d.monster.get(&m.def)).is_some_and(|d| d.ist_angreifbar());
            if !angreifbar {
                w.sende(sid, "Hier wird nicht gekämpft.");
                return;
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
        s.sp.ausdauer -= z.ausdauer_kosten;
        if z.abklingzeit > 0 {
            s.abklingzeiten.insert(zid.clone(), jetzt + Duration::from_secs(z.abklingzeit));
        }
    }

    // Texte (eine Technik mit Waffenschlag meldet sich beim Schlag selbst)
    let waffenschlag = z.waffen_faktor > 0.0 && ziel_monster.is_some();
    if !waffenschlag {
        let (st_selbst, st_raum, st_ziel) = if z.ist_technik() {
            ("Du setzt {zauber} ein.", "{name} setzt {zauber} ein.", "{name} setzt {zauber} bei dir ein.")
        } else {
            ("Du sprichst {zauber}.", "{name} spricht {zauber}.", "{name} spricht {zauber} auf dich.")
        };
        let selbst = if z.text_selbst.is_empty() { st_selbst } else { &z.text_selbst };
        let im_raum = if z.text_raum.is_empty() { st_raum } else { &z.text_raum };
        let am_ziel = if z.text_ziel.is_empty() { st_ziel } else { &z.text_ziel };
        w.sende(sid, &format!("{{magenta}}{}{{/}}", ersetzen(selbst)));
        let mut ausser = vec![sid];
        for p in &ziel_spieler {
            if *p != sid {
                w.sende(*p, &format!("{{magenta}}{}{{/}}", ersetzen(am_ziel)));
                ausser.push(*p);
            }
        }
        w.sende_raum(&raum, &format!("{{magenta}}{}{{/}}", ersetzen(im_raum)), &ausser);
    }

    // Wirkungen
    if let Some(mid) = ziel_monster {
        // Auf den Gegner: Schaden, Gift (Schaden mit Dauer), Betäuben.
        // Alles andere (z.B. Heilen) wirkt auf den Zaubernden selbst.
        let sofort: i32 = z.wirkungen.iter().filter(|w| w.art == "schaden" && w.dauer <= 0.0).map(|w| w.wert).sum();
        if waffenschlag {
            let t = kampf::Technik {
                name: &z.name,
                faktor: z.waffen_faktor,
                zusatz: sofort,
                text_selbst: &z.text_selbst,
                text_raum: &z.text_raum,
            };
            kampf::technik(w, sid, mid, &t);
        } else if sofort > 0 {
            kampf::zauber_trifft(w, sid, mid, sofort, &z.fertigkeit);
        }
        for wi in &z.wirkungen {
            let lebt = w.monster.contains_key(&mid);
            match wi.art.as_str() {
                "schaden" if wi.dauer > 0.0 && lebt => {
                    crate::wirkung::auf_monster(w, mid, &z, wi, sid);
                    w.sende(sid, &format!("{} wirkt nun {} Sekunden lang.", z.name, wi.dauer.round()));
                }
                "betaeuben" if lebt => {
                    if let Some(m) = w.monster.get_mut(&mid) {
                        m.bereit_ab = m.bereit_ab.max(Instant::now()) + Duration::from_secs_f64(wi.wert.max(1) as f64);
                    }
                    let n = gross(&w.monster_name(mid));
                    w.sende(sid, &format!("{{hellgelb}}{n} ist benommen!{{/}}"));
                }
                "schaden" | "betaeuben" => {}
                _ if wi.dauer > 0.0 => crate::wirkung::auf_spieler(w, sid, &z, wi, sid),
                art => wirkung_anwenden(w, sid, art, wi.wert, &wi.raum),
            }
        }
        // Wer einen Gegner verzaubert, ist mit ihm im Kampf
        if w.monster.contains_key(&mid) {
            kampf::verwickeln(w, sid, mid);
        }
    } else {
        for p in ziel_spieler {
            for wi in &z.wirkungen {
                if wi.dauer > 0.0 {
                    crate::wirkung::auf_spieler(w, p, &z, wi, sid);
                } else {
                    wirkung_anwenden(w, p, &wi.art, wi.wert, &wi.raum);
                }
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

pub(crate) fn wirkung_anwenden(w: &mut Welt, sid: u64, art: &str, wert: i32, raum: &str) {
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
        "ausdauer" => {
            let vorher = s.sp.ausdauer;
            s.sp.ausdauer = (vorher + wert).min(s.sp.max_ausdauer);
            let plus = s.sp.ausdauer - vorher;
            s.sende(&format!("{{gelb}}Neue Kraft durchströmt dich. (+{plus} AP){{/}}"));
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
            s.sende("{grau}(Gegen andere Spieler wird hier nicht gekämpft.){/}");
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Baumeister-Befehle
// ---------------------------------------------------------------------------

fn admin_befehl(w: &mut Welt, sid: u64, befehl: &str, rest: &str) -> bool {
    match befehl {
        "rolle" => rolle_vergeben(w, sid, rest),
        "rauswerfen" | "kick" => {
            let (name, grund) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
            let Some(pid) = w.spieler.iter().find(|(_, s)| s.sp.name.eq_ignore_ascii_case(name)).map(|(id, _)| *id) else {
                w.sende(sid, "Dieser Spieler ist nicht online.");
                return true;
            };
            if pid == sid {
                w.sende(sid, "Dich selbst kannst du nicht hinauswerfen. Tippe 'ende'.");
                return true;
            }
            let name = w.spieler[&pid].sp.name.clone();
            let grund = if grund.trim().is_empty() { String::new() } else { format!(" Grund: {}", grund.trim()) };
            w.sende(pid, &format!("{{hellrot}}Du wurdest von einem Admin aus der Welt geworfen.{grund}{{/}}"));
            w.abmelden(pid);
            w.sende(sid, &format!("{name} wurde hinausgeworfen."));
            println!("{} hat {name} hinausgeworfen.{grund}", w.spieler[&sid].sp.name);
        }
        "verkuende" | "verkuenden" => {
            if rest.is_empty() {
                w.sende(sid, "Was willst du verkünden?");
            } else {
                let von = w.spieler[&sid].sp.name.clone();
                w.sende_alle(&format!("\n{{hellgelb}}*** Verkündung von {von}: {rest} ***{{/}}\n"), &[]);
            }
        }
        "gebegeld" | "geldgeben" => {
            let teile: Vec<&str> = rest.split_whitespace().collect();
            let (Some(name), Some(Ok(n))) = (teile.first(), teile.get(1).map(|z| z.parse::<u64>())) else {
                w.sende(sid, "Benutzung: gebegeld <spieler> <menge>");
                return true;
            };
            let Some(pid) = w.spieler.iter().find(|(_, s)| s.sp.name.eq_ignore_ascii_case(name)).map(|(id, _)| *id) else {
                w.sende(sid, "Dieser Spieler ist nicht online.");
                return true;
            };
            if let Some(s) = w.spieler.get_mut(&pid) {
                s.sp.geld += n;
            }
            w.sende(pid, &format!("{{hellgelb}}Du erhältst {}.{{/}}", crate::handel::betrag(w, n)));
            if pid != sid {
                w.sende(sid, "Erledigt.");
            }
        }
        "erfahrung" => {
            let teile: Vec<&str> = rest.split_whitespace().collect();
            let (Some(name), Some(Ok(ep))) = (teile.first(), teile.get(1).map(|z| z.parse::<u64>())) else {
                w.sende(sid, "Benutzung: erfahrung <spieler> <menge>");
                return true;
            };
            let Some(pid) = w.spieler.iter().find(|(_, s)| s.sp.name.eq_ignore_ascii_case(name)).map(|(id, _)| *id) else {
                w.sende(sid, "Dieser Spieler ist nicht online.");
                return true;
            };
            w.sende(pid, &format!("{{hellgruen}}Du erhältst {ep} Erfahrungspunkte.{{/}}"));
            kampf::erfahrung_geben(w, pid, ep);
            if pid != sid {
                w.sende(sid, "Erledigt.");
            }
        }
        "speichernalle" => {
            w.alle_speichern();
            w.sende(sid, &format!("{} Spieler gespeichert.", w.spieler.len()));
        }
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
    let mut t = format!("{{fett}}Spielstand von {}{{/}}{}\n\n", sp.voller_name(), w.spieler[&sid].rolle.marke());

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
    t.push_str(&format!(
        "  Ausdauer     {} {:>4}/{}\n",
        balken(sp.ausdauer, sp.max_ausdauer, 20, "gelb"),
        sp.ausdauer,
        sp.max_ausdauer
    ));
    let naechste = kampf::ep_fuer_naechste(w, sp.stufe);
    t.push_str(&format!(
        "\n  {{fett}}Stufe {}{{/}}      {} {:>4}/{} EP\n",
        sp.stufe,
        balken(sp.erfahrung as i32, naechste as i32, 20, "hellmagenta"),
        sp.erfahrung,
        naechste
    ));
    if sp.freie_punkte > 0 {
        t.push_str(&format!(
            "  {{hellgelb}}{} {} zu verteilen{{/}} {{grau}}(steigere <wert>){{/}}\n",
            sp.freie_punkte,
            if sp.freie_punkte == 1 { "Punkt" } else { "Punkte" }
        ));
    }
    t.push_str(&format!("  {{grau}}Siege: {}   Tode: {}{{/}}\n", sp.siege, sp.tode));
    t.push_str(&format!("  Geld: {{hellgelb}}{}{{/}}\n", crate::handel::betrag(w, sp.geld)));
    let mut herkunft = Vec::new();
    if let Some(r) = w.d.rassen.get(&sp.rasse) {
        herkunft.push(format!("Rasse: {}", r.name));
    }
    if let Some(g) = w.d.gilden.get(&sp.gilde) {
        herkunft.push(format!("Gilde: {}", g.name));
    }
    let berufe: Vec<String> = sp.berufe.iter().filter_map(|b| w.d.berufe.get(b).map(|b| b.name.clone())).collect();
    if !berufe.is_empty() {
        herkunft.push(format!("Beruf: {}", berufe.join(", ")));
    }
    if !herkunft.is_empty() {
        t.push_str(&format!("  {}\n", herkunft.join("   ")));
    }
    let effekte: Vec<String> = w.spieler[&sid].effekte.iter().map(|e| e.name.clone()).collect();
    if !effekte.is_empty() {
        t.push_str(&format!("  {{hellcyan}}Es wirkt: {}{{/}} {{grau}}(wirkungen){{/}}\n", effekte.join(", ")));
    }

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
            boni.remove(k);
            // Ausrüstung, Rasse und Zauber zusammen
            let bonus = kampf::wert(w, sid, k) - v;
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
    "farben", "karte", "kartengroesse", "kartenlage", "kurz", "breite", "prompt", "ankuendigungen", "npc", "autoangriff", "kampfzahlen",
    "titel", "beschreibung", "passwort",
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
    let lage = match e.karte_daneben.unwrap_or(w.d.welt.karte_daneben) {
        true => "daneben",
        false => "oben",
    };
    t.push_str(&zeile("kartenlage", lage, "Raumtext neben der Karte (daneben) oder darunter (oben)"));
    t.push_str(&zeile("kurz", an_aus(e.kurz), "beim Gehen keine Raumbeschreibung (schau zeigt sie)"));
    let breite = if e.breite_auto { format!("auto ({})", w.spieler[&sid].breite()) } else { e.breite.to_string() };
    t.push_str(&zeile("breite", &breite, "Zeilenbreite: auto (passt sich dem Fenster an) oder 40 bis 200"));
    t.push_str(&zeile("prompt", an_aus(e.prompt), "Eingabezeile mit Lebenspunkten und Mana"));
    t.push_str("\n{fett}Mitteilungen{/}\n");
    t.push_str(&zeile("ankuendigungen", an_aus(e.ankuendigungen), "melden, wenn Spieler kommen und gehen"));
    t.push_str(&zeile("npc", an_aus(e.npc_sprueche), "Sprüche von Bewohnern und Tieren"));
    t.push_str("\n{fett}Kampf{/}\n");
    t.push_str(&zeile("autoangriff", an_aus(e.autoangriff), "automatisch weiter angreifen, sobald du bereit bist"));
    t.push_str(&zeile("kampfzahlen", an_aus(e.kampfzahlen), "Schaden als Zahl anzeigen"));
    t.push_str("\n{fett}Deine Figur{/}\n");
    let titel = if sp.titel.is_empty() { "(keiner)".to_string() } else { sp.titel.clone() };
    t.push_str(&format!("  {{gelb}}{:<15}{{/}} {titel}\n", "titel"));
    let besch = if sp.beschreibung.is_empty() { "(keine)".to_string() } else { sp.beschreibung.clone() };
    t.push_str(&format!("  {{gelb}}{:<15}{{/}} {besch}\n", "beschreibung"));
    t.push_str(&format!("  {{gelb}}{:<15}{{/}} {{grau}}ändern mit: spielerbuch passwort <alt> <neu>{{/}}\n", "passwort"));
    {
        let p = crate::telnet::sperren(&w.spieler[&sid].proto);
        t.push_str(&format!(
            "\n{{fett}}Dein Programm{{/}}\n  {{gelb}}{:<15}{{/}} {}\n  {{gelb}}{:<15}{{/}} {}\n",
            "client",
            p.client_name(),
            "protokolle",
            p.aktiv()
        ));
    }
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
        "autoangriff" | "auto" => Some(&mut e.autoangriff),
        "kampfzahlen" | "zahlen" => Some(&mut e.kampfzahlen),
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
            "kartenlage" | "lage" => match normalisieren(wert).as_str() {
                "daneben" | "neben" | "rechts" | "seitlich" => {
                    e.karte_daneben = Some(true);
                    "Die Raumbeschreibung steht jetzt neben der Karte (wenn das Fenster breit genug ist).".into()
                }
                "oben" | "ueber" | "darueber" | "unten" | "darunter" => {
                    e.karte_daneben = Some(false);
                    "Die Karte steht jetzt über der Raumbeschreibung.".into()
                }
                _ => "Bitte daneben oder oben angeben.".into(),
            },
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
                    e.breite_auto = false;
                    format!("Texte werden jetzt auf {b} Zeichen umbrochen.")
                }
                _ if normalisieren(wert) == "auto" => {
                    e.breite_auto = true;
                    "Die Zeilenbreite passt sich jetzt deinem Fenster an (wenn dein Programm sie meldet).".into()
                }
                _ => "Bitte 'auto' oder eine Zahl von 40 bis 200 angeben.".into(),
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

/// rolle <name> <spieler|baumeister|admin> – funktioniert auch, wenn der Spieler offline ist.
fn rolle_vergeben(w: &mut Welt, sid: u64, rest: &str) {
    let teile: Vec<&str> = rest.split_whitespace().collect();
    let [name, rolle_text] = teile.as_slice() else {
        w.sende(sid, "Benutzung: rolle <spieler> spieler|baumeister|admin");
        return;
    };
    let Some(neu) = Rolle::aus_text(rolle_text) else {
        w.sende(sid, "Rollen: spieler, baumeister, admin");
        return;
    };
    if w.spieler[&sid].sp.name.eq_ignore_ascii_case(name) {
        w.sende(sid, "Deine eigene Rolle kannst du nicht ändern.");
        return;
    }
    let in_datei = w.d.welt.admins.iter().chain(w.d.welt.baumeister.iter()).any(|n| n.eq_ignore_ascii_case(name));

    let online = w.spieler.iter().find(|(_, s)| s.sp.name.eq_ignore_ascii_case(name)).map(|(id, _)| *id);
    let (anzeige, wirksam) = if let Some(pid) = online {
        let Some(s) = w.spieler.get_mut(&pid) else { return };
        s.sp.rolle = neu;
        let sp = s.sp.clone();
        let wirksam = w.rolle_von(&sp);
        if let Some(s) = w.spieler.get_mut(&pid) {
            s.rolle = wirksam;
        }
        w.speichern(&sp);
        w.sende(pid, &format!("{{hellgruen}}Deine Rolle ist jetzt: {}. Tippe 'hilfe' für deine Befehle.{{/}}", wirksam.name()));
        (sp.name, wirksam)
    } else {
        let verz = w.spieler_verzeichnis();
        let mut sp = match spieler::laden(&verz, name) {
            Ok(sp) => sp,
            Err(_) => {
                w.sende(sid, &format!("Eine Figur namens '{name}' gibt es nicht."));
                return;
            }
        };
        sp.rolle = neu;
        w.speichern(&sp);
        let wirksam = w.rolle_von(&sp);
        (sp.name, wirksam)
    };
    let mut t = format!("{anzeige} ist jetzt {}.", wirksam.name());
    if in_datei && wirksam != neu {
        t.push_str(" {grau}(Der Name steht in welt.toml unter admins/baumeister – dort muss er entfernt werden.){/}");
    }
    w.sende(sid, &t);
    println!("{} hat {anzeige} die Rolle {} gegeben.", w.spieler[&sid].sp.name, neu.name());
}

/// toete <monster> / angriff (ohne Ziel: weiter auf das aktuelle Ziel)
fn toeten(w: &mut Welt, sid: u64, rest: &str) {
    let rest = rest.trim();
    let rest = rest.strip_suffix(" an").unwrap_or(rest).trim();
    let raum = w.spieler[&sid].sp.raum.clone();
    if rest.is_empty() {
        let s = &w.spieler[&sid];
        match s.kampf_ziel {
            Some(_) if s.bereit_ab <= Instant::now() => {
                kampf::spieler_schlaegt(w, sid);
            }
            Some(_) => {
                let r = (s.bereit_ab - Instant::now()).as_secs_f64();
                w.sende(sid, &format!("Du bist noch nicht bereit ({r:.1} s)."));
            }
            None => w.sende(sid, "Wen willst du angreifen?"),
        }
        return;
    }
    if let Some(mid) = finde_monster(w, &raum, rest) {
        kampf::angreifen(w, sid, mid);
    } else if finde_spieler_im_raum(w, &raum, rest).is_some() {
        w.sende(sid, "Gegen andere Spieler wird hier nicht gekämpft.");
    } else {
        w.sende(sid, "So etwas siehst du hier nicht.");
    }
}
