//! `/proc/net/dev`: empfangene und gesendete Bytes je Schnittstelle.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schnittstelle {
    pub name: String,
    pub empfangen: u64,
    pub gesendet: u64,
}

/// Summe über alle Schnittstellen außer `lo`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Netz {
    pub empfangen: u64,
    pub gesendet: u64,
}

pub fn parse_net_dev(text: &str) -> Vec<Schnittstelle> {
    text.lines()
        .filter_map(|zeile| {
            let (name, rest) = zeile.split_once(':')?;
            let felder: Vec<&str> = rest.split_ascii_whitespace().collect();
            // 8 Empfangs- und 8 Sendefelder; Bytes stehen an Stelle 0 und 8.
            if felder.len() < 16 {
                return None;
            }
            Some(Schnittstelle {
                name: name.trim().to_string(),
                empfangen: felder[0].parse().ok()?,
                gesendet: felder[8].parse().ok()?,
            })
        })
        .collect()
}

pub fn summe(schnittstellen: &[Schnittstelle]) -> Netz {
    schnittstellen
        .iter()
        .filter(|s| s.name != "lo")
        .fold(Netz::default(), |acc, s| Netz {
            empfangen: acc.empfangen.saturating_add(s.empfangen),
            gesendet: acc.gesendet.saturating_add(s.gesendet),
        })
}

/// Bytes je Sekunde aus zwei Zählerständen; Zählerrücksprung ergibt 0.
pub fn rate(alt: u64, neu: u64, sekunden: f64) -> f64 {
    if sekunden <= 0.0 || neu < alt {
        0.0
    } else {
        (neu - alt) as f64 / sekunden
    }
}

pub fn lese_netz() -> Option<Netz> {
    let text = std::fs::read_to_string("/proc/net/dev").ok()?;
    Some(summe(&parse_net_dev(&text)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn net_dev_beispiel() {
        let s = parse_net_dev(include_str!("../../tests/daten/net-dev"));
        assert_eq!(s.len(), 2, "lo und eth0");
        assert_eq!(s[0].name, "lo");
        assert_eq!(s[0].empfangen, 419_692);
        assert_eq!(s[1].name, "eth0");
        assert_eq!(s[1].empfangen, 116_599_530);
        assert_eq!(s[1].gesendet, 153_291_395);
        let n = summe(&s);
        assert_eq!(n.empfangen, 116_599_530, "lo zählt nicht mit");
        assert_eq!(n.gesendet, 153_291_395);
    }

    #[test]
    fn rate_pro_sekunde() {
        assert!((rate(1000, 3048, 2.0) - 1024.0).abs() < 1e-9);
        assert_eq!(rate(5000, 10, 1.0), 0.0, "Rücksprung");
        assert_eq!(rate(1, 2, 0.0), 0.0);
    }
}
