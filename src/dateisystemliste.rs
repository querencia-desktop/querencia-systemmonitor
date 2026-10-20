//! Reiter „Dateisysteme“: Gerät, Einhängepunkt, Typ, Gesamt, Frei,
//! Verfügbar, Belegt (Balken mit Prozent). Aus /proc/mounts und statvfs.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use crate::daten::dateisysteme::{self, Dateisystem};
use crate::daten::einheiten;
use crate::i18n::gettext;

mod imp {
    use super::*;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::DateisystemObjekt)]
    pub struct DateisystemObjekt {
        #[property(get)]
        pub geraet: RefCell<String>,
        #[property(get)]
        pub pfad: RefCell<String>,
        #[property(get)]
        pub typ: RefCell<String>,
        #[property(get)]
        pub gesamt_text: RefCell<String>,
        #[property(get)]
        pub frei_text: RefCell<String>,
        #[property(get)]
        pub verfuegbar_text: RefCell<String>,
        #[property(get)]
        pub belegt_text: RefCell<String>,
        #[property(get)]
        pub belegt_anteil: Cell<f64>,
        pub daten: RefCell<Dateisystem>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DateisystemObjekt {
        const NAME: &'static str = "QsmDateisystemObjekt";
        type Type = super::DateisystemObjekt;
    }

    #[glib::derived_properties]
    impl ObjectImpl for DateisystemObjekt {}
}

glib::wrapper! {
    pub struct DateisystemObjekt(ObjectSubclass<imp::DateisystemObjekt>);
}

macro_rules! text_setzen {
    ($self:ident, $feld:ident, $notify:ident, $wert:expr) => {{
        let neu: String = $wert;
        if *$self.imp().$feld.borrow() != neu {
            $self.imp().$feld.replace(neu);
            $self.$notify();
        }
    }};
}

impl DateisystemObjekt {
    fn neu(d: &Dateisystem) -> Self {
        let o: Self = glib::Object::new();
        o.aktualisieren(d);
        o
    }

    fn aktualisieren(&self, d: &Dateisystem) {
        text_setzen!(self, geraet, notify_geraet, d.geraet.clone());
        text_setzen!(self, pfad, notify_pfad, d.pfad.clone());
        text_setzen!(self, typ, notify_typ, d.typ.clone());
        text_setzen!(
            self,
            gesamt_text,
            notify_gesamt_text,
            einheiten::bytes(d.gesamt)
        );
        text_setzen!(self, frei_text, notify_frei_text, einheiten::bytes(d.frei));
        text_setzen!(
            self,
            verfuegbar_text,
            notify_verfuegbar_text,
            einheiten::bytes(d.verfuegbar)
        );
        let anteil = d.belegt_anteil();
        text_setzen!(
            self,
            belegt_text,
            notify_belegt_text,
            format!(
                "{} ({})",
                einheiten::bytes(d.belegt()),
                einheiten::prozent(anteil * 100.0)
            )
        );
        if (self.imp().belegt_anteil.get() - anteil).abs() > 1e-6 {
            self.imp().belegt_anteil.set(anteil);
            self.notify_belegt_anteil();
        }
        self.imp().daten.replace(d.clone());
    }
}

fn textfabrik(prop: &'static str, xalign: f32) -> gtk::SignalListItemFactory {
    let f = gtk::SignalListItemFactory::new();
    f.connect_setup(move |_, obj| {
        let Some(item) = obj.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let label = gtk::Label::new(None);
        label.set_xalign(xalign);
        label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        item.set_child(Some(&label));
        let item_expr = gtk::PropertyExpression::new(
            gtk::ListItem::static_type(),
            gtk::Expression::NONE,
            "item",
        );
        gtk::PropertyExpression::new(DateisystemObjekt::static_type(), Some(&item_expr), prop)
            .bind(&label, "label", Some(item));
    });
    f
}

fn balkenfabrik() -> gtk::SignalListItemFactory {
    let f = gtk::SignalListItemFactory::new();
    f.connect_setup(move |_, obj| {
        let Some(item) = obj.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let balken = gtk::ProgressBar::new();
        balken.set_show_text(true);
        balken.set_valign(gtk::Align::Center);
        balken.set_hexpand(true);
        item.set_child(Some(&balken));
        let item_expr = gtk::PropertyExpression::new(
            gtk::ListItem::static_type(),
            gtk::Expression::NONE,
            "item",
        );
        gtk::PropertyExpression::new(
            DateisystemObjekt::static_type(),
            Some(&item_expr),
            "belegt-anteil",
        )
        .bind(&balken, "fraction", Some(item));
        gtk::PropertyExpression::new(
            DateisystemObjekt::static_type(),
            Some(&item_expr),
            "belegt-text",
        )
        .bind(&balken, "text", Some(item));
    });
    f
}

