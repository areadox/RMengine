//! Update-Funktion: prüft auf GitHub, ob es eine neue Version gibt,
//! und tauscht Programme aus. Die Weltdaten (Ordner `daten`) und die
//! Spielstände (Ordner `spieler`) werden dabei NIE angefasst.
//!
//! Diese Datei benutzen Engine UND Welteditor (dort per `#[path]` eingebunden).
//! Beide haben dieselbe Versionsnummer.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const REPO: &str = "areadox/RMengine";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Die Programme, die zu RMengine gehören.
pub const ENGINE: &str = "rmengine";
pub const EDITOR: &str = "rmeditor";

/// Name einer Programmdatei im Release für dieses Betriebssystem,
/// z.B. programm = "rmengine" -> "rmengine-windows-x86_64.exe".
fn asset_name(programm: &str) -> Option<String> {
    if cfg!(all(windows, target_arch = "x86_64")) {
        Some(format!("{programm}-windows-x86_64.exe"))
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some(format!("{programm}-linux-x86_64"))
    } else {
        None
    }
}

/// Wo liegt ein Programm? Neben dem gerade laufenden.
pub fn programm_pfad(programm: &str) -> Option<PathBuf> {
    let name = if cfg!(windows) { format!("{programm}.exe") } else { programm.to_string() };
    std::env::current_exe().ok().and_then(|p| p.parent().map(|o| o.join(name)))
}

#[derive(Clone)]
pub struct Release {
    pub version: String,
    pub notizen: String,
    pub dateien: Vec<(String, String)>, // (Name, Download-Adresse)
}

fn agent(zeit: Duration) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(zeit)
        .try_proxy_from_env(true)
        .user_agent(&format!("rmengine-updater/{VERSION}"))
        .build()
}

/// Fragt GitHub nach der neuesten veröffentlichten Version.
pub fn neueste_version(zeit: Duration) -> Result<Release, String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let antwort: serde_json::Value = agent(zeit)
        .get(&url)
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| match e {
            ureq::Error::Status(404, _) => "Es gibt noch keine veröffentlichte Version.".to_string(),
            e => format!("GitHub ist nicht erreichbar: {e}"),
        })?
        .into_json()
        .map_err(|e| format!("Unerwartete Antwort von GitHub: {e}"))?;
    let tag = antwort["tag_name"].as_str().unwrap_or_default();
    let dateien = antwort["assets"]
        .as_array()
        .map(|liste| {
            liste
                .iter()
                .filter_map(|a| Some((a["name"].as_str()?.to_string(), a["browser_download_url"].as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default();
    Ok(Release {
        version: tag.trim_start_matches('v').to_string(),
        notizen: antwort["body"].as_str().unwrap_or_default().to_string(),
        dateien,
    })
}

/// Nur den Abschnitt "## Was ist neu?" aus dem Release-Text.
pub fn neuerungen(notizen: &str) -> Option<String> {
    let start = notizen.find("## Was ist neu?")? + "## Was ist neu?".len();
    let rest = &notizen[start..];
    let ende = rest.find("\n## ").unwrap_or(rest.len());
    let text = rest[..ende].trim();
    (!text.is_empty()).then(|| text.to_string())
}

/// "1.2.10" > "1.2.9"
pub fn ist_neuer(neu: &str, alt: &str) -> bool {
    let zahlen = |v: &str| -> Vec<u64> { v.split(['.', '-']).take(3).map(|t| t.parse().unwrap_or(0)).collect() };
    zahlen(neu) > zahlen(alt)
}

/// Kurze Prüfung beim Start – meldet nur, ob es etwas Neues gibt.
pub fn beim_start_pruefen() {
    std::thread::spawn(|| {
        if let Ok(r) = neueste_version(Duration::from_secs(4)) {
            if ist_neuer(&r.version, VERSION) {
                println!();
                println!("  *** Neue Version {} verfügbar (du hast {VERSION}). ***", r.version);
                println!("  *** Zum Aktualisieren: Server beenden und 'rmengine --update' ausführen. ***");
                println!();
            }
        }
    });
}

/// Tauscht die gewählten Programme (z.B. ["rmengine", "rmeditor"]) gegen die Version
/// aus dem Release aus. Programme, die nicht neben diesem liegen, werden übersprungen.
/// Gibt die Namen der aktualisierten Programme zurück.
pub fn programme_aktualisieren(r: &Release, programme: &[&str], melden: &mut dyn FnMut(String)) -> Result<Vec<String>, String> {
    let mut fertig = Vec::new();
    for programm in programme {
        let Some(pfad) = programm_pfad(programm).filter(|p| p.exists()) else {
            melden(format!("{programm}: liegt nicht hier – übersprungen."));
            continue;
        };
        let name = asset_name(programm).ok_or("Für dieses Betriebssystem gibt es kein automatisches Update.")?;
        let url = r
            .dateien
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, u)| u.clone())
            .ok_or_else(|| format!("Im Release fehlt die Datei '{name}'."))?;
        melden(format!("Lade {name} herunter ..."));
        let daten = herunterladen(&url)?;
        if daten.len() < 100_000 {
            return Err(format!("{name}: Die heruntergeladene Datei ist zu klein – Update abgebrochen."));
        }
        programm_ersetzen(&pfad, &daten)?;
        melden(format!("{programm} aktualisiert."));
        fertig.push(programm.to_string());
    }
    // Anleitung mitaktualisieren (die Weltdaten bleiben unberührt)
    if let (Some((_, u)), Some(ordner)) = (
        r.dateien.iter().find(|(n, _)| n == "ANLEITUNG.md"),
        std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)),
    ) {
        if let Ok(text) = herunterladen(u) {
            let _ = fs::write(ordner.join("ANLEITUNG.md"), text);
        }
    }
    Ok(fertig)
}

