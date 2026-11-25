//! Die eingebauten Befehle mit ihren Standardwerten.
//!
//! Diese Datei benutzen Engine UND Welteditor (dort per `#[path]` eingebunden),
//! deshalb enthält sie nur Daten und keine Verweise auf andere Module.
//! In `daten/befehle.toml` kann man für jeden Befehl die Wörter, die Rolle,
//! den Hilfetext und die Syntax ändern oder ihn ganz abschalten.

pub struct Eingebaut {
    /// Fester Name des Befehls (so heißt er in befehle.toml: [befehl.<id>])
    pub id: &'static str,
    /// Wörter, mit denen man ihn aufruft – das erste ist der Hauptname
    pub woerter: &'static [&'static str],
    /// spieler, baumeister oder admin
    pub rolle: &'static str,
    pub kategorie: &'static str,
    pub syntax: &'static str,
    pub hilfe: &'static str,
    /// Darf man ihn benutzen, während man tot ist?
    pub als_toter: bool,
}

pub const ROLLEN: &[&str] = &["spieler", "baumeister", "admin"];

/// Reihenfolge der Abschnitte in der Hilfe.
pub const KATEGORIEN: &[&str] = &[
    "Umsehen und Bewegen",
    "Gegenstände",
    "Unterhaltung",
    "Kampf",
    "Zauber",
    "Handel",
    "Aufgaben",
    "Gemeinschaft",
    "Deine Figur",
    "Baumeister",
    "Admin",
];

const fn b(
    id: &'static str,
    woerter: &'static [&'static str],
    rolle: &'static str,
    kategorie: &'static str,
    syntax: &'static str,
    hilfe: &'static str,
    als_toter: bool,
) -> Eingebaut {
    Eingebaut { id, woerter, rolle, kategorie, syntax, hilfe, als_toter }
}

const UMSEHEN: &str = "Umsehen und Bewegen";
const DINGE: &str = "Gegenstände";
const REDEN: &str = "Unterhaltung";
const KAMPF: &str = "Kampf";
const ZAUBER: &str = "Zauber";
const FIGUR: &str = "Deine Figur";
const HANDEL: &str = "Handel";
const AUFGABEN: &str = "Aufgaben";
const GEMEINSCHAFT: &str = "Gemeinschaft";
const BAU: &str = "Baumeister";
const ADMIN: &str = "Admin";

