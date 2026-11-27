//! Aufgaben (kleine, oft wiederholbare Aufträge) und Quests (größere Geschichten).
//!
//! Ein NPC (`geber`) bietet sie an, wenn man ihn nach einer Aufgabe fragt
//! ("sage hast du eine Aufgabe für mich"). Mit `annehmen` übernimmt man sie.
//! Jeder Schritt hat eine Art:
//!
//! | art      | erledigt, wenn ...                                      |
//! |----------|---------------------------------------------------------|
//! | toeten   | man `anzahl` Monster mit der ID `ziel` besiegt hat      |
//! | bringen  | man `anzahl` Gegenstände `ziel` beim NPC `an` abgibt    |
//! | besuchen | man den Raum `ziel` betritt                             |
//! | sprechen | man mit dem NPC `ziel` redet                            |
//!
//! Die Schritte werden der Reihe nach erledigt. Danach gibt man die Aufgabe
//! beim Geber ab (`abgeben` oder einfach mit ihm reden) und bekommt die Belohnung.

use crate::daten::{QuestDef, SchrittDef};
use crate::handel::{betrag, im_inventar};
use crate::spieler::QuestStand;
use crate::text::{gross, normalisieren};
use crate::welt::Welt;

fn art_name(q: &QuestDef) -> &'static str {
    if q.art == "aufgabe" {
        "Aufgabe"
    } else {
        "Quest"
    }
}

/// Text eines Schritts, z.B. "Töte 3x eine Ratte"
pub fn schritt_text(w: &Welt, q: &QuestDef, sch: &SchrittDef) -> String {
    if !sch.text.is_empty() {
        return sch.text.clone();
    }
    let mal = if sch.anzahl > 1 { format!("{}x ", sch.anzahl) } else { String::new() };
    let npc = |id: &str| w.d.monster.get(id).map(|m| m.name.clone()).unwrap_or_else(|| id.to_string());
    let npc_dativ = |id: &str| dativ(w, id);
    match sch.art.as_str() {
        "toeten" => format!("Besiege {mal}{}", npc(&sch.ziel)),
        "bringen" => {
            let an = if sch.an.is_empty() { &q.geber } else { &sch.an };
            format!("Bringe {mal}{} zu {}", w.d.gegenstand_name(&sch.ziel), npc_dativ(an))
        }
        "besuchen" => format!("Gehe zu: {}", w.d.raum(&sch.ziel).map(|r| r.name.clone()).unwrap_or_else(|| sch.ziel.clone())),
        "sprechen" => format!("Sprich mit {}", npc_dativ(&sch.ziel)),
        "skript" => format!("Erledige: {}", sch.ziel),
        a => a.to_string(),
    }
}

/// "dem Wirt" (Feld dativ), sonst der Name
fn dativ(w: &Welt, id: &str) -> String {
    match w.d.monster.get(id) {
        Some(m) if !m.dativ.is_empty() => m.dativ.clone(),
        Some(m) => m.name.clone(),
        None => id.to_string(),
    }
}

/// Darf der Spieler die Quest gerade annehmen?
fn verfuegbar(w: &Welt, sid: u64, q: &QuestDef) -> bool {
    let sp = &w.spieler[&sid].sp;
    !sp.quests.contains_key(&q.id)
        && (q.wiederholbar || !sp.erledigt.contains_key(&q.id))
        && sp.stufe >= q.mindest_stufe
        && q.voraussetzungen.iter().all(|v| sp.erledigt.contains_key(v))
        && (q.gilde.is_empty() || sp.gilde == q.gilde)
}

/// Quests, die NPCs im Raum anbieten: (NPC-Name, Quest)
fn angebote_im_raum(w: &Welt, sid: u64) -> Vec<(String, QuestDef)> {
    let raum = &w.spieler[&sid].sp.raum;
    let mut v = Vec::new();
    for mid in w.monster_im_raum(raum) {
        let Some(d) = w.monster.get(&mid).and_then(|m| w.d.monster.get(&m.def)) else { continue };
        for q in w.d.quests.values().filter(|q| q.geber == d.id && verfuegbar(w, sid, q)) {
            v.push((d.name.clone(), q.clone()));
        }
    }
    v
}

