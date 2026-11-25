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

/// Füllt mit Leerzeichen auf die sichtbare Breite auf (Farbcodes zählen nicht).
pub fn auffuellen(t: &str, breite: usize) -> String {
    let fehlt = breite.saturating_sub(sichtbare_laenge(t));
    format!("{t}{}", " ".repeat(fehlt))
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

/// Bricht Absätze fließend um: Die ersten `n` Zeilen sind höchstens `schmal` breit
/// (neben der Karte), alle weiteren höchstens `breit`. Jeder Absatz ist
/// (Text, Leerzeile davor?).
pub fn umbrechen_fliessend(absaetze: &[(String, bool)], schmal: usize, n: usize, breit: usize) -> Vec<String> {
    let mut zeilen: Vec<String> = Vec::new();
    for (absatz, leer_davor) in absaetze {
        if *leer_davor && !zeilen.is_empty() {
            zeilen.push(String::new());
        }
        let max = |z: usize| if z < n { schmal } else { breit };
        let mut zeile = String::new();
        let mut laenge = 0;
        for wort in absatz.split_whitespace() {
            let wl = sichtbare_laenge(wort);
            if laenge > 0 && laenge + 1 + wl > max(zeilen.len()) {
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
    }
    zeilen
}

/// Die Farben, die am Ende eines Textes noch eingeschaltet sind (zum Wiedereinschalten
/// in der nächsten Zeile, z.B. "{fett}{gelb}").
pub fn offene_farben(t: &str) -> String {
    let mut offen: Vec<&str> = Vec::new();
    let mut rest = t;
    while let Some(pos) = rest.find('{') {
        let danach = &rest[pos + 1..];
        match danach.find('}') {
            Some(ende) if ist_farbe(&danach[..ende]) => {
                let name = &danach[..ende];
                if name == "/" {
                    offen.clear();
                } else {
                    offen.push(name);
                }
                rest = &danach[ende + 1..];
            }
            _ => rest = danach,
        }
    }
    offen.iter().map(|n| format!("{{{n}}}")).collect()
}

/// Setzt zwei Spalten nebeneinander: links (z.B. die Karte), rechts der Text.
/// Zeilen, die unter der linken Spalte weitergehen, beginnen ganz links.
pub fn nebeneinander(links: &[String], rechts: &[String], abstand: usize) -> String {
    let lb = links.iter().map(|z| sichtbare_laenge(z)).max().unwrap_or(0);
    let mut aus = String::new();
    let mut farbe = String::new();
    for i in 0..links.len().max(rechts.len()) {
        let r = rechts.get(i).filter(|r| !r.is_empty());
        if let Some(l) = links.get(i) {
            // Nur auffüllen, wenn rechts etwas folgt (keine Leerzeichen am Zeilenende)
            aus.push_str(&if r.is_some() { auffuellen(l, lb + abstand) } else { l.clone() });
            aus.push_str("{/}");
        }
        if let Some(r) = r {
            // Farben pro Zeile schließen, damit sie nicht in die Karte hineinlaufen
            let mit = format!("{farbe}{r}");
            farbe = offene_farben(&mit);
            aus.push_str(&mit);
            aus.push_str("{/}");
        }
        aus.push('\n');
    }
    aus
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

/// Dativ aus dem Namen raten, wenn keiner angegeben ist:
/// "eine fette Ratte" -> "einer fetten Ratte", "ein grauer Wolf" -> "einem grauen Wolf",
/// "der Wirt" -> "dem Wirt", "die Spinne" -> "der Spinne", "das Pferd" -> "dem Pferd".
pub fn dativ_raten(name: &str) -> String {
    let woerter: Vec<&str> = name.split_whitespace().collect();
    let Some((erstes, rest)) = woerter.split_first() else { return name.to_string() };
    let artikel = match erstes.to_lowercase().as_str() {
        "ein" => "einem",
        "eine" => "einer",
        "der" | "das" => "dem",
        "die" => "der",
        _ => return name.to_string(),
    };
    let gross_anfang = erstes.chars().next().is_some_and(|c| c.is_uppercase());
    let mut aus = vec![if gross_anfang { gross(artikel) } else { artikel.to_string() }];
    // Adjektive (klein geschrieben, vor dem Hauptwort) bekommen -en
    let n = rest.len();
    for (i, w) in rest.iter().enumerate() {
        let adjektiv = i + 1 < n && w.chars().next().is_some_and(|c| c.is_lowercase());
        if adjektiv {
            let stamm = w.strip_suffix("er").or_else(|| w.strip_suffix("es")).or_else(|| w.strip_suffix('e')).unwrap_or(w);
            aus.push(format!("{stamm}en"));
        } else {
            aus.push(w.to_string());
        }
    }
    aus.join(" ")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fliessend_umbrechen() {
        let abs = vec![("eins zwei drei vier fuenf sechs".to_string(), false)];
        // zwei schmale Zeilen (10), dann breit (40)
        let z = umbrechen_fliessend(&abs, 10, 2, 40);
        assert_eq!(z, vec!["eins zwei", "drei vier", "fuenf sechs"]);
    }

    #[test]
    fn farben_laufen_nicht_in_die_karte() {
        assert_eq!(offene_farben("a {fett}{gelb}b"), "{fett}{gelb}");
        assert_eq!(offene_farben("{rot}a{/} b"), "");
        let t = nebeneinander(&["{gruen}#{/}".into(), "##".into()], &["{rot}x".into(), "y".into()], 1);
        assert_eq!(t, "{gruen}#{/}  {/}{rot}x{/}\n## {/}{rot}y{/}\n");
    }

    #[test]
    fn dativ() {
        assert_eq!(super::dativ_raten("eine fette Ratte"), "einer fetten Ratte");
        assert_eq!(super::dativ_raten("ein grauer Wolf"), "einem grauen Wolf");
        assert_eq!(super::dativ_raten("der Wirt"), "dem Wirt");
        assert_eq!(super::dativ_raten("Schwester Alwine"), "Schwester Alwine");
    }
}
