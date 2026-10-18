//! `/proc/meminfo`: Arbeitsspeicher und Auslagerung in Bytes.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Speicher {
    pub gesamt: u64,
    pub frei: u64,
    pub verfuegbar: u64,
    pub puffer: u64,
    pub zwischenspeicher: u64,
    pub auslagerung_gesamt: u64,
    pub auslagerung_frei: u64,
}

impl Speicher {
    /// Belegt im Sinne der GNOME-Systemüberwachung: ohne Puffer und
    /// Zwischenspeicher, also was Programme wirklich halten.
    pub fn belegt(&self) -> u64 {
        self.gesamt.saturating_sub(self.verfuegbar)
    }

    pub fn auslagerung_belegt(&self) -> u64 {
        self.auslagerung_gesamt
            .saturating_sub(self.auslagerung_frei)
    }

    /// Anteil 0–1; 0 bei leerem Gesamtwert (keine Auslagerung eingerichtet).
    pub fn anteil(belegt: u64, gesamt: u64) -> f64 {
        if gesamt == 0 {
            0.0
        } else {
            (belegt as f64 / gesamt as f64).clamp(0.0, 1.0)
        }
    }
}

/// Liest die Zeilen `Name:   12345 kB`; fehlende Werte bleiben 0. `None`
/// nur ohne `MemTotal`.
pub fn parse_meminfo(text: &str) -> Option<Speicher> {
    let mut s = Speicher::default();
    let mut gesamt_gefunden = false;
    for zeile in text.lines() {
        let Some((name, rest)) = zeile.split_once(':') else {
            continue;
        };
        let mut teile = rest.split_ascii_whitespace();
        let Some(zahl) = teile.next().and_then(|z| z.parse::<u64>().ok()) else {
            continue;
        };
        // Alle Werte stehen in kB (so meldet es der Kernel, auch ohne Einheit).
        let bytes = match teile.next() {
            Some("kB") | None => zahl.saturating_mul(1024),
            Some(_) => zahl,
        };
        match name {
            "MemTotal" => {
                s.gesamt = bytes;
                gesamt_gefunden = true;
            }
            "MemFree" => s.frei = bytes,
            "MemAvailable" => s.verfuegbar = bytes,
            "Buffers" => s.puffer = bytes,
            "Cached" => s.zwischenspeicher = bytes,
            "SwapTotal" => s.auslagerung_gesamt = bytes,
            "SwapFree" => s.auslagerung_frei = bytes,
            _ => {}
        }
    }
    if gesamt_gefunden && s.verfuegbar == 0 {
        // Alte Kernel ohne MemAvailable: Näherung wie `free` vor 3.14.
        s.verfuegbar = s.frei + s.puffer + s.zwischenspeicher;
    }
    gesamt_gefunden.then_some(s)
}

pub fn lese_meminfo() -> Option<Speicher> {
    parse_meminfo(&std::fs::read_to_string("/proc/meminfo").ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meminfo_beispiel() {
        let s = parse_meminfo(include_str!("../../tests/daten/meminfo")).unwrap();
        assert_eq!(s.gesamt, 16_302_152 * 1024);
        assert!(s.verfuegbar > 0 && s.verfuegbar < s.gesamt);
        assert!(s.belegt() < s.gesamt);
        assert!(s.auslagerung_belegt() <= s.auslagerung_gesamt);
    }

    #[test]
    fn meminfo_feste_werte() {
        let text = "MemTotal:       1000 kB\nMemFree:  100 kB\nMemAvailable: 400 kB\nBuffers: 50 kB\nCached: 250 kB\nSwapTotal: 2000 kB\nSwapFree: 1500 kB\nHugePagesize: 2048 kB\n";
        let s = parse_meminfo(text).unwrap();
        assert_eq!(s.belegt(), 600 * 1024);
        assert_eq!(s.auslagerung_belegt(), 500 * 1024);
        assert!((Speicher::anteil(s.belegt(), s.gesamt) - 0.6).abs() < 1e-9);
        assert_eq!(Speicher::anteil(5, 0), 0.0);
    }

    #[test]
    fn meminfo_ohne_memavailable() {
        let s =
            parse_meminfo("MemTotal: 1000 kB\nMemFree: 100 kB\nBuffers: 50 kB\nCached: 250 kB\n")
                .unwrap();
        assert_eq!(s.verfuegbar, 400 * 1024);
        assert!(parse_meminfo("MemFree: 1 kB\n").is_none());
    }
}