fn sortierer(kennung: &'static str) -> gtk::CustomSorter {
    gtk::CustomSorter::new(move |a, b| {
        let (Some(a), Some(b)) = (
            a.downcast_ref::<DateisystemObjekt>(),
            b.downcast_ref::<DateisystemObjekt>(),
        ) else {
            return gtk::Ordering::Equal;
        };
        let (da, db) = (a.imp().daten.borrow(), b.imp().daten.borrow());
        let ord = match kennung {
            "geraet" => da.geraet.cmp(&db.geraet),
            "typ" => da.typ.cmp(&db.typ),
            "gesamt" => da.gesamt.cmp(&db.gesamt),
            "frei" => da.frei.cmp(&db.frei),
            "verfuegbar" => da.verfuegbar.cmp(&db.verfuegbar),
            "belegt" => da
                .belegt_anteil()
                .partial_cmp(&db.belegt_anteil())
                .unwrap_or(std::cmp::Ordering::Equal),
            _ => std::cmp::Ordering::Equal,
        };
        ord.then(da.pfad.cmp(&db.pfad)).into()
    })
}

pub struct Dateisystemliste {
    pub widget: gtk::Box,
    tabelle: gtk::ColumnView,
    store: gio::ListStore,
    objekte: RefCell<HashMap<String, DateisystemObjekt>>,
    alle: Cell<bool>,
}

impl Dateisystemliste {
    pub fn neu(alle: bool) -> Rc<Self> {
        let store = gio::ListStore::new::<DateisystemObjekt>();
        let tabelle = gtk::ColumnView::new(None::<gtk::SingleSelection>);
        let sortiert = gtk::SortListModel::new(Some(store.clone()), tabelle.sorter());
        let auswahl = gtk::SingleSelection::new(Some(sortiert));
        auswahl.set_autoselect(false);
        auswahl.set_can_unselect(true);
        tabelle.set_model(Some(&auswahl));
        type Spalte = (&'static str, String, Option<&'static str>, f32, i32, bool);
        // TRANSLATORS: Spaltenköpfe der Dateisysteme (gnome-system-monitor 2.x).
        let spalten: [Spalte; 7] = [
            ("geraet", gettext("Device"), Some("geraet"), 0.0, 140, false),
            ("pfad", gettext("Directory"), Some("pfad"), 0.0, -1, true),
            ("typ", gettext("Type"), Some("typ"), 0.0, 96, false),
            (
                "gesamt",
                gettext("Total"),
                Some("gesamt-text"),
                1.0,
                86,
                false,
            ),
            ("frei", gettext("Free"), Some("frei-text"), 1.0, 86, false),
            (
                "verfuegbar",
                gettext("Available"),
                Some("verfuegbar-text"),
                1.0,
                90,
                false,
            ),
            ("belegt", gettext("Used"), None, 0.0, 150, false),
        ];
        let mut erste = None;
        for (kennung, titel, prop, xalign, breite, expand) in spalten {
            let fabrik = match prop {
                Some(p) => textfabrik(p, xalign),
                None => balkenfabrik(),
            };
            let spalte = gtk::ColumnViewColumn::builder()
                .title(titel.as_str())
                .factory(&fabrik)
                .sorter(&sortierer(kennung))
                .resizable(true)
                .expand(expand)
                .id(kennung)
                .build();
            if breite > 0 {
                spalte.set_fixed_width(breite);
            }
            tabelle.append_column(&spalte);
            if erste.is_none() {
                erste = Some(spalte);
            }
        }
        tabelle.sort_by_column(erste.as_ref(), gtk::SortType::Ascending);

        let scroll = gtk::ScrolledWindow::new();
        scroll.set_child(Some(&tabelle));
        scroll.set_vexpand(true);
        scroll.set_has_frame(true);
        scroll.set_margin_top(6);
        scroll.set_margin_start(6);
        scroll.set_margin_end(6);
        let schalter = gtk::CheckButton::with_mnemonic(&gettext(
            "Show _all file systems (also tmpfs, overlay, squashfs)",
        ));
        schalter.set_action_name(Some("win.alle-dateisysteme"));
        schalter.set_margin_start(6);
        schalter.set_margin_top(6);
        schalter.set_margin_bottom(6);
        let widget = gtk::Box::new(gtk::Orientation::Vertical, 0);
        widget.append(&scroll);
        widget.append(&schalter);
        let liste = Rc::new(Self {
            widget,
            tabelle,
            store,
            objekte: RefCell::new(HashMap::new()),
            alle: Cell::new(alle),
        });
        liste.aktualisieren();
        liste
    }

    pub fn alle_setzen(&self, alle: bool) {
        self.alle.set(alle);
        self.aktualisieren();
    }

    pub fn aktualisieren(&self) -> usize {
        let liste = dateisysteme::lese_dateisysteme(self.alle.get());
        let mut objekte = self.objekte.borrow_mut();
        let mut gesehen = std::collections::HashSet::new();
        for d in &liste {
            gesehen.insert(d.pfad.clone());
            match objekte.get(&d.pfad) {
                Some(o) => o.aktualisieren(d),
                None => {
                    let o = DateisystemObjekt::neu(d);
                    self.store.append(&o);
                    objekte.insert(d.pfad.clone(), o);
                }
            }
        }
        let weg: Vec<String> = objekte
            .keys()
            .filter(|p| !gesehen.contains(*p))
            .cloned()
            .collect();
        for pfad in weg {
            if let Some(o) = objekte.remove(&pfad)
                && let Some(pos) = self.store.find(&o)
            {
                self.store.remove(pos);
            }
        }
        if let Some(s) = self.tabelle.sorter() {
            s.changed(gtk::SorterChange::Different);
        }
        liste.len()
    }
}
