//! Das Hauptfenster: Menüleiste, drei Reiter (GtkNotebook), Statuszeile.
//!
//!   Überwachung | Bearbeiten | Ansicht | Hilfe
//! Keine eigene Titelleiste (`set_titlebar` wird nie gerufen): labwc
//! zeichnet die klassische Server-Titelleiste „Systemüberwachung“.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gtk::prelude::*;
use gtk::{gdk, gio, glib};

use crate::dateisystemliste::Dateisystemliste;
use crate::daten::cpu::{self, Stat};
use crate::daten::einheiten;
use crate::daten::prozesse;
use crate::daten::speicher;
use crate::dialoge;
use crate::einstellungen::{Einstellungen, SPALTEN};
use crate::fenster_titel;
use crate::i18n::{gettext, gettext_f, ngettext_f};
use crate::prozessliste::{ProzessObjekt, Prozessliste};
use crate::ressourcen::Ressourcen;
use crate::sys;

const CSS: &str = include_str!("style.css");

pub fn css_laden() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(CSS);
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

/// Tastenkürzel wie die GNOME-2-Systemüberwachung (Beenden Strg+E,
/// Abwürgen Strg+K) plus Entf für Beenden und Strg+F für die Suche.
pub fn kuerzel_installieren(app: &gtk::Application) {
    for (action, keys) in [
        ("win.quit", &["<Control>q"][..]),
        ("win.beenden", &["Delete", "<Control>e"]),
        ("win.abwuergen", &["<Control>k"]),
        ("win.prioritaet", &["<Control>n"]),
        ("win.eigenschaften", &["<Alt>Return"]),
        ("win.suchen", &["<Control>f"]),
        ("win.aktualisieren", &["F5", "<Control>r"]),
    ] {
        app.set_accels_for_action(action, keys);
    }
}

fn section(items: &[(String, &str)]) -> gio::Menu {
    let m = gio::Menu::new();
    for (label, action) in items {
        m.append(Some(label), Some(action));
    }
    m
}

fn submenu(sections: &[gio::Menu]) -> gio::Menu {
    let m = gio::Menu::new();
    for s in sections {
        m.append_section(None, s);
    }
    m
}

/// Einträge für Prozesse (Menü „Bearbeiten“ und Kontextmenü).
fn prozess_eintraege() -> gio::Menu {
    // TRANSLATORS: Menü „Edit“ und Kontextmenü der Prozessliste (Wortschatz
    // gnome-system-monitor 2.x); Zugriffstasten verschieden halten.
    submenu(&[
        section(&[
            (gettext("_End Process"), "win.beenden"),
            (gettext("_Kill Process"), "win.abwuergen"),
        ]),
        section(&[(gettext("Change P_riority…"), "win.prioritaet")]),
        section(&[(gettext("_Properties"), "win.eigenschaften")]),
    ])
}

/// Menüeintrag einer Spalte im Untermenü „Spalten“ (mit Zugriffstaste).
fn spalte_menue(kennung: &str) -> String {
    // TRANSLATORS: Untermenü „Columns“; Zugriffstasten verschieden halten.
    match kennung {
        "benutzer" => gettext("_User"),
        "pid" => gettext("_PID"),
        "cpu" => gettext("% _CPU"),
        "speicher" => gettext("_Memory"),
        "nice" => gettext("_Nice"),
        "zustand" => gettext("_Status"),
        other => other.to_string(),
    }
}