pub const EINGEBAUT: &[Eingebaut] = &[
    // Umsehen und Bewegen
    b("schau", &["schau", "schaue", "l", "look", "umsehen", "betrachte", "untersuche", "unt", "x"], "spieler", UMSEHEN, "schau [etwas]", "Umsehen oder etwas genauer ansehen", true),
    b("riech", &["riech", "rieche", "riechen", "schnupper", "schnuppere", "smell"], "spieler", UMSEHEN, "riech [etwas]", "Riechen", false),
    b("hoer", &["hoer", "hoere", "hoeren", "horch", "horche", "lausche", "listen"], "spieler", UMSEHEN, "hoer [etwas]", "Lauschen", false),
    b("taste", &["taste", "tasten", "fuehl", "fuehle", "beruehre", "fass", "fasse", "touch", "feel"], "spieler", UMSEHEN, "taste [etwas]", "Anfassen", false),
    b("gehe", &["gehe", "geh", "go"], "spieler", UMSEHEN, "gehe <richtung>", "Gehen; kurz: n o s w no nw so sw, h(och), r(unter), rein, raus", false),
    b("karte", &["karte", "map", "k"], "spieler", UMSEHEN, "karte [größe]", "Die Karte der Umgebung zeigen", true),
    b("weltkarte", &["weltkarte", "landkarte", "worldmap"], "spieler", UMSEHEN, "weltkarte", "Die ganze Oberwelt (mit einer Landkarte)", false),
    b("zeit", &["zeit", "uhr", "uhrzeit", "datum", "wetter", "time"], "spieler", UMSEHEN, "zeit", "Uhrzeit, Jahreszeit und Wetter", true),
    // Gegenstände
    b("nimm", &["nimm", "nehme", "take", "get"], "spieler", DINGE, "nimm <ding|alles>", "Etwas aufheben", false),
    b("lege", &["lege", "leg", "ablegen", "drop", "fallenlassen"], "spieler", DINGE, "lege <ding|alles>", "Etwas fallen lassen", false),
    b("inventar", &["inventar", "i", "inv"], "spieler", DINGE, "inventar", "Zeigen, was du bei dir hast", true),
    b("stecke", &["stecke", "steck", "packe", "pack", "put"], "spieler", DINGE, "stecke <ding|alles> in <behälter>", "Etwas in eine Tasche oder einen Rucksack stecken (heraus: nimm ... aus ...)", false),
    b("anlegen", &["anlegen", "anziehen", "ausruesten", "zuecke", "wear", "wield"], "spieler", DINGE, "anlegen <ding>", "Waffe oder Rüstung anlegen", false),
    b("ausziehen", &["ausziehen", "abnehmen", "remove"], "spieler", DINGE, "ausziehen <ding>", "Ausrüstung wieder abnehmen", false),
    b("benutze", &["benutze", "benutzen", "trinke", "trink", "iss", "esse", "use", "drink", "eat"], "spieler", DINGE, "benutze <ding>", "Trank trinken, Essen essen ...", false),
    // Unterhaltung
    b("sage", &["sage", "sag", "say"], "spieler", REDEN, "sage <text>", "Etwas sagen (kurz: 'text) – NPCs hören zu", true),
    b("frage", &["frage", "frag", "ask"], "spieler", REDEN, "frage <npc> nach <thema>", "Einen NPC etwas fragen", false),
    b("rede", &["rede", "red", "sprich", "sprech", "talk"], "spieler", REDEN, "rede mit <npc>", "Ein Gespräch beginnen", false),
    b("emote", &["emote", "me", "/me"], "spieler", REDEN, "emote <text>", "Eine Handlung zeigen (kurz: :text)", true),
    b("rufe", &["rufe", "ruf", "shout"], "spieler", REDEN, "rufe <text>", "Etwas rufen, das alle hören", true),
    b("wer", &["wer", "who"], "spieler", REDEN, "wer", "Wer ist gerade da?", true),
    // Kampf
    b("toete", &["toete", "tote", "angriff", "angreifen", "greif", "greife", "attackiere", "kill", "kaempfe", "schlag", "schlage"], "spieler", KAMPF, "toete <monster>", "Angreifen (danach geht es automatisch weiter)", false),
    b("schaetze", &["schaetze", "einschaetzen", "consider", "con"], "spieler", KAMPF, "schaetze <monster>", "Wie gefährlich ist es?", false),
    b("ausweichen", &["ausweichen", "weiche", "duck", "ducke", "dodge"], "spieler", KAMPF, "ausweichen", "Kurz in Deckung: Angriffe treffen seltener", false),
    b("haltung", &["haltung"], "spieler", KAMPF, "haltung <art>", "angriffslustig, ausgewogen oder defensiv", false),
    b("flieh", &["flieh", "fliehe", "flucht", "flee"], "spieler", KAMPF, "flieh", "Aus dem Kampf fliehen (klappt nicht immer)", false),
    b("stopp", &["stopp", "stop", "aufhoeren"], "spieler", KAMPF, "stopp", "Nicht mehr angreifen", false),
    b("wiederbeleben", &["wiederbeleben", "wiederbelebe", "belebe", "rette", "revive"], "spieler", KAMPF, "wiederbeleben <name>", "Einen toten Mitspieler zurückholen", false),
    b("aufgeben", &["aufgeben"], "spieler", KAMPF, "aufgeben", "Als Toter sofort im Tempel erwachen", true),
    // Zauber
    b("zauber", &["zauber", "spells", "zaubersprueche"], "spieler", ZAUBER, "zauber", "Deine Zaubersprüche", false),
    b("zaubere", &["zaubere", "zauber_sprechen", "cast", "z"], "spieler", ZAUBER, "zaubere <nr|zauber> [ziel]", "Einen Zauber sprechen", false),
    b("spezial", &["spezial", "technik", "tech", "sa"], "spieler", KAMPF, "spezial <nr|technik> [ziel]", "Eine Kampftechnik einsetzen (kostet Mana/Ausdauer)", false),
    b("bereite", &["bereite", "bereit", "bereithalten"], "spieler", KAMPF, "bereite [nr|name|nichts]", "Zauber (mit Zauberstab) oder Technik, die der Autoangriff von selbst einsetzt", false),
    b("wirkungen", &["wirkungen", "effekte", "buffs", "affects"], "spieler", ZAUBER, "wirkungen", "Was gerade auf dich wirkt (Schilde, Stärkungen ...)", true),
    // Handel
    b("waren", &["waren", "angebot", "laden", "shop"], "spieler", HANDEL, "waren [händler]", "Was Händler und Lehrmeister hier anbieten", false),
    b("kaufe", &["kaufe", "kauf", "buy"], "spieler", HANDEL, "kaufe [anzahl] <nr|ware>", "Etwas bei einem Händler kaufen", false),
    b("verkaufe", &["verkaufe", "verkauf", "sell"], "spieler", HANDEL, "verkaufe [anzahl] <ding>", "Etwas an einen Händler verkaufen", false),
    b("lerne", &["lerne", "lernen", "learn"], "spieler", HANDEL, "lerne <nr|fertigkeit|zauber>", "Bei einem Lehrmeister lernen (kostet Geld)", false),
    b("geld", &["geld", "muenzen", "gold", "money"], "spieler", HANDEL, "geld", "Wie viel Geld du hast", true),
    // Aufgaben
    b("aufgaben", &["aufgaben", "quests", "auftraege", "aufgabenbuch"], "spieler", AUFGABEN, "aufgaben [erledigt]", "Deine Aufgaben und Quests", true),
    b("annehmen", &["annehmen", "akzeptiere", "accept"], "spieler", AUFGABEN, "annehmen [nr|aufgabe]", "Eine angebotene Aufgabe übernehmen", false),
    b("abgeben", &["abgeben", "abschliessen"], "spieler", AUFGABEN, "abgeben", "Erledigte Aufgaben und Gegenstände beim NPC abgeben", false),
    b("abbrechen", &["abbrechen"], "spieler", AUFGABEN, "abbrechen <nr|aufgabe>", "Eine Aufgabe aufgeben", false),
    // Gemeinschaft
    b("rasse", &["rasse", "race"], "spieler", GEMEINSCHAFT, "rasse [name]", "Deine Rasse (Figuren ohne Rasse können hier wählen)", true),
    b("gilde", &["gilde", "guild"], "spieler", GEMEINSCHAFT, "gilde", "Deine Gilde und wer von ihr da ist", true),
    b("beruf", &["beruf", "berufe", "zunft"], "spieler", GEMEINSCHAFT, "beruf", "Deine Berufe", true),
    b("beitreten", &["beitreten", "beitritt", "join"], "spieler", GEMEINSCHAFT, "beitreten [nr|gilde|beruf]", "Bei einem NPC einer Gilde oder einem Beruf beitreten", false),
    b("austreten", &["austreten", "austritt"], "spieler", GEMEINSCHAFT, "austreten [nr|gilde|beruf]", "Aus einer Gilde oder einem Beruf austreten", false),
    // Deine Figur
    b("spielstand", &["spielstand", "sp", "werte", "score", "status"], "spieler", FIGUR, "spielstand", "Werte, Stufe, Ausrüstung und Fortschritte", true),
    b("steigere", &["steigere", "verbessere", "trainiere"], "spieler", FIGUR, "steigere <wert>", "Punkte aus dem Stufenaufstieg verteilen", false),
    b("fertigkeiten", &["fertigkeiten", "fert", "skills"], "spieler", FIGUR, "fertigkeiten", "Deine Fertigkeiten", false),
    b("spielerbuch", &["spielerbuch", "buch", "sb", "einstellungen", "optionen"], "spieler", FIGUR, "spielerbuch [einstellung wert]", "Einstellungen: Karte, Farben, Titel, Passwort ...", true),
    b("lies", &["lies", "lese", "read"], "spieler", FIGUR, "lies <etwas>", "Etwas lesen, z.B. das Spielerbuch", false),
    b("farbe", &["farbe", "farben", "color"], "spieler", FIGUR, "farbe an|aus", "Farben ein- oder ausschalten", false),
    b("speichern", &["speichern", "save"], "spieler", FIGUR, "speichern", "Deine Figur speichern", false),
    b("hilfe", &["hilfe", "help", "?"], "spieler", FIGUR, "hilfe [befehl]", "Diese Hilfe; mit Befehl: alles über ihn", true),
    b("ende", &["ende", "quit", "exit", "tschuess"], "spieler", FIGUR, "ende", "Das Spiel verlassen", true),
    // Baumeister
    b("neuladen", &["neuladen", "reload"], "baumeister", BAU, "neuladen", "Alle Weltdaten neu laden (ohne Neustart)", false),
    b("pruefen", &["pruefen", "check"], "baumeister", BAU, "pruefen", "Weltdaten prüfen, ohne sie zu laden", false),
    b("wo", &["wo"], "baumeister", BAU, "wo", "ID, Koordinaten und Datei des Raums", false),
    b("springe", &["springe", "goto"], "baumeister", BAU, "springe <raum-id>", "Zu einem Raum springen", false),
    b("erschaffe", &["erschaffe", "create"], "baumeister", BAU, "erschaffe <gegenstand-id>", "Einen Gegenstand erschaffen", false),
    b("beschwoere", &["beschwoere", "summon"], "baumeister", BAU, "beschwoere <monster-id>", "Ein Monster oder einen NPC herbeirufen", false),
    b("liste", &["liste", "list"], "baumeister", BAU, "liste raeume|gegenstaende|monster|...", "Alles einer Art auflisten", false),
    b("zuruecksetzen", &["zuruecksetzen", "reset"], "baumeister", BAU, "zuruecksetzen", "Gegenstände und Monster sofort auffüllen", false),
    // Admin
    b("rolle", &["rolle"], "admin", ADMIN, "rolle <spieler> spieler|baumeister|admin", "Rolle vergeben (auch offline)", false),
    b("rauswerfen", &["rauswerfen", "kick"], "admin", ADMIN, "rauswerfen <spieler> [grund]", "Spieler vom Server werfen", false),
    b("verkuende", &["verkuende", "verkuenden"], "admin", ADMIN, "verkuende <text>", "Nachricht an alle", false),
    b("lehre", &["lehre", "teach"], "admin", ADMIN, "lehre <spieler> <id> [stufe]", "Fertigkeit oder Zauber beibringen", false),
    b("erfahrung", &["erfahrung"], "admin", ADMIN, "erfahrung <spieler> <menge>", "Erfahrungspunkte geben (zum Testen)", false),
    b("gebegeld", &["gebegeld", "geldgeben"], "admin", ADMIN, "gebegeld <spieler> <menge>", "Geld geben (zum Testen)", false),
    b("speichernalle", &["speichernalle"], "admin", ADMIN, "speichernalle", "Alle Spieler sofort speichern", false),
];

pub fn finde(id: &str) -> Option<&'static Eingebaut> {
    EINGEBAUT.iter().find(|e| e.id == id)
}

/// Wörter, die als Richtung verstanden werden (dürfen nicht für Befehle benutzt werden).
pub const RICHTUNGSWOERTER: &[&str] = &[
    "n", "nord", "norden", "north", "no", "ne", "nordost", "nordosten", "northeast", "o", "e", "ost", "osten", "east",
    "so", "se", "suedost", "suedosten", "southeast", "s", "sued", "sueden", "south", "sw", "suedwest", "suedwesten",
    "southwest", "w", "west", "westen", "nw", "nordwest", "nordwesten", "northwest", "h", "hoch", "oben", "rauf",
    "u", "up", "r", "runter", "unten", "d", "down", "rein", "hinein", "betreten", "betrete", "enter", "in", "raus",
    "hinaus", "verlassen", "verlasse", "leave", "out",
];
