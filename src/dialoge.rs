//! Dialoge als gewöhnliche, modale Fenster ohne eigene Titelleiste: labwc
//! dekoriert sie wie jedes andere Fenster. Escape und der Schließen-Knopf
//! brechen ab. Fenstersymbol und App-Zuordnung wie beim Querencia Editor
//! (GTK-4.22-Symbolabsturz: nie die PNG-Symbole der App-ID als
//! Fenstersymbol, nur ein eigener SVG-Name aus der Ressource).

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{gdk, gio, glib};

use crate::daten::einheiten;
use crate::daten::prozesse::Eigenschaften;
use crate::i18n::{gettext, gettext_f};
use crate::{APP_NAME, fenster_titel};

/// Noch nicht ausgeführte Antwort (wird genau einmal genommen).
type Antwort = Rc<RefCell<Option<Box<dyn FnOnce()>>>>;

/// Schließt `window` erst im nächsten Leerlauf, nicht aus dem gerade
/// laufenden Tasten- oder Klick-Handler heraus.
pub fn spaeter_schliessen(window: &gtk::Window) {
    let window = window.clone();
    glib::idle_add_local_once(move || window.close());
}

fn modales_fenster(parent: &impl IsA<gtk::Window>, title: &str) -> gtk::Window {
    let window = gtk::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(title)
        .resizable(false)
        .destroy_with_parent(true)
        .build();
    window.add_css_class("qsm-dialog");
    an_anwendung_binden(&window, parent);
    window
}

/// Dialoge gehören zur laufenden Anwendung (wie das Hauptfenster), damit
/// sie die App-ID tragen und labwc das Symbol über die Desktop-Datei findet.
pub fn an_anwendung_binden(window: &gtk::Window, parent: &impl IsA<gtk::Window>) {
    if let Some(app) = parent.as_ref().application() {
        window.set_application(Some(&app));
    }
    fenstersymbol_setzen(window);
}

/// Name des Fenstersymbols, das GTK labwc schickt (xdg-toplevel-icon):
/// bewusst ein eigener, nur als SVG vorhandener Name statt der App-ID.
const FENSTERSYMBOL: &str = "org.querencia.Systemmonitor-fenster";

/// Bettet das Fenstersymbol (offizielles Logo C2, nur SVG) als Ressource ein
/// und macht es dem Symbolthema bekannt; läuft nach dem Start von GTK.
pub fn fenstersymbol_registrieren() {
    let bytes =
        glib::Bytes::from_static(include_bytes!("../data/resource/systemmonitor.gresource"));
    match gio::Resource::from_data(&bytes) {
        Ok(resource) => {
            gio::resources_register(&resource);
            if let Some(display) = gdk::Display::default() {
                gtk::IconTheme::for_display(&display)
                    .add_resource_path("/org/querencia/Systemmonitor/icons");
            }
        }
        Err(err) => eprintln!("querencia-systemmonitor: Ressourcen nicht lesbar: {err}"),
    }
}

pub fn fenstersymbol_setzen(window: &impl IsA<gtk::Window>) {
    window.as_ref().set_icon_name(Some(FENSTERSYMBOL));
}