fn menu_model() -> gio::Menu {
    let bar = gio::Menu::new();
    bar.append_submenu(
        // TRANSLATORS: Menüleiste (gnome-system-monitor 2.x): Monitor, Edit,
        // View, Help; Zugriffstasten je Menü verschieden halten.
        Some(&gettext("_Monitor")),
        &submenu(&[
            section(&[(gettext("_Refresh"), "win.aktualisieren")]),
            section(&[(gettext("_Quit"), "win.quit")]),
        ]),
    );
    let bearbeiten = prozess_eintraege();
    bearbeiten.append_section(
        None,
        &section(&[(gettext("_Search for Process…"), "win.suchen")]),
    );
    bar.append_submenu(Some(&gettext("_Edit")), &bearbeiten);

    let filter = section(&[
        (gettext("A_ll Processes"), "win.filter::alle"),
        (gettext("M_y Processes"), "win.filter::eigene"),
        (gettext("_Active Processes"), "win.filter::aktive"),
    ]);
    let intervall = gio::Menu::new();
    for s in [1, 2, 3, 5, 10] {
        intervall.append(
            Some(&ngettext_f("{n} second", "{n} seconds", s, &[])),
            Some(&format!("win.intervall::{s}")),
        );
    }
    let spalten = gio::Menu::new();
    for kennung in SPALTEN {
        spalten.append(
            Some(&spalte_menue(kennung)),
            Some(&format!("win.spalte-{kennung}")),
        );
    }
    let ansicht = gio::Menu::new();
    ansicht.append_section(None, &filter);
    let weitere = gio::Menu::new();
    weitere.append_submenu(Some(&gettext("Update _Interval")), &intervall);
    weitere.append_submenu(Some(&gettext("_Columns")), &spalten);
    ansicht.append_section(None, &weitere);
    ansicht.append_section(
        None,
        &section(&[(gettext("Show All _File Systems"), "win.alle-dateisysteme")]),
    );
    bar.append_submenu(Some(&gettext("_View")), &ansicht);
    bar.append_submenu(
        Some(&gettext("_Help")),
        &submenu(&[section(&[(gettext("_About"), "win.about")])]),
    );
    bar
}

pub struct Fenster {
    window: gtk::ApplicationWindow,
    notebook: gtk::Notebook,
    prozesse: Rc<Prozessliste>,
    /// Hält den Messtimer der Graphen am Leben (schwache Referenz im Timer).
    #[allow(dead_code)]
    ressourcen: Rc<Ressourcen>,
    dateisysteme: Rc<Dateisystemliste>,
    status_cpu: gtk::Label,
    status_speicher: gtk::Label,
    status_prozesse: gtk::Label,
    einstellungen: RefCell<Einstellungen>,
    timer: RefCell<Option<glib::SourceId>>,
    cpu_alt: RefCell<Option<Stat>>,
    btime: std::cell::Cell<u64>,
}

thread_local! {
    static FENSTER: RefCell<Option<Rc<Fenster>>> = const { RefCell::new(None) };
}

impl Fenster {
    /// Das (einzige) Fenster der Anwendung, bei Bedarf neu angelegt.
    pub fn holen(app: &gtk::Application) -> Rc<Self> {
        if let Some(f) = FENSTER.with(|f| f.borrow().clone()) {
            return f;
        }
        let f = Self::bauen(app);
        FENSTER.with(|slot| *slot.borrow_mut() = Some(f.clone()));
        f
    }

