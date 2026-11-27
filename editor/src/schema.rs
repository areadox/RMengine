//! Welche Felder jede Inhaltsart hat – daraus baut der Editor die Formulare.
//!
//! Neue Felder in der Engine trägt man hier ein, dann kann man sie im Editor
//! bearbeiten. Der Editor arbeitet direkt auf den TOML-Dateien (Kommentare bleiben).

/// Woraus man bei Verweisen auswählen kann.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Ziel {
    Gegenstand,
    /// Monster und NPCs
    Wesen,
    Npc,
    Fertigkeit,
    Zauber,
    /// Fertigkeiten und Zauber (für Lehrmeister)
    Lehrbar,
    Raum,
    Rasse,
    Gilde,
    Beruf,
    Quest,
    /// Eigene Befehle
    Befehl,
    /// Körperbau
    Koerper,
    /// Wetterarten
    Wetter,
    Beschreibung,
    /// Dieselbe Art wie der Eintrag (für erbt)
    Gleich,
}

#[derive(Clone, Copy)]
pub enum Typ {
    Text,
    Mehrzeilig,
    /// Mehrzeiliges Bild aus Zeichen (Festbreitenschrift)
    Bild,
    /// Ein Text oder eine Liste von Texten (eine Zeile = eine Möglichkeit)
    TextOderListe,
    Zahl,
    Komma,
    Schalter,
    /// Komma-getrennte Wörter
    Liste,
    Auswahl(&'static [&'static str]),
    Verweis(Ziel),
    VerweisListe(Ziel),
    /// Mehrere Werte aus einer festen Liste, z.B. Gegenstandsarten
    AuswahlListe(&'static [&'static str]),
    /// Name -> Zahl, z.B. eigenschaften = { schaden = 3 }
    Werte(&'static [&'static str]),
    /// ID -> Zahl, z.B. fertigkeiten = { schwertkampf = 5 }
    VerweisWerte(Ziel),
    /// Eine Zahl oder [von, bis]
    Geld,
    /// Ein Geldbetrag in der kleinsten Einheit (zeigt z.B. "= 2 Gold 5 Silber")
    Betrag,
    /// Liste von Untereinträgen; `inline` = als [ { ... } ] statt [[a.b]]
    Tabellen { felder: &'static [Feld], inline: bool },
    /// Eine Untertabelle, z.B. [npc.haendler]
    Unter(&'static [Feld]),
    /// Details (Name -> Text oder ausführlich mit Unter-Details)
    Details,
    /// Quest-Schritt: Ziel hängt von der Art ab
    SchrittZiel,
}

#[derive(Clone, Copy)]
pub struct Feld {
    pub name: &'static str,
    pub titel: &'static str,
    pub typ: Typ,
    pub hilfe: &'static str,
}

const fn f(name: &'static str, titel: &'static str, typ: Typ, hilfe: &'static str) -> Feld {
    Feld { name, titel, typ, hilfe }
}

/// Wo die Einträge einer Art stehen.
#[derive(Clone, Copy, PartialEq)]
pub enum Ablage {
    /// [[schluessel]] in beliebigen Dateien
    Liste,
    /// [npc] – eine Datei pro Eintrag
    EineDatei,
    /// Eine feste Tabelle in einer festen Datei, z.B. [welt] in welt.toml
    Fest { datei: &'static str },
}

pub struct Art {
    /// Name in der Datei: [[gegenstand]], [npc], [welt] ...
    pub schluessel: &'static str,
    pub titel: &'static str,
    pub einzahl: &'static str,
    pub symbol: &'static str,
    /// Ordner für neue Dateien
    pub ordner: &'static str,
    pub ablage: Ablage,
    pub felder: &'static [Feld],
    pub hilfe: &'static str,
}

impl Art {
    /// Für Verweise: Monster und NPCs teilen sich die IDs.
    pub fn gruppe(&self) -> &'static str {
        if self.schluessel == "npc" {
            "monster"
        } else {
            self.schluessel
        }
    }
    pub fn hat_id(&self) -> bool {
        !matches!(self.ablage, Ablage::Fest { .. })
    }
}

// ---------------------------------------------------------------------------
// Bausteine
// ---------------------------------------------------------------------------

pub const GEGENSTAND_TYPEN: &[&str] =
    &["waffe", "ruestung", "trank", "nahrung", "getraenk", "schluessel", "werkzeug", "schmuck", "behaelter", "sonstiges"];
pub const PLAETZE: &[&str] =
    &["waffe", "schild", "kopf", "koerper", "haende", "beine", "fuesse", "hals", "ring", "umhang", "licht", "ruecken", "guertel"];
pub const ZIELE: &[&str] = &["selbst", "spieler", "gegner", "raum"];
pub const ZAUBER_TYPEN: &[&str] = &["zauber", "technik"];
pub const WIRKUNGEN: &[&str] =
    &["heilen", "mana", "ausdauer", "schaden", "teleport", "schild", "ruestung", "bonus", "kraft", "tempo", "betaeuben"];
pub const QUEST_ARTEN: &[&str] = &["aufgabe", "quest"];
pub const SCHRITT_ARTEN: &[&str] = &["toeten", "bringen", "besuchen", "sprechen", "befehl", "skript"];
pub const ROLLEN: &[&str] = &["spieler", "baumeister", "admin"];
const EIGENSCHAFTEN: &[&str] = &["schaden", "ruestung", "heilung", "mana", "essen", "trinken", "landkarte", "staerke", "geschick", "ausdauer", "intelligenz", "weisheit"];
const WERTE: &[&str] = &["staerke", "geschick", "ausdauer", "intelligenz", "weisheit"];

const VERERBUNG: [Feld; 2] = [
    f("erbt", "Erbt von", Typ::Verweis(Ziel::Gleich), "Alle Felder, die hier leer sind, kommen aus dieser Vorlage."),
    f("vorlage", "Ist eine Vorlage", Typ::Schalter, "Vorlagen gibt es nicht in der Welt, man kann nur von ihnen erben."),
];

const SINNE: [Feld; 4] = [
    f("geruch", "Geruch", Typ::Text, "Was man mit 'riech' wahrnimmt."),
    f("geraeusch", "Geräusch", Typ::Text, "Was man mit 'hoer' wahrnimmt."),
    f("gefuehl", "Gefühl", Typ::Text, "Was man mit 'taste' wahrnimmt."),
    f("nutzt", "Nutzt Beschreibungen", Typ::VerweisListe(Ziel::Beschreibung), "Allgemeine Beschreibungen aus daten/beschreibungen."),
];

pub const DETAIL_FELDER: &[Feld] = &[
    f("text", "Text", Typ::Mehrzeilig, ""),
    f("woerter", "Weitere Wörter", Typ::Liste, "Unter diesen Wörtern findet man das Detail auch."),
    SINNE[0],
    SINNE[1],
    SINNE[2],
    SINNE[3],
    f("details", "Unter-Details", Typ::Details, "Noch genauer hinsehen: Details im Detail."),
];

macro_rules! felder {
    ($($x:expr),* $(,)?) => { &[$($x),*] };
}

// ---------------------------------------------------------------------------
// Die Arten
// ---------------------------------------------------------------------------

const WIRKUNG_FELDER: &[Feld] = felder![
    f("art", "Art", Typ::Auswahl(WIRKUNGEN), "heilen, mana, ausdauer, schaden: sofort (oder mit Dauer immer wieder). schild, ruestung, bonus, kraft, tempo: brauchen eine Dauer. betaeuben: Gegner Sekunden lang lahm."),
    f("wert", "Wert", Typ::Zahl, "Menge, Prozent (kraft, tempo) oder Sekunden (betaeuben)."),
    f("dauer", "Dauer (s)", Typ::Komma, "0 = sofort. Sonst wirkt es so viele Sekunden."),
    f("intervall", "Alle ... s", Typ::Komma, "Bei heilen/mana/ausdauer/schaden mit Dauer: wie oft (Standard 3)."),
    f("attribut", "Attribut", Typ::Auswahl(WERTE), "Nur bei bonus: welcher Wert steigt."),
    f("raum", "Raum", Typ::Verweis(Ziel::Raum), "Nur bei teleport."),
];

const GEGENSTAND: &[Feld] = felder![
    f("id", "ID", Typ::Text, "Eindeutiger Name in der Datei, nur Kleinbuchstaben, Ziffern und _."),
    f("name", "Name", Typ::Text, "Mit Artikel, z.B. \"ein roter Apfel\"."),
    f("akkusativ", "Akkusativ", Typ::Text, "\"Du nimmst ...\", z.B. \"einen roten Apfel\"."),
    f("dativ", "Dativ", Typ::Text, "Behälter mit Adjektiv: \"einer bestickten Tasche\" (für \"aus der bestickten Tasche\")."),
    f("beschreibung", "Beschreibung", Typ::Mehrzeilig, ""),
    f("bodentext", "Text am Boden", Typ::Text, "z.B. \"Ein Apfel liegt im Gras.\""),
    f("schluesselwoerter", "Schlüsselwörter", Typ::Liste, "Wörter, mit denen man ihn anspricht."),
    f("typ", "Typ", Typ::Auswahl(GEGENSTAND_TYPEN), ""),
    f("platz", "Platz", Typ::Auswahl(PLAETZE), "Wo man ihn anlegt (Waffen, Rüstung)."),
    f("wert", "Wert (Preis)", Typ::Betrag, "In der kleinsten Geldeinheit. Händler verkaufen und kaufen nach diesem Wert."),
    f("gewicht", "Gewicht", Typ::Zahl, ""),
    f("tragbar", "Tragbar", Typ::Schalter, "Kann man ihn aufheben?"),
    f("erholung", "Erholung (s)", Typ::Komma, "Waffe: Sekunden zwischen zwei Angriffen."),
    f("haende", "Hände zum Halten", Typ::Zahl, "Waffe, Schild, Fackel: Standard 1. Zweihänder: 2."),
    f("skript", "Skript (Lua)", Typ::Bild, "Befehle gelten, solange man ihn dabeihat; bei_nehmen(function(s, id, raum) ... end) usw. beziehen sich auf ihn."),
    f("fertigkeit", "Fertigkeit", Typ::Verweis(Ziel::Fertigkeit), "Waffe: welche Fertigkeit geübt wird."),
    f("eigenschaften", "Eigenschaften", Typ::Werte(EIGENSCHAFTEN), "schaden, ruestung, heilung, mana, essen (sättigt), trinken (löscht Durst) oder Werte wie staerke."),
    f("benutzen_text", "Text beim Benutzen", Typ::Text, ""),
    f("fassungsvermoegen", "Behälter: fasst", Typ::Zahl, "So viel Gewicht passt hinein. Leer/0 = kein Behälter."),
    f("max_dinge", "Behälter: höchstens Dinge", Typ::Zahl, "Leer = beliebig viele."),
    f("gewicht_reduktion", "Behälter: leichter %", Typ::Zahl, "Der Inhalt wiegt so viel Prozent weniger (0 bis 100)."),
    f("nur_typen", "Behälter: nur Typen", Typ::Liste, "z.B. trank (leer = alles passt hinein)"),
    SINNE[0], SINNE[1], SINNE[2], SINNE[3],
    f("details", "Details", Typ::Details, "Was man genauer ansehen kann."),
    VERERBUNG[0], VERERBUNG[1],
];

const BEUTE: &[Feld] = felder![
    f("gegenstand", "Gegenstand", Typ::Verweis(Ziel::Gegenstand), ""),
    f("chance", "Chance %", Typ::Zahl, "Standard 100."),
];

const WESEN_GRUND: [Feld; 23] = [
    f("id", "ID", Typ::Text, ""),
    f("name", "Name", Typ::Text, "Mit Artikel, z.B. \"eine fette Ratte\"."),
    f("akkusativ", "Akkusativ", Typ::Text, "\"Du greifst ... an\"."),
    f("dativ", "Dativ", Typ::Text, "\"Du winkst ... zu\"."),
    f("beschreibung", "Beschreibung", Typ::Mehrzeilig, ""),
    f("raumtext", "Text im Raum", Typ::Text, "z.B. \"Eine Ratte huscht umher.\""),
    f("schluesselwoerter", "Schlüsselwörter", Typ::Liste, ""),
    f("stufe", "Stufe", Typ::Zahl, ""),
    f("lebenspunkte", "Lebenspunkte", Typ::Zahl, ""),
    f("mana", "Mana", Typ::Zahl, ""),
    f("schaden", "Schaden", Typ::Zahl, "Leer = 2 + 2 × Stufe."),
    f("ruestung", "Rüstung", Typ::Zahl, ""),
    f("erholung", "Erholung (s)", Typ::Komma, "Sekunden zwischen zwei Angriffen."),
    f("angriffe", "Angriffe", Typ::Liste, "z.B. beißt, kratzt"),
    f("aggressiv", "Greift an", Typ::Schalter, "Greift Spieler von selbst an."),
    f("angreifbar", "Angreifbar", Typ::Schalter, "Leer: Monster ja, NPCs nein."),
    f("erfahrung", "Erfahrung", Typ::Zahl, "Leer = aus Stufe und LP berechnet."),
    f("geld", "Geld beim Sieg", Typ::Geld, "Eine Zahl oder von–bis."),
    f("beute", "Beute", Typ::Tabellen { felder: BEUTE, inline: true }, ""),
    f("zauber", "Zauber", Typ::VerweisListe(Ziel::Zauber), "Kampfzauber, die es benutzt."),
    f("fertigkeiten", "Fertigkeiten", Typ::VerweisWerte(Ziel::Fertigkeit), ""),
    f("werte", "Werte", Typ::Werte(WERTE), ""),
    f("todestext", "Todestext", Typ::Text, "Mit * am Anfang: \"*bricht zusammen.\""),
];
const WESEN_REST: [Feld; 10] = [
    f("wandert", "Wandert umher", Typ::Schalter, ""),
    f("wander_chance", "Wander-Chance %", Typ::Zahl, ""),
    f("sprueche", "Sprüche", Typ::Liste, "Was es ab und zu sagt (mit * eine Handlung)."),
    f("sprech_chance", "Sprech-Chance %", Typ::Zahl, ""),
    SINNE[0], SINNE[1], SINNE[2], SINNE[3],
    f("details", "Details", Typ::Details, ""),
    VERERBUNG[0],
];

const LEICHE: &[Feld] = felder![
    f("aktiv", "Hinterlässt eine Leiche", Typ::Schalter, "Standard: ja (wenn Kampf → Leichen-Minuten größer als 0)."),
    f("name", "Name", Typ::Text, "z.B. \"der Kadaver eines Wolfs\". Leer = \"die Leiche von ...\"."),
    f("akkusativ", "Akkusativ", Typ::Text, "\"den Kadaver eines Wolfs\""),
    f("dativ", "Dativ", Typ::Text, "\"dem Kadaver eines Wolfs\" (für \"nimm alles aus ...\")"),
    f("bodentext", "Text im Raum", Typ::Text, "z.B. \"Der Kadaver eines Wolfs liegt im Laub.\""),
    f("beschreibung", "Beschreibung", Typ::Mehrzeilig, "Was man bei \"schau leiche\" sieht."),
    f("schluesselwoerter", "Schlüsselwörter", Typ::Liste, "Zusätzlich zu leiche, kadaver, ueberreste."),
    f("details", "Details", Typ::Details, "z.B. fell, zaehne, wunde – mit Unter-Details und Sinnen."),
    f("geruch", "Geruch", Typ::Text, ""),
    f("geraeusch", "Geräusch", Typ::Text, ""),
    f("gefuehl", "Gefühl", Typ::Text, ""),
    f("nutzt", "Nutzt Beschreibungen", Typ::VerweisListe(Ziel::Beschreibung), ""),
    f("minuten", "Bleibt liegen (Minuten)", Typ::Zahl, "Leer = Kampf → Leichen-Minuten."),
    f("verwest", "Text beim Verschwinden", Typ::Text, "Leer = \"... zerfällt zu Staub.\""),
];

const MONSTER: &[Feld] = felder![
    WESEN_GRUND[0], WESEN_GRUND[1], WESEN_GRUND[2], WESEN_GRUND[3], WESEN_GRUND[4], WESEN_GRUND[5], WESEN_GRUND[6],
    WESEN_GRUND[7], WESEN_GRUND[8], WESEN_GRUND[9], WESEN_GRUND[10], WESEN_GRUND[11], WESEN_GRUND[12], WESEN_GRUND[13],
    WESEN_GRUND[14], WESEN_GRUND[15], WESEN_GRUND[16], WESEN_GRUND[17], WESEN_GRUND[18], WESEN_GRUND[19], WESEN_GRUND[20],
    WESEN_GRUND[21], WESEN_GRUND[22],
    WESEN_REST[0], WESEN_REST[1], WESEN_REST[2], WESEN_REST[3], WESEN_REST[4], WESEN_REST[5], WESEN_REST[6], WESEN_REST[7],
    WESEN_REST[8], WESEN_REST[9], VERERBUNG[1],
    f("koerper", "Körperbau", Typ::Verweis(Ziel::Koerper), "Maul, Pfoten ...: damit greift es an, und man kann sie ansehen."),
    f("skript", "Skript (Lua)", Typ::Bild, "Befehle gelten, solange es im Raum ist; bei_angriff(function(s, monster, raum) ... end) usw. beziehen sich darauf."),
    f("leiche", "Leiche", Typ::Unter(LEICHE), "Wie die Leiche nach dem Kampf aussieht. Die Beute liegt darin."),
];

const GESPRAECH: &[Feld] = felder![
    f("begruessung", "Begrüßung", Typ::TextOderListe, "Auf \"hallo\". Mehrere Zeilen = zufällig. {spieler} = wer fragt."),
    f("beruf", "Beruf", Typ::TextOderListe, "\"Was ist dein Beruf?\""),
    f("aufgabe", "Aufgabe", Typ::TextOderListe, "\"Hast du eine Aufgabe für mich?\" (Quests des NPCs kommen dazu)"),
    f("name", "Name", Typ::TextOderListe, "\"Wie heißt du?\""),
    f("neuigkeiten", "Neuigkeiten", Typ::TextOderListe, ""),
    f("ort", "Ort", Typ::TextOderListe, "\"Wo bin ich?\""),
    f("abschied", "Abschied", Typ::TextOderListe, ""),
    f("unbekannt", "Unbekannt", Typ::TextOderListe, "Wenn der NPC die Frage nicht versteht."),
];
const THEMA: &[Feld] = felder![
    f("woerter", "Stichwörter", Typ::Liste, ""),
    f("antwort", "Antwort", Typ::TextOderListe, ""),
];
const HAENDLER: &[Feld] = felder![
    f("laden", "Ladenname", Typ::Text, "Überschrift der Liste, z.B. \"Gretas Krämerladen\" (leer = Name des NPCs)."),
    f("waren", "Waren", Typ::VerweisListe(Ziel::Gegenstand), "Was er verkauft (unbegrenzt)."),
    f("ankauf", "Kauft an", Typ::Schalter, "Standard: ja"),
    f("ankauf_typen", "Kauft nur", Typ::AuswahlListe(GEGENSTAND_TYPEN), "Leer = alles."),
    f("verkauf_prozent", "Verkauf %", Typ::Zahl, "Preis in Prozent vom Wert (Standard 100)."),
    f("ankauf_prozent", "Ankauf %", Typ::Zahl, "Was er zahlt, in Prozent vom Wert (Standard 50)."),
];
const LEHRT: &[Feld] = felder![
    f("id", "Fertigkeit/Zauber", Typ::Verweis(Ziel::Lehrbar), ""),
    f("preis", "Preis", Typ::Betrag, "Fertigkeit: pro Stufe. Zauber: einmal."),
    f("bis", "Bis Stufe", Typ::Zahl, "Fertigkeit: höchstens bis hier."),
    f("stufe", "Ab Spielerstufe", Typ::Zahl, ""),
    f("gilde", "Nur für Gilde", Typ::Verweis(Ziel::Gilde), ""),
];

const NPC: &[Feld] = felder![
    WESEN_GRUND[0], WESEN_GRUND[1], WESEN_GRUND[2], WESEN_GRUND[3], WESEN_GRUND[4], WESEN_GRUND[5], WESEN_GRUND[6],
    f("gespraech", "Gespräch", Typ::Unter(GESPRAECH), "Antworten auf die Standardfragen."),
    f("thema", "Eigene Themen", Typ::Tabellen { felder: THEMA, inline: false }, ""),
    f("haendler", "Händler", Typ::Unter(HAENDLER), "Wenn ausgefüllt, kann man bei ihm kaufen und verkaufen."),
    f("lehrt", "Lehrt", Typ::Tabellen { felder: LEHRT, inline: false }, "Fertigkeiten und Zauber gegen Geld."),
    f("gilde", "Nimmt auf in Gilde", Typ::Verweis(Ziel::Gilde), "Spieler treten hier bei (beitreten)."),
    f("beruf", "Nimmt auf in Beruf", Typ::Verweis(Ziel::Beruf), ""),
    WESEN_GRUND[7], WESEN_GRUND[8], WESEN_GRUND[9], WESEN_GRUND[10], WESEN_GRUND[11], WESEN_GRUND[12], WESEN_GRUND[13],
    WESEN_GRUND[14], WESEN_GRUND[15], WESEN_GRUND[16], WESEN_GRUND[17], WESEN_GRUND[18], WESEN_GRUND[19], WESEN_GRUND[20],
    WESEN_GRUND[21], WESEN_GRUND[22],
    WESEN_REST[0], WESEN_REST[1], WESEN_REST[2], WESEN_REST[3], WESEN_REST[4], WESEN_REST[5], WESEN_REST[6], WESEN_REST[7],
    WESEN_REST[8], WESEN_REST[9],
    f("koerper", "Körperbau", Typ::Verweis(Ziel::Koerper), "Wenn man ihn angreifen kann: damit wehrt er sich."),
    f("skript", "Skript (Lua)", Typ::Bild, "Befehle gelten, solange er im Raum ist; bei_gespraech(function(s, text, npc, raum) ... end), bei_bekommen(...) usw. beziehen sich auf ihn."),
    f("leiche", "Leiche", Typ::Unter(LEICHE), "Nur wenn man ihn angreifen kann."),
];

const FERTIGKEIT: &[Feld] = felder![
    f("id", "ID", Typ::Text, ""),
    f("name", "Name", Typ::Text, ""),
    f("beschreibung", "Beschreibung", Typ::Mehrzeilig, ""),
    f("kategorie", "Kategorie", Typ::Text, "Zum Gruppieren, z.B. kampf, zauberei, handwerk."),
    f("max_stufe", "Höchste Stufe", Typ::Zahl, "Standard 100."),
    f("attribut", "Passender Wert", Typ::Auswahl(WERTE), ""),
    f("voraussetzungen", "Voraussetzungen", Typ::VerweisListe(Ziel::Fertigkeit), "Muss man vorher können."),
    f("lernchance", "Lernchance %", Typ::Zahl, "Chance, beim Benutzen eine Stufe aufzusteigen (Standard 10)."),
];

const ZAUBER: &[Feld] = felder![
    f("id", "ID", Typ::Text, ""),
    f("name", "Name", Typ::Text, ""),
    f("typ", "Typ", Typ::Auswahl(ZAUBER_TYPEN), "zauber (zaubere ...) oder technik (spezial ...)."),
    f("beschreibung", "Beschreibung", Typ::Mehrzeilig, ""),
    f("schluesselwoerter", "Schlüsselwörter", Typ::Liste, ""),
    f("ziel", "Ziel", Typ::Auswahl(ZIELE), ""),
    f("mana_kosten", "Mana-Kosten", Typ::Zahl, ""),
    f("ausdauer_kosten", "Ausdauer-Kosten", Typ::Zahl, ""),
    f("waffen_faktor", "Waffen-Faktor", Typ::Komma, "Technik: Schlag mit der Waffe, Schaden mal diesen Faktor (z.B. 1.5)."),
    f("fertigkeit", "Fertigkeit", Typ::Verweis(Ziel::Fertigkeit), "Wird geübt und muss mindest_stufe haben."),
    f("mindest_stufe", "Mindeststufe", Typ::Zahl, ""),
    f("abklingzeit", "Abklingzeit (s)", Typ::Zahl, ""),
    f("gilde", "Nur für Gilde", Typ::Verweis(Ziel::Gilde), ""),
    f("wirkungen", "Wirkungen", Typ::Tabellen { felder: WIRKUNG_FELDER, inline: true }, ""),
    f("text_selbst", "Text für dich", Typ::Text, "{name}, {ziel}, {zauber}"),
    f("text_ziel", "Text für das Ziel", Typ::Text, ""),
    f("text_raum", "Text für den Raum", Typ::Text, ""),
    f("text_ende", "Text am Ende", Typ::Text, "Wenn eine Wirkung mit Dauer endet."),
    VERERBUNG[0], VERERBUNG[1],
];

const GEMEINSCHAFT: &[Feld] = felder![
    f("id", "ID", Typ::Text, ""),
    f("name", "Name", Typ::Text, ""),
    f("beschreibung", "Beschreibung", Typ::Mehrzeilig, ""),
    f("werte", "Zuschläge auf Werte", Typ::Werte(WERTE), ""),
    f("lebenspunkte", "+ Lebenspunkte", Typ::Zahl, ""),
    f("mana", "+ Mana", Typ::Zahl, ""),
    f("ausdauer", "+ Ausdauer", Typ::Zahl, ""),
    f("fertigkeiten", "Fertigkeiten", Typ::VerweisWerte(Ziel::Fertigkeit), "Mindestens diese Stufen."),
    f("zauber", "Zauber/Techniken", Typ::VerweisListe(Ziel::Zauber), ""),
];
const RASSE: &[Feld] = felder![
    GEMEINSCHAFT[0], GEMEINSCHAFT[1], GEMEINSCHAFT[2], GEMEINSCHAFT[3], GEMEINSCHAFT[4], GEMEINSCHAFT[5], GEMEINSCHAFT[6],
    GEMEINSCHAFT[7], GEMEINSCHAFT[8],
    f("waehlbar", "Wählbar", Typ::Schalter, "Bei der Erschaffung wählbar (Standard: ja)."),
    f("koerper", "Körperbau", Typ::Verweis(Ziel::Koerper), "Leer = der Körperbau aus Welt."),
];
const GILDE: &[Feld] = felder![
    GEMEINSCHAFT[0], GEMEINSCHAFT[1], GEMEINSCHAFT[2],
    f("aufnahme_stufe", "Ab Stufe", Typ::Zahl, ""),
    f("preis", "Aufnahmegebühr", Typ::Betrag, ""),
    f("rassen", "Nur für Rassen", Typ::VerweisListe(Ziel::Rasse), "Leer = alle."),
    f("titel", "Titel", Typ::Text, "Wird bei der Aufnahme gesetzt, z.B. \"der Krieger\"."),
    f("aufnahme_text", "Text bei Aufnahme", Typ::Text, ""),
    GEMEINSCHAFT[3], GEMEINSCHAFT[4], GEMEINSCHAFT[5], GEMEINSCHAFT[6], GEMEINSCHAFT[7], GEMEINSCHAFT[8],
];

const SCHRITT: &[Feld] = felder![
    f("art", "Art", Typ::Auswahl(SCHRITT_ARTEN), ""),
    f("ziel", "Ziel", Typ::SchrittZiel, "Monster (toeten), Gegenstand (bringen), Raum (besuchen), NPC (sprechen) oder eigener Befehl (befehl). Bei skript: ein freies Wort, das ein Skript mit s:fortschritt(\"...\") meldet."),
    f("anzahl", "Anzahl", Typ::Zahl, "Standard 1."),
    f("an", "Bringen an", Typ::Verweis(Ziel::Npc), "Leer = der Geber."),
    f("text", "Text im Aufgabenbuch", Typ::Text, "Leer = automatisch."),
];
const BELOHNUNG: &[Feld] = felder![
    f("erfahrung", "Erfahrung", Typ::Zahl, ""),
    f("geld", "Geld", Typ::Betrag, ""),
    f("gegenstaende", "Gegenstände", Typ::VerweisListe(Ziel::Gegenstand), ""),
    f("fertigkeiten", "Fertigkeiten +", Typ::VerweisWerte(Ziel::Fertigkeit), "Steigen um so viele Stufen."),
    f("zauber", "Zauber", Typ::VerweisListe(Ziel::Zauber), ""),
    f("titel", "Titel", Typ::Text, ""),
];
const BEFEHL: &[Feld] = felder![
    f("id", "ID", Typ::Text, "Interner Name, z.B. \"beten\"."),
    f("woerter", "Wörter", Typ::Liste, "Womit man ihn aufruft, z.B. bete, beten. Das erste ist der Hauptname."),
    f("hilfe", "Hilfe", Typ::Text, "Steht in der Befehlsliste (hilfe)."),
    f("syntax", "Aufruf", Typ::Text, "z.B. \"segne <spieler>\" – für die Hilfe."),
    f("rolle", "Rolle", Typ::Auswahl(ROLLEN), "Wer ihn benutzen darf (Standard: spieler)."),
    f("aktiv", "Aktiv", Typ::Schalter, "Aus = abgeschaltet."),
    // Bedingungen
    f("raeume", "Nur in Räumen", Typ::VerweisListe(Ziel::Raum), "Leer = überall."),
    f("gebiete", "Nur in Gebieten", Typ::Liste, "z.B. wald, dorf. Leer = überall."),
    f("nicht_hier", "Text: falscher Ort", Typ::Text, "Leer = \"Das geht hier nicht.\""),
    // Mit einem Gegenstand
    f("gegenstand_typen", "Mit Gegenstand der Art", Typ::AuswahlListe(GEGENSTAND_TYPEN), "z.B. waffe: \"fuehre schwert\" geht nur mit Waffen. Gesetzt = der Befehl braucht einen Gegenstand."),
    f("gegenstand_ort", "Gegenstand ist", Typ::Auswahl(&["dabei", "inventar", "angelegt", "raum"]), "dabei = im Inventar oder angelegt (Standard), raum = liegt im Raum."),
    f("mit_gegenstand", "Mit dem Gegenstand", Typ::Auswahl(&["anlegen", "ausziehen", "benutzen", "verbrauchen", "fallenlassen", "nehmen"]), "Was danach mit ihm passiert. Leer = nichts (nur Texte und Wirkungen)."),
    f("ohne_gegenstand", "Text: kein Gegenstand", Typ::Text, "Wenn man nur \"fuehre\" tippt. Leer = \"Womit? (z.B. fuehre <waffe>)\""),
    f("falscher_gegenstand", "Text: falsche Art", Typ::Text, "z.B. \"{ding} ist keine Waffe.\" Leer = \"Mit einem Laib Brot geht das nicht.\""),
    f("braucht", "Braucht Gegenstände", Typ::VerweisListe(Ziel::Gegenstand), "Muss man dabeihaben (bleiben erhalten)."),
    f("nimm", "Verbraucht Gegenstände", Typ::VerweisListe(Ziel::Gegenstand), "Werden weggenommen."),
    f("mindest_stufe", "Ab Stufe", Typ::Zahl, ""),
    f("gilde", "Nur für Gilde", Typ::Verweis(Ziel::Gilde), ""),
    f("beruf", "Nur mit Beruf", Typ::Verweis(Ziel::Beruf), ""),
    f("fertigkeit", "Braucht Fertigkeit", Typ::Verweis(Ziel::Fertigkeit), ""),
    f("fertigkeit_stufe", "... ab Stufe", Typ::Zahl, ""),
    f("mana_kosten", "Kostet Mana", Typ::Zahl, ""),
    f("ausdauer_kosten", "Kostet Ausdauer", Typ::Zahl, ""),
    f("kosten", "Kostet Geld", Typ::Betrag, ""),
    f("abklingzeit", "Abklingzeit (s)", Typ::Zahl, "So lange kann man ihn danach nicht wieder benutzen."),
    f("nicht_im_kampf", "Nicht im Kampf", Typ::Schalter, ""),
    f("nur_draussen", "Nur draußen", Typ::Schalter, "Nicht in Häusern und Höhlen (drinnen)."),
    f("tageszeiten", "Nur zu Tageszeiten", Typ::AuswahlListe(&["morgen", "tag", "abend", "nacht"]), "Leer = immer (Tag und Nacht in Welt → Zeit)."),
    f("jahreszeiten", "Nur in Jahreszeiten", Typ::AuswahlListe(&["fruehling", "sommer", "herbst", "winter"]), "Leer = immer."),
    f("wetter", "Nur bei Wetter", Typ::VerweisListe(Ziel::Wetter), "Leer = bei jedem Wetter."),
    f("nicht_jetzt", "Text: falsche Zeit", Typ::Text, "Wenn Tageszeit, Jahreszeit oder Wetter nicht passen. Leer = \"Das geht jetzt nicht.\""),
    // Texte
    f("selbst", "Text für dich", Typ::Mehrzeilig, "Ohne Ziel. {name} = wer den Befehl benutzt, {ding} / {ding_akk} / {ding_dativ} = der Gegenstand (\"ein Schwert\" / \"ein Schwert\" / \"einem Schwert\"), {rest} = was dahinter getippt wurde."),
    f("andere", "Text für andere", Typ::Mehrzeilig, "Was die anderen im Raum lesen."),
    f("selbst_ziel", "Mit Ziel: für dich", Typ::Mehrzeilig, "z.B. \"Du segnest {ziel}.\" Leer = kein Ziel möglich. {ziel_dativ} = \"dem Wirt\"."),
    f("ziel", "Mit Ziel: fürs Ziel", Typ::Mehrzeilig, "Was das Ziel liest (nur Spieler)."),
    f("andere_ziel", "Mit Ziel: für andere", Typ::Mehrzeilig, ""),
    // Zufall
    f("chance", "Erfolgschance %", Typ::Zahl, "0 oder 100 = klappt immer. Kosten werden auch bei Misserfolg fällig."),
    f("misserfolg", "Text bei Misserfolg", Typ::Mehrzeilig, ""),
    f("misserfolg_andere", "Misserfolg für andere", Typ::Mehrzeilig, ""),
    // Wirkungen
    f("lp", "Lebenspunkte +/-", Typ::Zahl, "Heilt (oder schadet, negativ)."),
    f("mp", "Mana +/-", Typ::Zahl, ""),
    f("ausdauer", "Ausdauer +", Typ::Zahl, ""),
    f("essen", "Sättigt", Typ::Zahl, "Bei Hunger und Durst: so viel Essen."),
    f("trinken", "Löscht Durst", Typ::Zahl, "Bei Hunger und Durst: so viel Trinken, z.B. an einem Brunnen."),
    f("geld", "Gibt Geld", Typ::Betrag, ""),
    f("erfahrung", "Gibt Erfahrung", Typ::Zahl, ""),
    f("gib", "Gibt Gegenstände", Typ::VerweisListe(Ziel::Gegenstand), ""),
    f("teleport", "Versetzt nach", Typ::Verweis(Ziel::Raum), ""),
    f("zauber", "Wirkung eines Zaubers", Typ::Verweis(Ziel::Zauber), "Legt dessen Wirkungen auf dich, ohne Kosten – z.B. einen Schild."),
    f("befehle", "Danach ausführen", Typ::TextOderListe, "Weitere Befehle, eine Zeile pro Befehl, z.B. \"sage Hallo {rest}\"."),
];

const QUEST: &[Feld] = felder![
    f("id", "ID", Typ::Text, ""),
    f("name", "Name", Typ::Text, ""),
    f("art", "Art", Typ::Auswahl(QUEST_ARTEN), "aufgabe = klein, quest = groß."),
    f("geber", "Geber (NPC)", Typ::Verweis(Ziel::Npc), "Bietet sie an, hier gibt man sie ab."),
    f("beschreibung", "Beschreibung", Typ::Mehrzeilig, "Steht im Aufgabenbuch."),
    f("angebot", "NPC bietet an", Typ::Mehrzeilig, ""),
    f("annahme", "NPC bei Annahme", Typ::Text, ""),
    f("abschluss", "NPC bei Abgabe", Typ::Text, ""),
    f("schritte", "Schritte", Typ::Tabellen { felder: SCHRITT, inline: false }, "Werden der Reihe nach erledigt."),
    f("belohnung", "Belohnung", Typ::Unter(BELOHNUNG), ""),
    f("mindest_stufe", "Ab Stufe", Typ::Zahl, ""),
    f("voraussetzungen", "Vorher erledigt", Typ::VerweisListe(Ziel::Quest), ""),
    f("gilde", "Nur für Gilde", Typ::Verweis(Ziel::Gilde), ""),
    f("wiederholbar", "Wiederholbar", Typ::Schalter, ""),
    f("abgabe", "Beim Geber abgeben", Typ::Schalter, "Standard: ja. Nein = sofort fertig."),
];

const BESCHREIBUNG: &[Feld] = felder![
    f("id", "ID", Typ::Text, ""),
    f("woerter", "Wörter", Typ::Liste, "Unter diesen Wörtern findet man sie (z.B. baum, eiche)."),
    f("text", "Text", Typ::Mehrzeilig, ""),
    SINNE[0], SINNE[1], SINNE[2], SINNE[3],
    f("details", "Details", Typ::Details, ""),
    VERERBUNG[0], VERERBUNG[1],
];

const TEIL: &[Feld] = felder![
    f("id", "ID", Typ::Text, "z.B. rechte_hand, maul"),
    f("name", "Name", Typ::Text, "z.B. \"rechte Hand\", \"Maul\""),
    f("woerter", "Weitere Wörter", Typ::Liste, "Zum Ansehen, z.B. hand, pfote."),
    f("text", "Beschreibung", Typ::Mehrzeilig, ""),
    f("halten", "Kann halten", Typ::Schalter, "Waffe, Schild, Fackel. So viele Teile, so viele Hände."),
    f("angriff", "Angriff", Typ::Text, "Monster greifen damit an, z.B. \"beißt\" oder \"schlägt mit der Pfote nach {ziel_dativ}\". Bei Spielern: was die anderen sehen."),
    f("angriff_du", "Angriff (Spieler)", Typ::Text, "Spieler ohne Waffe, z.B. \"schlägst {ziel} mit der Faust\" (\"Du ...\")."),
    f("plaetze", "Trägt Ausrüstung", Typ::AuswahlListe(PLAETZE), "z.B. kopf, hals. Waffe, Schild und Licht hält man (Kann halten)."),
    f("text_andere", "Text für andere", Typ::Mehrzeilig, "So sehen andere dieses Teil (\"schau bert hand\"). {name} = der Besitzer."),
    f("schaden", "Schaden", Typ::Zahl, "0 = der normale Schaden des Monsters."),
    f("geruch", "Geruch", Typ::Text, ""),
    f("geraeusch", "Geräusch", Typ::Text, ""),
    f("gefuehl", "Gefühl", Typ::Text, ""),
    f("nutzt", "Nutzt Beschreibungen", Typ::VerweisListe(Ziel::Beschreibung), ""),
    f("details", "Details", Typ::Details, "Noch genauer: z.B. finger, krallen."),
];
const ZEIT: &[Feld] = felder![
    f("aktiv", "Tag und Nacht", Typ::Schalter, "Standard: aus. Die Zeit läuft aus der echten Uhr."),
    f("minuten_pro_stunde", "Echte Minuten pro Stunde", Typ::Komma, "2 = ein Spieltag dauert 48 Minuten."),
    f("tage_pro_jahreszeit", "Tage pro Jahreszeit", Typ::Zahl, "Standard 7 (ein Jahr = 28 Tage)."),
    f("morgen", "Morgen ab", Typ::Zahl, "Stunde, Standard 6."),
    f("tag", "Tag ab", Typ::Zahl, "Standard 9."),
    f("abend", "Abend ab", Typ::Zahl, "Standard 18."),
    f("nacht", "Nacht ab", Typ::Zahl, "Standard 21."),
    f("text_morgen", "Meldung: Morgen", Typ::Text, "An alle draußen, wenn der Morgen beginnt."),
    f("text_tag", "Meldung: Tag", Typ::Text, ""),
    f("text_abend", "Meldung: Abend", Typ::Text, ""),
    f("text_nacht", "Meldung: Nacht", Typ::Text, ""),
    f("zusatz_morgen", "Draußen am Morgen", Typ::Text, "Steht unter der Raumbeschreibung."),
    f("zusatz_tag", "Draußen am Tag", Typ::Text, ""),
    f("zusatz_abend", "Draußen am Abend", Typ::Text, ""),
    f("zusatz_nacht", "Draußen in der Nacht", Typ::Text, ""),
    f("text_fruehling", "Meldung: Frühling", Typ::Text, "An alle, wenn die Jahreszeit beginnt."),
    f("text_sommer", "Meldung: Sommer", Typ::Text, ""),
    f("text_herbst", "Meldung: Herbst", Typ::Text, ""),
    f("text_winter", "Meldung: Winter", Typ::Text, ""),
    f("wetter_minuten", "Wetterwechsel (Minuten)", Typ::Zahl, "Ungefähr so oft (echte Minuten) wechselt das Wetter. Wetterarten: ☁ Wetter."),
];
const WETTER: &[Feld] = felder![
    f("id", "ID", Typ::Text, ""),
    f("name", "Name", Typ::Text, "z.B. \"Regen\" (für den Befehl zeit)"),
    f("text", "Text draußen", Typ::Text, "Steht unter der Raumbeschreibung, z.B. \"Es regnet.\""),
    f("beginn", "Meldung beim Beginn", Typ::Text, "z.B. \"Es beginnt zu regnen.\""),
    f("gewicht", "Häufigkeit", Typ::Zahl, "Höher = häufiger (Standard 1)."),
    f("jahreszeiten", "Nur in Jahreszeiten", Typ::AuswahlListe(&["fruehling", "sommer", "herbst", "winter"]), "Leer = immer."),
];
const KOERPER: &[Feld] = felder![
    f("id", "ID", Typ::Text, "z.B. humanoid, wolf"),
    f("name", "Name", Typ::Text, "z.B. \"Menschengestalt\""),
    f("teile", "Körperteile", Typ::Tabellen { felder: TEIL, inline: false }, ""),
    f("figur", "Figur fürs Inventar", Typ::Bild, "Leer = die Figur aus 🎨 Darstellung. Vorschau unten."),
    f("figur_plaetze", "Plätze an der Figur", Typ::Tabellen { felder: FIGUR_PLATZ, inline: false }, ""),
];

const WELT: &[Feld] = felder![
    f("name", "Name der Welt", Typ::Text, ""),
    f("port", "Port", Typ::Zahl, "Mit diesem Port verbinden sich die Spieler."),
    f("willkommen", "Willkommenstext", Typ::Mehrzeilig, ""),
    f("admins", "Admins", Typ::Liste, "Figurennamen, die immer Admin sind."),
    f("baumeister", "Baumeister", Typ::Liste, ""),
    f("waehrung", "Name des Geldes", Typ::Text, "z.B. Taler – nur, wenn es unter 💰 Geld keine Einheiten gibt."),
    f("max_berufe", "Berufe höchstens", Typ::Zahl, ""),
    f("techniken_nur_gilden", "Techniken nur für Gilden", Typ::Schalter, "Standard: ja. Jede Kampftechnik braucht dann gilde = \"...\"."),
    f("koerper", "Körperbau der Spieler", Typ::Verweis(Ziel::Koerper), "Wenn die Rasse keinen eigenen hat. Bestimmt z.B., wie viele Hände man zum Halten hat."),
    f("tragkraft_basis", "Tragkraft", Typ::Zahl, "So viel Gewicht trägt jeder (beide 0 = unbegrenzt)."),
    f("tragkraft_pro_staerke", "Tragkraft pro Stärke", Typ::Zahl, "Plus so viel pro Punkt Stärke."),
    f("karte_beim_umsehen", "Karte beim Umsehen", Typ::Schalter, ""),
    f("karten_radius", "Kartengröße", Typ::Zahl, "Räume in jede Richtung."),
    f("karte_daneben", "Text neben der Karte", Typ::Schalter, ""),
    f("karten_fertigkeit", "Karten-Fertigkeit", Typ::Verweis(Ziel::Fertigkeit), "Diese Fertigkeit macht die Karte größer."),
    f("karten_fertigkeit_stufen", "... alle Stufen", Typ::Zahl, "Alle so viele Stufen ein Feld mehr."),
    f("karten_fertigkeit_max", "... höchstens", Typ::Zahl, "So viele Felder mehr höchstens."),
    f("stadtkarte_stil", "Stil der Stadtkarte", Typ::Auswahl(&["kasten", "linien"]), ""),
    f("karten_hintergrund", "Kartenhintergrund", Typ::Text, "Ein Zeichen, z.B. * oder Leerzeichen."),
    f("nur_besuchte_raeume_auf_karte", "Nebel des Krieges", Typ::Schalter, "Nur besuchte Räume auf der Karte."),
    f("ausgaenge_beidseitig", "Rückwege automatisch", Typ::Schalter, ""),
    f("takt_sekunden", "Takt (s)", Typ::Zahl, ""),
    f("zuruecksetzen_minuten", "Auffüllen (min)", Typ::Zahl, ""),
    f("speichern_minuten", "Speichern (min)", Typ::Zahl, ""),
    f("regeneration_lp", "Erholung LP/Takt", Typ::Zahl, ""),
    f("regeneration_mp", "Erholung MP/Takt", Typ::Zahl, ""),
];
const START: &[Feld] = felder![
    f("lebenspunkte", "Lebenspunkte", Typ::Zahl, ""),
    f("mana", "Mana", Typ::Zahl, ""),
    f("geld", "Startgeld", Typ::Betrag, "In der kleinsten Geldeinheit."),
    f("gegenstaende", "Gegenstände", Typ::VerweisListe(Ziel::Gegenstand), ""),
    f("zauber", "Zauber", Typ::VerweisListe(Ziel::Zauber), ""),
    f("fertigkeiten", "Fertigkeiten", Typ::VerweisWerte(Ziel::Fertigkeit), ""),
    f("werte", "Grundwerte", Typ::Werte(WERTE), "Diese Werte gibt es in der Welt."),
];
const ERNAEHRUNG: &[Feld] = felder![
    f("aktiv", "Hunger und Durst", Typ::Schalter, "Standard: aus."),
    f("max_essen", "Satt bei", Typ::Zahl, "Standard 100."),
    f("max_trinken", "Ohne Durst bei", Typ::Zahl, "Standard 100."),
    f("essen_pro_minute", "Hunger pro Minute", Typ::Komma, "So viel geht pro Minute verloren (Standard 1 = 100 Minuten bis leer)."),
    f("trinken_pro_minute", "Durst pro Minute", Typ::Komma, "Standard 1,5."),
    f("hungrig_ab", "Hungrig unter", Typ::Zahl, "Standard 30."),
    f("durstig_ab", "Durstig unter", Typ::Zahl, "Standard 30."),
    f("schaden_pro_minute", "LP-Verlust pro Minute bei 0", Typ::Zahl, "Sterben kann man daran nicht. Standard 2."),
    f("ohne_erholung", "Keine Erholung bei 0", Typ::Schalter, "LP und Mana erholen sich nicht, solange Essen oder Trinken leer ist. Standard: ja."),
    f("nahrung_standard", "Essen ohne Angabe", Typ::Zahl, "So viel sättigt Nahrung ohne eigenschaften.essen (Standard 25)."),
    f("getraenk_standard", "Getränk ohne Angabe", Typ::Zahl, "So viel löscht ein Getränk ohne eigenschaften.trinken (Standard 30)."),
    f("nur_spieler", "Nur für Spieler", Typ::Schalter, "Baumeister und Admins werden nicht hungrig (Standard: ja)."),
];

const KAMPF: &[Feld] = felder![
    f("auto_angriff", "Automatisch weiter", Typ::Schalter, ""),
    f("waffenlos_schaden", "Schaden ohne Waffe", Typ::Zahl, ""),
    f("waffenlos_erholung", "Erholung ohne Waffe (s)", Typ::Komma, ""),
    f("waffenlos_fertigkeit", "Fertigkeit ohne Waffe", Typ::Verweis(Ziel::Fertigkeit), ""),
    f("zauber_erholung", "Erholung nach Zauber (s)", Typ::Komma, ""),
    f("ausdauer", "Ausdauer am Anfang", Typ::Zahl, ""),
    f("ausdauer_pro_angriff", "Ausdauer pro Angriff", Typ::Zahl, ""),
    f("ausdauer_erholung", "Ausdauer-Erholung/Takt", Typ::Zahl, ""),
    f("ep_basis", "EP-Basis", Typ::Komma, "EP für nächste Stufe: Basis × Stufe ^ Faktor"),
    f("ep_faktor", "EP-Faktor", Typ::Komma, ""),
    f("lp_pro_stufe", "LP pro Stufe", Typ::Zahl, ""),
    f("mp_pro_stufe", "MP pro Stufe", Typ::Zahl, ""),
    f("ausdauer_pro_stufe", "Ausdauer pro Stufe", Typ::Zahl, ""),
    f("punkte_pro_stufe", "Punkte pro Stufe", Typ::Zahl, ""),
    f("wiederbelebung_lp_prozent", "LP nach Wiederbelebung %", Typ::Zahl, ""),
    f("tod_wartezeit", "Wartezeit nach Tod (s)", Typ::Zahl, ""),
    f("tod_ep_verlust_prozent", "EP-Verlust beim Tod %", Typ::Zahl, ""),
    f("tod_gegenstaende", "Gegenstände beim Tod", Typ::Auswahl(&["behalten", "fallen_lassen"]), ""),
    f("monster_erholung_prozent", "Monster-Erholung %/Takt", Typ::Zahl, ""),
    f("leichen_minuten", "Leichen-Minuten", Typ::Zahl, "So lange bleiben Leichen liegen (0 = keine Leichen). Standard 5."),
    f("pluendern_sekunden", "Nur Sieger plündern (s)", Typ::Zahl, "So lange darf nur plündern, wer den Gegner besiegt hat (0 = jeder sofort). Standard 60."),
    f("beute_in_leiche", "Beute in der Leiche", Typ::Schalter, "ja = \"nimm alles aus leiche\", nein = Beute fällt zu Boden. Standard: ja."),
];

const EINHEIT: &[Feld] = felder![
    f("name", "Name", Typ::Text, "z.B. Gold"),
    f("mehrzahl", "Mehrzahl", Typ::Text, "z.B. Taler, Münzen (leer = wie der Name)"),
    f("kurz", "Kurzform", Typ::Text, "z.B. G – für die Anzeige \"kurz\""),
    f("wert", "Wert", Typ::Zahl, "So viele der kleinsten Einheit. Die kleinste hat 1, z.B. Kupfer 1, Silber 10, Gold 100."),
];
const GELD: &[Feld] = felder![
    f("anzeige", "Anzeige", Typ::Auswahl(crate::darstellung::GELD_ANZEIGEN), "lang = 2 Gold 5 Silber, kurz = 2G 5S"),
    f("einheiten", "Einheiten", Typ::Tabellen { felder: EINHEIT, inline: false }, "Ohne Einheiten gibt es nur eine Währung (Welt → Name des Geldes)."),
];
const DARSTELLUNG: &[Feld] = felder![
    f("haendler", "Händler und Lehrer", Typ::Auswahl(crate::darstellung::HAENDLER_STILE), "Wie 'waren' aussieht."),
    f("spielstand", "Spielstand", Typ::Auswahl(crate::darstellung::SPIELSTAND_STILE), "beides: sp = kompakt, sp voll = Bogen."),
    f("spielerbuch", "Spielerbuch", Typ::Auswahl(crate::darstellung::SPIELERBUCH_STILE), ""),
    f("rahmen", "Rahmen", Typ::Auswahl(crate::darstellung::RAHMEN_STILE), "ascii = nur einfache Zeichen (+-|). Alte Programme bekommen das immer."),
    f("inventar", "Inventar", Typ::Auswahl(crate::darstellung::INVENTAR_STILE), "figur = angelegte Dinge an einer Figur, der Rest nach Art geordnet daneben."),
    f("figur", "Eigene Figur", Typ::Bild, "Leer = die eingebaute Figur. Tipp: unten in der Vorschau \"Eingebaute Figur übernehmen\"."),
    f("figur_plaetze", "Plätze an der Figur", Typ::Tabellen { felder: FIGUR_PLATZ, inline: false }, "Wo die Namen der angelegten Dinge stehen. Leer bei eingebauter Figur = Standard."),
];
const FIGUR_PLATZ: &[Feld] = felder![
    f("platz", "Platz", Typ::Auswahl(PLAETZE), ""),
    f("zeile", "Zeile", Typ::Zahl, "Ab 0 (oberste Zeile der Figur)."),
    f("spalte", "Spalte", Typ::Zahl, "Ab 0."),
    f("ausrichtung", "Ausrichtung", Typ::Auswahl(crate::darstellung::AUSRICHTUNGEN), "links: Name beginnt an der Spalte. rechts: Name endet dort (für Plätze links der Figur)."),
];

pub const ARTEN: &[Art] = &[
    Art { schluessel: "gegenstand", titel: "Gegenstände", einzahl: "Gegenstand", symbol: "🎒", ordner: "gegenstaende", ablage: Ablage::Liste, felder: GEGENSTAND, hilfe: "Waffen, Rüstungen, Tränke, Essen und alles andere." },
    Art { schluessel: "monster", titel: "Monster", einzahl: "Monster", symbol: "🐺", ordner: "monster", ablage: Ablage::Liste, felder: MONSTER, hilfe: "Wesen, gegen die man kämpft. In Räume setzt man sie unter 🗺 Räume." },
    Art { schluessel: "npc", titel: "NPCs", einzahl: "NPC", symbol: "👤", ordner: "npcs", ablage: Ablage::EineDatei, felder: NPC, hilfe: "Bewohner mit Gesprächen. Sie können Händler, Lehrmeister und Gildenmeister sein und Aufgaben vergeben. Jeder NPC hat eine eigene Datei." },
    Art { schluessel: "fertigkeit", titel: "Fertigkeiten", einzahl: "Fertigkeit", symbol: "📈", ordner: "fertigkeiten", ablage: Ablage::Liste, felder: FERTIGKEIT, hilfe: "" },
    Art { schluessel: "zauber", titel: "Zauber und Techniken", einzahl: "Zauber", symbol: "✨", ordner: "zauber", ablage: Ablage::Liste, felder: ZAUBER, hilfe: "Zauber (zaubere) und Kampftechniken (spezial). Wirkungen mit Dauer sind Schilde und Stärkungen." },
    Art { schluessel: "quest", titel: "Aufgaben und Quests", einzahl: "Quest", symbol: "📜", ordner: "quests", ablage: Ablage::Liste, felder: QUEST, hilfe: "Ein NPC bietet sie an (geber). Die Schritte werden der Reihe nach erledigt." },
    Art { schluessel: "befehl", titel: "Eigene Befehle", einzahl: "Befehl", symbol: "⌨", ordner: "eigene_befehle", ablage: Ablage::Liste, felder: BEFEHL, hilfe: "Neue Befehle mit Bedingungen (Ort, Gegenstände, Gilde, Kosten) und Wirkungen (Heilen, Geld, Gegenstände, Teleport, Zauber, weitere Befehle). Für reine Texte gibt es Gesten unter ⌨ Befehle." },
    Art { schluessel: "koerper", titel: "Körperbau", einzahl: "Körper", symbol: "✋", ordner: "koerper", ablage: Ablage::Liste, felder: KOERPER, hilfe: "Körperteile für Rassen und Monster: Hände zum Halten, Maul und Pfoten zum Angreifen – alles mit Beschreibung und Details." },
    Art { schluessel: "wetter", titel: "Wetter", einzahl: "Wetter", symbol: "☁", ordner: "wetter", ablage: Ablage::Liste, felder: WETTER, hilfe: "Wetterarten. Das Wetter wechselt von selbst, wenn unter Welt → Tag, Nacht und Jahreszeiten die Zeit an ist." },
    Art { schluessel: "rasse", titel: "Rassen", einzahl: "Rasse", symbol: "👥", ordner: "rassen", ablage: Ablage::Liste, felder: RASSE, hilfe: "Werden bei der Erschaffung einer Figur gewählt." },
    Art { schluessel: "gilde", titel: "Gilden", einzahl: "Gilde", symbol: "🛡", ordner: "gilden", ablage: Ablage::Liste, felder: GILDE, hilfe: "Man ist in höchstens einer Gilde. Beitreten bei einem NPC mit dieser Gilde." },
    Art { schluessel: "beruf", titel: "Berufe und Zünfte", einzahl: "Beruf", symbol: "⚒", ordner: "berufe", ablage: Ablage::Liste, felder: GILDE, hilfe: "Man kann mehrere Berufe haben (Welt → Berufe höchstens)." },
    Art { schluessel: "beschreibung", titel: "Allgemeine Beschreibungen", einzahl: "Beschreibung", symbol: "📖", ordner: "beschreibungen", ablage: Ablage::Liste, felder: BESCHREIBUNG, hilfe: "Bäume, Himmel, Wasser ... – Räume und Dinge verweisen darauf mit 'nutzt'." },
    Art { schluessel: "welt", titel: "Welt", einzahl: "Welt", symbol: "🌍", ordner: "", ablage: Ablage::Fest { datei: "welt.toml" }, felder: WELT, hilfe: "Grundeinstellungen. Den Startraum wählt man unter 🗺 Räume." },
    Art { schluessel: "start", titel: "Start neuer Figuren", einzahl: "Start", symbol: "🐣", ordner: "", ablage: Ablage::Fest { datei: "welt.toml" }, felder: START, hilfe: "Womit neue Figuren beginnen." },
    Art { schluessel: "geld", titel: "Geld und Währung", einzahl: "Geld", symbol: "💰", ordner: "", ablage: Ablage::Fest { datei: "welt.toml" }, felder: GELD, hilfe: "Einheiten wie Gold, Silber, Kupfer. Alle Preise in den Daten stehen in der kleinsten Einheit." },
    Art { schluessel: "darstellung", titel: "Darstellung", einzahl: "Darstellung", symbol: "🎨", ordner: "", ablage: Ablage::Fest { datei: "welt.toml" }, felder: DARSTELLUNG, hilfe: "Wie Handelslisten, Spielstand und Spielerbuch im Spiel aussehen – mit Vorschau." },
    Art { schluessel: "ernaehrung", titel: "Hunger und Durst", einzahl: "Hunger und Durst", symbol: "🍞", ordner: "", ablage: Ablage::Fest { datei: "welt.toml" }, felder: ERNAEHRUNG, hilfe: "Essen (Typ nahrung) sättigt, Getränke (Typ getraenk) löschen den Durst. Wie viel, steht bei den Gegenständen unter Eigenschaften: essen, trinken." },
    Art { schluessel: "zeit", titel: "Tag, Nacht und Jahreszeiten", einzahl: "Zeit", symbol: "🌙", ordner: "", ablage: Ablage::Fest { datei: "welt.toml" }, felder: ZEIT, hilfe: "Die Spielzeit läuft aus der echten Uhr – für alle gleich. Räume können eine Beschreibung für die Nacht haben (unter 🗺 Räume)." },
    Art { schluessel: "kampf", titel: "Kampfregeln", einzahl: "Kampf", symbol: "⚔", ordner: "", ablage: Ablage::Fest { datei: "welt.toml" }, felder: KAMPF, hilfe: "Den Raum für Wiederbelebungen wählt man unter 🗺 Räume." },
];

/// Felder, ohne die die Engine einen Eintrag nicht laden kann.
/// `eltern` ist die Art (z.B. "quest") oder der Name einer Liste (z.B. "schritte").
pub fn pflicht(eltern: &str, feld: &str) -> bool {
    match (eltern, feld) {
        ("gegenstand" | "monster" | "npc" | "fertigkeit" | "zauber" | "rasse" | "gilde" | "beruf" | "quest", "id" | "name") => true,
        ("beschreibung", "id") | ("quest", "geber") => true,
        ("schritte", "art" | "ziel") | ("lehrt", "id") | ("beute", "gegenstand") | ("wirkungen", "art") => true,
        ("thema", "woerter" | "antwort") => true,
        _ => false,
    }
}

pub fn art(schluessel: &str) -> Option<&'static Art> {
    ARTEN.iter().find(|a| a.schluessel == schluessel)
}