fn npc_sagt(w: &Welt, sid: u64, npc: &str, text: &str) {
    let raum = w.spieler[&sid].sp.raum.clone();
    let text = text.replace("{spieler}", &w.spieler[&sid].sp.name);
    let zeile = match text.trim().strip_prefix('*') {
        Some(h) => format!("{} {}", gross(npc), h.trim()),
        None => format!("{{hellcyan}}{} sagt: „{}“{{/}}", gross(npc), text.trim()),
    };
    w.sende_raum(&raum, &zeile, &[]);
}

/// Der Spieler hat mit einem NPC gesprochen (sage, frage, rede).
/// `nach_aufgabe`: Er hat nach einer Aufgabe gefragt (oder ein Gespräch begonnen).
pub fn angesprochen(w: &mut Welt, sid: u64, mid: u64, nach_aufgabe: bool) {
    let Some(npc) = w.monster.get(&mid).and_then(|m| w.d.monster.get(&m.def)).cloned() else { return };
    fortschritt(w, sid, "sprechen", &npc.id, 1);
    let abgegeben = abgeben_bei(w, sid, &npc.id, &npc.name);
    if !nach_aufgabe || abgegeben {
        return;
    }
    let angebote: Vec<QuestDef> = w.d.quests.values().filter(|q| q.geber == npc.id && verfuegbar(w, sid, q)).cloned().collect();
    // Nummern wie bei "annehmen <nr>" (über alle Angebote im Raum)
    let alle: Vec<String> = angebote_im_raum(w, sid).into_iter().map(|(_, q)| q.id).collect();
    for q in &angebote {
        let text = if q.angebot.is_empty() { format!("Ich hätte da etwas für dich: {}.", q.name) } else { q.angebot.clone() };
        npc_sagt(w, sid, &npc.name, &text);
        let nr = alle.iter().position(|id| *id == q.id).map(|i| i + 1).unwrap_or(1);
        w.sende(sid, &format!("{{grau}}({}: {} – annehmen mit {{gelb}}annehmen {nr}{{/}}{{grau}}){{/}}", art_name(q), q.name));
    }
    // Läuft schon etwas bei diesem NPC? Dann daran erinnern.
    if angebote.is_empty() {
        let laufend: Vec<QuestDef> = w.spieler[&sid].sp.quests.keys().filter_map(|id| w.d.quests.get(id)).filter(|q| q.geber == npc.id).cloned().collect();
        for q in laufend {
            if let Some(st) = w.spieler[&sid].sp.quests.get(&q.id) {
                if let Some(sch) = q.schritte.get(st.schritt) {
                    w.sende(sid, &format!("{{grau}}({}: {} – {}){{/}}", art_name(&q), q.name, schritt_text(w, &q, sch)));
                }
            }
        }
    }
}

/// Befehl "annehmen [quest]"
pub fn annehmen(w: &mut Welt, sid: u64, rest: &str) {
    let angebote = angebote_im_raum(w, sid);
    if angebote.is_empty() {
        w.sende(sid, "Hier bietet dir niemand eine Aufgabe an. Frag die Leute: sage hast du eine aufgabe?");
        return;
    }
    let such = normalisieren(rest);
    let gewaehlt = if let Some(n) = crate::gemeinschaft::nummer(rest) {
        angebote.into_iter().nth(n)
    } else if such.is_empty() {
        if angebote.len() > 1 {
            let mut t = String::from("Welche Aufgabe willst du annehmen?\n");
            for (i, (npc, q)) in angebote.iter().enumerate() {
                t.push_str(&format!("  {{gelb}}{:>2}.{{/}} {} {{grau}}({}, von {}){{/}}\n", i + 1, q.name, art_name(q), npc));
            }
            t.push_str("Tippe: annehmen <nr>");
            w.sende(sid, &t);
            return;
        }
        angebote.into_iter().next()
    } else {
        angebote.into_iter().find(|(_, q)| normalisieren(&q.name).contains(&such) || normalisieren(&q.id) == such)
    };
    let Some((npc, q)) = gewaehlt else {
        w.sende(sid, "Diese Aufgabe wird hier nicht angeboten.");
        return;
    };
    if let Some(s) = w.spieler.get_mut(&sid) {
        s.sp.quests.insert(q.id.clone(), QuestStand::default());
    }
    let text = if q.annahme.is_empty() { "Viel Glück!".to_string() } else { q.annahme.clone() };
    npc_sagt(w, sid, &npc, &text);
    w.sende(sid, &format!("{{hellgelb}}Neue {}: {}{{/}}", art_name(&q), q.name));
    if let Some(sch) = q.schritte.first() {
        w.sende(sid, &format!("  → {}", schritt_text(w, &q, sch)));
    }
    // Vielleicht ist der erste Schritt schon erledigt (z.B. Gegenstände dabei)
    raum_pruefen(w, sid);
    if q.schritte.is_empty() {
        weiter(w, sid, &q.id);
    }
    let sp = w.spieler[&sid].sp.clone();
    w.speichern(&sp);
}

