//! Der laufende Zustand der Welt: wer ist wo, was liegt herum, was tun die Monster.

use crate::daten::{self, Daten};
use crate::spieler::{self, Rolle, SpielerDaten};
use crate::text::{self, gross};
use rand::seq::SliceRandom;
use rand::Rng;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::sync::mpsc::UnboundedSender;

pub type Sender = UnboundedSender<Vec<u8>>;

/// Ein eingeloggter Spieler.
pub struct Sitzung {
    pub sp: SpielerDaten,
    pub sender: Sender,
    /// Die tatsächliche Rolle: gespeicherte Rolle oder welt.toml, je nachdem was höher ist
    pub rolle: Rolle,
    pub abklingzeiten: HashMap<String, Instant>,
}

impl Sitzung {
    /// Darf die Welt bauen (Baumeister oder Admin)?
    pub fn baut(&self) -> bool {
        self.rolle >= Rolle::Baumeister
    }

    pub fn sende(&self, t: &str) {
        let _ = self.sender.send(text::fuer_telnet(t, self.sp.farben, true));
    }
    pub fn sende_ohne_umbruch(&self, t: &str) {
        let _ = self.sender.send(text::fuer_telnet(t, self.sp.farben, false));
    }
}

/// Ein Monster, das gerade in der Welt herumläuft.
pub struct Monster {
    pub def: String,
    pub raum: String,
    /// Der Raum, in dem es erschaffen wurde.
    pub heimat: String,
    pub lebenspunkte: i32,
    pub mana: i32,
}

/// Gespeicherte Rolle – aber wer in welt.toml unter admins/baumeister steht,
/// hat mindestens diese Rolle (so gibt es immer einen ersten Admin).
fn rolle_berechnen(d: &Daten, sp: &SpielerDaten) -> Rolle {
    let steht_in = |liste: &[String]| liste.iter().any(|n| n.eq_ignore_ascii_case(&sp.name));
    let aus_datei = if steht_in(&d.welt.admins) {
        Rolle::Admin
    } else if steht_in(&d.welt.baumeister) {
        Rolle::Baumeister
    } else {
        Rolle::Spieler
    };
    sp.rolle.max(aus_datei)
}

pub struct Welt {
    pub d: Daten,
    pub daten_ordner: PathBuf,
    /// Gegenstände, die in den Räumen am Boden liegen.
    pub boden: HashMap<String, Vec<String>>,
    pub monster: BTreeMap<u64, Monster>,
    pub spieler: BTreeMap<u64, Sitzung>,
    naechste_id: u64,
    takte: u64,
}

impl Welt {
    pub fn neu(d: Daten, daten_ordner: &Path) -> Welt {
        let mut w = Welt {
            d,
            daten_ordner: daten_ordner.to_path_buf(),
            boden: HashMap::new(),
            monster: BTreeMap::new(),
            spieler: BTreeMap::new(),
            naechste_id: 1,
            takte: 0,
        };
        w.bevoelkern();
        w
    }

    pub fn neue_id(&mut self) -> u64 {
        self.naechste_id += 1;
        self.naechste_id
    }

    /// Legt alle Gegenstände und Monster so an, wie es in den Raumdaten steht.
    fn bevoelkern(&mut self) {
        self.boden.clear();
        self.monster.clear();
        let mut monster = Vec::new();
        for (raum, gegenstaende, liste) in self.d.bevoelkerung() {
            self.boden.insert(raum.clone(), gegenstaende);
            for m in liste {
                monster.push((m, raum.clone()));
            }
        }
        for (m, raum) in monster {
            self.monster_erschaffen(&m, &raum);
        }
    }

    pub fn monster_erschaffen(&mut self, def: &str, raum: &str) -> Option<u64> {
        let m = self.d.monster.get(def)?;
        let monster = Monster {
            def: def.to_string(),
            raum: raum.to_string(),
            heimat: raum.to_string(),
            lebenspunkte: m.lebenspunkte,
            mana: m.mana,
        };
        let id = self.neue_id();
        self.monster.insert(id, monster);
        Some(id)
    }

    pub fn monster_name(&self, id: u64) -> String {
        self.monster
            .get(&id)
            .and_then(|m| self.d.monster.get(&m.def))
            .map(|d| d.name.clone())
            .unwrap_or_else(|| "etwas".into())
    }

    pub fn monster_akk(&self, id: u64) -> String {
        match self.monster.get(&id).and_then(|m| self.d.monster.get(&m.def)) {
            Some(d) if !d.akkusativ.is_empty() => d.akkusativ.clone(),
            _ => self.monster_name(id),
        }
    }

