//! Spieltests: Eine echte Welt (Kopie der Beispielwelt), ein Spieler tippt
//! Befehle, geprüft wird, was er zu lesen bekommt.

use crate::befehle;
use crate::telnet::{Ausgabe, Protokolle};
use crate::welt::Welt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

static ZAEHLER: AtomicU32 = AtomicU32::new(0);

struct Test {
    w: Welt,
    sid: u64,
    rx: UnboundedReceiver<Ausgabe>,
    ordner: PathBuf,
}

impl Drop for Test {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.ordner);
    }
}

fn kopieren(von: &std::path::Path, nach: &std::path::Path) {
    std::fs::create_dir_all(nach).unwrap();
    for e in std::fs::read_dir(von).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            kopieren(&p, &nach.join(e.file_name()));
        } else {
            std::fs::copy(&p, nach.join(e.file_name())).unwrap();
        }
    }
}

impl Test {
    /// Neue Welt aus der Beispielwelt, ein Spieler steht im Raum `raum`
    fn neu(raum: &str) -> Test {
        let ordner = std::env::temp_dir().join(format!("rmengine_spieltest_{}_{}", std::process::id(), ZAEHLER.fetch_add(1, Ordering::Relaxed)));
        let _ = std::fs::remove_dir_all(&ordner);
        kopieren(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("daten"), &ordner.join("daten"));
        let daten = ordner.join("daten");
        let d = crate::daten::laden(&daten).expect("Beispielwelt lädt");
        let mut w = Welt::neu(d, &daten);
        // Keine Monster, die dazwischenfunken (außer der Test will welche)
        w.monster.retain(|_, m| w.d.monster.get(&m.def).is_some_and(|d| d.ist_npc));
        let mut sp = w.neuer_spieler("Tester", "geheim");
        sp.raum = raum.to_string();
        sp.einstellungen.karte = false;
        sp.farben = false;
        let (tx, rx) = unbounded_channel();
        let sid = w.anmelden(sp, tx, Arc::new(Mutex::new(Protokolle::default()))).expect("anmelden");
        let mut t = Test { w, sid, rx, ordner };
        t.lesen();
        t
    }

    fn lesen(&mut self) -> String {
        let mut t = String::new();
        while let Ok(a) = self.rx.try_recv() {
            if let Ausgabe::Daten(d) = a {
                t.push_str(&String::from_utf8_lossy(&d));
            }
        }
        t.replace('\r', "")
    }

    /// Einen Befehl tippen und lesen, was zurückkommt
    fn tippe(&mut self, befehl: &str) -> String {
        befehle::ausfuehren(&mut self.w, self.sid, befehl);
        self.lesen()
    }

    fn geld(&self) -> u64 {
        self.w.spieler[&self.sid].sp.geld
    }

    fn gib(&mut self, id: &str) {
        self.w.spieler.get_mut(&self.sid).unwrap().sp.inventar.push(id.to_string());
    }

    fn hat(&self, id: &str) -> bool {
        self.w.spieler[&self.sid].sp.inventar.iter().any(|g| g == id)
    }

    fn raum(&self) -> String {
        self.w.spieler[&self.sid].sp.raum.clone()
    }
}

#[test]
fn falltuer_oeffnen_und_schliessen() {
    let mut t = Test::neu("taverne");
    assert!(t.tippe("schau").contains("unten (Falltür zu)"));
    assert!(t.tippe("runter").contains("Die Falltür ist geschlossen."));
    assert_eq!(t.raum(), "taverne");
    assert!(t.tippe("oeffne luke").contains("Du öffnest die Falltür."));
    t.tippe("runter");
    assert_eq!(t.raum(), "keller");
    // Von unten ist es dieselbe Tür
    assert!(t.tippe("schliesse falltuer").contains("Du schließt die Falltür."));
    assert!(t.tippe("hoch").contains("geschlossen"));
    assert!(t.tippe("schliesse tuer auf").contains("kein Schloss"));
}

#[test]
fn geheimtuer_hinter_dem_fass() {
    let mut t = Test::neu("keller");
    let vorher = t.tippe("schau");
    assert!(!vorher.contains("Westen"), "Geheimtür darf man nicht sehen: {vorher}");
    assert!(t.tippe("west").contains("Dorthin kannst du nicht gehen"));
    assert!(t.tippe("schieb das fass").contains("Tür zum Vorschein"));
    assert!(t.tippe("schau").contains("Westen"));
    let ep = t.w.spieler[&t.sid].sp.erfahrung;
    assert!(t.tippe("west").contains("verborgene Kammer"));
    assert!(t.w.spieler[&t.sid].sp.erfahrung > ep || t.w.spieler[&t.sid].sp.stufe > 1, "Erfahrung für die Entdeckung");
    // Von innen sieht man den Ausgang immer
    assert!(t.tippe("schau").contains("Osten"));
    // Nach dem Auffüllen ist die Tür wieder zu
    t.tippe("ost");
    t.w.zuruecksetzen();
    assert!(!t.tippe("schau").contains("Westen"));
}

