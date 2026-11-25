//! Wie Listen im Spiel aussehen: Handelslisten, Spielstand und Spielerbuch –
//! in verschiedenen Stilen, dazu die Anzeige von Geldbeträgen.
//!
//! Diese Datei benutzen Engine UND Welteditor (dort per `#[path]` eingebunden,
//! für die Vorschau). Sie bekommt fertige Daten und liefert Text mit
//! Farbmarkierungen wie `{gelb}...{/}`; sie kennt die Welt nicht.

// ---------------------------------------------------------------------------
// Stile
// ---------------------------------------------------------------------------

pub const HAENDLER_STILE: &[&str] = &["einfach", "tabelle", "pergament", "kategorien"];
pub const SPIELSTAND_STILE: &[&str] = &["einfach", "bogen", "kompakt", "beides"];
pub const SPIELERBUCH_STILE: &[&str] = &["einfach", "buch", "schalter"];
pub const RAHMEN_STILE: &[&str] = &["unicode", "ascii"];
pub const GELD_ANZEIGEN: &[&str] = &["lang", "kurz"];
pub const INVENTAR_STILE: &[&str] = &["einfach", "figur"];
pub const AUSRICHTUNGEN: &[&str] = &["links", "rechts"];

// ---------------------------------------------------------------------------
// Text-Hilfen (Farbmarkierungen zählen nicht zur Breite)
// ---------------------------------------------------------------------------

/// Text ohne Farbmarkierungen wie {gelb} oder {/}.
pub fn ohne_farben(t: &str) -> String {
    let mut aus = String::with_capacity(t.len());
    let mut rest = t;
    while let Some(pos) = rest.find('{') {
        aus.push_str(&rest[..pos]);
        let danach = &rest[pos + 1..];
        match danach.find('}') {
            Some(ende) if ende <= 12 && danach[..ende].chars().all(|c| c.is_ascii_lowercase() || c == '/') && ende > 0 => {
                rest = &danach[ende + 1..];
            }
            _ => {
                aus.push('{');
                rest = danach;
            }
        }
    }
    aus.push_str(rest);
    aus
}

pub fn breite(t: &str) -> usize {
    ohne_farben(t).chars().count()
}

/// Links ausrichten und mit Leerzeichen auffüllen (zu Langes wird mit … gekürzt).
pub fn links(t: &str, n: usize) -> String {
    let b = breite(t);
    if b > n {
        return kuerzen(t, n);
    }
    format!("{t}{}", " ".repeat(n - b))
}

pub fn rechts(t: &str, n: usize) -> String {
    let b = breite(t);
    if b > n {
        return kuerzen(t, n);
    }
    format!("{}{t}", " ".repeat(n - b))
}

pub fn mitte(t: &str, n: usize) -> String {
    let b = breite(t);
    if b >= n {
        return kuerzen(t, n);
    }
    let l = (n - b) / 2;
    format!("{}{t}{}", " ".repeat(l), " ".repeat(n - b - l))
}

/// Kürzt auf n sichtbare Zeichen (Farben gehen dabei verloren).
fn kuerzen(t: &str, n: usize) -> String {
    let roh = ohne_farben(t);
    if n == 0 {
        return String::new();
    }
    let mut s: String = roh.chars().take(n.saturating_sub(1)).collect();
    s.push('…');
    s
}

/// Rahmen- und Sonderzeichen durch einfache Zeichen ersetzen (für alte Programme).
pub fn ascii_zeichen(c: char) -> Option<&'static str> {
    Some(match c {
        '═' | '─' | '━' | '╌' => "-",
        '║' | '│' | '┃' => "|",
        '╔' | '╗' | '╚' | '╝' | '╠' | '╣' | '╦' | '╩' | '╬' | '╟' | '╢' | '╤' | '╧' | '┌' | '┐' | '└' | '┘' | '├' | '┤'
        | '┬' | '┴' | '┼' => "+",
        '█' | '■' => "#",
        '░' | '□' => "-",
        '◆' | '•' => "*",
        '‹' => "<",
        '›' => ">",
        '…' => "...",
        '✓' => "v",
        _ => return None,
    })
}

pub fn ascii(t: &str) -> String {
    let mut aus = String::with_capacity(t.len());
    for c in t.chars() {
        match ascii_zeichen(c) {
            Some(e) => aus.push_str(e),
            None => aus.push(c),
        }
    }
    aus
}

// ---------------------------------------------------------------------------
// Geld
// ---------------------------------------------------------------------------

/// Eine Geldeinheit, z.B. Gold = 100 Kupfer.
#[derive(Clone, Debug)]
pub struct Einheit {
    pub name: String,
    pub mehrzahl: String,
    pub kurz: String,
    /// Wert in der kleinsten Einheit
    pub wert: u64,
}

/// "2 Gold 5 Silber" (lang) oder "2G 5S" (kurz). Beträge sind immer in der kleinsten Einheit.
pub fn betrag(n: u64, einheiten: &[Einheit], kurz: bool) -> String {
    let mut liste: Vec<&Einheit> = einheiten.iter().filter(|e| e.wert > 0).collect();
    liste.sort_by(|a, b| b.wert.cmp(&a.wert));
    let Some(kleinste) = liste.last() else { return n.to_string() };
    let form = |anzahl: u64, e: &Einheit| -> String {
        if kurz && !e.kurz.is_empty() {
            format!("{anzahl}{}", e.kurz)
        } else {
            let name = if anzahl == 1 || e.mehrzahl.is_empty() { &e.name } else { &e.mehrzahl };
            format!("{anzahl} {name}")
        }
    };
    if n == 0 {
        return form(0, kleinste);
    }
    let mut rest = n;
    let mut teile = Vec::new();
    for e in &liste {
        let anzahl = rest / e.wert;
        if anzahl > 0 {
            teile.push(form(anzahl, e));
            rest -= anzahl * e.wert;
        }
    }
    teile.join(" ")
}

// ---------------------------------------------------------------------------
// Handelslisten
// ---------------------------------------------------------------------------

