//! Reiter „Ressourcen“: Verlaufsgraphen der letzten 60 Sekunden für CPU
//! (gesamt und je Kern), Arbeitsspeicher/Auslagerung und Netzwerk.
//! Gezeichnet mit GtkDrawingArea und cairo; eigene Messung jede Sekunde,
//! unabhängig vom Intervall der Prozessliste.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gtk::prelude::*;
use gtk::{cairo, glib};

use crate::daten::cpu::{self, Stat};
use crate::daten::einheiten;
use crate::daten::netz::{self, Netz};
use crate::daten::speicher::{self, Speicher};

/// Sekunden im Fenster (ein Messpunkt je Sekunde).
pub const PUNKTE: usize = 60;

/// Feste Farbpalette, Orange (Querencia-Akzent) zuerst.
pub const PALETTE: [(f64, f64, f64); 8] = [
    (0.941, 0.467, 0.275), // #F07746 Orange
    (0.204, 0.396, 0.643), // #3465A4 Blau
    (0.451, 0.824, 0.086), // #73D216 Grün
    (0.459, 0.314, 0.482), // #75507B Violett
    (0.757, 0.490, 0.067), // #C17D11 Braun
    (0.800, 0.000, 0.000), // #CC0000 Rot
    (0.929, 0.831, 0.000), // #EDD400 Gelb
    (0.333, 0.341, 0.325), // #555753 Grau
];

pub fn farbe(index: usize) -> (f64, f64, f64) {
    PALETTE[index % PALETTE.len()]
}

/// Verlaufsreihe: neuester Wert vorn.
#[derive(Debug, Clone, Default)]
pub struct Reihe(pub VecDeque<f64>);

impl Reihe {
    pub fn push(&mut self, wert: f64) {
        self.0.push_front(wert);
        self.0.truncate(PUNKTE + 1);
    }

    pub fn max(&self) -> f64 {
        self.0.iter().copied().fold(0.0, f64::max)
    }
}

#[derive(Debug, Default)]
pub struct Verlauf {
    pub cpu_gesamt: Reihe,
    pub kerne: Vec<Reihe>,
    pub speicher: Reihe,
    pub auslagerung: Reihe,
    pub empfangen: Reihe,
    pub gesendet: Reihe,
    pub netz_summe: Netz,
}

/// Runde Obergrenze für die Netzwerkskala: 2er-Potenzen in KiB/s, mindestens 1.
pub fn netz_skala(max_bytes: f64) -> f64 {
    let kib = (max_bytes / 1024.0).max(1.0);
    let mut grenze = 1.0_f64;
    while grenze < kib {
        grenze *= 2.0;
    }
    grenze * 1024.0
}

fn graph_bereich(w: i32, h: i32) -> (f64, f64, f64, f64) {
    // (links, oben, breite, höhe) des Zeichenbereichs
    let links = 58.0;
    let oben = 8.0;
    let unten = 20.0;
    let rechts = 10.0;
    (
        links,
        oben,
        (f64::from(w) - links - rechts).max(10.0),
        (f64::from(h) - oben - unten).max(10.0),
    )
}

