//! Lesbare Einheiten, deutsch (Dezimalkomma), binäre Vielfache wie die
//! GNOME-Systemüberwachung (KiB, MiB, GiB).

const STUFEN: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];

/// Zahl mit `stellen` Nachkommastellen und Dezimalkomma.
pub fn zahl(wert: f64, stellen: usize) -> String {
    format!("{wert:.stellen$}").replace('.', ",")
}

/// „512 B“, „12,3 MiB“, „1,5 GiB“ – eine Nachkommastelle ab KiB.
pub fn bytes(wert: u64) -> String {
    let mut v = wert as f64;
    let mut stufe = 0;
    while v >= 1024.0 && stufe < STUFEN.len() - 1 {
        v /= 1024.0;
        stufe += 1;
    }
    if stufe == 0 {
        format!("{wert} B")
    } else {
        format!("{} {}", zahl(v, 1), STUFEN[stufe])
    }
}

/// Datenrate „12,3 KiB/s“; unter 1 KiB/s mit einer Nachkommastelle in KiB/s
/// (so bleibt die Spalte ruhig), darüber wie [`bytes`].
pub fn rate(bytes_pro_sekunde: f64) -> String {
    let v = bytes_pro_sekunde.max(0.0);
    if v < 1024.0 * 1024.0 {
        format!("{} KiB/s", zahl(v / 1024.0, 1))
    } else {
        format!("{}/s", bytes(v as u64))
    }
}

/// „12 %“ (gerundet).
pub fn prozent(wert: f64) -> String {
    format!("{} %", wert.round().clamp(0.0, f64::MAX) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_stufen() {
        assert_eq!(bytes(0), "0 B");
        assert_eq!(bytes(1023), "1023 B");
        assert_eq!(bytes(1024), "1,0 KiB");
        assert_eq!(bytes(1536), "1,5 KiB");
        assert_eq!(bytes(12 * 1024 * 1024 + 300 * 1024), "12,3 MiB");
        assert_eq!(bytes(16_302_152 * 1024), "15,5 GiB");
        assert_eq!(bytes(3 * 1024_u64.pow(4)), "3,0 TiB");
        assert_eq!(bytes(u64::MAX), "16384,0 PiB");
    }

    #[test]
    fn rate_und_prozent() {
        assert_eq!(rate(0.0), "0,0 KiB/s");
        assert_eq!(rate(512.0), "0,5 KiB/s");
        assert_eq!(rate(12.3 * 1024.0), "12,3 KiB/s");
        assert_eq!(rate(2.5 * 1024.0 * 1024.0), "2,5 MiB/s");
        assert_eq!(rate(-5.0), "0,0 KiB/s");
        assert_eq!(prozent(12.4), "12 %");
        assert_eq!(prozent(12.6), "13 %");
        assert_eq!(prozent(-1.0), "0 %");
        assert_eq!(zahl(1.25, 2), "1,25");
    }
}