pub struct Zeile {
    pub nr: usize,
    pub name: String,
    pub preis: String,
    /// Kurzinfo, z.B. "+40 LP" oder "bis Stufe 20, du: 3"
    pub info: String,
}

pub struct Gruppe {
    /// Leer = ohne Überschrift
    pub titel: String,
    pub zeilen: Vec<Zeile>,
}

pub struct Laden {
    pub titel: String,
    pub waren: Vec<Gruppe>,
    /// Zusatzzeile unter den Waren, z.B. "Kauft an: alles (50 % des Werts)"
    pub ankauf: String,
    pub lehren: Vec<Zeile>,
    pub lehren_titel: String,
}

/// Eine Handelsliste in einem Stil. `breite` = Zeilenbreite des Spielers.
pub fn laden(stil: &str, l: &Laden, breite: usize) -> String {
    match stil {
        "tabelle" if breite >= 48 => laden_tabelle(l, breite.min(70)),
        "pergament" if breite >= 44 => laden_pergament(l, breite.min(60)),
        "kategorien" if breite >= 40 => laden_kategorien(l, breite.min(72)),
        _ => laden_einfach(l),
    }
}

fn laden_einfach(l: &Laden) -> String {
    let mut t = String::new();
    let hat_waren = l.waren.iter().any(|g| !g.zeilen.is_empty());
    if hat_waren || !l.ankauf.is_empty() {
        t.push_str(&format!("{{fett}}{} verkauft:{{/}}\n", l.titel));
        for g in &l.waren {
            if !g.titel.is_empty() {
                t.push_str(&format!("  {{cyan}}{}{{/}}\n", g.titel));
            }
            for z in &g.zeilen {
                let info = if z.info.is_empty() { String::new() } else { format!(" {{grau}}({}){{/}}", z.info) };
                t.push_str(&format!("  {{gelb}}{:>2}.{{/}} {} {{hellgelb}}{}{{/}}{info}\n", z.nr, links(&z.name, 30), z.preis));
            }
        }
        if !l.ankauf.is_empty() {
            t.push_str(&format!("  {{grau}}{}{{/}}\n", l.ankauf));
        }
    }
    if !l.lehren.is_empty() {
        t.push_str(&format!("{{fett}}{} {}:{{/}}\n", l.titel, l.lehren_titel));
        for z in &l.lehren {
            let info = if z.info.is_empty() { String::new() } else { format!(" {{grau}}({}){{/}}", z.info) };
            t.push_str(&format!("  {{gelb}}{:>2}.{{/}} {} {{hellgelb}}{}{{/}}{info}\n", z.nr, links(&z.name, 24), z.preis));
        }
    }
    t
}

fn laden_tabelle(l: &Laden, w: usize) -> String {
    // ║ Nr │ Name ... │ Preis ║  -> innen w-2 Zeichen
    let innen = w - 2;
    let laengster = l.waren.iter().flat_map(|g| g.zeilen.iter()).chain(l.lehren.iter()).map(|z| breite(&z.preis)).max().unwrap_or(5);
    let preis_b = (laengster + 2).clamp(8, innen / 2);
    let name_b = innen - 4 - 1 - 1 - preis_b; // " Nr " │ Name │ Preis
    let g = |t: &str| format!("{{grau}}{t}{{/}}");
    let zeile = |nr: String, name: String, preis: String| {
        format!(
            "{}{}{}{}{}{}{}\n",
            g("║"),
            format!(" {} ", rechts(&nr, 2)),
            g("│"),
            format!(" {}", links(&name, name_b - 1)),
            g("│"),
            format!("{} ", rechts(&preis, preis_b - 1)),
            g("║")
        )
    };
    let trenner = |l: &str, m: &str, r: &str, linie: &str| {
        g(&format!("{l}{}{m}{}{m}{}{r}", linie.repeat(4), linie.repeat(name_b), linie.repeat(preis_b))) + "\n"
    };
    let mut t = String::new();
    t.push_str(&g(&format!("╔{}╗", "═".repeat(innen))));
    t.push('\n');
    t.push_str(&format!("{}{}{}\n", g("║"), mitte(&format!("{{fett}}{{hellgelb}}~ {} ~{{/}}", l.titel), innen), g("║")));
    let abschnitt = |t: &mut String, titel: &str, zeilen: Vec<(String, String, String)>| {
        t.push_str(&trenner("╠", "╤", "╣", "═"));
        t.push_str(&zeile("Nr".into(), format!("{{fett}}{titel}{{/}}"), "Preis".into()));
        t.push_str(&trenner("╟", "┼", "╢", "─"));
        for (nr, name, preis) in zeilen {
            t.push_str(&zeile(nr, name, preis));
        }
    };
    let mut waren = Vec::new();
    for gr in &l.waren {
        if !gr.titel.is_empty() {
            waren.push((String::new(), format!("{{cyan}}{}{{/}}", gr.titel), String::new()));
        }
        for z in &gr.zeilen {
            let name = if z.info.is_empty() { z.name.clone() } else { format!("{} {{grau}}({}){{/}}", z.name, z.info) };
            waren.push((format!("{{gelb}}{}{{/}}", z.nr), name, format!("{{hellgelb}}{}{{/}}", z.preis)));
        }
    }
    if !waren.is_empty() {
        abschnitt(&mut t, "Ware", waren);
    }
    if !l.lehren.is_empty() {
        let lehren = l
            .lehren
            .iter()
            .map(|z| {
                let name = if z.info.is_empty() { z.name.clone() } else { format!("{} {{grau}}({}){{/}}", z.name, z.info) };
                (format!("{{gelb}}{}{{/}}", z.nr), name, format!("{{hellgelb}}{}{{/}}", z.preis))
            })
            .collect();
        abschnitt(&mut t, &l.lehren_titel, lehren);
    }
    if !l.ankauf.is_empty() {
        t.push_str(&trenner("╟", "┴", "╢", "─"));
        t.push_str(&format!("{}{}{}\n", g("║"), links(&format!(" {{grau}}{}{{/}}", l.ankauf), innen), g("║")));
        t.push_str(&g(&format!("╚{}╝", "═".repeat(innen))));
    } else {
        t.push_str(&g(&format!("╚{}╧{}╧{}╝", "═".repeat(4), "═".repeat(name_b), "═".repeat(preis_b))));
    }
    t.push('\n');
    t
}