/// Gitter, Beschriftung, Reihen. `max` ist der Wert am oberen Rand.
fn zeichnen(
    cr: &cairo::Context,
    w: i32,
    h: i32,
    reihen: &[(usize, &Reihe)],
    max: f64,
    oben_text: &str,
    mitte_text: &str,
) {
    let (x0, y0, bw, bh) = graph_bereich(w, h);
    cr.set_source_rgb(1.0, 1.0, 1.0);
    let _ = cr.paint();
    cr.set_line_width(1.0);
    // Gitter: 4 waagerechte Felder, 6 senkrechte (je 10 s)
    cr.set_source_rgb(0.85, 0.85, 0.85);
    for i in 0..=4 {
        let y = (y0 + bh * f64::from(i) / 4.0).round() + 0.5;
        cr.move_to(x0, y);
        cr.line_to(x0 + bw, y);
    }
    for i in 0..=6 {
        let x = (x0 + bw * f64::from(i) / 6.0).round() + 0.5;
        cr.move_to(x, y0);
        cr.line_to(x, y0 + bh);
    }
    let _ = cr.stroke();
    // Beschriftung
    cr.set_source_rgb(0.3, 0.3, 0.3);
    cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    cr.set_font_size(10.0);
    let rechtsbuendig = |cr: &cairo::Context, text: &str, x: f64, y: f64| {
        if let Ok(ext) = cr.text_extents(text) {
            cr.move_to(x - ext.width() - 4.0, y);
            let _ = cr.show_text(text);
        }
    };
    rechtsbuendig(cr, oben_text, x0, y0 + 9.0);
    rechtsbuendig(cr, mitte_text, x0, y0 + bh / 2.0 + 4.0);
    rechtsbuendig(cr, "0", x0, y0 + bh);
    for (i, text) in [(0, "60 s"), (3, "30 s"), (6, "0")] {
        let x = x0 + bw * f64::from(i) / 6.0;
        if let Ok(ext) = cr.text_extents(text) {
            let tx = (x - ext.width() / 2.0).clamp(x0, x0 + bw - ext.width());
            cr.move_to(tx, y0 + bh + 14.0);
            let _ = cr.show_text(text);
        }
    }
    // Reihen: neuester Punkt rechts
    cr.set_line_width(1.5);
    cr.set_line_join(cairo::LineJoin::Round);
    let schritt = bw / PUNKTE as f64;
    let max = max.max(f64::EPSILON);
    for (farbindex, reihe) in reihen {
        let (r, g, b) = farbe(*farbindex);
        cr.set_source_rgb(r, g, b);
        let mut erster = true;
        for (i, wert) in reihe.0.iter().enumerate() {
            let x = x0 + bw - schritt * i as f64;
            let y = y0 + bh - (wert / max).clamp(0.0, 1.0) * bh;
            if erster {
                cr.move_to(x, y);
                erster = false;
            } else {
                cr.line_to(x, y);
            }
        }
        let _ = cr.stroke();
    }
    // Rahmen
    cr.set_line_width(1.0);
    cr.set_source_rgb(0.6, 0.6, 0.6);
    cr.rectangle(x0.round() + 0.5, y0.round() + 0.5, bw.round(), bh.round());
    let _ = cr.stroke();
}

fn farbfeld(index: usize) -> gtk::DrawingArea {
    let a = gtk::DrawingArea::new();
    a.set_content_width(12);
    a.set_content_height(12);
    a.set_valign(gtk::Align::Center);
    a.set_draw_func(move |_, cr, w, h| {
        let (r, g, b) = farbe(index);
        cr.set_source_rgb(r, g, b);
        cr.rectangle(0.0, 0.0, f64::from(w), f64::from(h));
        let _ = cr.fill();
        cr.set_source_rgb(0.3, 0.3, 0.3);
        cr.set_line_width(1.0);
        cr.rectangle(0.5, 0.5, f64::from(w) - 1.0, f64::from(h) - 1.0);
        let _ = cr.stroke();
    });
    a
}

fn legende(index: usize, text: &str) -> (gtk::Box, gtk::Label) {
    let b = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    b.set_margin_end(12);
    let l = gtk::Label::new(Some(text));
    l.set_xalign(0.0);
    b.append(&farbfeld(index));
    b.append(&l);
    (b, l)
}

fn abschnitt(titel: &str, area: &gtk::DrawingArea, legenden: &gtk::FlowBox) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let t = gtk::Label::new(Some(titel));
    t.add_css_class("qsm-abschnitt");
    t.set_xalign(0.0);
    let rahmen = gtk::Box::new(gtk::Orientation::Vertical, 0);
    rahmen.add_css_class("qsm-graph");
    rahmen.append(area);
    area.set_vexpand(true);
    area.set_hexpand(true);
    b.append(&t);
    b.append(&rahmen);
    b.append(legenden);
    b.set_vexpand(true);
    b
}

pub struct Ressourcen {
    pub widget: gtk::Box,
    verlauf: Rc<RefCell<Verlauf>>,
    cpu_area: gtk::DrawingArea,
    speicher_area: gtk::DrawingArea,
    netz_area: gtk::DrawingArea,
    cpu_labels: Vec<gtk::Label>,
    speicher_label: gtk::Label,
    auslagerung_label: gtk::Label,
    empfangen_label: gtk::Label,
    gesendet_label: gtk::Label,
    cpu_alt: RefCell<Option<Stat>>,
    netz_alt: RefCell<Option<(Netz, Instant)>>,
    pub messungen: std::cell::Cell<u64>,
}