    /// Lädt alle Weltdaten neu von der Festplatte – ohne Neustart des Servers.
    pub fn neuladen(&mut self) -> Result<Vec<String>, Vec<String>> {
        let neu = daten::laden(&self.daten_ordner)?;
        let warnungen = neu.warnungen.clone();
        self.d = neu;
        self.bevoelkern();
        let start = self.d.welt.startraum.clone();
        let d = &self.d;
        for s in self.spieler.values_mut() {
            if !d.raum_existiert(&s.sp.raum) {
                s.sp.raum = start.clone();
            }
            s.sp.inventar.retain(|g| d.gegenstaende.contains_key(g));
            s.sp.ausruestung.retain(|_, g| d.gegenstaende.contains_key(g));
            s.sp.fertigkeiten.retain(|f, _| d.fertigkeiten.contains_key(f));
            s.sp.zauber.retain(|z| d.zauber.contains_key(z));
            s.rolle = rolle_berechnen(d, &s.sp);
        }
        Ok(warnungen)
    }

    // ------------------------------------------------------------------
    // Nachrichten
    // ------------------------------------------------------------------

    pub fn sende(&self, sid: u64, t: &str) {
        if let Some(s) = self.spieler.get(&sid) {
            s.sende(t);
        }
    }

    pub fn sende_raum(&self, raum: &str, t: &str, ausser: &[u64]) {
        for (id, s) in &self.spieler {
            if s.sp.raum == raum && !ausser.contains(id) {
                s.sende(t);
            }
        }
    }

    /// Nur an Spieler, die Ankündigungen (kommen/gehen) sehen wollen.
    pub fn sende_ankuendigung(&self, t: &str, ausser: &[u64]) {
        for (id, s) in &self.spieler {
            if !ausser.contains(id) && s.sp.einstellungen.ankuendigungen {
                s.sende(t);
            }
        }
    }

    pub fn sende_alle(&self, t: &str, ausser: &[u64]) {
        for (id, s) in &self.spieler {
            if !ausser.contains(id) {
                s.sende(t);
            }
        }
    }

    pub fn spieler_im_raum(&self, raum: &str) -> Vec<u64> {
        self.spieler.iter().filter(|(_, s)| s.sp.raum == raum).map(|(id, _)| *id).collect()
    }

    pub fn monster_im_raum(&self, raum: &str) -> Vec<u64> {
        self.monster.iter().filter(|(_, m)| m.raum == raum).map(|(id, _)| *id).collect()
    }

    // ------------------------------------------------------------------
    // Spieler an- und abmelden
    // ------------------------------------------------------------------

    pub fn spieler_verzeichnis(&self) -> PathBuf {
        let p = PathBuf::from(&self.d.welt.spieler_verzeichnis);
        if p.is_absolute() {
            p
        } else {
            // relativ zum Ordner, in dem der Datenordner liegt
            self.daten_ordner.parent().unwrap_or(Path::new(".")).join(p)
        }
    }

    pub fn rolle_von(&self, sp: &SpielerDaten) -> Rolle {
        rolle_berechnen(&self.d, sp)
    }

    pub fn ist_online(&self, name: &str) -> bool {
        self.spieler.values().any(|s| s.sp.name.eq_ignore_ascii_case(name))
    }

    pub fn neuer_spieler(&self, name: &str, passwort: &str) -> SpielerDaten {
        let st = &self.d.start;
        let mut sp = SpielerDaten {
            name: name.to_string(),
            raum: self.d.welt.startraum.clone(),
            lebenspunkte: st.lebenspunkte,
            max_lebenspunkte: st.lebenspunkte,
            mana: st.mana,
            max_mana: st.mana,
            inventar: st.gegenstaende.clone(),
            fertigkeiten: st.fertigkeiten.clone(),
            zauber: st.zauber.clone(),
            werte: st.werte.clone(),
            erstellt: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            ..Default::default()
        };
        sp.passwort_setzen(passwort);
        sp
    }

    pub fn anmelden(&mut self, mut sp: SpielerDaten, sender: Sender) -> Result<u64, String> {
        if self.ist_online(&sp.name) {
            return Err("Diese Figur ist bereits in der Welt unterwegs.".into());
        }
        if !self.d.raum_existiert(&sp.raum) {
            sp.raum = self.d.welt.startraum.clone();
        }
        if !crate::oberwelt::ist_oberwelt(&sp.raum) {
            sp.besucht.insert(sp.raum.clone());
        }
        let rolle = self.rolle_von(&sp);
        let name = sp.name.clone();
        let raum = sp.raum.clone();
        let id = self.neue_id();
        self.spieler.insert(id, Sitzung { sp, sender, rolle, abklingzeiten: HashMap::new() });
        self.sende_raum(&raum, &format!("{name} erscheint."), &[id]);
        self.sende_ankuendigung(&format!("{{grau}}[{name} hat die Welt betreten.]{{/}}"), &[id]);
        println!("Anmeldung: {name}");
        Ok(id)
    }

    pub fn abmelden(&mut self, sid: u64) {
        if let Some(s) = self.spieler.remove(&sid) {
            self.speichern(&s.sp);
            self.sende_raum(&s.sp.raum, &format!("{} verschwindet.", s.sp.name), &[]);
            self.sende_ankuendigung(&format!("{{grau}}[{} hat die Welt verlassen.]{{/}}", s.sp.name), &[]);
            println!("Abmeldung: {}", s.sp.name);
        }
    }