fn laden_pergament(l: &Laden, w: usize) -> String {
    let innen = w - 5; // "  |" + innen + "|"
    let g = |t: &str| format!("{{grau}}{t}{{/}}");
    let zeile = |t: &str| format!("  {}{}{}\n", g("|"), links(t, innen), g("|"));
    let mut t = String::new();
    t.push_str(&g(&format!("   {}", "_".repeat(innen))));
    t.push('\n');
    t.push_str(&g(&format!("  /{}\\", " ".repeat(innen))));
    t.push('\n');
    let punkte = |nr: usize, name: &str, preis: &str| {
        let vorn = format!("  {nr:>2}. {name} ");
        let hinten = format!(" {preis}  ");
        let frei = innen.saturating_sub(breite(&vorn) + breite(&hinten)).max(1);
        format!("{{gelb}}  {nr:>2}.{{/}} {name} {{grau}}{}{{/}} {{hellgelb}}{preis}{{/}}  ", ".".repeat(frei))
    };
    let hat_waren = l.waren.iter().any(|g| !g.zeilen.is_empty());
    if hat_waren {
        t.push_str(&zeile(&mitte(&format!("{{fett}}~ {} bietet feil ~{{/}}", l.titel), innen)));
        t.push_str(&zeile(""));
        for gr in &l.waren {
            if !gr.titel.is_empty() {
                t.push_str(&zeile(&format!("  {{cyan}}{}{{/}}", gr.titel)));
            }
            for z in &gr.zeilen {
                t.push_str(&zeile(&punkte(z.nr, &z.name, &z.preis)));
            }
        }
    }
    if !l.lehren.is_empty() {
        if hat_waren {
            t.push_str(&zeile(""));
        }
        t.push_str(&zeile(&mitte(&format!("{{fett}}~ {} {} ~{{/}}", l.titel, l.lehren_titel), innen)));
        t.push_str(&zeile(""));
        for z in &l.lehren {
            t.push_str(&zeile(&punkte(z.nr, &z.name, &z.preis)));
            if !z.info.is_empty() {
                t.push_str(&zeile(&format!("       {{grau}}{}{{/}}", z.info)));
            }
        }
    }
    if !l.ankauf.is_empty() {
        t.push_str(&zeile(""));
        t.push_str(&zeile(&format!("  {{grau}}{}{{/}}", l.ankauf)));
    }
    t.push_str(&g(&format!("  \\{}/", "_".repeat(innen))));
    t.push('\n');
    t
}

fn laden_kategorien(l: &Laden, w: usize) -> String {
    let linie = |titel: &str| {
        let t = format!(" {titel} ");
        let rest = w.saturating_sub(breite(&t));
        let a = rest / 2;
        format!("{{grau}}{}{{/}}{{fett}}{{hellgelb}}{t}{{/}}{{grau}}{}{{/}}\n", "━".repeat(a), "━".repeat(rest - a))
    };
    let preis_b = l.waren.iter().flat_map(|g| g.zeilen.iter()).chain(l.lehren.iter()).map(|z| breite(&z.preis)).max().unwrap_or(4);
    let name_b = 34usize.min(w.saturating_sub(16 + preis_b));
    let eintrag = |z: &Zeile| {
        let vorn = format!("   {:>2}. {} ", z.nr, z.name);
        let frei = (name_b + 6).saturating_sub(breite(&vorn)).max(1);
        let info = if z.info.is_empty() { String::new() } else { format!("  {{grau}}{}{{/}}", z.info) };
        format!(
            "   {{gelb}}{:>2}.{{/}} {} {{grau}}{}{{/}} {{hellgelb}}{}{{/}}{info}\n",
            z.nr,
            z.name,
            "·".repeat(frei),
            rechts(&z.preis, preis_b)
        )
    };
    let mut t = String::new();
    let hat_waren = l.waren.iter().any(|g| !g.zeilen.is_empty());
    if hat_waren {
        t.push_str(&linie(&l.titel));
        for gr in &l.waren {
            if !gr.titel.is_empty() {
                t.push_str(&format!(" {{cyan}}◆ {}{{/}}\n", gr.titel));
            }
            for z in &gr.zeilen {
                t.push_str(&eintrag(z));
            }
        }
    }
    if !l.lehren.is_empty() {
        t.push_str(&linie(&format!("{} {}", l.titel, l.lehren_titel)));
        for z in &l.lehren {
            t.push_str(&eintrag(z));
        }
    }
    t.push_str(&format!("{{grau}}{}{{/}}\n", "━".repeat(w)));
    if !l.ankauf.is_empty() {
        t.push_str(&format!(" {{grau}}{}{{/}}\n", l.ankauf));
    }
    t
}

// ---------------------------------------------------------------------------
// Spielstand
// ---------------------------------------------------------------------------

pub struct Balken {
    pub name: String,
    /// Kurz für die kompakte Ansicht, z.B. "LP"
    pub kurz: String,
    pub akt: i64,
    pub max: i64,
    pub farbe: String,
    pub zusatz: String,
}

pub struct Bogen {
    /// "Lyra der Zauberlehrling"
    pub name: String,
    /// "[Admin]" usw. (mit Farben)
    pub marke: String,
    /// "Elfe · Gilde der Magier · Stufe 3"
    pub untertitel: String,
    pub stufe: u32,
    /// LP, MP, AP
    pub balken: Vec<Balken>,
    /// Erfahrung bis zur nächsten Stufe
    pub erfahrung: Balken,
    /// z.B. "2 Punkte zu verteilen (steigere <wert>)"
    pub hinweise: Vec<String>,
    /// (Anzeigename, Wert gesamt, Bonus)
    pub werte: Vec<(String, i32, i32)>,
    /// (Platz, Gegenstand)
    pub ausruestung: Vec<(String, String)>,
    pub geld: String,
    pub gewicht: String,
    pub statistik: String,
    pub wirkungen: String,
    /// Weitere Abschnitte für die einfache Ansicht: (Titel, Zeilen)
    pub abschnitte: Vec<(String, Vec<(String, String)>)>,
}