/// Befehl "aufgaben [erledigt]"
pub fn anzeigen(w: &Welt, sid: u64, rest: &str) {
    let sp = &w.spieler[&sid].sp;
    if normalisieren(rest).starts_with("erledigt") {
        if sp.erledigt.is_empty() {
            w.sende(sid, "Du hast noch keine Aufgabe erledigt.");
            return;
        }
        let mut t = String::from("{fett}Erledigt:{/}\n");
        for (id, mal) in &sp.erledigt {
            let name = w.d.quests.get(id).map(|q| q.name.clone()).unwrap_or_else(|| id.clone());
            let wie_oft = if *mal > 1 { format!(" ({mal}x)") } else { String::new() };
            t.push_str(&format!("  {name}{wie_oft}\n"));
        }
        w.sende(sid, t.trim_end());
        return;
    }
    if sp.quests.is_empty() {
        w.sende(sid, "Du hast gerade keine Aufgaben. Frag die Leute: sage hast du eine aufgabe?\n{grau}(aufgaben erledigt zeigt, was du schon geschafft hast){/}");
        return;
    }
    let mut t = String::from("{fett}Deine Aufgaben:{/}\n");
    for (nr, (id, st)) in sp.quests.iter().enumerate() {
        let Some(q) = w.d.quests.get(id) else { continue };
        t.push_str(&format!("{{gelb}}{:>2}.{{/}} {{hellgelb}}{}{{/}} {{grau}}({}){{/}}\n", nr + 1, q.name, art_name(q)));
        if !q.beschreibung.is_empty() {
            t.push_str(&format!("  {{grau}}{}{{/}}\n", q.beschreibung.trim()));
        }
        for (i, sch) in q.schritte.iter().enumerate() {
            let (marke, extra) = match i.cmp(&st.schritt) {
                std::cmp::Ordering::Less => ("{gruen}✓{/}", String::new()),
                std::cmp::Ordering::Equal if sch.anzahl > 1 => ("→", format!(" ({}/{})", st.zaehler, sch.anzahl)),
                std::cmp::Ordering::Equal => ("→", String::new()),
                std::cmp::Ordering::Greater => ("{grau}·{/}", String::new()),
            };
            t.push_str(&format!("  {marke} {}{extra}\n", schritt_text(w, q, sch)));
        }
        if st.schritt >= q.schritte.len() {
            let geber = dativ(w, &q.geber);
            t.push_str(&format!("  → {{hellgruen}}Fertig! Kehre zu {geber} zurück.{{/}}\n"));
        }
    }
    t.push_str("{grau}(abbrechen <nr> gibt eine Aufgabe auf){/}");
    w.sende(sid, &t);
}

/// Befehl "abbrechen <quest>"
pub fn abbrechen(w: &mut Welt, sid: u64, rest: &str) {
    let such = normalisieren(rest);
    let mut laufend = w.spieler[&sid].sp.quests.keys();
    let id = match crate::gemeinschaft::nummer(rest) {
        Some(n) => laufend.nth(n).cloned(),
        None => laufend
        .find(|id| !such.is_empty() && w.d.quests.get(*id).is_some_and(|q| normalisieren(&q.name).contains(&such) || normalisieren(&q.id) == such))
        .cloned(),
    };
    let Some(id) = id else {
        w.sende(sid, "Welche Aufgabe willst du abbrechen? (aufgaben zeigt sie mit Nummern)");
        return;
    };
    let name = w.d.quests.get(&id).map(|q| q.name.clone()).unwrap_or_default();
    if let Some(s) = w.spieler.get_mut(&sid) {
        s.sp.quests.remove(&id);
    }
    w.sende(sid, &format!("Du gibst die Aufgabe „{name}“ auf."));
}