fn escape_schliesst(window: &gtk::Window) {
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let w = window.downgrade();
    keys.connect_key_pressed(move |_, key, _, _| {
        if key == gdk::Key::Escape {
            if let Some(w) = w.upgrade() {
                spaeter_schliessen(&w);
            }
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    window.add_controller(keys);
}

fn knopfzeile(buttons: &[&gtk::Button]) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    row.set_halign(gtk::Align::End);
    row.set_margin_top(12);
    for b in buttons {
        row.append(*b);
    }
    row
}

/// Frage im GNOME-2-Stil: Symbol, fette Hauptzeile, Erklärung.
fn meldung(icon: &str, primary: &str, secondary: &str) -> gtk::Box {
    let image = gtk::Image::from_icon_name(icon);
    image.set_pixel_size(48);
    image.set_valign(gtk::Align::Start);
    let text = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let p = gtk::Label::new(None);
    p.set_markup(&format!(
        "<b><big>{}</big></b>",
        glib::markup_escape_text(primary)
    ));
    p.set_wrap(true);
    p.set_max_width_chars(52);
    p.set_xalign(0.0);
    let s = gtk::Label::new(Some(secondary));
    s.set_wrap(true);
    s.set_max_width_chars(52);
    s.set_xalign(0.0);
    text.append(&p);
    text.append(&s);
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.append(&image);
    row.append(&text);
    row
}

fn inhalt(child: &impl IsA<gtk::Widget>, buttons: &gtk::Box) -> gtk::Box {
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    outer.set_margin_top(12);
    outer.set_margin_bottom(12);
    outer.set_margin_start(12);
    outer.set_margin_end(12);
    outer.append(child);
    outer.append(buttons);
    outer
}

fn schliessknopf(window: &gtk::Window, button: &gtk::Button) {
    let w = window.downgrade();
    button.connect_clicked(move |_| {
        if let Some(w) = w.upgrade() {
            spaeter_schliessen(&w);
        }
    });
}

/// Rückfrage mit einem bestätigenden Knopf (`bestaetigung`, mit Mnemonic);
/// `ja` läuft genau einmal, erst nachdem der Dialog geschlossen ist.
pub fn frage(
    parent: &impl IsA<gtk::Window>,
    primary: &str,
    secondary: &str,
    bestaetigung: &str,
    gefaehrlich: bool,
    ja: impl FnOnce() + 'static,
) {
    let window = modales_fenster(parent, &fenster_titel());
    let cancel = gtk::Button::with_mnemonic(&gettext("_Cancel"));
    let ok = gtk::Button::with_mnemonic(bestaetigung);
    ok.add_css_class(if gefaehrlich {
        "destructive-action"
    } else {
        "suggested-action"
    });
    let body = meldung("dialog-warning", primary, secondary);
    window.set_child(Some(&inhalt(&body, &knopfzeile(&[&cancel, &ok]))));
    window.set_default_widget(Some(&ok));
    escape_schliesst(&window);
    schliessknopf(&window, &cancel);
    let antwort: Antwort = Rc::new(RefCell::new(Some(Box::new(ja))));
    let w = window.downgrade();
    ok.connect_clicked(move |_| {
        let Some(f) = antwort.borrow_mut().take() else {
            return;
        };
        if let Some(w) = w.upgrade() {
            spaeter_schliessen(&w);
        }
        glib::idle_add_local_once(f);
    });
    window.present();
    ok.grab_focus();
}

/// Fehlermeldung mit einem Knopf „Schließen“.
pub fn fehler(parent: &impl IsA<gtk::Window>, primary: &str, secondary: &str) {
    let window = modales_fenster(parent, &fenster_titel());
    let close = gtk::Button::with_mnemonic(&gettext("_Close"));
    let body = meldung("dialog-error", primary, secondary);
    window.set_child(Some(&inhalt(&body, &knopfzeile(&[&close]))));
    window.set_default_widget(Some(&close));
    escape_schliesst(&window);
    schliessknopf(&window, &close);
    window.present();
    close.grab_focus();
}

/// „Priorität ändern“: Nice-Wert −20 … 19 mit Regler; `setzen(nice)` meldet
/// einen Fehlertext zurück, der dann angezeigt wird.
pub fn prioritaet(
    parent: &impl IsA<gtk::Window>,
    name: &str,
    pid: i32,
    aktuell: i32,
    setzen: impl Fn(i32) -> Result<(), String> + 'static,
) {
    let window = modales_fenster(parent, &gettext("Change Priority"));
    let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let titel = gtk::Label::new(None);
    titel.set_markup(&format!(
        "<b>{}</b>",
        gettext_f(
            "Change the priority of process “{name}” (PID {pid})",
            &[
                ("name", glib::markup_escape_text(name).as_str()),
                ("pid", &pid.to_string()),
            ],
        )
    ));
    titel.set_xalign(0.0);
    let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, -20.0, 19.0, 1.0);
    scale.set_value(f64::from(aktuell));
    scale.set_draw_value(true);
    scale.set_digits(0);
    scale.set_size_request(360, -1);
    // TRANSLATORS: Marken des Reglers im Dialog „Change Priority“.
    scale.add_mark(
        -20.0,
        gtk::PositionType::Bottom,
        Some(&gettext("Very High")),
    );
    scale.add_mark(0.0, gtk::PositionType::Bottom, Some(&gettext("Normal")));
    scale.add_mark(19.0, gtk::PositionType::Bottom, Some(&gettext("Very Low")));
    let hinweis = gtk::Label::new(Some(&gettext(
        "Nice value: lower values mean higher priority.\n\
         Without root privileges the priority can only be lowered.",
    )));
    hinweis.add_css_class("dim-label");
    hinweis.set_xalign(0.0);
    body.append(&titel);
    body.append(&scale);
    body.append(&hinweis);
    let cancel = gtk::Button::with_mnemonic(&gettext("_Cancel"));
    // TRANSLATORS: Knopf; Zugriffstaste verschieden von „_Cancel“.
    let ok = gtk::Button::with_mnemonic(&gettext("Change _Priority"));
    ok.add_css_class("suggested-action");
    window.set_child(Some(&inhalt(&body, &knopfzeile(&[&cancel, &ok]))));
    window.set_default_widget(Some(&ok));
    escape_schliesst(&window);
    schliessknopf(&window, &cancel);
    let w = window.downgrade();
    let s = scale.clone();
    ok.connect_clicked(move |_| {
        let Some(w) = w.upgrade() else { return };
        match setzen(s.value().round() as i32) {
            Ok(()) => spaeter_schliessen(&w),
            Err(text) => fehler(&w, &gettext("The priority could not be changed."), &text),
        }
    });
    window.present();
    ok.grab_focus();
}

fn zeile(grid: &gtk::Grid, zeile: i32, name: &str, wert: &str) {
    let l = gtk::Label::new(Some(name));
    l.set_xalign(1.0);
    l.set_valign(gtk::Align::Start);
    l.add_css_class("dim-label");
    let v = gtk::Label::new(Some(wert));
    v.set_xalign(0.0);
    v.set_selectable(true);
    // Nicht fokussierbar: sonst wählt GTK beim Öffnen den ersten Wert aus.
    v.set_focusable(false);
    v.set_wrap(true);
    v.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    // Mindestbreite, damit lange Pfade nicht zeichenweise umbrechen (GTK
    // misst das Fenster sonst mit absurder Mindesthöhe).
    v.set_width_chars(48);
    v.set_max_width_chars(72);
    grid.attach(&l, 0, zeile, 1, 1);
    grid.attach(&v, 1, zeile, 1, 1);
}

/// Kopfdaten für den Eigenschaften-Dialog (aus der Prozessliste).
pub struct Kopf<'a> {
    pub name: &'a str,
    pub pid: i32,
    pub benutzer: &'a str,
    pub zustand: &'a str,
    pub speicher: u64,
    pub nice: i32,
}

