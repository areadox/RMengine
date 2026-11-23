//! Hilfsfunktionen rund um Text: Farben, Zeilenumbruch, Vergleich von Wörtern.
//!
//! In allen Texten der Weltdaten kann man Farben so setzen:
//!   "Ein {rot}gefährlicher{/} Ort."
//! `{/}` schaltet die Farbe wieder zurück.

/// Alle Farbnamen, die in Texten als `{name}` erlaubt sind.
pub const FARBEN: &[(&str, &str)] = &[
    ("/", "\x1b[0m"),
    ("fett", "\x1b[1m"),
    ("schwarz", "\x1b[30m"),
    ("rot", "\x1b[31m"),
    ("gruen", "\x1b[32m"),
    ("gelb", "\x1b[33m"),
    ("blau", "\x1b[34m"),
    ("magenta", "\x1b[35m"),
    ("cyan", "\x1b[36m"),
    ("weiss", "\x1b[37m"),
    ("grau", "\x1b[90m"),
    ("hellrot", "\x1b[91m"),
    ("hellgruen", "\x1b[92m"),
    ("hellgelb", "\x1b[93m"),
    ("hellblau", "\x1b[94m"),
    ("hellmagenta", "\x1b[95m"),
    ("hellcyan", "\x1b[96m"),
    ("hellweiss", "\x1b[97m"),
];

pub fn ist_farbe(name: &str) -> bool {
    FARBEN.iter().any(|(n, _)| *n == name)
}

/// Ersetzt `{farbe}`-Markierungen durch Terminal-Farbcodes (oder entfernt sie).
pub fn farben_umwandeln(t: &str, an: bool) -> String {
    let mut aus = String::with_capacity(t.len() + 16);
    let mut rest = t;
    let mut benutzt = false;
    while let Some(pos) = rest.find('{') {
        aus.push_str(&rest[..pos]);
        let danach = &rest[pos + 1..];
        if let Some(ende) = danach.find('}') {
            let name = &danach[..ende];
            if let Some((_, code)) = FARBEN.iter().find(|(n, _)| *n == name) {
                if an {
                    aus.push_str(code);
                    benutzt = true;
                }
                rest = &danach[ende + 1..];
                continue;
            }
        }
        aus.push('{');
        rest = danach;
    }
    aus.push_str(rest);
    if benutzt {
        aus.push_str("\x1b[0m");
    }
    aus
}

/// Länge eines Textes, wie man ihn auf dem Bildschirm sieht (ohne Farbmarkierungen).
pub fn sichtbare_laenge(t: &str) -> usize {
    farben_umwandeln(t, false).chars().count()
}

/// Bricht einen Fließtext auf die gegebene Breite um.
/// Einzelne Zeilenumbrüche im Text werden zu Leerzeichen,
/// eine Leerzeile beginnt einen neuen Absatz.
pub fn umbrechen(t: &str, breite: usize) -> String {
    let mut absaetze = Vec::new();
    for absatz in t.trim().split("\n\n") {
        let mut zeilen: Vec<String> = Vec::new();
        let mut zeile = String::new();
        let mut laenge = 0;
        for wort in absatz.split_whitespace() {
            let wl = sichtbare_laenge(wort);
            if laenge > 0 && laenge + 1 + wl > breite {
                zeilen.push(std::mem::take(&mut zeile));
                laenge = 0;
            }
            if laenge > 0 {
                zeile.push(' ');
                laenge += 1;
            }
            zeile.push_str(wort);
            laenge += wl;
        }
        if !zeile.is_empty() {
            zeilen.push(zeile);
        }
        absaetze.push(zeilen.join("\n"));
    }
    absaetze.join("\n\n")
}

/// Bereitet Text für das Versenden über Telnet vor.
pub fn fuer_telnet(t: &str, farben: bool, zeilenende: bool) -> Vec<u8> {
    let mut s = farben_umwandeln(t, farben).replace("\r\n", "\n").replace('\n', "\r\n");
    if zeilenende {
        s.push_str("\r\n");
    }
    s.into_bytes()
}

/// Kleinschreibung und Umlaute ersetzen, damit "Süden", "sueden" und "SÜDEN" gleich sind.
pub fn normalisieren(t: &str) -> String {
    t.trim()
        .to_lowercase()
        .replace('ä', "ae")
        .replace('ö', "oe")
        .replace('ü', "ue")
        .replace('ß', "ss")
}

/// Erster Buchstabe groß: "eine Ratte" -> "Eine Ratte".
pub fn gross(t: &str) -> String {
    let mut c = t.chars();
    match c.next() {
        Some(erster) => erster.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// ["a", "b", "c"] -> "a, b und c"
pub fn aufzaehlen(teile: &[String]) -> String {
    match teile.len() {
        0 => String::new(),
        1 => teile[0].clone(),
        n => format!("{} und {}", teile[..n - 1].join(", "), teile[n - 1]),
    }
}

/// Wie viele Buchstaben muss man ändern, um von `a` nach `b` zu kommen?
pub fn abstand(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut vorher: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut jetzt = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let kosten = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            jetzt[j] = (vorher[j] + 1).min(jetzt[j - 1] + 1).min(vorher[j - 1] + kosten);
        }
        vorher = jetzt;
    }
    vorher[b.len()]
}

/// Sucht bei einem Tippfehler das ähnlichste bekannte Wort.
pub fn vorschlag<'a>(wort: &str, kandidaten: impl Iterator<Item = &'a String>) -> Option<String> {
    let grenze = (wort.chars().count() / 3).max(2);
    kandidaten
        .map(|k| (abstand(wort, k), k))
        .filter(|(d, _)| *d <= grenze)
        .min_by_key(|(d, _)| *d)
        .map(|(_, k)| k.clone())
}