fn balken_text(akt: i64, max: i64, n: usize, voll: char, leer: char, farbe: &str) -> String {
    let anteil = if max > 0 { (akt.max(0) as f64 / max as f64).min(1.0) } else { 0.0 };
    let v = (anteil * n as f64).round() as usize;
    format!("{{{farbe}}}{}{{/}}{{grau}}{}{{/}}", voll.to_string().repeat(v), leer.to_string().repeat(n - v))
}

/// `stil`: einfach, bogen oder kompakt ("beides" wählt die Engine je nach Befehl).
pub fn spielstand(stil: &str, b: &Bogen, breite: usize) -> String {
    match stil {
        "bogen" if breite >= 60 => spielstand_bogen(b, breite.min(72)),
        "kompakt" => spielstand_kompakt(b, breite.clamp(40, 90)),
        _ => spielstand_einfach(b),
    }
}

fn spielstand_einfach(b: &Bogen) -> String {
    let mut t = format!("{{fett}}Spielstand von {}{{/}}{}\n", b.name, b.marke);
    if !b.untertitel.is_empty() {
        t.push_str(&format!("{{grau}}{}{{/}}\n", b.untertitel));
    }
    t.push('\n');
    for x in &b.balken {
        t.push_str(&format!(
            "  {} [{}] {:>4}/{}\n",
            links(&x.name, 12),
            balken_text(x.akt, x.max, 20, '#', '-', &x.farbe),
            x.akt,
            x.max
        ));
    }
    let e = &b.erfahrung;
    t.push_str(&format!(
        "\n  {{fett}}{}{{/}} [{}] {:>4}/{} EP\n",
        links(&format!("Stufe {}", b.stufe), 12),
        balken_text(e.akt, e.max, 20, '#', '-', &e.farbe),
        e.akt,
        e.max
    ));
    for h in &b.hinweise {
        t.push_str(&format!("  {{hellgelb}}{h}{{/}}\n"));
    }
    t.push_str(&format!("  {{grau}}{}{{/}}\n", b.statistik));
    t.push_str(&format!("  Geld: {{hellgelb}}{}{{/}}\n", b.geld));
    if !b.wirkungen.is_empty() {
        t.push_str(&format!("  {{hellcyan}}Es wirkt: {}{{/}}\n", b.wirkungen));
    }
    if !b.werte.is_empty() {
        t.push_str("\n{gelb}Werte{/}\n");
        for (name, wert, bonus) in &b.werte {
            t.push_str(&format!("  {} {:>4}{}\n", links(name, 13), wert, bonus_text(*bonus)));
        }
    }
    t.push_str("\n{gelb}Ausrüstung{/}\n");
    if b.ausruestung.is_empty() {
        t.push_str("  nichts angelegt\n");
    }
    for (platz, ding) in &b.ausruestung {
        t.push_str(&format!("  {} {}\n", links(platz, 13), ding));
    }
    for (titel, zeilen) in &b.abschnitte {
        t.push_str(&format!("\n{{gelb}}{titel}{{/}}\n"));
        for (k, v) in zeilen {
            t.push_str(&format!("  {} {}\n", links(k, 13), v));
        }
    }
    t.trim_end().to_string()
}

fn bonus_text(bonus: i32) -> String {
    match bonus {
        0 => String::new(),
        b if b > 0 => format!(" {{hellgruen}}(+{b}){{/}}"),
        b => format!(" {{hellrot}}({b}){{/}}"),
    }
}

fn spielstand_bogen(b: &Bogen, w: usize) -> String {
    let innen = w - 2;
    let g = |t: &str| format!("{{grau}}{t}{{/}}");
    let zeile = |t: &str| format!("{}{}{}\n", g("│"), links(&format!(" {t}"), innen), g("│"));
    let links_b = innen / 2 - 1;
    let rechts_b = innen - links_b - 1;
    let mut t = String::new();
    t.push_str(&g(&format!("┌{}┐", "─".repeat(innen))));
    t.push('\n');
    t.push_str(&format!("{}{}{}\n", g("│"), mitte(&format!("{{fett}}{{hellgelb}}{}{{/}}{}", b.name.to_uppercase(), if b.marke.is_empty() { String::new() } else { format!(" {}", b.marke) }), innen), g("│")));
    if !b.untertitel.is_empty() {
        t.push_str(&format!("{}{}{}\n", g("│"), mitte(&format!("{{grau}}{}{{/}}", b.untertitel), innen), g("│")));
    }
    t.push_str(&g(&format!("├{}┤", "─".repeat(innen))));
    t.push('\n');
    let balken_b = 20;
    for x in b.balken.iter().chain(std::iter::once(&b.erfahrung)) {
        let zusatz = if x.zusatz.is_empty() { String::new() } else { format!(" {{grau}}{}{{/}}", x.zusatz) };
        t.push_str(&zeile(&format!(
            "{} {} {}{zusatz}",
            links(&x.name, 9),
            balken_text(x.akt, x.max, balken_b, '█', '░', &x.farbe),
            rechts(&format!("{}/{}", x.akt, x.max), 8)
        )));
    }
    for h in &b.hinweise {
        t.push_str(&zeile(&format!("{{hellgelb}}{h}{{/}}")));
    }
    t.push_str(&g(&format!("├{}┬{}┤", "─".repeat(links_b), "─".repeat(rechts_b))));
    t.push('\n');
    let mut l: Vec<String> = vec!["{gelb}WERTE{/}".into()];
    l.extend(b.werte.iter().map(|(n, v, bo)| format!("{} {:>3}{}", links(n, 12), v, bonus_text(*bo))));
    let mut r: Vec<String> = vec!["{gelb}AUSRÜSTUNG{/}".into()];
    if b.ausruestung.is_empty() {
        r.push("nichts angelegt".into());
    }
    r.extend(b.ausruestung.iter().map(|(p, d)| format!("{} {d}", links(p, 8))));
    r.push(String::new());
    r.push(format!("{} {{hellgelb}}{}{{/}}", links("Geld", 8), b.geld));
    r.push(format!("{} {}", links("Gewicht", 8), b.gewicht));
    for i in 0..l.len().max(r.len()) {
        let a = l.get(i).cloned().unwrap_or_default();
        let c = r.get(i).cloned().unwrap_or_default();
        t.push_str(&format!("{}{}{}{}{}\n", g("│"), links(&format!(" {a}"), links_b), g("│"), links(&format!(" {c}"), rechts_b), g("│")));
    }
    t.push_str(&g(&format!("├{}┴{}┤", "─".repeat(links_b), "─".repeat(rechts_b))));
    t.push('\n');
    t.push_str(&zeile(&format!("{{grau}}{}{{/}}", b.statistik)));
    if !b.wirkungen.is_empty() {
        t.push_str(&zeile(&format!("{{hellcyan}}Es wirkt: {}{{/}}", b.wirkungen)));
    }
    t.push_str(&g(&format!("└{}┘", "─".repeat(innen))));
    t
}