/// Eigenschaften: Befehlszeile, Arbeitsverzeichnis, Startzeit, Dateien.
pub fn eigenschaften(parent: &impl IsA<gtk::Window>, kopf: &Kopf<'_>, e: &Eigenschaften) {
    let Kopf {
        name,
        pid,
        benutzer,
        zustand,
        speicher,
        nice,
    } = *kopf;
    let window = gtk::Window::builder()
        .transient_for(parent)
        .title(gettext_f(
            "Properties of “{name}” (PID {pid})",
            &[("name", name), ("pid", &pid.to_string())],
        ))
        .default_width(640)
        .default_height(480)
        .destroy_with_parent(true)
        .build();
    window.add_css_class("qsm-dialog");
    an_anwendung_binden(&window, parent);
    let grid = gtk::Grid::new();
    grid.set_row_spacing(6);
    grid.set_column_spacing(12);
    let start = e
        .start
        .and_then(|s| glib::DateTime::from_unix_local(s as i64).ok())
        // TRANSLATORS: Datumsformat für „Started:“ (strftime wie GLib).
        .and_then(|d| d.format(&gettext("%Y-%m-%d %H:%M:%S")).ok())
        .map(|g| g.to_string())
        .unwrap_or_else(|| gettext("unknown"));
    let befehl = if e.befehlszeile.is_empty() {
        gettext("(kernel thread, no command line)")
    } else {
        e.befehlszeile.clone()
    };
    // TRANSLATORS: Zeilen im Dialog „Properties“.
    zeile(&grid, 0, &gettext("Process name:"), name);
    zeile(&grid, 1, &gettext("User:"), benutzer);
    zeile(&grid, 2, &gettext("PID:"), &pid.to_string());
    zeile(&grid, 3, &gettext("Status:"), zustand);
    zeile(
        &grid,
        4,
        &gettext("Memory (RSS):"),
        &einheiten::bytes(speicher),
    );
    zeile(&grid, 5, &gettext("Priority (Nice):"), &nice.to_string());
    zeile(&grid, 6, &gettext("Started:"), &start);
    zeile(&grid, 7, &gettext("Command line:"), &befehl);
    let nicht_lesbar = gettext("not readable");
    zeile(
        &grid,
        8,
        &gettext("Working directory:"),
        e.arbeitsverzeichnis.as_deref().unwrap_or(&nicht_lesbar),
    );
    let ueberschrift = gtk::Label::new(None);
    ueberschrift.set_markup(&format!(
        "<b>{}</b>",
        glib::markup_escape_text(&gettext("Open Files"))
    ));
    ueberschrift.set_xalign(0.0);
    ueberschrift.set_margin_top(12);
    let liste = gtk::TextView::new();
    liste.set_editable(false);
    liste.set_cursor_visible(false);
    liste.set_monospace(true);
    liste.set_left_margin(6);
    liste.set_top_margin(4);
    let text = match &e.dateien {
        Ok(d) if d.is_empty() => gettext("(none)"),
        Ok(d) => d.join("\n"),
        Err(grund) => gettext_f("Not readable: {reason}", &[("reason", grund)]),
    };
    liste.buffer().set_text(&text);
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_child(Some(&liste));
    scroll.set_vexpand(true);
    scroll.set_has_frame(true);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 6);
    body.append(&grid);
    body.append(&ueberschrift);
    body.append(&scroll);
    let close = gtk::Button::with_mnemonic(&gettext("_Close"));
    window.set_child(Some(&inhalt(&body, &knopfzeile(&[&close]))));
    window.set_default_widget(Some(&close));
    escape_schliesst(&window);
    schliessknopf(&window, &close);
    window.present();
    close.grab_focus();
}