    fn bauen(app: &gtk::Application) -> Rc<Self> {
        let einstellungen = Einstellungen::laden();
        app.set_menubar(Some(&menu_model()));
        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title(fenster_titel())
            .default_width(einstellungen.breite)
            .default_height(einstellungen.hoehe)
            .show_menubar(true)
            .build();
        window.add_css_class("qsm-window");
        dialoge::fenstersymbol_setzen(&window);
        if einstellungen.maximiert {
            window.maximize();
        }

        let prozesse = Prozessliste::neu(&einstellungen, &prozess_eintraege());
        let ressourcen = Ressourcen::neu();
        let dateisysteme = Dateisystemliste::neu(einstellungen.alle_dateisysteme);

        let notebook = gtk::Notebook::new();
        notebook.set_vexpand(true);
        notebook.append_page(
            &prozesse.widget,
            // TRANSLATORS: Reiter wie gnome-system-monitor 2.x.
            Some(&gtk::Label::with_mnemonic(&gettext("_Processes"))),
        );
        notebook.append_page(
            &ressourcen.widget,
            Some(&gtk::Label::with_mnemonic(&gettext("_Resources"))),
        );
        notebook.append_page(
            &dateisysteme.widget,
            Some(&gtk::Label::with_mnemonic(&gettext("_File Systems"))),
        );
        notebook.set_current_page(Some(einstellungen.reiter as u32));

        // TRANSLATORS: Statuszeile; „–“ steht, bis der erste Wert da ist.
        let status_cpu = gtk::Label::new(Some(&gettext_f("CPU: {value}", &[("value", "–")])));
        let status_speicher =
            gtk::Label::new(Some(&gettext_f("Memory: {value}", &[("value", "–")])));
        let status_prozesse =
            gtk::Label::new(Some(&gettext_f("Processes: {value}", &[("value", "–")])));
        let statusbar = gtk::Box::new(gtk::Orientation::Horizontal, 18);
        statusbar.add_css_class("qsm-statusbar");
        for l in [&status_cpu, &status_speicher, &status_prozesse] {
            l.set_xalign(0.0);
            statusbar.append(l);
        }
        status_prozesse.set_hexpand(true);

        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        vbox.append(&notebook);
        vbox.append(&statusbar);
        window.set_child(Some(&vbox));

        let fenster = Rc::new(Self {
            window,
            notebook,
            prozesse,
            ressourcen,
            dateisysteme,
            status_cpu,
            status_speicher,
            status_prozesse,
            einstellungen: RefCell::new(einstellungen),
            timer: RefCell::new(None),
            cpu_alt: RefCell::new(None),
            btime: std::cell::Cell::new(0),
        });
        fenster.aktionen_installieren();
        fenster.aktualisieren();
        fenster.timer_starten();

        let schwach = Rc::downgrade(&fenster);
        fenster
            .prozesse
            .auswahl
            .connect_selected_item_notify(move |_| {
                if let Some(f) = schwach.upgrade() {
                    f.aktionen_anpassen();
                }
            });
        let schwach = Rc::downgrade(&fenster);
        fenster.notebook.connect_switch_page(move |_, _, seite| {
            if let Some(f) = schwach.upgrade() {
                f.einstellungen.borrow_mut().reiter = seite as i32;
                f.aktionen_anpassen();
            }
        });
        let schwach = Rc::downgrade(&fenster);
        fenster.window.connect_close_request(move |_| {
            if let Some(f) = schwach.upgrade() {
                f.einstellungen_speichern();
                f.prozesse.aufraeumen();
                if let Some(t) = f.timer.borrow_mut().take() {
                    t.remove();
                }
            }
            glib::Propagation::Proceed
        });
        fenster.aktionen_anpassen();
        fenster
    }

    pub fn zeigen(&self) {
        self.window.present();
    }

    fn aktionen_installieren(self: &Rc<Self>) {
        // `schwach()` braucht den Eintrag in FENSTER; hier ist er noch nicht
        // gesetzt, deshalb ein eigener Weak.
        let schwach = Rc::downgrade(self);
        type Rueckruf = Box<dyn Fn(&Rc<Fenster>)>;
        let aktion = |name: &str, f: Rueckruf| {
            let a = gio::SimpleAction::new(name, None);
            let schwach = schwach.clone();
            a.connect_activate(move |_, _| {
                if let Some(f2) = schwach.upgrade() {
                    f(&f2);
                }
            });
            self.window.add_action(&a);
            a
        };
        aktion("quit", Box::new(|f| f.window.close()));
        aktion("aktualisieren", Box::new(|f| f.aktualisieren()));
        aktion(
            "suchen",
            Box::new(|f| {
                f.notebook.set_current_page(Some(0));
                f.prozesse.fokus_suche();
            }),
        );
        aktion(
            "beenden",
            Box::new(|f| f.signal_mit_rueckfrage(sys::Signal::Beenden)),
        );
        aktion(
            "abwuergen",
            Box::new(|f| f.signal_mit_rueckfrage(sys::Signal::Abwuergen)),
        );
        aktion("prioritaet", Box::new(|f| f.prioritaet_aendern()));
        aktion("eigenschaften", Box::new(|f| f.eigenschaften_zeigen()));
        aktion("about", Box::new(|f| dialoge::info(&f.window)));

        // Filter: Zustandsaktion mit Zeichenkette (Radio-Einträge im Menü).
        let e = self.einstellungen.borrow();
        let filter = gio::SimpleAction::new_stateful(
            "filter",
            Some(glib::VariantTy::STRING),
            &e.filter.to_variant(),
        );
        let schwach = Rc::downgrade(self);
        filter.connect_activate(move |a, wert| {
            let Some(wert) = wert.and_then(|v| v.get::<String>()) else {
                return;
            };
            a.set_state(&wert.to_variant());
            if let Some(f) = schwach.upgrade() {
                f.prozesse.filter_setzen(&wert);
                f.einstellungen.borrow_mut().filter = wert;
            }
        });
        self.window.add_action(&filter);

        let intervall = gio::SimpleAction::new_stateful(
            "intervall",
            Some(glib::VariantTy::STRING),
            &e.intervall.to_string().to_variant(),
        );
        let schwach = Rc::downgrade(self);
        intervall.connect_activate(move |a, wert| {
            let Some(wert) = wert.and_then(|v| v.get::<String>()) else {
                return;
            };
            let Ok(sek) = wert.parse::<u32>() else { return };
            let sek = sek.clamp(1, 10);
            a.set_state(&sek.to_string().to_variant());
            if let Some(f) = schwach.upgrade() {
                f.einstellungen.borrow_mut().intervall = sek;
                f.timer_starten();
            }
        });
        self.window.add_action(&intervall);

        for kennung in SPALTEN {
            let a = gio::SimpleAction::new_stateful(
                &format!("spalte-{kennung}"),
                None,
                &e.spalte_sichtbar(kennung).to_variant(),
            );
            let schwach = Rc::downgrade(self);
            a.connect_activate(move |a, _| {
                let neu = !a.state().and_then(|s| s.get::<bool>()).unwrap_or(true);
                a.set_state(&neu.to_variant());
                if let Some(f) = schwach.upgrade() {
                    f.prozesse.spalte_sichtbar_setzen(kennung, neu);
                    f.einstellungen.borrow_mut().spalte_setzen(kennung, neu);
                }
            });
            self.window.add_action(&a);
        }

        let alle = gio::SimpleAction::new_stateful(
            "alle-dateisysteme",
            None,
            &e.alle_dateisysteme.to_variant(),
        );
        let schwach = Rc::downgrade(self);
        alle.connect_activate(move |a, _| {
            let neu = !a.state().and_then(|s| s.get::<bool>()).unwrap_or(false);
            a.set_state(&neu.to_variant());
            if let Some(f) = schwach.upgrade() {
                f.dateisysteme.alle_setzen(neu);
                f.einstellungen.borrow_mut().alle_dateisysteme = neu;
            }
        });
        self.window.add_action(&alle);
        drop(e);
    }