fn spielstand_kompakt(b: &Bogen, w: usize) -> String {
    let mut t = String::new();
    let kopf = format!(" {} ", format!("{} {}", b.name, b.marke).trim_end());
    let rechts_t = format!(" {} ", b.untertitel);
    let frei = w.saturating_sub(breite(&kopf) + breite(&rechts_t) + 4).max(2);
    t.push_str(&format!(
        "{{grau}}══{{/}}{{fett}}{{hellgelb}}{kopf}{{/}}{{grau}}{}{{/}}{rechts_t}{{grau}}══{{/}}\n",
        "═".repeat(frei)
    ));
    let kurz = |x: &Balken| format!("{} [{}] {}/{}", x.kurz, balken_text(x.akt, x.max, 10, '■', '□', &x.farbe), x.akt, x.max);
    let balken: Vec<String> = b.balken.iter().map(kurz).collect();
    t.push_str(&format!(" {}\n", balken.join("   ")));
    t.push_str(&format!(
        " {}   Geld {{hellgelb}}{}{{/}}   Gewicht {}\n",
        kurz(&b.erfahrung),
        b.geld,
        b.gewicht
    ));
    let werte: Vec<String> = b
        .werte
        .iter()
        .map(|(n, v, _)| format!("{{gelb}}{}{{/}} {v}", n.chars().take(3).collect::<String>().to_uppercase()))
        .collect();
    let waffe = b.ausruestung.iter().find(|(p, _)| p == "Waffe").map(|(_, d)| format!("   Waffe: {d}")).unwrap_or_default();
    t.push_str(&format!(" {}{waffe}\n", werte.join("  ")));
    t.push_str(&format!(" {{grau}}{}{{/}}\n", b.statistik));
    for h in &b.hinweise {
        t.push_str(&format!(" {{hellgelb}}{h}{{/}}\n"));
    }
    if !b.wirkungen.is_empty() {
        t.push_str(&format!(" {{hellcyan}}Es wirkt: {}{{/}}\n", b.wirkungen));
    }
    let laenge = breite(t.lines().next().unwrap_or_default());
    t.push_str(&format!("{{grau}}{}{{/}}", "═".repeat(laenge)));
    t
}

// ---------------------------------------------------------------------------
// Spielerbuch
// ---------------------------------------------------------------------------

pub struct Einstellung {
    pub nr: usize,
    pub name: String,
    /// Angezeigter Wert, z.B. "an", "daneben", "auto (99)"
    pub wert: String,
    /// Ein-/Aus-Schalter: Some(an?)
    pub schalter: Option<bool>,
    /// Bei Auswahl: alle Möglichkeiten (der aktuelle Wert ist `wert`)
    pub auswahl: Vec<String>,
    pub hilfe: String,
}

pub struct Buch {
    pub titel: String,
    pub abschnitte: Vec<(String, Vec<Einstellung>)>,
    /// Zeilen ohne Einstellung (Figur, Programm ...): (Name, Wert)
    pub info: Vec<(String, Vec<(String, String)>)>,
    pub fuss: String,
}

pub fn spielerbuch(stil: &str, b: &Buch, breite: usize) -> String {
    match stil {
        "buch" if breite >= 56 => spielerbuch_buch(b, breite.min(70)),
        "schalter" => spielerbuch_schalter(b),
        _ => spielerbuch_einfach(b),
    }
}

fn wert_farbig(e: &Einstellung) -> String {
    match e.schalter {
        Some(true) => "{hellgruen}an{/}".into(),
        Some(false) => "{grau}aus{/}".into(),
        None => e.wert.clone(),
    }
}

fn spielerbuch_einfach(b: &Buch) -> String {
    let mut t = format!("{{fett}}{}{{/}}\n", b.titel);
    for (titel, liste) in &b.abschnitte {
        t.push_str(&format!("\n{{fett}}{titel}{{/}}\n"));
        for e in liste {
            t.push_str(&format!("  {{gelb}}{}{{/}} {} {{grau}}{}{{/}}\n", links(&e.name, 15), links(&wert_farbig(e), 9), e.hilfe));
        }
    }
    for (titel, zeilen) in &b.info {
        t.push_str(&format!("\n{{fett}}{titel}{{/}}\n"));
        for (k, v) in zeilen {
            t.push_str(&format!("  {{gelb}}{}{{/}} {v}\n", links(k, 15)));
        }
    }
    t.push_str(&format!("\n{}", b.fuss));
    t
}