const LOGO_64: &[u8] = include_bytes!("../data/logo/querencia-logo-64.png");

/// „Info“: offizielles Querencia-Logo C2, Name, Version, Lizenz.
pub fn info(parent: &impl IsA<gtk::Window>) {
    // TRANSLATORS: Fenstertitel des Info-Dialogs; {name} ist der Programmname.
    let window = modales_fenster(parent, &gettext_f("About {name}", &[("name", APP_NAME)]));
    let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    body.set_halign(gtk::Align::Center);
    if let Ok(texture) = gdk::Texture::from_bytes(&glib::Bytes::from_static(LOGO_64)) {
        let image = gtk::Image::from_paintable(Some(&texture));
        image.set_pixel_size(64);
        body.append(&image);
    }
    let name = gtk::Label::new(None);
    name.set_markup(&format!(
        "<b><big>{APP_NAME} {}</big></b>",
        env!("CARGO_PKG_VERSION")
    ));
    let text = gtk::Label::new(Some(&gettext(
        "System monitor for Querencia Desktop\n\
         modelled on the GNOME 2 System Monitor:\n\
         processes, resources, file systems.",
    )));
    text.set_justify(gtk::Justification::Center);
    let tech = gtk::Label::new(Some(&gettext_f(
        "GTK {version} · data from /proc",
        &[(
            "version",
            &format!(
                "{}.{}.{}",
                gtk::major_version(),
                gtk::minor_version(),
                gtk::micro_version()
            ),
        )],
    )));
    tech.add_css_class("dim-label");
    let license = gtk::Label::new(Some(&gettext("License: GNU GPL, version 3 or later")));
    license.add_css_class("dim-label");
    body.append(&name);
    body.append(&text);
    body.append(&tech);
    body.append(&license);
    let close = gtk::Button::with_mnemonic(&gettext("_Close"));
    window.set_child(Some(&inhalt(&body, &knopfzeile(&[&close]))));
    window.set_default_widget(Some(&close));
    escape_schliesst(&window);
    schliessknopf(&window, &close);
    window.present();
    close.grab_focus();
}