    /// Eingriffe nur bei eigenem, ausgewähltem Prozess auf dem Reiter Prozesse.
    fn aktionen_anpassen(&self) {
        let auf_prozessen = self.notebook.current_page() == Some(0);
        let auswahl = self.prozesse.ausgewaehlt();
        let eigen = auf_prozessen && auswahl.as_ref().is_some_and(ProzessObjekt::eigen);
        let irgendein = auf_prozessen && auswahl.is_some();
        for (name, an) in [
            ("beenden", eigen),
            ("abwuergen", eigen),
            ("prioritaet", eigen),
            ("eigenschaften", irgendein),
        ] {
            if let Some(a) = self
                .window
                .lookup_action(name)
                .and_downcast::<gio::SimpleAction>()
            {
                a.set_enabled(an);
            }
        }
    }

    fn timer_starten(self: &Rc<Self>) {
        if let Some(alt) = self.timer.borrow_mut().take() {
            alt.remove();
        }
        let sek = u64::from(self.einstellungen.borrow().intervall.clamp(1, 10));
        let schwach = Rc::downgrade(self);
        let id =
            glib::timeout_add_local(Duration::from_secs(sek), move || match schwach.upgrade() {
                Some(f) => {
                    f.aktualisieren();
                    glib::ControlFlow::Continue
                }
                None => glib::ControlFlow::Break,
            });
        *self.timer.borrow_mut() = Some(id);
    }

    /// Prozessliste, Dateisysteme und Statuszeile neu lesen.
    pub fn aktualisieren(&self) {
        let Some(stat) = cpu::lese_stat() else { return };
        self.btime.set(stat.btime);
        let anzahl = self.prozesse.aktualisieren(&stat);
        let cpu_text = match self.cpu_alt.borrow().as_ref() {
            Some(alt) => einheiten::prozent(cpu::auslastung(&alt.gesamt, &stat.gesamt)),
            None => "–".to_string(),
        };
        *self.cpu_alt.borrow_mut() = Some(stat);
        self.status_cpu
            .set_text(&gettext_f("CPU: {value}", &[("value", &cpu_text)]));
        if let Some(s) = speicher::lese_meminfo() {
            self.status_speicher.set_text(&gettext_f(
                "Memory: {used} of {total}",
                &[
                    ("used", &einheiten::bytes(s.belegt())),
                    ("total", &einheiten::bytes(s.gesamt)),
                ],
            ));
        }
        self.status_prozesse.set_text(&gettext_f(
            "Processes: {value}",
            &[("value", &anzahl.to_string())],
        ));
        self.dateisysteme.aktualisieren();
        self.aktionen_anpassen();
    }

