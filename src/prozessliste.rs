//! Reiter „Prozesse“: Suchfeld, Tabelle (GtkColumnView), Knopf „Prozess
//! beenden“. Jede Zeile ist ein `ProzessObjekt`, das über die Aktualisierung
//! hinweg dasselbe GObject bleibt – so überlebt die Auswahl, und die Zellen
//! (an Eigenschaften gebunden) ändern nur Text, der sich wirklich ändert.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, gio, glib};

use crate::daten::cpu::Stat;
use crate::daten::einheiten;
use crate::daten::prozesse::{self, Benutzer, Prozess};
use crate::einstellungen::{self, Einstellungen, SPALTEN};
use crate::sys;

mod imp {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::ProzessObjekt)]
    pub struct ProzessObjekt {
        #[property(get)]
        pub name: RefCell<String>,
        #[property(get)]
        pub benutzer: RefCell<String>,
        #[property(get)]
        pub pid_text: RefCell<String>,
        #[property(get)]
        pub cpu_text: RefCell<String>,
        #[property(get)]
        pub speicher_text: RefCell<String>,
        #[property(get)]
        pub nice_text: RefCell<String>,
        #[property(get)]
        pub zustand: RefCell<String>,
        pub daten: RefCell<Prozess>,
        pub cpu: Cell<f64>,
        pub eigen: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProzessObjekt {
        const NAME: &'static str = "QsmProzessObjekt";
        type Type = super::ProzessObjekt;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ProzessObjekt {}
}

glib::wrapper! {
    pub struct ProzessObjekt(ObjectSubclass<imp::ProzessObjekt>);
}

/// Setzt ein Textfeld und meldet die Eigenschaft nur bei Änderung.
macro_rules! text_setzen {
    ($self:ident, $feld:ident, $notify:ident, $wert:expr) => {{
        let neu: String = $wert;
        if *$self.imp().$feld.borrow() != neu {
            $self.imp().$feld.replace(neu);
            $self.$notify();
        }
    }};
}

impl ProzessObjekt {
    fn neu(p: &Prozess, eigen: bool) -> Self {
        let o: Self = glib::Object::new();
        o.imp().eigen.set(eigen);
        o.aktualisieren(p, 0.0);
        o
    }

    fn aktualisieren(&self, p: &Prozess, cpu: f64) {
        text_setzen!(self, name, notify_name, p.name.clone());
        text_setzen!(self, benutzer, notify_benutzer, p.benutzer.clone());
        text_setzen!(self, pid_text, notify_pid_text, p.pid.to_string());
        text_setzen!(
            self,
            cpu_text,
            notify_cpu_text,
            format!("{}", cpu.round() as u64)
        );
        text_setzen!(
            self,
            speicher_text,
            notify_speicher_text,
            einheiten::bytes(p.rss)
        );
        text_setzen!(self, nice_text, notify_nice_text, p.nice.to_string());
        text_setzen!(
            self,
            zustand,
            notify_zustand,
            prozesse::zustand_text(p.zustand).to_string()
        );
        self.imp().cpu.set(cpu);
        self.imp().daten.replace(p.clone());
    }

    pub fn prozess(&self) -> Prozess {
        self.imp().daten.borrow().clone()
    }

    pub fn pid(&self) -> i32 {
        self.imp().daten.borrow().pid
    }

    pub fn cpu(&self) -> f64 {
        self.imp().cpu.get()
    }

    /// Gehört dem angemeldeten Benutzer (nur dann darf man eingreifen).
    pub fn eigen(&self) -> bool {
        self.imp().eigen.get()
    }

    fn aktiv(&self) -> bool {
        self.cpu() > 0.0 || self.imp().daten.borrow().zustand == 'R'
    }
}

/// Zellenfabrik: ein Label je Zelle, per Ausdruck an die Eigenschaft `prop`
/// gebunden; der Widget-Name trägt die Zeilenposition (Kontextmenü).
fn fabrik(prop: &'static str, xalign: f32) -> gtk::SignalListItemFactory {
    let f = gtk::SignalListItemFactory::new();
    f.connect_setup(move |_, obj| {
        let Some(item) = obj.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let label = gtk::Label::new(None);
        label.set_xalign(xalign);
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        label.set_single_line_mode(true);
        item.set_child(Some(&label));
        let item_expr = gtk::PropertyExpression::new(
            gtk::ListItem::static_type(),
            gtk::Expression::NONE,
            "item",
        );
        let prop_expr =
            gtk::PropertyExpression::new(ProzessObjekt::static_type(), Some(&item_expr), prop);
        prop_expr.bind(&label, "label", Some(item));
        let pos_expr = gtk::PropertyExpression::new(
            gtk::ListItem::static_type(),
            gtk::Expression::NONE,
            "position",
        );
        pos_expr.bind(&label, "name", Some(item));
    });
    f
}

fn objekt(o: &glib::Object) -> Option<&ProzessObjekt> {
    o.downcast_ref::<ProzessObjekt>()
}

fn sortierer(kennung: &'static str) -> gtk::CustomSorter {
    gtk::CustomSorter::new(move |a, b| {
        let (Some(a), Some(b)) = (objekt(a), objekt(b)) else {
            return gtk::Ordering::Equal;
        };
        let (pa, pb) = (a.prozess(), b.prozess());
        let ord = match kennung {
            "name" => pa
                .name
                .to_lowercase()
                .cmp(&pb.name.to_lowercase())
                .then(pa.pid.cmp(&pb.pid)),
            "benutzer" => pa.benutzer.cmp(&pb.benutzer).then(pa.pid.cmp(&pb.pid)),
            "pid" => pa.pid.cmp(&pb.pid),
            "cpu" => a
                .cpu()
                .partial_cmp(&b.cpu())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(pa.pid.cmp(&pb.pid)),
            "speicher" => pa.rss.cmp(&pb.rss).then(pa.pid.cmp(&pb.pid)),
            "nice" => pa.nice.cmp(&pb.nice).then(pa.pid.cmp(&pb.pid)),
            "zustand" => prozesse::zustand_text(pa.zustand)
                .cmp(prozesse::zustand_text(pb.zustand))
                .then(pa.pid.cmp(&pb.pid)),
            _ => std::cmp::Ordering::Equal,
        };
        ord.into()
    })
}

pub struct Prozessliste {
    pub widget: gtk::Box,
    pub suche: gtk::SearchEntry,
    pub tabelle: gtk::ColumnView,
    pub auswahl: gtk::SingleSelection,
    store: gio::ListStore,
    filter: gtk::CustomFilter,
    spalten: Vec<(&'static str, gtk::ColumnViewColumn)>,
    objekte: RefCell<HashMap<i32, ProzessObjekt>>,
    filtermodus: Rc<RefCell<String>>,
    suchtext: Rc<RefCell<String>>,
    benutzer: Benutzer,
    eigene_uid: u32,
    seitengroesse: u64,
    takte_alt: Cell<Option<u64>>,
    kontextmenue: gtk::PopoverMenu,
}

impl Prozessliste {
    pub fn neu(einstellungen: &Einstellungen, kontext: &gio::Menu) -> Rc<Self> {
        let store = gio::ListStore::new::<ProzessObjekt>();
        let filtermodus = Rc::new(RefCell::new(einstellungen.filter.clone()));
        let suchtext = Rc::new(RefCell::new(String::new()));
        let filter = {
            let modus = filtermodus.clone();
            let text = suchtext.clone();
            gtk::CustomFilter::new(move |o| {
                let Some(p) = objekt(o) else { return false };
                let modus_ok = match modus.borrow().as_str() {
                    einstellungen::FILTER_EIGENE => p.eigen(),
                    einstellungen::FILTER_AKTIVE => p.aktiv(),
                    _ => true,
                };
                if !modus_ok {
                    return false;
                }
                let text = text.borrow();
                if text.is_empty() {
                    return true;
                }
                let d = p.prozess();
                d.name.to_lowercase().contains(text.as_str())
                    || d.pid.to_string() == text.as_str()
                    || d.benutzer.to_lowercase().contains(text.as_str())
            })
        };
        let gefiltert = gtk::FilterListModel::new(Some(store.clone()), Some(filter.clone()));
        let tabelle = gtk::ColumnView::new(None::<gtk::SingleSelection>);
        tabelle.set_show_row_separators(false);
        tabelle.set_show_column_separators(false);
        tabelle.set_reorderable(false);
        let sortiert = gtk::SortListModel::new(Some(gefiltert), tabelle.sorter());
        let auswahl = gtk::SingleSelection::new(Some(sortiert));
        auswahl.set_autoselect(false);
        auswahl.set_can_unselect(true);
        tabelle.set_model(Some(&auswahl));

        let mut spalten = Vec::new();
        let definitionen: [(&'static str, &str, &'static str, f32, i32, bool); 7] = [
            ("name", "Prozessname", "name", 0.0, -1, true),
            ("benutzer", "Benutzer", "benutzer", 0.0, 110, false),
            ("pid", "PID", "pid-text", 1.0, 70, false),
            ("cpu", "% CPU", "cpu-text", 1.0, 70, false),
            ("speicher", "Speicher", "speicher-text", 1.0, 100, false),
            ("nice", "Nice", "nice-text", 1.0, 56, false),
            ("zustand", "Zustand", "zustand", 0.0, 120, false),
        ];
        for (kennung, titel, prop, xalign, breite, expand) in definitionen {
            let spalte = gtk::ColumnViewColumn::builder()
                .title(titel)
                .factory(&fabrik(prop, xalign))
                .sorter(&sortierer(kennung))
                .resizable(true)
                .expand(expand)
                .id(kennung)
                .build();
            if breite > 0 {
                spalte.set_fixed_width(breite);
            }
            if kennung != "name" {
                spalte.set_visible(einstellungen.spalte_sichtbar(kennung));
            }
            tabelle.append_column(&spalte);
            spalten.push((kennung, spalte));
        }

        let suche = gtk::SearchEntry::new();
        suche.set_placeholder_text(Some("Prozess suchen (Name, PID, Benutzer)"));
        suche.set_hexpand(true);
        let suchzeile = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        suchzeile.set_margin_top(6);
        suchzeile.set_margin_start(6);
        suchzeile.set_margin_end(6);
        let suchlabel = gtk::Label::with_mnemonic("_Suchen:");
        suchlabel.set_mnemonic_widget(Some(&suche));
        suchzeile.append(&suchlabel);
        suchzeile.append(&suche);

        let scroll = gtk::ScrolledWindow::new();
        scroll.set_child(Some(&tabelle));
        scroll.set_vexpand(true);
        scroll.set_hexpand(true);
        scroll.set_has_frame(true);
        scroll.set_margin_top(6);
        scroll.set_margin_start(6);
        scroll.set_margin_end(6);

        let knopf = gtk::Button::with_mnemonic("_Prozess beenden");
        knopf.set_action_name(Some("win.beenden"));
        let knopfzeile = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        knopfzeile.set_halign(gtk::Align::End);
        knopfzeile.set_margin_top(6);
        knopfzeile.set_margin_bottom(6);
        knopfzeile.set_margin_end(6);
        knopfzeile.append(&knopf);

        let widget = gtk::Box::new(gtk::Orientation::Vertical, 0);
        widget.append(&suchzeile);
        widget.append(&scroll);
        widget.append(&knopfzeile);

        let kontextmenue = gtk::PopoverMenu::from_model(Some(kontext));
        kontextmenue.set_parent(&tabelle);
        kontextmenue.set_has_arrow(false);
        kontextmenue.set_halign(gtk::Align::Start);

        let liste = Rc::new(Self {
            widget,
            suche,
            tabelle,
            auswahl,
            store,
            filter,
            spalten,
            objekte: RefCell::new(HashMap::new()),
            filtermodus,
            suchtext,
            benutzer: Benutzer::laden(),
            eigene_uid: std::fs::metadata("/proc/self")
                .map(|m| m.uid())
                .unwrap_or(u32::MAX),
            seitengroesse: sys::seitengroesse(),
            takte_alt: Cell::new(None),
            kontextmenue,
        });

        // Sortierung aus den Einstellungen.
        liste.sortieren(&einstellungen.sortierung, einstellungen.absteigend);

        // Suche: Text übernehmen, Filter neu auswerten.
        let schwach = Rc::downgrade(&liste);
        liste.suche.connect_search_changed(move |e| {
            if let Some(l) = schwach.upgrade() {
                l.suchtext.replace(e.text().to_lowercase());
                l.filter.changed(gtk::FilterChange::Different);
            }
        });
        // Escape im Suchfeld leert es und gibt den Fokus an die Tabelle.
        let schwach = Rc::downgrade(&liste);
        liste.suche.connect_stop_search(move |e| {
            e.set_text("");
            if let Some(l) = schwach.upgrade() {
                l.tabelle.grab_focus();
            }
        });
        // Doppelklick/Eingabe auf einer Zeile: Eigenschaften.
        liste.tabelle.connect_activate(|t, _| {
            let _ = t.activate_action("win.eigenschaften", None);
        });
        // Rechtsklick: Zeile auswählen, Kontextmenü.
        let geste = gtk::GestureClick::new();
        geste.set_button(gdk::BUTTON_SECONDARY);
        let schwach = Rc::downgrade(&liste);
        geste.connect_pressed(move |g, _, x, y| {
            let Some(l) = schwach.upgrade() else { return };
            if let Some(pos) = l.position_bei(x, y) {
                l.auswahl.set_selected(pos);
            }
            g.set_state(gtk::EventSequenceState::Claimed);
            l.kontextmenue
                .set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
            l.kontextmenue.popup();
        });
        liste.tabelle.add_controller(geste);
        liste
    }

    /// Zeilenposition unter dem Punkt (x, y) der Tabelle – über den
    /// Widget-Namen des Zellenlabels (siehe [`fabrik`]).
    fn position_bei(&self, x: f64, y: f64) -> Option<u32> {
        let getroffen = self.tabelle.pick(x, y, gtk::PickFlags::DEFAULT)?;
        let mut kandidat = Some(getroffen);
        while let Some(w) = kandidat {
            if let Ok(pos) = w.widget_name().parse::<u32>() {
                return Some(pos);
            }
            kandidat = w.first_child();
        }
        None
    }

    /// Liest die Prozesse neu und gleicht die Zeilen ab; `stat` ist die
    /// aktuelle `/proc/stat`-Messung für den CPU-Anteil. Liefert die Zahl
    /// aller Prozesse.
    pub fn aktualisieren(&self, stat: &Stat) -> usize {
        let gesamt = stat.gesamt.gesamt();
        let delta_gesamt = self
            .takte_alt
            .get()
            .map_or(0, |alt| gesamt.saturating_sub(alt));
        self.takte_alt.set(Some(gesamt));
        let kerne = stat.kerne.len().max(1);

        let prozesse = prozesse::lese_prozesse(self.seitengroesse, &self.benutzer);
        let anzahl = prozesse.len();
        let mut objekte = self.objekte.borrow_mut();
        let mut gesehen: HashMap<i32, ()> = HashMap::with_capacity(anzahl);
        for p in &prozesse {
            gesehen.insert(p.pid, ());
            match objekte.get(&p.pid) {
                // Dieselbe PID, aber ein anderer Prozess (Startzeit): neu anlegen.
                Some(o) if o.prozess().starttime == p.starttime => {
                    let alt = o.prozess().takte;
                    let cpu = if delta_gesamt > 0 {
                        prozesse::cpu_prozent(p.takte.saturating_sub(alt), delta_gesamt, kerne)
                    } else {
                        0.0
                    };
                    o.aktualisieren(p, cpu);
                }
                Some(o) => {
                    if let Some(pos) = self.store.find(o) {
                        self.store.remove(pos);
                    }
                    let neu = ProzessObjekt::neu(p, p.uid == self.eigene_uid);
                    self.store.append(&neu);
                    objekte.insert(p.pid, neu);
                }
                None => {
                    let neu = ProzessObjekt::neu(p, p.uid == self.eigene_uid);
                    self.store.append(&neu);
                    objekte.insert(p.pid, neu);
                }
            }
        }
        let verschwunden: Vec<i32> = objekte
            .keys()
            .filter(|pid| !gesehen.contains_key(pid))
            .copied()
            .collect();
        for pid in verschwunden {
            if let Some(o) = objekte.remove(&pid)
                && let Some(pos) = self.store.find(&o)
            {
                self.store.remove(pos);
            }
        }
        drop(objekte);
        // Werte haben sich geändert: Filter (aktive Prozesse) und Sortierung
        // neu auswerten. Die Auswahl bleibt am selben Objekt hängen.
        self.filter.changed(gtk::FilterChange::Different);
        if let Some(s) = self.tabelle.sorter() {
            s.changed(gtk::SorterChange::Different);
        }
        anzahl
    }

    pub fn ausgewaehlt(&self) -> Option<ProzessObjekt> {
        self.auswahl
            .selected_item()
            .and_then(|o| o.downcast::<ProzessObjekt>().ok())
    }

    pub fn filter_setzen(&self, modus: &str) {
        self.filtermodus.replace(modus.to_string());
        self.filter.changed(gtk::FilterChange::Different);
    }

    pub fn spalte_sichtbar_setzen(&self, kennung: &str, sichtbar: bool) {
        if let Some((_, s)) = self.spalten.iter().find(|(k, _)| *k == kennung) {
            s.set_visible(sichtbar);
        }
    }

    pub fn sortieren(&self, kennung: &str, absteigend: bool) {
        let kennung = if SPALTEN.contains(&kennung) || kennung == "name" {
            kennung
        } else {
            "name"
        };
        if let Some((_, s)) = self.spalten.iter().find(|(k, _)| *k == kennung) {
            let richtung = if absteigend {
                gtk::SortType::Descending
            } else {
                gtk::SortType::Ascending
            };
            self.tabelle.sort_by_column(Some(s), richtung);
        }
    }

    /// Aktuelle Sortierung (Spaltenkennung, absteigend).
    pub fn sortierung(&self) -> (String, bool) {
        let Some(s) = self
            .tabelle
            .sorter()
            .and_downcast::<gtk::ColumnViewSorter>()
        else {
            return ("name".to_string(), false);
        };
        let kennung = s
            .primary_sort_column()
            .and_then(|c| c.id())
            .map(|id| id.to_string())
            .unwrap_or_else(|| "name".to_string());
        (kennung, s.primary_sort_order() == gtk::SortType::Descending)
    }

    pub fn fokus_suche(&self) {
        self.suche.grab_focus();
    }
}