fn spielerbuch_buch(b: &Buch, w: usize) -> String {
    let innen = w - 4;
    let g = |t: &str| format!("{{grau}}{t}{{/}}");
    let zeile = |t: &str| format!("  {}{}{}\n", g("║"), links(&format!(" {t}"), innen), g("║"));
    let mut t = String::new();
    t.push_str(&g(&format!("  ╔{}╗", "═".repeat(innen))));
    t.push('\n');
    t.push_str(&format!("  {}{}{}\n", g("║"), mitte(&format!("{{fett}}{{hellgelb}}~ {} ~{{/}}", b.titel), innen), g("║")));
    t.push_str(&g(&format!("  ╟{}╢", "─".repeat(innen))));
    t.push('\n');
    let mut erst = true;
    for (titel, liste) in &b.abschnitte {
        if !erst {
            t.push_str(&zeile(""));
        }
        erst = false;
        t.push_str(&zeile(&format!("{{fett}}{}{{/}}", titel.to_uppercase())));
        for e in liste {
            t.push_str(&zeile(&format!(
                "{{gelb}}{:>2}{{/}} {} [{}] {{grau}}{}{{/}}",
                e.nr,
                links(&e.name, 14),
                mitte(&wert_farbig(e), 10),
                e.hilfe
            )));
        }
    }
    for (titel, zeilen) in &b.info {
        t.push_str(&zeile(""));
        t.push_str(&zeile(&format!("{{fett}}{}{{/}}", titel.to_uppercase())));
        for (k, v) in zeilen {
            t.push_str(&zeile(&format!("   {} {v}", links(k, 14))));
        }
    }
    t.push_str(&g(&format!("  ╟{}╢", "─".repeat(innen))));
    t.push('\n');
    t.push_str(&zeile(&b.fuss));
    t.push_str(&g(&format!("  ╚{}╝", "═".repeat(innen))));
    t
}

fn spielerbuch_schalter(b: &Buch) -> String {
    let mut t = format!("{{fett}}~~~ {} ~~~{{/}}\n", b.titel);
    for (titel, liste) in &b.abschnitte {
        t.push_str(&format!("{{grau}}{titel}{{/}}\n"));
        for e in liste {
            let feld = match e.schalter {
                Some(true) => "{hellgruen}[x]{/}".to_string(),
                Some(false) => "{grau}[ ]{/}".to_string(),
                None if !e.auswahl.is_empty() => {
                    let teile: Vec<String> = e
                        .auswahl
                        .iter()
                        .map(|a| if *a == e.wert { format!("{{hellgruen}}[{a}]{{/}}") } else { format!("{{grau}}{a}{{/}}") })
                        .collect();
                    format!("‹ {} ›", teile.join(" │ "))
                }
                None => format!("{{hellgelb}}{}{{/}}", e.wert),
            };
            if e.schalter.is_some() {
                t.push_str(&format!("  {{gelb}}{:>2}{{/}} {feld} {}\n", e.nr, e.name));
            } else {
                t.push_str(&format!("  {{gelb}}{:>2}{{/}} {} {feld}\n", e.nr, links(&e.name, 15)));
            }
        }
    }
    for (titel, zeilen) in &b.info {
        t.push_str(&format!("{{grau}}{titel}{{/}}\n"));
        for (k, v) in zeilen {
            t.push_str(&format!("     {} {v}\n", links(k, 15)));
        }
    }
    t.push_str(&b.fuss);
    t
}

// ---------------------------------------------------------------------------
// Inventar mit Figur
// ---------------------------------------------------------------------------

/// Wo ein angelegter Gegenstand an der Figur steht.
#[derive(Debug, Clone)]
pub struct FigurPlatz {
    pub platz: String,
    /// Zeile und Spalte im Bild (ab 0)
    pub zeile: usize,
    pub spalte: usize,
    /// true = der Name endet an der Spalte (für Plätze links der Figur)
    pub rechts: bool,
}

/// Die eingebaute Figur.
pub const STANDARD_FIGUR: &str = r"                      ___
                     /   \
                     |   |
                     \___/
                   ___/ \___
                  /         \
                 / /|     |\ \
                / / |     | \ \
                 /  |     |  \
                    |  _  |
                    / / \ \
                   / /   \ \
                 _/ /     \ \_
                |__/       \__|";

/// Wo die Namen an der eingebauten Figur stehen.
pub fn standard_plaetze() -> Vec<FigurPlatz> {
    let p = |platz: &str, zeile, spalte, rechts| FigurPlatz { platz: platz.into(), zeile, spalte, rechts };
    vec![
        p("kopf", 1, 27, false),
        p("hals", 3, 27, false),
        p("umhang", 4, 29, false),
        p("koerper", 5, 30, false),
        p("ruecken", 6, 32, false),
        p("schild", 7, 33, false),
        p("haende", 8, 31, false),
        p("guertel", 9, 28, false),
        p("beine", 11, 29, false),
        p("fuesse", 13, 32, false),
        p("licht", 5, 17, true),
        p("waffe", 7, 15, true),
        p("ring", 8, 16, true),
    ]
}

pub struct Inventar {
    /// (Platz-ID, Name des Gegenstands)
    pub angelegt: Vec<(String, String)>,
    /// Überschrift -> Einträge
    pub gruppen: Vec<(String, Vec<String>)>,
    /// Zeilen unter allem (Behälter, Gewicht, Geld)
    pub fuss: Vec<String>,
}