    pub fn speichern(&self, sp: &SpielerDaten) {
        if let Err(e) = spieler::speichern(&self.spieler_verzeichnis(), sp) {
            eprintln!("Konnte {} nicht speichern: {e}", sp.name);
        }
    }

    pub fn alle_speichern(&self) {
        for s in self.spieler.values() {
            self.speichern(&s.sp);
        }
    }

    // ------------------------------------------------------------------
    // Der Takt: alle paar Sekunden passiert etwas in der Welt
    // ------------------------------------------------------------------

    pub fn takt(&mut self) {
        self.takte += 1;
        let pro_minute = (60 / self.d.welt.takt_sekunden.max(1)).max(1);

        // Erholung
        let (lp, mp) = (self.d.welt.regeneration_lp, self.d.welt.regeneration_mp);
        let takt = self.d.welt.takt_sekunden;
        for s in self.spieler.values_mut() {
            s.sp.spielzeit += takt;
            s.sp.lebenspunkte = (s.sp.lebenspunkte + lp).min(s.sp.max_lebenspunkte);
            s.sp.mana = (s.sp.mana + mp).min(s.sp.max_mana);
        }

        self.monster_handeln();

        let reset = self.d.welt.zuruecksetzen_minuten * pro_minute;
        if reset > 0 && self.takte % reset == 0 {
            self.zuruecksetzen();
        }
        let speichern = self.d.welt.speichern_minuten * pro_minute;
        if speichern > 0 && self.takte % speichern == 0 {
            self.alle_speichern();
        }
    }

    fn monster_handeln(&mut self) {
        enum Aktion {
            Gehen(u64, daten::Richtung, String),
            Sagen(String, String),
        }
        let mut rng = rand::thread_rng();
        let mut aktionen = Vec::new();

        for (id, m) in &self.monster {
            let Some(def) = self.d.monster.get(&m.def) else { continue };
            let ausgaenge = self.d.ausgaenge(&m.raum);

            if def.wandert && rng.gen_range(0..100) < def.wander_chance {
                // Monster bleiben in ihrem Gebiet (falls ihr Heimatraum eines hat).
                let gebiet = self.d.gebiet(&m.heimat);
                let moeglich: Vec<_> = ausgaenge
                    .iter()
                    .filter(|(_, ziel)| gebiet.is_empty() || self.d.gebiet(ziel) == gebiet)
                    .collect();
                if let Some((richt, ziel)) = moeglich.choose(&mut rng) {
                    aktionen.push(Aktion::Gehen(*id, **richt, (*ziel).clone()));
                    continue;
                }
            }

            if !def.sprueche.is_empty()
                && rng.gen_range(0..100) < def.sprech_chance
                && self.spieler.values().any(|s| s.sp.raum == m.raum)
            {
                if let Some(spruch) = def.sprueche.choose(&mut rng) {
                    let name = gross(&def.name);
                    let t = match spruch.strip_prefix('*') {
                        Some(handlung) => format!("{name} {}", handlung.trim()),
                        None => format!("{name} sagt: „{spruch}“"),
                    };
                    aktionen.push(Aktion::Sagen(m.raum.clone(), t));
                }
            }
        }

        for a in aktionen {
            match a {
                Aktion::Gehen(id, richt, ziel) => {
                    let name = gross(&self.monster_name(id));
                    let Some(m) = self.monster.get_mut(&id) else { continue };
                    let alt = std::mem::replace(&mut m.raum, ziel.clone());
                    self.sende_raum(&alt, &format!("{name} {}.", richt.geht()), &[]);
                    self.sende_raum(&ziel, &format!("{name} {}.", richt.kommt()), &[]);
                }
                Aktion::Sagen(raum, t) => {
                    for s in self.spieler.values() {
                        if s.sp.raum == raum && s.sp.einstellungen.npc_sprueche {
                            s.sende(&t);
                        }
                    }
                }
            }
        }
    }

    /// Fehlende Gegenstände und Monster wieder auffüllen.
    pub fn zuruecksetzen(&mut self) {
        let mut neue_monster = Vec::new();
        for (raum_id, gegenstaende, monster) in self.d.bevoelkerung() {
            let boden = self.boden.entry(raum_id.clone()).or_default();
            let mut vorhanden = boden.clone();
            for g in &gegenstaende {
                if let Some(pos) = vorhanden.iter().position(|x| x == g) {
                    vorhanden.remove(pos);
                } else {
                    boden.push(g.clone());
                }
            }
            let mut lebend: Vec<&String> =
                self.monster.values().filter(|m| m.heimat == raum_id).map(|m| &m.def).collect();
            for m in &monster {
                if let Some(pos) = lebend.iter().position(|x| *x == m) {
                    lebend.remove(pos);
                } else {
                    neue_monster.push((m.clone(), raum_id.clone()));
                }
            }
        }
        for (m, raum) in neue_monster {
            if let Some(id) = self.monster_erschaffen(&m, &raum) {
                let name = gross(&self.monster_name(id));
                self.sende_raum(&raum, &format!("{name} taucht auf."), &[]);
            }
        }
    }
}
