//! Eine leere Welt anlegen (Engine: --neue-welt, Editor: Weltauswahl).

use std::path::Path;

/// Legt eine leere Welt an: welt.toml, ein Startraum und die üblichen Ordner.
/// Eine vorhandene Welt wird nie angetastet (Ergebnis false).
pub fn anlegen(ordner: &Path) -> std::io::Result<bool> {
    if ordner.join("welt.toml").exists() {
        return Ok(false);
    }
    for u in [
        "raeume", "gegenstaende", "monster", "npcs", "fertigkeiten", "zauber", "beschreibungen",
        "vorlagen", "rassen", "gilden", "berufe", "quests", "befehle", "eigene_befehle",
    ] {
        std::fs::create_dir_all(ordner.join(u))?;
    }
    std::fs::write(ordner.join("raeume").join("start.toml"), include_str!("leere_welt/start.toml"))?;
    std::fs::write(ordner.join("welt.toml"), include_str!("leere_welt/welt.toml"))?;
    Ok(true)
}