impl Ressourcen {
    pub fn neu() -> Rc<Self> {
        let kerne = cpu::lese_stat().map_or(1, |s| s.kerne.len().max(1));
        let verlauf = Rc::new(RefCell::new(Verlauf {
            kerne: vec![Reihe::default(); kerne],
            ..Default::default()
        }));

        // CPU
        let cpu_area = gtk::DrawingArea::new();
        cpu_area.set_content_height(120);
        {
            let v = verlauf.clone();
            cpu_area.set_draw_func(move |_, cr, w, h| {
                let v = v.borrow();
                let mut reihen: Vec<(usize, &Reihe)> = v
                    .kerne
                    .iter()
                    .enumerate()
                    .map(|(i, r)| (i + 1, r))
                    .collect();
                reihen.push((0, &v.cpu_gesamt));
                zeichnen(cr, w, h, &reihen, 100.0, "100 %", "50 %");
            });
        }
        let cpu_legenden = gtk::FlowBox::new();
        cpu_legenden.set_selection_mode(gtk::SelectionMode::None);
        cpu_legenden.set_max_children_per_line(8);
        cpu_legenden.set_column_spacing(6);
        cpu_legenden.set_row_spacing(2);
        let mut cpu_labels = Vec::new();
        let (b, l) = legende(0, "CPU gesamt: 0 %");
        cpu_legenden.insert(&b, -1);
        cpu_labels.push(l);
        for i in 0..kerne {
            let (b, l) = legende(i + 1, &format!("CPU{}: 0 %", i + 1));
            cpu_legenden.insert(&b, -1);
            cpu_labels.push(l);
        }

        // Speicher
        let speicher_area = gtk::DrawingArea::new();
        speicher_area.set_content_height(100);
        {
            let v = verlauf.clone();
            speicher_area.set_draw_func(move |_, cr, w, h| {
                let v = v.borrow();
                zeichnen(
                    cr,
                    w,
                    h,
                    &[(0, &v.speicher), (1, &v.auslagerung)],
                    100.0,
                    "100 %",
                    "50 %",
                );
            });
        }
        let speicher_legenden = gtk::FlowBox::new();
        speicher_legenden.set_selection_mode(gtk::SelectionMode::None);
        speicher_legenden.set_max_children_per_line(2);
        let (b, speicher_label) = legende(0, "Speicher: –");
        speicher_legenden.insert(&b, -1);
        let (b, auslagerung_label) = legende(1, "Auslagerung: –");
        speicher_legenden.insert(&b, -1);

        // Netzwerk
        let netz_area = gtk::DrawingArea::new();
        netz_area.set_content_height(100);
        {
            let v = verlauf.clone();
            netz_area.set_draw_func(move |_, cr, w, h| {
                let v = v.borrow();
                let max = netz_skala(v.empfangen.max().max(v.gesendet.max()));
                let oben = einheiten::rate(max);
                let mitte = einheiten::rate(max / 2.0);
                zeichnen(
                    cr,
                    w,
                    h,
                    &[(0, &v.empfangen), (1, &v.gesendet)],
                    max,
                    &oben,
                    &mitte,
                );
            });
        }
        let netz_legenden = gtk::FlowBox::new();
        netz_legenden.set_selection_mode(gtk::SelectionMode::None);
        netz_legenden.set_max_children_per_line(2);
        let (b, empfangen_label) = legende(0, "Empfangen: –");
        netz_legenden.insert(&b, -1);
        let (b, gesendet_label) = legende(1, "Gesendet: –");
        netz_legenden.insert(&b, -1);

        let widget = gtk::Box::new(gtk::Orientation::Vertical, 12);
        widget.set_margin_top(8);
        widget.set_margin_bottom(8);
        widget.set_margin_start(8);
        widget.set_margin_end(8);
        widget.append(&abschnitt("CPU-Verlauf", &cpu_area, &cpu_legenden));
        widget.append(&abschnitt(
            "Speicher- und Auslagerungsverlauf",
            &speicher_area,
            &speicher_legenden,
        ));
        widget.append(&abschnitt("Netzwerkverlauf", &netz_area, &netz_legenden));

        let r = Rc::new(Self {
            widget,
            verlauf,
            cpu_area,
            speicher_area,
            netz_area,
            cpu_labels,
            speicher_label,
            auslagerung_label,
            empfangen_label,
            gesendet_label,
            cpu_alt: RefCell::new(None),
            netz_alt: RefCell::new(None),
            messungen: std::cell::Cell::new(0),
        });
        r.messen();
        let schwach = Rc::downgrade(&r);
        glib::timeout_add_local(Duration::from_secs(1), move || match schwach.upgrade() {
            Some(r) => {
                r.messen();
                glib::ControlFlow::Continue
            }
            None => glib::ControlFlow::Break,
        });
        r
    }