/// Der Update-Vorgang der Engine (`rmengine --update`): Engine und Editor.
#[allow(dead_code)]
pub fn aktualisieren() -> Result<(), String> {
    println!("RMengine {VERSION} – suche nach Updates ...");
    let r = neueste_version(Duration::from_secs(20))?;
    if !ist_neuer(&r.version, VERSION) {
        println!("Du hast bereits die neueste Version ({VERSION}).");
        return Ok(());
    }
    println!("Neue Version gefunden: {}\n", r.version);
    if let Some(neu) = neuerungen(&r.notizen) {
        println!("Was ist neu?\n{neu}\n");
    }
    programme_aktualisieren(&r, &[ENGINE, EDITOR], &mut |t| println!("{t}"))?;
    println!("Fertig! RMengine ist jetzt auf Version {}.", r.version);
    println!("Deine Welt im Ordner 'daten' und die Spielstände wurden nicht verändert.");
    Ok(())
}

fn herunterladen(url: &str) -> Result<Vec<u8>, String> {
    let antwort = agent(Duration::from_secs(300)).get(url).call().map_err(|e| format!("Download fehlgeschlagen: {e}"))?;
    let mut daten = Vec::new();
    antwort
        .into_reader()
        .take(200 * 1024 * 1024)
        .read_to_end(&mut daten)
        .map_err(|e| format!("Download abgebrochen: {e}"))?;
    Ok(daten)
}

/// Tauscht eine Programmdatei aus. Auch unter Windows erlaubt:
/// Eine laufende .exe kann man umbenennen, nur nicht überschreiben.
fn programm_ersetzen(programm: &Path, daten: &[u8]) -> Result<(), String> {
    let neu = mit_endung(programm, "neu");
    let alt = mit_endung(programm, "alt");
    let _ = fs::remove_file(&alt);
    {
        let mut f = fs::File::create(&neu).map_err(|e| format!("Kann nicht schreiben ({}): {e}", neu.display()))?;
        f.write_all(daten).map_err(|e| e.to_string())?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&neu, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }
    fs::rename(programm, &alt).map_err(|e| format!("Kann das alte Programm nicht beiseitelegen: {e}"))?;
    if let Err(e) = fs::rename(&neu, programm) {
        let _ = fs::rename(&alt, programm); // zurückrollen
        return Err(format!("Kann das neue Programm nicht einsetzen: {e}"));
    }
    Ok(())
}

fn mit_endung(p: &Path, endung: &str) -> PathBuf {
    let mut s = p.as_os_str().to_owned();
    s.push(format!(".{endung}"));
    PathBuf::from(s)
}

static START_PFAD: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// Der Pfad des Programms beim Start. Nach einem Update zeigt `current_exe()` unter
/// Linux auf die umbenannte alte Datei – für den Neustart brauchen wir aber die neue.
pub fn eigener_pfad() -> PathBuf {
    START_PFAD.get().cloned().unwrap_or_else(|| std::env::current_exe().unwrap_or_default())
}

/// Beim Programmstart aufrufen: merkt sich den eigenen Pfad und
/// räumt die Sicherungen vom letzten Update weg.
pub fn aufraeumen() {
    if let Ok(p) = std::env::current_exe() {
        let _ = START_PFAD.set(p);
    }
    for programm in [ENGINE, EDITOR] {
        if let Some(p) = programm_pfad(programm) {
            let _ = fs::remove_file(mit_endung(&p, "alt"));
        }
    }
}

/// Startet das laufende Programm neu (mit denselben Argumenten) – nach einem Update.
#[allow(dead_code)]
pub fn neustart() -> ! {
    let programm = eigener_pfad();
    let args: Vec<String> = std::env::args().skip(1).collect();
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // exec ersetzt den laufenden Prozess; offene Verbindungen werden dabei geschlossen
        let fehler = std::process::Command::new(&programm).args(&args).exec();
        eprintln!("Neustart fehlgeschlagen: {fehler}");
        std::process::exit(1);
    }
    #[cfg(not(unix))]
    {
        let _ = std::process::Command::new(&programm).args(&args).spawn();
        std::process::exit(0);
    }
}
