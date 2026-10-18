//! Querencia Systemmonitor – Systemüberwachung für Querencia Desktop.
//!
//! Vorbild ist die GNOME-2-Systemüberwachung: Reiter Prozesse, Ressourcen,
//! Dateisysteme; Menüleiste, Statuszeile. GTK 4 ohne libadwaita und ohne
//! eigene Titelleiste, damit labwc die klassische Server-Titelleiste
//! zeichnet. Daten kommen direkt aus `/proc`.
//!
//! Eine Instanz je Sitzung (GApplication `org.querencia.Systemmonitor`):
//! ein zweiter Aufruf holt das Fenster nach vorn.

mod dateisystemliste;
mod daten;
mod dialoge;
mod einstellungen;
mod fenster;
mod prozessliste;
mod ressourcen;
mod sys;

use gtk::prelude::*;
use gtk::{gio, glib};

pub const APP_ID: &str = "org.querencia.Systemmonitor";
pub const APP_NAME: &str = "Querencia Systemmonitor";
pub const FENSTER_TITEL: &str = "Systemüberwachung";

fn main() -> glib::ExitCode {
    let app = gtk::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::default())
        .build();
    app.connect_startup(|app| {
        dialoge::fenstersymbol_registrieren();
        fenster::css_laden();
        fenster::kuerzel_installieren(app);
    });
    app.connect_activate(|app| {
        fenster::Fenster::holen(app).zeigen();
    });
    app.run()
}