/// Befehl "abgeben": Aufgaben bei NPCs im Raum abgeben (und Gegenstände übergeben).
pub fn abgeben(w: &mut Welt, sid: u64) {
    let raum = w.spieler[&sid].sp.raum.clone();
    let npcs: Vec<(String, String)> = w
        .monster_im_raum(&raum)
        .into_iter()
        .filter_map(|mid| w.monster.get(&mid).and_then(|m| w.d.monster.get(&m.def)).map(|d| (d.id.clone(), d.name.clone())))
        .collect();
    let mut etwas = false;
    for (id, name) in npcs {
        etwas |= abgeben_bei(w, sid, &id, &name);
    }
    if !etwas {
        w.sende(sid, "Hier kannst du nichts abgeben.");
    }
}

/// Gegenstände übergeben und fertige Aufgaben abschließen. true = es ist etwas passiert.
fn abgeben_bei(w: &mut Welt, sid: u64, npc_id: &str, npc_name: &str) -> bool {
    let mut etwas = false;
    let ids: Vec<String> = w.spieler[&sid].sp.quests.keys().cloned().collect();
    for qid in ids {
        let Some(q) = w.d.quests.get(&qid).cloned() else { continue };
        // Bringen: der aktuelle Schritt verlangt Gegenstände für diesen NPC
        loop {
            let Some(st) = w.spieler[&sid].sp.quests.get(&qid).cloned() else { break };
            let Some(sch) = q.schritte.get(st.schritt) else { break };
            let an = if sch.an.is_empty() { &q.geber } else { &sch.an };
            if sch.art != "bringen" || an != npc_id {
                break;
            }
            let fehlt = sch.anzahl.saturating_sub(st.zaehler);
            let hat = im_inventar(w, sid, &sch.ziel).min(fehlt);
            if hat == 0 {
                break;
            }
            if let Some(s) = w.spieler.get_mut(&sid) {
                for _ in 0..hat {
                    if let Some(pos) = s.sp.inventar.iter().position(|g| *g == sch.ziel) {
                        s.sp.inventar.remove(pos);
                    }
                }
            }
            let ding = w.d.gegenstand_name(&sch.ziel);
            let _ = npc_name;
            w.sende(sid, &format!("Du gibst {} {hat}x {ding}.", dativ(w, npc_id)));
            etwas = true;
            let vorher = st.schritt;
            zaehlen(w, sid, &q, hat);
            if w.spieler[&sid].sp.quests.get(&qid).is_none_or(|s| s.schritt == vorher) {
                break;
            }
        }
        // Fertig und beim Geber?
        let fertig = w.spieler[&sid].sp.quests.get(&qid).is_some_and(|st| st.schritt >= q.schritte.len());
        if fertig && q.geber == npc_id {
            abschliessen(w, sid, &q, npc_name);
            etwas = true;
        }
    }
    etwas
}

/// Fortschritt melden, z.B. ("toeten", "ratte", 1) oder ("besuchen", "keller", 1).
pub fn fortschritt(w: &mut Welt, sid: u64, art: &str, ziel: &str, n: u32) {
    let Some(s) = w.spieler.get(&sid) else { return };
    let passend: Vec<QuestDef> = s
        .sp
        .quests
        .iter()
        .filter_map(|(id, st)| {
            let q = w.d.quests.get(id)?;
            let sch = q.schritte.get(st.schritt)?;
            (sch.art == art && sch.ziel == ziel).then(|| q.clone())
        })
        .collect();
    for q in passend {
        zaehlen(w, sid, &q, n);
    }
}

/// Zählt im aktuellen Schritt weiter und geht ggf. zum nächsten.
fn zaehlen(w: &mut Welt, sid: u64, q: &QuestDef, n: u32) {
    let Some(st) = w.spieler.get_mut(&sid).and_then(|s| s.sp.quests.get_mut(&q.id)) else { return };
    let Some(sch) = q.schritte.get(st.schritt) else { return };
    st.zaehler += n;
    if st.zaehler < sch.anzahl {
        let (z, a) = (st.zaehler, sch.anzahl);
        w.sende(sid, &format!("{{gelb}}{}: {} ({z}/{a}){{/}}", q.name, schritt_text(w, q, sch)));
        return;
    }
    w.sende(sid, &format!("{{hellgelb}}{}: ✓ {}{{/}}", q.name, schritt_text(w, q, sch)));
    weiter(w, sid, &q.id);
}

