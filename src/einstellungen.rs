//! `~/.config/querencia/systemmonitor.ini` (über `XDG_CONFIG_HOME`):
//! Fenstergröße, Intervall, Filter, Sortierung, sichtbare Spalten.
//! Gelesen und geschrieben mit GKeyFile, also im INI-Format.

use std::path::PathBuf;

use gtk::glib;

pub const FILTER_ALLE: &str = "alle";
pub const FILTER_EIGENE: &str = "eigene";
pub const FILTER_AKTIVE: &str = "aktive";

/// Kennungen der optionalen Spalten (Name ist immer sichtbar).
pub const SPALTEN: [&str; 6] = ["benutzer", "pid", "cpu", "speicher", "nice", "zustand"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Einstellungen {
    pub breite: i32,
    pub hoehe: i32,
    pub maximiert: bool,
    /// Aktualisierung der Prozessliste in Sekunden (1–10).
    pub intervall: u32,
    pub filter: String,
    pub sortierung: String,
    pub absteigend: bool,
    pub spalten: Vec<String>,
    pub alle_dateisysteme: bool,
    pub reiter: i32,
}

impl Default for Einstellungen {
    fn default() -> Self {
        Self {
            breite: 820,
            hoehe: 600,
            maximiert: false,
            intervall: 2,
            filter: FILTER_ALLE.to_string(),
            sortierung: "name".to_string(),
            absteigend: false,
            spalten: SPALTEN.iter().map(|s| s.to_string()).collect(),
            alle_dateisysteme: false,
            reiter: 0,
        }
    }
}

impl Einstellungen {
    pub fn pfad() -> PathBuf {
        glib::user_config_dir()
            .join("querencia")
            .join("systemmonitor.ini")
    }

    pub fn laden() -> Self {
        match std::fs::read_to_string(Self::pfad()) {
            Ok(text) => Self::aus_text(&text),
            Err(_) => Self::default(),
        }
    }

    /// Fehlende oder unbrauchbare Werte bleiben beim Standard.
    pub fn aus_text(text: &str) -> Self {
        let mut e = Self::default();
        let kf = glib::KeyFile::new();
        if kf.load_from_data(text, glib::KeyFileFlags::NONE).is_err() {
            return e;
        }
        if let Ok(v) = kf.integer("Fenster", "Breite") {
            e.breite = v.clamp(400, 8192);
        }
        if let Ok(v) = kf.integer("Fenster", "Hoehe") {
            e.hoehe = v.clamp(300, 8192);
        }
        if let Ok(v) = kf.boolean("Fenster", "Maximiert") {
            e.maximiert = v;
        }
        if let Ok(v) = kf.integer("Fenster", "Reiter") {
            e.reiter = v.clamp(0, 2);
        }
        if let Ok(v) = kf.integer("Prozesse", "Intervall") {
            e.intervall = v.clamp(1, 10) as u32;
        }
        if let Ok(v) = kf.string("Prozesse", "Filter")
            && [FILTER_ALLE, FILTER_EIGENE, FILTER_AKTIVE].contains(&v.as_str())
        {
            e.filter = v.to_string();
        }
        if let Ok(v) = kf.string("Prozesse", "Sortierung")
            && (v == "name" || SPALTEN.contains(&v.as_str()))
        {
            e.sortierung = v.to_string();
        }
        if let Ok(v) = kf.boolean("Prozesse", "Absteigend") {
            e.absteigend = v;
        }
        if let Ok(v) = kf.string_list("Prozesse", "Spalten") {
            e.spalten = v
                .iter()
                .map(|s| s.to_string())
                .filter(|s| SPALTEN.contains(&s.as_str()))
                .collect();
        }
        if let Ok(v) = kf.boolean("Dateisysteme", "AlleAnzeigen") {
            e.alle_dateisysteme = v;
        }
        e
    }

    pub fn als_text(&self) -> String {
        let kf = glib::KeyFile::new();
        kf.set_integer("Fenster", "Breite", self.breite);
        kf.set_integer("Fenster", "Hoehe", self.hoehe);
        kf.set_boolean("Fenster", "Maximiert", self.maximiert);
        kf.set_integer("Fenster", "Reiter", self.reiter);
        kf.set_integer("Prozesse", "Intervall", self.intervall as i32);
        kf.set_string("Prozesse", "Filter", &self.filter);
        kf.set_string("Prozesse", "Sortierung", &self.sortierung);
        kf.set_boolean("Prozesse", "Absteigend", self.absteigend);
        // Listenform von GKeyFile („a;b;“), gelesen mit string_list.
        let spalten: String = self.spalten.iter().map(|s| format!("{s};")).collect();
        kf.set_string("Prozesse", "Spalten", &spalten);
        kf.set_boolean("Dateisysteme", "AlleAnzeigen", self.alle_dateisysteme);
        format!(
            "# Querencia Systemmonitor – wird beim Beenden geschrieben.\n{}",
            kf.to_data()
        )
    }

    pub fn speichern(&self) -> std::io::Result<()> {
        let pfad = Self::pfad();
        if let Some(dir) = pfad.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(pfad, self.als_text())
    }

    pub fn spalte_sichtbar(&self, kennung: &str) -> bool {
        self.spalten.iter().any(|s| s == kennung)
    }

    pub fn spalte_setzen(&mut self, kennung: &str, sichtbar: bool) {
        self.spalten.retain(|s| s != kennung);
        if sichtbar {
            self.spalten.push(kennung.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hin_und_zurueck() {
        let mut e = Einstellungen {
            breite: 1024,
            hoehe: 700,
            maximiert: true,
            intervall: 5,
            filter: FILTER_EIGENE.to_string(),
            sortierung: "cpu".to_string(),
            absteigend: true,
            spalten: vec!["pid".to_string(), "cpu".to_string()],
            alle_dateisysteme: true,
            reiter: 1,
        };
        let text = e.als_text();
        assert!(
            text.contains("[Fenster]") && text.contains("Breite=1024"),
            "{text}"
        );
        assert_eq!(Einstellungen::aus_text(&text), e);
        e.spalte_setzen("pid", false);
        e.spalte_setzen("zustand", true);
        assert!(
            !e.spalte_sichtbar("pid") && e.spalte_sichtbar("zustand") && e.spalte_sichtbar("cpu")
        );
    }

    #[test]
    fn unbrauchbares_bleibt_standard() {
        let e = Einstellungen::aus_text(
            "[Prozesse]\nIntervall=99\nFilter=quatsch\nSortierung=egal\nSpalten=pid;quatsch;\n[Fenster]\nBreite=10\n",
        );
        assert_eq!(e.intervall, 10, "auf 1–10 begrenzt");
        assert_eq!(e.filter, FILTER_ALLE);
        assert_eq!(e.sortierung, "name");
        assert_eq!(e.spalten, vec!["pid".to_string()]);
        assert_eq!(e.breite, 400);
        assert_eq!(
            Einstellungen::aus_text("kein ini ["),
            Einstellungen::default()
        );
        assert_eq!(Einstellungen::aus_text(""), Einstellungen::default());
    }
}