    /// Eine Messung: /proc/stat, /proc/meminfo, /proc/net/dev.
    fn messen(&self) {
        let jetzt = Instant::now();
        if let Some(stat) = cpu::lese_stat() {
            let mut alt = self.cpu_alt.borrow_mut();
            if let Some(alt) = alt.as_ref() {
                let mut v = self.verlauf.borrow_mut();
                let gesamt = cpu::auslastung(&alt.gesamt, &stat.gesamt);
                v.cpu_gesamt.push(gesamt);
                self.cpu_labels[0].set_text(&format!("CPU gesamt: {}", einheiten::prozent(gesamt)));
                for (i, kern) in stat.kerne.iter().enumerate() {
                    let wert = alt.kerne.get(i).map_or(0.0, |a| cpu::auslastung(a, kern));
                    if let Some(reihe) = v.kerne.get_mut(i) {
                        reihe.push(wert);
                    }
                    if let Some(l) = self.cpu_labels.get(i + 1) {
                        l.set_text(&format!("CPU{}: {}", i + 1, einheiten::prozent(wert)));
                    }
                }
            }
            *alt = Some(stat);
        }
        if let Some(s) = speicher::lese_meminfo() {
            let mut v = self.verlauf.borrow_mut();
            let sp = Speicher::anteil(s.belegt(), s.gesamt);
            let au = Speicher::anteil(s.auslagerung_belegt(), s.auslagerung_gesamt);
            v.speicher.push(sp * 100.0);
            v.auslagerung.push(au * 100.0);
            self.speicher_label.set_text(&format!(
                "Speicher: {} ({}) von {}",
                einheiten::bytes(s.belegt()),
                einheiten::prozent(sp * 100.0),
                einheiten::bytes(s.gesamt)
            ));
            self.auslagerung_label.set_text(&format!(
                "Auslagerung: {} ({}) von {}",
                einheiten::bytes(s.auslagerung_belegt()),
                einheiten::prozent(au * 100.0),
                einheiten::bytes(s.auslagerung_gesamt)
            ));
        }
        if let Some(n) = netz::lese_netz() {
            let mut alt = self.netz_alt.borrow_mut();
            if let Some((a, zeit)) = alt.as_ref() {
                let sek = jetzt.duration_since(*zeit).as_secs_f64();
                let mut v = self.verlauf.borrow_mut();
                let rx = netz::rate(a.empfangen, n.empfangen, sek);
                let tx = netz::rate(a.gesendet, n.gesendet, sek);
                v.empfangen.push(rx);
                v.gesendet.push(tx);
                v.netz_summe = n;
                self.empfangen_label.set_text(&format!(
                    "Empfangen: {} (gesamt {})",
                    einheiten::rate(rx),
                    einheiten::bytes(n.empfangen)
                ));
                self.gesendet_label.set_text(&format!(
                    "Gesendet: {} (gesamt {})",
                    einheiten::rate(tx),
                    einheiten::bytes(n.gesendet)
                ));
            }
            *alt = Some((n, jetzt));
        }
        self.messungen.set(self.messungen.get() + 1);
        self.cpu_area.queue_draw();
        self.speicher_area.queue_draw();
        self.netz_area.queue_draw();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reihe_begrenzt() {
        let mut r = Reihe::default();
        for i in 0..200 {
            r.push(f64::from(i));
        }
        assert_eq!(r.0.len(), PUNKTE + 1);
        assert_eq!(r.0[0], 199.0, "neuester Wert vorn");
        assert_eq!(r.max(), 199.0);
        assert_eq!(Reihe::default().max(), 0.0);
    }

    #[test]
    fn netz_skala_zweierpotenzen() {
        assert_eq!(netz_skala(0.0), 1024.0);
        assert_eq!(netz_skala(1024.0), 1024.0);
        assert_eq!(netz_skala(1025.0), 2048.0);
        assert_eq!(netz_skala(100.0 * 1024.0), 128.0 * 1024.0);
    }

    #[test]
    fn palette_beginnt_mit_orange() {
        assert_eq!(farbe(0), (0.941, 0.467, 0.275));
        assert_eq!(farbe(8), farbe(0), "zyklisch");
    }
}