/// Zum nächsten Schritt – oder fertig.
fn weiter(w: &mut Welt, sid: u64, qid: &str) {
    let Some(q) = w.d.quests.get(qid).cloned() else { return };
    let Some(st) = w.spieler.get_mut(&sid).and_then(|s| s.sp.quests.get_mut(qid)) else { return };
    if st.schritt < q.schritte.len() {
        st.schritt += 1;
        st.zaehler = 0;
    }
    let schritt = st.schritt;
    match q.schritte.get(schritt) {
        Some(sch) => {
            w.sende(sid, &format!("  → {}", schritt_text(w, &q, sch)));
            // Ein Besuch-Schritt für den Raum, in dem man steht, ist sofort erledigt
            raum_pruefen(w, sid);
        }
        None if q.abgabe => {
            // Steht der Geber schon neben einem, wird gleich abgegeben – dann keine Erinnerung
            let raum = w.spieler[&sid].sp.raum.clone();
            let da = w.monster_im_raum(&raum).iter().any(|m| w.monster.get(m).is_some_and(|m| m.def == q.geber));
            if da {
                return;
            }
            let geber = dativ(w, &q.geber);
            w.sende(sid, &format!("{{hellgruen}}Alles erledigt! Kehre zu {geber} zurück.{{/}}"));
        }
        None => abschliessen(w, sid, &q, ""),
    }
}

/// Ist ein Besuch-Schritt für den aktuellen Raum fällig?
pub fn raum_pruefen(w: &mut Welt, sid: u64) {
    let Some(raum) = w.spieler.get(&sid).map(|s| s.sp.raum.clone()) else { return };
    fortschritt(w, sid, "besuchen", &raum, 1);
}

/// Belohnung geben und als erledigt eintragen.
fn abschliessen(w: &mut Welt, sid: u64, q: &QuestDef, npc_name: &str) {
    if let Some(s) = w.spieler.get_mut(&sid) {
        s.sp.quests.remove(&q.id);
        *s.sp.erledigt.entry(q.id.clone()).or_insert(0) += 1;
    }
    if !npc_name.is_empty() {
        let text = if q.abschluss.is_empty() { "Hab vielen Dank!".to_string() } else { q.abschluss.clone() };
        npc_sagt(w, sid, npc_name, &text);
    }
    w.sende(sid, &format!("\n{{fett}}{{hellgelb}}*** {} erledigt: {} ***{{/}}", art_name(q), q.name));
    let b = q.belohnung.clone();
    let mut erhalten = Vec::new();
    if b.geld > 0 {
        erhalten.push(betrag(w, b.geld));
    }
    for g in &b.gegenstaende {
        erhalten.push(w.d.gegenstand_name(g));
    }
    let mut fertigkeiten = Vec::new();
    for (f, n) in &b.fertigkeiten {
        if let Some(def) = w.d.fertigkeiten.get(f) {
            fertigkeiten.push((def.id.clone(), def.name.clone(), def.max_stufe, *n));
        }
    }
    let zauber: Vec<(String, String)> = b.zauber.iter().filter_map(|z| w.d.zauber.get(z).map(|d| (d.id.clone(), d.name.clone()))).collect();
    if let Some(s) = w.spieler.get_mut(&sid) {
        s.sp.geld += b.geld;
        s.sp.inventar.extend(b.gegenstaende.iter().cloned());
        for (id, name, max, n) in fertigkeiten {
            let e = s.sp.fertigkeiten.entry(id).or_insert(0);
            *e = (*e + n).min(max);
            erhalten.push(format!("{name} +{n}"));
        }
        for (id, name) in zauber {
            if !s.sp.zauber.contains(&id) {
                s.sp.zauber.push(id);
                erhalten.push(format!("Zauber {name}"));
            }
        }
        if !b.titel.is_empty() {
            s.sp.titel = b.titel.clone();
            erhalten.push(format!("Titel „{}“", b.titel));
        }
    }
    if !erhalten.is_empty() {
        w.sende(sid, &format!("{{hellgruen}}Belohnung: {}{{/}}", erhalten.join(", ")));
    }
    if b.erfahrung > 0 {
        w.sende(sid, &format!("{{hellgruen}}Du erhältst {} Erfahrungspunkte.{{/}}", b.erfahrung));
        crate::kampf::erfahrung_geben(w, sid, b.erfahrung);
    }
    let sp = w.spieler[&sid].sp.clone();
    w.speichern(&sp);
}