#[test]
fn kriech_unter_den_tisch_nur_einmal() {
    let mut t = Test::neu("taverne");
    let geld = t.geld();
    assert!(t.tippe("kriech").contains("Wohin willst du kriechen"));
    assert!(t.tippe("kriech unter den tisch").contains("Silbermünze"));
    assert_eq!(t.geld(), geld + 10);
    assert!(t.tippe("kriech unter den tisch").contains("Krümeln"));
    assert_eq!(t.geld(), geld + 10);
    // In einem anderen Raum gibt es den Befehl nicht
    let mut t = Test::neu("dorfplatz");
    assert!(t.tippe("kriech unter den tisch").contains("Wie bitte"));
}

#[test]
fn wirt_nimmt_rattenschwanz_aber_kein_brot() {
    let mut t = Test::neu("taverne");
    let geld = t.geld();
    t.gib("rattenschwanz");
    let a = t.tippe("gib rattenschwanz an wirt");
    assert!(a.contains("Wieder eine weniger"), "{a}");
    assert!(!t.hat("rattenschwanz"));
    assert_eq!(t.geld(), geld + 5);
    t.gib("brot");
    assert!(t.tippe("gib brot wirt").contains("Was soll ich denn damit"));
    assert!(t.hat("brot"));
    // Ein Skript übernimmt die Antwort auf "geheim"
    assert!(t.tippe("frage wirt nach geheimnis").contains("Großvater"));
}

#[test]
fn zauberstab_skript_nur_wenn_man_ihn_hat() {
    let mut t = Test::neu("dorfplatz");
    assert!(t.tippe("schwinge stab").contains("Wie bitte"));
    t.gib("zauberstab");
    assert!(t.tippe("schwinge stab").contains("bunte Funken"));
    assert!(t.tippe("anlegen zauberstab").contains("Kribbeln"));
}

#[test]
fn leiche_mit_beute_und_pluenderrecht() {
    let mut t = Test::neu("keller");
    let mid = t.w.monster_erschaffen("ratte", "keller").expect("Ratte");
    t.w.monster.get_mut(&mid).unwrap().ziele.push(t.sid);
    crate::kampf::monster_stirbt(&mut t.w, mid);
    let a = t.tippe("schau leiche");
    assert!(a.contains("Kadaver"), "{a}");
    // Beute steckt drin (Rattenschwanz mit 50 % – also hineinlegen, damit der Test sicher ist)
    t.w.leichen[0].inhalt.push("apfel".into());
    assert!(t.tippe("nimm alles aus leiche").contains("Du nimmst"));
    assert!(t.hat("apfel"));
}

#[test]
fn gegenstaende_am_boden_bleiben_beim_neuladen() {
    let mut t = Test::neu("gaestezimmer");
    t.gib("zauberstab");
    t.tippe("lege zauberstab");
    t.w.neuladen().expect("neuladen");
    assert!(t.tippe("schau").contains("Zauberstab"));
    // ... und beim Neustart (gespeichert und wieder geladen)
    t.w.alle_speichern();
    let d = crate::daten::laden(&t.w.daten_ordner).unwrap();
    let w2 = Welt::neu(d, &t.w.daten_ordner.clone());
    assert!(w2.boden.get("gaestezimmer").is_some_and(|b| b.iter().any(|g| g == "zauberstab")));
}

#[test]
fn haende_begrenzen_was_man_haelt() {
    let mut t = Test::neu("dorfplatz");
    for g in ["dolch", "holzschild", "fackel"] {
        t.gib(g);
    }
    t.tippe("anlegen dolch");
    t.tippe("anlegen holzschild");
    assert!(t.tippe("anlegen fackel").contains("keine Hand mehr frei"));
    t.tippe("ausziehen dolch");
    t.tippe("ausziehen holzschild");
    t.gib("streitaxt");
    assert!(t.tippe("anlegen streitaxt").contains("Du legst"));
    assert!(t.tippe("anlegen fackel").contains("keine Hand mehr frei"));
}

#[test]
fn passwort_mit_argon2_und_alte_bleiben_gueltig() {
    let t = Test::neu("dorfplatz");
    let sp = t.w.neuer_spieler("Neu", "geheim");
    assert!(sp.passwort_hash.starts_with("$argon2"));
    assert!(sp.passwort_stimmt("geheim") && !sp.passwort_stimmt("falsch"));
    let mut alt = sp.clone();
    alt.salz = "abc".into();
    alt.passwort_hash = crate::spieler::passwort_hash("geheim", "abc");
    assert!(alt.passwort_veraltet() && alt.passwort_stimmt("geheim"));
}