/// Kurzer Name ohne Artikel: "ein roter Heiltrank" -> "Roter Heiltrank"
pub fn kurzname(name: &str) -> String {
    const ARTIKEL: &[&str] = &["ein", "eine", "einen", "einer", "eines", "der", "die", "das", "den", "dem", "des", "etwas"];
    let rest = match name.split_once(' ') {
        Some((a, r)) if ARTIKEL.contains(&a.to_lowercase().as_str()) => r,
        _ => name,
    };
    let mut c = rest.chars();
    match c.next() {
        Some(e) => e.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// Inventar mit Figur. None, wenn es nicht in die Breite passt.
pub fn inventar_figur(inv: &Inventar, bild: &str, plaetze: &[FigurPlatz], breite: usize) -> Option<String> {
    const MAX_NAME: usize = 18;
    // Bild als Raster; true = gehört zu einem Namen (wird farbig)
    let mut raster: Vec<Vec<(char, bool)>> = bild.lines().map(|z| z.trim_end().chars().map(|c| (c, false)).collect()).collect();
    let mut ohne_platz: Vec<String> = Vec::new();
    for (platz, name) in &inv.angelegt {
        let Some(p) = plaetze.iter().find(|p| p.platz == *platz) else {
            ohne_platz.push(kurzname(name));
            continue;
        };
        let mut n: Vec<char> = kurzname(name).chars().collect();
        let max = if p.rechts { (p.spalte + 1).min(MAX_NAME) } else { MAX_NAME };
        if n.len() > max {
            n.truncate(max.saturating_sub(1));
            n.push('…');
        }
        let start = if p.rechts { (p.spalte + 1).saturating_sub(n.len()) } else { p.spalte };
        while raster.len() <= p.zeile {
            raster.push(Vec::new());
        }
        let z = &mut raster[p.zeile];
        while z.len() < start + n.len() {
            z.push((' ', false));
        }
        for (i, c) in n.into_iter().enumerate() {
            z[start + i] = (c, true);
        }
    }
    // Liste rechts
    let mut liste: Vec<String> = Vec::new();
    let mut gruppen = inv.gruppen.clone();
    if !ohne_platz.is_empty() {
        gruppen.insert(0, ("Angelegt".into(), ohne_platz));
    }
    for (titel, eintraege) in &gruppen {
        if eintraege.is_empty() {
            continue;
        }
        if !liste.is_empty() {
            liste.push(String::new());
        }
        liste.push(format!("{{gelb}}{titel}{{/}}"));
        liste.extend(eintraege.iter().cloned());
    }
    if liste.is_empty() {
        liste.push("{grau}Du trägst nichts bei dir.{/}".into());
    }
    let links_breite = raster.iter().map(|z| z.len()).max().unwrap_or(0) + 3;
    let rest = breite.saturating_sub(links_breite + 1);
    if rest < 16 {
        return None;
    }
    let mut t = String::new();
    for i in 0..raster.len().max(liste.len()) {
        let mut zeile = String::new();
        let mut farbig = false;
        let leer = Vec::new();
        let z = raster.get(i).unwrap_or(&leer);
        for (c, name) in z {
            if *name != farbig {
                zeile.push_str(if *name { "{hellcyan}" } else { "{/}" });
                farbig = *name;
            }
            zeile.push(*c);
        }
        if farbig {
            zeile.push_str("{/}");
        }
        zeile.push_str(&" ".repeat(links_breite - z.len()));
        if let Some(e) = liste.get(i) {
            zeile.push_str(&links(e, rest));
        }
        t.push_str(zeile.trim_end());
        t.push('\n');
    }
    for f in &inv.fuss {
        t.push_str(f);
        t.push('\n');
    }
    Some(t.trim_end().to_string())
}

// ---------------------------------------------------------------------------
// Beispiele (für die Vorschau im Editor und für Tests)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub fn beispiel_inventar(geld: &str) -> Inventar {
    let g = |t: &str, e: &[&str]| (t.to_string(), e.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    Inventar {
        angelegt: vec![
            ("waffe".into(), "ein Messer".into()),
            ("kopf".into(), "eine Lederkappe".into()),
            ("koerper".into(), "ein Lederwams".into()),
            ("licht".into(), "eine Fackel".into()),
        ],
        gruppen: vec![
            g("Waffen", &["Kurzschwert"]),
            g("Tränke", &["Roter Heiltrank (2)"]),
            g("Sonstiges", &["Karte", "Spielerbuch"]),
            g("Essen", &["Handvoll Schwarzbeeren"]),
        ],
        fuss: vec![format!("{{grau}}Gewicht 9/70{{/}}   Geld: {{hellgelb}}{geld}{{/}}")],
    }
}

#[allow(dead_code)]
pub fn beispiel_laden(gruppiert: bool, geld: &dyn Fn(u64) -> String) -> Laden {
    let z = |nr, name: &str, preis, info: &str| Zeile { nr, name: name.into(), preis: geld(preis), info: info.into() };
    let waren = if gruppiert {
        vec![
            Gruppe { titel: "Essen".into(), zeilen: vec![z(1, "Ein Laib Brot", 20, ""), z(2, "Ein roter Apfel", 15, "")] },
            Gruppe {
                titel: "Tränke".into(),
                zeilen: vec![z(3, "Ein roter Heiltrank", 250, "+40 LP"), z(4, "Ein blauer Manatrank", 250, "+30 MP")],
            },
            Gruppe {
                titel: "Taschen und Behälter".into(),
                zeilen: vec![z(5, "Ein Lederbeutel", 50, "fasst 8"), z(6, "Ein Rucksack", 300, "fasst 40, -25 %")],
            },
        ]
    } else {
        vec![Gruppe {
            titel: String::new(),
            zeilen: vec![
                z(1, "Ein Laib Brot", 20, ""),
                z(2, "Ein roter Apfel", 15, ""),
                z(3, "Ein roter Heiltrank", 250, "+40 LP"),
                z(4, "Ein blauer Manatrank", 250, "+30 MP"),
                z(5, "Ein Lederbeutel", 50, "fasst 8"),
                z(6, "Ein Rucksack", 300, "fasst 40, -25 %"),
            ],
        }]
    };
    Laden {
        titel: "Gretas Krämerladen".into(),
        waren,
        ankauf: "Kauft an: alles (50 % des Werts)".into(),
        lehren: vec![Zeile { nr: 1, name: "Orientierung".into(), preis: format!("{} je Stufe", geld(80)), info: "bis 30, du: 5".into() }],
        lehren_titel: "lehrt".into(),
    }
}

#[allow(dead_code)]
pub fn beispiel_bogen(geld: &dyn Fn(u64) -> String) -> Bogen {
    let bk = |name: &str, kurz: &str, akt, max, farbe: &str| Balken {
        name: name.into(),
        kurz: kurz.into(),
        akt,
        max,
        farbe: farbe.into(),
        zusatz: String::new(),
    };
    Bogen {
        name: "Lyra der Zauberlehrling".into(),
        marke: String::new(),
        untertitel: "Elfe · Gilde der Magier · Stufe 3".into(),
        stufe: 3,
        balken: vec![bk("Leben", "LP", 90, 90, "gruen"), bk("Mana", "MP", 63, 95, "cyan"), bk("Ausdauer", "AP", 100, 100, "gelb")],
        erfahrung: Balken {
            name: "Erfahrung".into(),
            kurz: "EP".into(),
            akt: 70,
            max: 100,
            farbe: "hellmagenta".into(),
            zusatz: "bis Stufe 4".into(),
        },
        hinweise: vec!["2 Punkte zu verteilen (steigere <wert>)".into()],
        werte: vec![
            ("Stärke".into(), 9, -1),
            ("Geschick".into(), 12, 2),
            ("Ausdauer".into(), 10, 0),
            ("Intelligenz".into(), 12, 2),
            ("Weisheit".into(), 10, 0),
        ],
        ausruestung: vec![("Waffe".into(), "ein Zauberstab".into()), ("Rücken".into(), "ein Rucksack".into())],
        geld: geld(1475),
        gewicht: "7/67".into(),
        statistik: "Siege 5 · Tode 0 · Erkundet 5/26 · Spielzeit 12 Min".into(),
        wirkungen: "Schutzschild".into(),
        abschnitte: vec![(
            "Können".into(),
            vec![("Fertigkeiten".into(), "4 (Magie 7, Orientierung 5)".into()), ("Zauber".into(), "3 (Feuerpfeil …)".into())],
        )],
    }
}

#[allow(dead_code)]
pub fn beispiel_buch() -> Buch {
    let s = |nr, name: &str, an: bool, hilfe: &str| Einstellung {
        nr,
        name: name.into(),
        wert: String::new(),
        schalter: Some(an),
        auswahl: Vec::new(),
        hilfe: hilfe.into(),
    };
    let a = |nr, name: &str, wert: &str, moeglich: &[&str], hilfe: &str| Einstellung {
        nr,
        name: name.into(),
        wert: wert.into(),
        schalter: None,
        auswahl: moeglich.iter().map(|m| m.to_string()).collect(),
        hilfe: hilfe.into(),
    };
    Buch {
        titel: "Spielerbuch von Lyra".into(),
        abschnitte: vec![
            (
                "Anzeige".into(),
                vec![
                    s(1, "farben", true, "farbige Texte"),
                    s(2, "karte", true, "Karte beim Umsehen"),
                    a(3, "kartengroesse", "normal", &["klein", "normal", "gross"], "Größe der Karte"),
                    a(4, "kartenlage", "daneben", &["daneben", "oben"], "Text neben der Karte"),
                    s(5, "kurz", false, "ohne Raumbeschreibung"),
                ],
            ),
            ("Kampf".into(), vec![s(6, "autoangriff", true, "weiter angreifen"), s(7, "kampfzahlen", true, "Schaden als Zahl")]),
        ],
        info: vec![("Deine Figur".into(), vec![("titel".into(), "der Zauberlehrling".into())])],
        fuss: "Ändern: spielerbuch <nr|einstellung> [wert]".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn einheiten() -> Vec<Einheit> {
        let e = |n: &str, k: &str, w| Einheit { name: n.into(), mehrzahl: n.into(), kurz: k.into(), wert: w };
        vec![e("Gold", "G", 100), e("Silber", "S", 10), e("Kupfer", "K", 1)]
    }

    #[test]
    fn geld_anzeigen() {
        assert_eq!(betrag(1475, &einheiten(), false), "14 Gold 7 Silber 5 Kupfer");
        assert_eq!(betrag(1475, &einheiten(), true), "14G 7S 5K");
        assert_eq!(betrag(300, &einheiten(), false), "3 Gold");
        assert_eq!(betrag(0, &einheiten(), false), "0 Kupfer");
        let taler = vec![Einheit { name: "Taler".into(), mehrzahl: "Taler".into(), kurz: "T".into(), wert: 1 }];
        assert_eq!(betrag(5, &taler, false), "5 Taler");
        let muenze = vec![Einheit { name: "Münze".into(), mehrzahl: "Münzen".into(), kurz: String::new(), wert: 1 }];
        assert_eq!(betrag(1, &muenze, false), "1 Münze");
        assert_eq!(betrag(3, &muenze, true), "3 Münzen");
    }

    /// Alle Zeilen eines gerahmten Stils sind gleich breit.
    #[test]
    fn rahmen_sind_buendig() {
        let geld = |n: u64| betrag(n, &einheiten(), true);
        for stil in ["tabelle", "pergament"] {
            let t = laden(stil, &beispiel_laden(false, &geld), 78);
            let breiten: Vec<usize> = t.lines().skip(if stil == "pergament" { 2 } else { 0 }).map(breite).collect();
            assert!(breiten.windows(2).all(|w| w[0] == w[1]), "{stil}: {breiten:?}\n{}", ohne_farben(&t));
        }
        let t = spielstand("bogen", &beispiel_bogen(&geld), 78);
        let breiten: Vec<usize> = t.lines().map(breite).collect();
        assert!(breiten.windows(2).all(|w| w[0] == w[1]), "{breiten:?}\n{}", ohne_farben(&t));
        let t = spielerbuch("buch", &beispiel_buch(), 78);
        let breiten: Vec<usize> = t.lines().map(breite).collect();
        assert!(breiten.windows(2).all(|w| w[0] == w[1]), "{breiten:?}\n{}", ohne_farben(&t));
    }

    #[test]
    fn inventar_figur() {
        let inv = beispiel_inventar("1 Gold");
        let t = super::inventar_figur(&inv, STANDARD_FIGUR, &standard_plaetze(), 78).expect("passt in 78");
        let roh = ohne_farben(&t);
        for name in ["Messer", "Lederkappe", "Lederwams", "Fackel", "Kurzschwert", "Waffen"] {
            assert!(roh.contains(name), "{name} fehlt:\n{roh}");
        }
        assert!(roh.lines().all(|z| z.chars().count() <= 78), "{roh}");
        assert!(super::inventar_figur(&inv, STANDARD_FIGUR, &standard_plaetze(), 50).is_none());
        assert_eq!(kurzname("ein roter Heiltrank"), "Roter Heiltrank");
        assert_eq!(kurzname("Handvoll Beeren"), "Handvoll Beeren");
    }
}
