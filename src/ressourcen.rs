//! Reiter „Ressourcen“: Verlaufsgraphen der letzten 60 Sekunden für CPU
//! (gesamt und je Kern), Arbeitsspeicher/Auslagerung und Netzwerk.
//! Gezeichnet mit GtkDrawingArea und cairo; eigene Messung jede Sekunde,
//! unabhängig vom Intervall der Prozessliste.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gtk::prelude::*;
use gtk::{cairo, gdk, glib};

use crate::daten::cpu::{self, Stat};
use crate::daten::einheiten;
use crate::daten::netz::{self, Netz};
use crate::daten::speicher::{self, Speicher};
use crate::i18n::{gettext, gettext_f};

/// Sekunden im Fenster (ein Messpunkt je Sekunde).
pub const PUNKTE: usize = 60;

/// Feste Farbpalette für helle Themen, Orange (Querencia-Akzent) zuerst.
/// Jede Linie hat auf Weiß mindestens 3:1 (WCAG 1.4.11, Grün 3,35:1). Seit
/// 0.1.4 sind Orange, Grün und Gelb dunkler (bis 0.1.3 #F07746 2,83:1,
/// #73D216 1,92:1, #EDD400 1,50:1); Orange bleibt in der Helligkeit klar
/// vom Rot getrennt. Geprüft mit `kontrast-systemmonitor.py`.
pub const PALETTE: [(f64, f64, f64); 8] = [
    (0.824, 0.329, 0.000), // #D25400 Orange
    (0.204, 0.396, 0.643), // #3465A4 Blau
    (0.118, 0.635, 0.188), // #1EA230 Grün
    (0.459, 0.314, 0.482), // #75507B Violett
    (0.757, 0.490, 0.067), // #C17D11 Braun
    (0.800, 0.000, 0.000), // #CC0000 Rot
    (0.541, 0.494, 0.000), // #8A7E00 Gelb
    (0.333, 0.341, 0.325), // #555753 Grau
];

/// Palette für dunkle Themen: die hellen Tango-Töne, Orange im Akzent. Jede
/// Linie hat auf dem dunklen Grund #2b2a27 mindestens 3:1 (WCAG 1.4.11,
/// Rot 3,43:1); geprüft mit `kontrast-systemmonitor.py`.
pub const PALETTE_DUNKEL: [(f64, f64, f64); 8] = [
    (0.941, 0.467, 0.275), // #F07746 Orange
    (0.447, 0.624, 0.812), // #729FCF Blau
    (0.541, 0.886, 0.204), // #8AE234 Grün
    (0.678, 0.498, 0.659), // #AD7FA8 Violett
    (0.914, 0.725, 0.431), // #E9B96E Braun
    (0.937, 0.161, 0.161), // #EF2929 Rot
    (0.988, 0.914, 0.310), // #FCE94F Gelb
    (0.827, 0.843, 0.812), // #D3D7CF Grau
];

pub fn farbe(index: usize) -> (f64, f64, f64) {
    PALETTE[index % PALETTE.len()]
}

pub fn farbe_dunkel(index: usize) -> (f64, f64, f64) {
    PALETTE_DUNKEL[index % PALETTE_DUNKEL.len()]
}

type Rgb = (f64, f64, f64);

fn rgb(c: gdk::RGBA) -> Rgb {
    (
        f64::from(c.red()),
        f64::from(c.green()),
        f64::from(c.blue()),
    )
}

/// Relative Helligkeit nach WCAG 2.x (sRGB linearisiert).
pub fn luminanz((r, g, b): Rgb) -> f64 {
    let lin = |v: f64| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

/// a·(1 − t) + b·t
fn mischen(a: Rgb, b: Rgb, t: f64) -> Rgb {
    (
        a.0 + (b.0 - a.0) * t,
        a.1 + (b.1 - a.1) * t,
        a.2 + (b.2 - a.2) * t,
    )
}

/// Farben eines Verlaufsgraphen. cairo kennt keine Themenfarben: Der Grund
/// kommt als CSS-Farbe der Zeichenfläche (Klasse `qsm-kurve`, `style.css`:
/// `color: @theme_base_color`), die Schrift als Farbe des Rahmens (erbt
/// `@theme_fg_color`). Ist der Grund dunkel, gelten die Schrift des Themas,
/// daraus gemischte Gitter- und Rahmenfarben und [`PALETTE_DUNKEL`]; sonst die
/// festen Grauwerte und [`PALETTE`] (Rahmen seit 0.1.4 dunkler, 3,54:1). Die
/// Wahl folgt damit dem tatsächlich geladenen Thema, nicht der Einstellung
/// (Thema 0.1 bleibt hell).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Graphfarben {
    pub grund: Rgb,
    pub gitter: Rgb,
    pub schrift: Rgb,
    pub rahmen: Rgb,
    pub dunkel: bool,
}

