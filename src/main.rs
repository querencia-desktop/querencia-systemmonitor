//! Querencia Systemmonitor – Systemüberwachung für Querencia Desktop.
//!
//! Vorbild ist die GNOME-2-Systemüberwachung: Reiter Prozesse, Ressourcen,
//! Dateisysteme; Menüleiste, Statuszeile. GTK 4 ohne libadwaita und ohne
//! eigene Titelleiste, damit labwc die klassische Server-Titelleiste
//! zeichnet. Daten kommen direkt aus `/proc`. Texte über GNU gettext
//! (`i18n`): msgid englisch, Deutsch aus `po/de.po`.
//!
//! Eine Instanz je Sitzung (GApplication `org.querencia.Systemmonitor`):
//! ein zweiter Aufruf holt das Fenster nach vorn.

mod dateisystemliste;
mod daten;
mod dialoge;
mod einstellungen;
mod fenster;
mod i18n;
mod prozessliste;
mod ressourcen;
mod sys;

use gtk::prelude::*;
use gtk::{gio, glib};

pub const APP_ID: &str = "org.querencia.Systemmonitor";
pub const APP_NAME: &str = "Querencia Systemmonitor";

/// Fenstertitel wie die GNOME-2-Systemüberwachung (übersetzt).
pub fn fenster_titel() -> String {
    // TRANSLATORS: Fenstertitel und Titel der Rückfragen.
    i18n::gettext("System Monitor")
}

fn main() -> glib::ExitCode {
    i18n::init();
    let app = gtk::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::default())
        .build();
    app.connect_startup(|app| {
        dialoge::fenstersymbol_registrieren();
        fenster::css_laden();
        fenster::kuerzel_installieren(app);
        // GTK 4.22 lädt beim Start das Thema neu und setzt dabei das
        // Farbschema zurück: Unter „dunkel“ bliebe ein neu gestartetes
        // Programm hell, bis zum ersten Wechsel. Die Meldung stößt das
        // Nachladen einmal an (hell bleibt unverändert; CSS-Welle dunkel).
        if let Some(s) = gtk::Settings::default() {
            s.notify("gtk-interface-color-scheme");
        }
    });
    app.connect_activate(|app| {
        fenster::Fenster::holen(app).zeigen();
    });
    app.run()
}
