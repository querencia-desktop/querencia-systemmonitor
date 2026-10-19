//! `/proc/stat`: Prozessorzeiten gesamt und je Kern, Startzeit des Systems.

/// Zeiten eines Prozessors in Takten (`CLK_TCK`), wie in `/proc/stat`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CpuZeiten {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub softirq: u64,
    pub steal: u64,
}

impl CpuZeiten {
    /// Alle Takte, auch Leerlauf.
    pub fn gesamt(&self) -> u64 {
        self.user
            + self.nice
            + self.system
            + self.idle
            + self.iowait
            + self.irq
            + self.softirq
            + self.steal
    }

    /// Leerlauf einschließlich Warten auf E/A.
    pub fn leerlauf(&self) -> u64 {
        self.idle + self.iowait
    }

    fn aus_feldern(felder: &[&str]) -> Option<Self> {
        let mut werte = [0u64; 8];
        for (ziel, feld) in werte.iter_mut().zip(felder) {
            *ziel = feld.parse().ok()?;
        }
        if felder.len() < 4 {
            return None;
        }
        Some(Self {
            user: werte[0],
            nice: werte[1],
            system: werte[2],
            idle: werte[3],
            iowait: werte[4],
            irq: werte[5],
            softirq: werte[6],
            steal: werte[7],
        })
    }
}

/// Inhalt von `/proc/stat`, soweit der Systemmonitor ihn braucht.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stat {
    pub gesamt: CpuZeiten,
    pub kerne: Vec<CpuZeiten>,
    /// Systemstart in Sekunden seit der Epoche (`btime`).
    pub btime: u64,
}

/// Liest `/proc/stat` aus `text`; `None`, wenn die Zeile `cpu` fehlt.
pub fn parse_stat(text: &str) -> Option<Stat> {
    let mut stat = Stat::default();
    let mut gesamt_gefunden = false;
    for zeile in text.lines() {
        let mut felder = zeile.split_ascii_whitespace();
        let Some(schluessel) = felder.next() else {
            continue;
        };
        let rest: Vec<&str> = felder.collect();
        if schluessel == "cpu" {
            stat.gesamt = CpuZeiten::aus_feldern(&rest)?;
            gesamt_gefunden = true;
        } else if let Some(nummer) = schluessel.strip_prefix("cpu") {
            if nummer.chars().all(|c| c.is_ascii_digit()) {
                stat.kerne.push(CpuZeiten::aus_feldern(&rest)?);
            }
        } else if schluessel == "btime" {
            stat.btime = rest.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        }
    }
    gesamt_gefunden.then_some(stat)
}

/// Auslastung in Prozent (0–100) zwischen zwei Messungen desselben Prozessors.
/// Ohne Fortschritt (gleiche Takte) 0.
pub fn auslastung(alt: &CpuZeiten, neu: &CpuZeiten) -> f64 {
    let gesamt = neu.gesamt().saturating_sub(alt.gesamt());
    if gesamt == 0 {
        return 0.0;
    }
    let leerlauf = neu.leerlauf().saturating_sub(alt.leerlauf());
    let aktiv = gesamt.saturating_sub(leerlauf);
    (aktiv as f64 * 100.0 / gesamt as f64).clamp(0.0, 100.0)
}

pub fn lese_stat() -> Option<Stat> {
    parse_stat(&std::fs::read_to_string("/proc/stat").ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BEISPIEL: &str = include_str!("../../tests/daten/stat");

    #[test]
    fn stat_beispiel_gesamt_und_kerne() {
        let stat = parse_stat(BEISPIEL).expect("cpu-Zeile");
        assert_eq!(stat.kerne.len(), 6, "sechs Kerne in der Beispieldatei");
        assert!(stat.gesamt.user > 0 && stat.gesamt.idle > 0);
        assert!(stat.btime > 1_700_000_000, "btime {}", stat.btime);
        // Die Summe der Kerne ist die Gesamtzeile (bis auf Rundung je Feld).
        let summe: u64 = stat.kerne.iter().map(CpuZeiten::gesamt).sum();
        let diff = summe.abs_diff(stat.gesamt.gesamt());
        assert!(diff <= 6 * 8, "Summe der Kerne weicht um {diff} Takte ab");
    }

    #[test]
    fn stat_feste_werte() {
        let text = "cpu  10 20 30 40 5 0 1 2 0 0\ncpu0 1 2 3 4 0 0 0 0 0 0\nintr 5 1\nctxt 9\nbtime 1700000000\n";
        let stat = parse_stat(text).unwrap();
        assert_eq!(
            stat.gesamt,
            CpuZeiten {
                user: 10,
                nice: 20,
                system: 30,
                idle: 40,
                iowait: 5,
                irq: 0,
                softirq: 1,
                steal: 2
            }
        );
        assert_eq!(stat.gesamt.gesamt(), 108);
        assert_eq!(stat.gesamt.leerlauf(), 45);
        assert_eq!(stat.kerne.len(), 1);
        assert_eq!(stat.btime, 1_700_000_000);
    }

    #[test]
    fn stat_ohne_cpu_zeile_ist_none() {
        assert!(parse_stat("intr 1\nbtime 5\n").is_none());
        assert!(parse_stat("").is_none());
    }

    #[test]
    fn auslastung_prozent() {
        let alt = CpuZeiten {
            user: 100,
            idle: 900,
            ..Default::default()
        };
        let neu = CpuZeiten {
            user: 150,
            idle: 950,
            ..Default::default()
        };
        assert!((auslastung(&alt, &neu) - 50.0).abs() < 1e-9);
        let voll = CpuZeiten {
            user: 300,
            idle: 900,
            ..Default::default()
        };
        assert!((auslastung(&alt, &voll) - 100.0).abs() < 1e-9);
        let nur_leerlauf = CpuZeiten {
            user: 100,
            idle: 1000,
            iowait: 10,
            ..Default::default()
        };
        assert_eq!(auslastung(&alt, &nur_leerlauf), 0.0);
        assert_eq!(auslastung(&alt, &alt), 0.0, "kein Fortschritt");
        assert_eq!(auslastung(&neu, &alt), 0.0, "rückwärts zählt nicht");
    }
}