impl Graphfarben {
    pub fn aus(grund: Rgb, schrift: Rgb) -> Self {
        if luminanz(grund) < 0.5 {
            Self {
                grund,
                gitter: mischen(grund, schrift, 0.15),
                schrift,
                rahmen: mischen(grund, schrift, 0.55),
                dunkel: true,
            }
        } else {
            Self {
                grund,
                gitter: (0.85, 0.85, 0.85),
                schrift: (0.3, 0.3, 0.3),
                rahmen: (0.533, 0.533, 0.533),
                dunkel: false,
            }
        }
    }

    fn von(area: &gtk::DrawingArea) -> Self {
        let schrift = area.parent().map_or((0.3, 0.3, 0.3), |p| rgb(p.color()));
        let grund = rgb(area.color());
        // Greift die CSS-Regel nicht, erbt die Fläche nur die Schrift: dann
        // wie bis 0.1.2 auf weißem Grund statt Schrift auf Schrift.
        if grund == schrift {
            return Self::aus((1.0, 1.0, 1.0), schrift);
        }
        Self::aus(grund, schrift)
    }

    pub fn linie(&self, index: usize) -> Rgb {
        if self.dunkel {
            farbe_dunkel(index)
        } else {
            farbe(index)
        }
    }
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
    let links = 72.0;
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
#[allow(clippy::too_many_arguments)]
fn zeichnen(
    cr: &cairo::Context,
    f: &Graphfarben,
    w: i32,
    h: i32,
    reihen: &[(usize, &Reihe)],
    max: f64,
    oben_text: &str,
    mitte_text: &str,
) {
    let (x0, y0, bw, bh) = graph_bereich(w, h);
    let (r, g, b) = f.grund;
    cr.set_source_rgb(r, g, b);
    let _ = cr.paint();
    cr.set_line_width(1.0);
    // Gitter: 4 waagerechte Felder, 6 senkrechte (je 10 s)
    let (r, g, b) = f.gitter;
    cr.set_source_rgb(r, g, b);
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
    let (r, g, b) = f.schrift;
    cr.set_source_rgb(r, g, b);
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
        let (r, g, b) = f.linie(*farbindex);
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
    let (r, g, b) = f.rahmen;
    cr.set_source_rgb(r, g, b);
    cr.rectangle(x0.round() + 0.5, y0.round() + 0.5, bw.round(), bh.round());
    let _ = cr.stroke();
}

fn farbfeld(index: usize) -> gtk::DrawingArea {
    let a = gtk::DrawingArea::new();
    a.set_content_width(12);
    a.set_content_height(12);
    a.set_valign(gtk::Align::Center);
    a.set_draw_func(move |a, cr, w, h| {
        // Dunkel, wenn die geerbte Schrift (@theme_fg_color) hell ist; dann
        // die dunkle Palette und ein Rand in der Schriftfarbe.
        let schrift = rgb(a.color());
        let dunkel = luminanz(schrift) > 0.5;
        let (r, g, b) = if dunkel {
            farbe_dunkel(index)
        } else {
            farbe(index)
        };
        cr.set_source_rgb(r, g, b);
        cr.rectangle(0.0, 0.0, f64::from(w), f64::from(h));
        let _ = cr.fill();
        let (r, g, b) = if dunkel { schrift } else { (0.3, 0.3, 0.3) };
        cr.set_source_rgb(r, g, b);
        cr.set_line_width(1.0);
        cr.rectangle(0.5, 0.5, f64::from(w) - 1.0, f64::from(h) - 1.0);
        let _ = cr.stroke();
    });
    a
}

/// „CPU total: 12 %“
fn cpu_gesamt_text(wert: &str) -> String {
    gettext_f("CPU total: {value}", &[("value", wert)])
}

/// „CPU3: 12 %“ (Kerne ab 1 gezählt)
fn cpu_kern_text(nummer: usize, wert: &str) -> String {
    gettext_f(
        "CPU{number}: {value}",
        &[("number", &nummer.to_string()), ("value", wert)],
    )
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
    // Trägt den Grund des Graphen als CSS-Farbe (siehe Graphfarben).
    area.add_css_class("qsm-kurve");
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
        cpu_area.set_content_height(104);
        {
            let v = verlauf.clone();
            cpu_area.set_draw_func(move |a, cr, w, h| {
                let v = v.borrow();
                let mut reihen: Vec<(usize, &Reihe)> = v
                    .kerne
                    .iter()
                    .enumerate()
                    .map(|(i, r)| (i + 1, r))
                    .collect();
                reihen.push((0, &v.cpu_gesamt));
                zeichnen(
                    cr,
                    &Graphfarben::von(a),
                    w,
                    h,
                    &reihen,
                    100.0,
                    "100 %",
                    "50 %",
                );
            });
        }
        let cpu_legenden = gtk::FlowBox::new();
        cpu_legenden.set_selection_mode(gtk::SelectionMode::None);
        cpu_legenden.set_max_children_per_line(8);
        cpu_legenden.set_column_spacing(6);
        cpu_legenden.set_row_spacing(2);
        let mut cpu_labels = Vec::new();
        // TRANSLATORS: Legenden im Reiter „Resources“; {value} ist z. B. „12 %“.
        let (b, l) = legende(0, &cpu_gesamt_text(&einheiten::prozent(0.0)));
        cpu_legenden.insert(&b, -1);
        cpu_labels.push(l);
        for i in 0..kerne {
            let (b, l) = legende(i + 1, &cpu_kern_text(i + 1, &einheiten::prozent(0.0)));
            cpu_legenden.insert(&b, -1);
            cpu_labels.push(l);
        }

        // Speicher
        let speicher_area = gtk::DrawingArea::new();
        speicher_area.set_content_height(84);
        {
            let v = verlauf.clone();
            speicher_area.set_draw_func(move |a, cr, w, h| {
                let v = v.borrow();
                zeichnen(
                    cr,
                    &Graphfarben::von(a),
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
        let (b, speicher_label) = legende(0, &gettext_f("Memory: {value}", &[("value", "–")]));
        speicher_legenden.insert(&b, -1);
        let (b, auslagerung_label) = legende(1, &gettext_f("Swap: {value}", &[("value", "–")]));
        speicher_legenden.insert(&b, -1);

        // Netzwerk
        let netz_area = gtk::DrawingArea::new();
        netz_area.set_content_height(84);
        {
            let v = verlauf.clone();
            netz_area.set_draw_func(move |a, cr, w, h| {
                let v = v.borrow();
                let max = netz_skala(v.empfangen.max().max(v.gesendet.max()));
                let oben = einheiten::rate(max);
                let mitte = einheiten::rate(max / 2.0);
                zeichnen(
                    cr,
                    &Graphfarben::von(a),
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
        let (b, empfangen_label) = legende(0, &gettext_f("Received: {value}", &[("value", "–")]));
        netz_legenden.insert(&b, -1);
        let (b, gesendet_label) = legende(1, &gettext_f("Sent: {value}", &[("value", "–")]));
        netz_legenden.insert(&b, -1);

        let widget = gtk::Box::new(gtk::Orientation::Vertical, 12);
        widget.set_margin_top(8);
        widget.set_margin_bottom(8);
        widget.set_margin_start(8);
        widget.set_margin_end(8);
        // TRANSLATORS: Abschnitte im Reiter „Resources“ (gnome-system-monitor 2.x).
        widget.append(&abschnitt(
            &gettext("CPU History"),
            &cpu_area,
            &cpu_legenden,
        ));
        widget.append(&abschnitt(
            &gettext("Memory and Swap History"),
            &speicher_area,
            &speicher_legenden,
        ));
        widget.append(&abschnitt(
            &gettext("Network History"),
            &netz_area,
            &netz_legenden,
        ));

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
                self.cpu_labels[0].set_text(&cpu_gesamt_text(&einheiten::prozent(gesamt)));
                for (i, kern) in stat.kerne.iter().enumerate() {
                    let wert = alt.kerne.get(i).map_or(0.0, |a| cpu::auslastung(a, kern));
                    if let Some(reihe) = v.kerne.get_mut(i) {
                        reihe.push(wert);
                    }
                    if let Some(l) = self.cpu_labels.get(i + 1) {
                        l.set_text(&cpu_kern_text(i + 1, &einheiten::prozent(wert)));
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
            self.speicher_label.set_text(&gettext_f(
                "Memory: {used} ({percent}) of {total}",
                &[
                    ("used", &einheiten::bytes(s.belegt())),
                    ("percent", &einheiten::prozent(sp * 100.0)),
                    ("total", &einheiten::bytes(s.gesamt)),
                ],
            ));
            self.auslagerung_label.set_text(&gettext_f(
                "Swap: {used} ({percent}) of {total}",
                &[
                    ("used", &einheiten::bytes(s.auslagerung_belegt())),
                    ("percent", &einheiten::prozent(au * 100.0)),
                    ("total", &einheiten::bytes(s.auslagerung_gesamt)),
                ],
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
                self.empfangen_label.set_text(&gettext_f(
                    "Received: {rate} (total {sum})",
                    &[
                        ("rate", &einheiten::rate(rx)),
                        ("sum", &einheiten::bytes(n.empfangen)),
                    ],
                ));
                self.gesendet_label.set_text(&gettext_f(
                    "Sent: {rate} (total {sum})",
                    &[
                        ("rate", &einheiten::rate(tx)),
                        ("sum", &einheiten::bytes(n.gesendet)),
                    ],
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
        assert_eq!(farbe(0), (0.824, 0.329, 0.000));
        assert_eq!(farbe(8), farbe(0), "zyklisch");
        assert_eq!(farbe_dunkel(0), (0.941, 0.467, 0.275), "Akzent dunkel");
        assert_eq!(farbe_dunkel(8), farbe_dunkel(0), "zyklisch");
    }

    fn kontrast(a: Rgb, b: Rgb) -> f64 {
        let (la, lb) = (luminanz(a), luminanz(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    /// Hell (Thema 0.1/0.2 hell, Grund #ffffff): Gitter und Beschriftung wie
    /// bis 0.1.3; Linien und Rahmen seit 0.1.4 mindestens 3:1.
    #[test]
    fn hell_3_zu_1() {
        let f = Graphfarben::aus((1.0, 1.0, 1.0), (0.235, 0.235, 0.235));
        assert!(!f.dunkel);
        assert_eq!(f.grund, (1.0, 1.0, 1.0));
        assert_eq!(f.gitter, (0.85, 0.85, 0.85));
        assert_eq!(f.schrift, (0.3, 0.3, 0.3));
        for (i, farbe) in PALETTE.iter().enumerate() {
            assert_eq!(f.linie(i), *farbe);
            let k = kontrast(f.linie(i), f.grund);
            assert!(k >= 3.0, "Linie {i}: {k:.2}:1");
        }
        assert!(kontrast(f.schrift, f.grund) >= 4.5);
        let k = kontrast(f.rahmen, f.grund);
        assert!(k >= 3.0, "Rahmen: {k:.2}:1");
        assert!(kontrast(f.gitter, f.grund) < k);
    }

    /// Dunkel bleibt wie 0.1.3 (0.1.4 ändert nur die helle Fassung).
    #[test]
    fn dunkel_wie_013() {
        let alt = [
            (0.941, 0.467, 0.275),
            (0.447, 0.624, 0.812),
            (0.541, 0.886, 0.204),
            (0.678, 0.498, 0.659),
            (0.914, 0.725, 0.431),
            (0.937, 0.161, 0.161),
            (0.988, 0.914, 0.310),
            (0.827, 0.843, 0.812),
        ];
        assert_eq!(PALETTE_DUNKEL, alt);
    }

    /// Dunkel (Thema 0.2: Grund #2b2a27, Schrift #dfdbd2): Linien und Rahmen
    /// mindestens 3:1, Beschriftung mindestens 4,5:1 (WCAG AA).
    #[test]
    fn dunkel_aa() {
        let hex = |h: u32| {
            (
                f64::from((h >> 16) & 0xff) / 255.0,
                f64::from((h >> 8) & 0xff) / 255.0,
                f64::from(h & 0xff) / 255.0,
            )
        };
        let f = Graphfarben::aus(hex(0x2b2a27), hex(0xdfdbd2));
        assert!(f.dunkel);
        assert_eq!(f.grund, hex(0x2b2a27));
        for i in 0..8 {
            let k = kontrast(f.linie(i), f.grund);
            assert!(k >= 3.0, "Linie {i}: {k:.2}:1");
        }
        assert!(kontrast(f.schrift, f.grund) >= 4.5);
        assert!(kontrast(f.rahmen, f.grund) >= 3.0);
        assert!(kontrast(f.gitter, f.grund) < kontrast(f.rahmen, f.grund));
    }
}