    fn einstellungen_speichern(&self) {
        let mut e = self.einstellungen.borrow_mut();
        e.maximiert = self.window.is_maximized();
        if !e.maximiert {
            e.breite = self.window.width().max(400);
            e.hoehe = self.window.height().max(300);
        }
        let (sortierung, absteigend) = self.prozesse.sortierung();
        e.sortierung = sortierung;
        e.absteigend = absteigend;
        if let Err(err) = e.speichern() {
            eprintln!(
                "querencia-systemmonitor: Einstellungen nicht gespeichert ({}): {err}",
                Einstellungen::pfad().display()
            );
        }
    }

    fn signal_mit_rueckfrage(self: &Rc<Self>, signal: sys::Signal) {
        let Some(o) = self.prozesse.ausgewaehlt() else {
            return;
        };
        if !o.eigen() {
            dialoge::fehler(
                &self.window,
                &gettext("Processes of other users are only shown."),
                &gettext("Only your own processes can be ended, killed or changed in priority."),
            );
            return;
        }
        let p = o.prozess();
        let (primary, secondary, knopf) = match signal {
            sys::Signal::Beenden => (
                gettext_f(
                    "End the process “{name}” (PID {pid})?",
                    &[("name", &p.name), ("pid", &p.pid.to_string())],
                ),
                gettext(
                    "Ending a process may lose unsaved data. The process receives the signal SIGTERM and can still clean up.",
                ),
                gettext("_End Process"),
            ),
            sys::Signal::Abwuergen => (
                gettext_f(
                    "Kill the process “{name}” (PID {pid})?",
                    &[("name", &p.name), ("pid", &p.pid.to_string())],
                ),
                gettext(
                    "Killing a process loses unsaved data. The process receives the signal SIGKILL and stops immediately.",
                ),
                gettext("_Kill Process"),
            ),
        };
        let schwach = Rc::downgrade(self);
        let name = p.name.clone();
        let pid = p.pid;
        dialoge::frage(
            &self.window,
            &primary,
            &secondary,
            &knopf,
            true,
            move || {
                let Some(f) = schwach.upgrade() else { return };
                match sys::signal(pid, signal) {
                    Ok(()) => f.aktualisieren(),
                    Err(text) => dialoge::fehler(
                        &f.window,
                        &gettext_f(
                            "“{name}” (PID {pid}) could not be ended.",
                            &[("name", &name), ("pid", &pid.to_string())],
                        ),
                        &text,
                    ),
                }
            },
        );
    }

    fn prioritaet_aendern(self: &Rc<Self>) {
        let Some(o) = self.prozesse.ausgewaehlt() else {
            return;
        };
        if !o.eigen() {
            dialoge::fehler(
                &self.window,
                &gettext("Processes of other users are only shown."),
                &gettext("The priority can only be changed for your own processes."),
            );
            return;
        }
        let p = o.prozess();
        let aktuell = sys::prioritaet(p.pid).unwrap_or(p.nice);
        let schwach = Rc::downgrade(self);
        dialoge::prioritaet(&self.window, &p.name, p.pid, aktuell, move |nice| {
            let r = sys::prioritaet_setzen(p.pid, nice);
            if r.is_ok()
                && let Some(f) = schwach.upgrade()
            {
                f.aktualisieren();
            }
            r
        });
    }

    fn eigenschaften_zeigen(&self) {
        let Some(o) = self.prozesse.ausgewaehlt() else {
            return;
        };
        let p = o.prozess();
        let e = prozesse::lese_eigenschaften(p.pid, self.btime.get(), sys::takt());
        let zustand = prozesse::zustand_text(p.zustand);
        let kopf = dialoge::Kopf {
            name: &p.name,
            pid: p.pid,
            benutzer: &p.benutzer,
            zustand: &zustand,
            speicher: p.rss,
            nice: p.nice,
        };
        dialoge::eigenschaften(&self.window, &kopf, &e);
    }
}
