//! `/proc/mounts` und `statvfs`: eingehängte Dateisysteme mit Belegung.

use crate::sys;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Einhaengung {
    pub geraet: String,
    pub pfad: String,
    pub typ: String,
}

/// Entschlüsselt die Oktal-Escapes von `/proc/mounts` (`\040` = Leerzeichen).
pub fn entschluesseln(feld: &str) -> String {
    let mut aus = String::with_capacity(feld.len());
    let mut zeichen = feld.chars().peekable();
    while let Some(c) = zeichen.next() {
        if c == '\\' {
            let oktal: String = zeichen.clone().take(3).collect();
            if oktal.len() == 3
                && oktal.chars().all(|d| ('0'..='7').contains(&d))
                && let Some(wert) = u8::from_str_radix(&oktal, 8).ok().map(char::from)
            {
                aus.push(wert);
                zeichen.nth(2);
                continue;
            }
        }
        aus.push(c);
    }
    aus
}

pub fn parse_mounts(text: &str) -> Vec<Einhaengung> {
    text.lines()
        .filter_map(|zeile| {
            let mut f = zeile.split_ascii_whitespace();
            Some(Einhaengung {
                geraet: entschluesseln(f.next()?),
                pfad: entschluesseln(f.next()?),
                typ: f.next()?.to_string(),
            })
        })
        .collect()
}

/// Kernel-Pseudodateisysteme ohne Speicherplatz; nie angezeigt.
pub fn ist_pseudo(typ: &str) -> bool {
    matches!(
        typ,
        "proc"
            | "sysfs"
            | "devpts"
            | "cgroup"
            | "cgroup2"
            | "pstore"
            | "bpf"
            | "tracefs"
            | "debugfs"
            | "securityfs"
            | "configfs"
            | "hugetlbfs"
            | "mqueue"
            | "binfmt_misc"
            | "autofs"
            | "efivarfs"
            | "fusectl"
            | "rpc_pipefs"
            | "nsfs"
            | "fuse.portal"
            | "fuse.gvfsd-fuse"
            | "selinuxfs"
            | "none"
    )
}

/// Flüchtige und Overlay-Dateisysteme (tmpfs, overlay, squashfs …): nur mit
/// „Alle Dateisysteme anzeigen“ sichtbar.
pub fn ist_fluechtig(typ: &str) -> bool {
    matches!(
        typ,
        "tmpfs" | "devtmpfs" | "ramfs" | "overlay" | "squashfs" | "fuse.snapfuse"
    )
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Dateisystem {
    pub geraet: String,
    pub pfad: String,
    pub typ: String,
    pub gesamt: u64,
    pub frei: u64,
    pub verfuegbar: u64,
}

impl Dateisystem {
    pub fn belegt(&self) -> u64 {
        self.gesamt.saturating_sub(self.frei)
    }

    /// Anteil 0–1 wie `df`: belegt / (belegt + verfügbar), also ohne die
    /// für root reservierten Blöcke.
    pub fn belegt_anteil(&self) -> f64 {
        belegt_anteil(self.belegt(), self.verfuegbar)
    }
}

pub fn belegt_anteil(belegt: u64, verfuegbar: u64) -> f64 {
    let nenner = belegt.saturating_add(verfuegbar);
    if nenner == 0 {
        0.0
    } else {
        (belegt as f64 / nenner as f64).clamp(0.0, 1.0)
    }
}

/// Auswahl der Einhängungen: ohne `alle` nur Geräte mit Speicherplatz, die
/// weder Pseudo- noch flüchtige Dateisysteme sind; mit `alle` auch tmpfs,
/// overlay und Co. Doppelte Einhängungen desselben Pfads (Bind-Mounts)
/// erscheinen einmal.
pub fn auswahl(einhaengungen: &[Einhaengung], alle: bool) -> Vec<&Einhaengung> {
    let mut gesehen = std::collections::HashSet::new();
    einhaengungen
        .iter()
        .filter(|e| !ist_pseudo(&e.typ))
        .filter(|e| alle || !ist_fluechtig(&e.typ))
        .filter(|e| gesehen.insert(e.pfad.as_str()))
        .collect()
}

pub fn lese_dateisysteme(alle: bool) -> Vec<Dateisystem> {
    let text = std::fs::read_to_string("/proc/mounts").unwrap_or_default();
    let einhaengungen = parse_mounts(&text);
    auswahl(&einhaengungen, alle)
        .into_iter()
        .filter_map(|e| {
            let (gesamt, frei, verfuegbar) = sys::statvfs(&e.pfad)?;
            if gesamt == 0 {
                return None;
            }
            Some(Dateisystem {
                geraet: e.geraet.clone(),
                pfad: e.pfad.clone(),
                typ: e.typ.clone(),
                gesamt,
                frei,
                verfuegbar,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mounts_beispiel() {
        let m = parse_mounts(include_str!("../../tests/daten/mounts"));
        assert!(m.len() > 10);
        let wurzel = m.iter().find(|e| e.pfad == "/").expect("Wurzel");
        assert_eq!(wurzel.geraet, "/dev/sda1");
        assert_eq!(wurzel.typ, "ext4");
        let sichtbar = auswahl(&m, false);
        assert!(sichtbar.iter().any(|e| e.pfad == "/"));
        assert!(
            sichtbar
                .iter()
                .all(|e| !ist_pseudo(&e.typ) && !ist_fluechtig(&e.typ))
        );
        let alle = auswahl(&m, true);
        assert!(alle.len() > sichtbar.len(), "tmpfs kommt dazu");
        assert!(alle.iter().any(|e| e.typ == "tmpfs"));
        assert!(alle.iter().all(|e| !ist_pseudo(&e.typ)));
    }

    #[test]
    fn escapes_und_doppelte() {
        assert_eq!(
            entschluesseln("/mnt/mein\\040Laufwerk"),
            "/mnt/mein Laufwerk"
        );
        assert_eq!(entschluesseln("/a\\011b\\\\c"), "/a\tb\\\\c");
        assert_eq!(entschluesseln("ohne"), "ohne");
        let m = parse_mounts(
            "/dev/sda1 / ext4 rw 0 0\n/dev/sda1 / ext4 rw 0 0\ntmpfs /run tmpfs rw 0 0\nproc /proc proc rw 0 0\nkaputt\n",
        );
        assert_eq!(auswahl(&m, false).len(), 1);
        assert_eq!(auswahl(&m, true).len(), 2);
    }

    #[test]
    fn belegung() {
        let fs = Dateisystem {
            geraet: "x".into(),
            pfad: "/".into(),
            typ: "ext4".into(),
            gesamt: 1000,
            frei: 400,
            verfuegbar: 350,
        };
        assert_eq!(fs.belegt(), 600);
        assert!((fs.belegt_anteil() - 600.0 / 950.0).abs() < 1e-9);
        assert_eq!(belegt_anteil(0, 0), 0.0);
        assert_eq!(belegt_anteil(5, 0), 1.0);
    }

    /// Mit `alle`: Im Container (CI) ist die Wurzel ein overlay und ohne
    /// „Alle Dateisysteme anzeigen“ ausgeblendet; die Auswahl selbst prüft
    /// `mounts_beispiel`.
    #[test]
    fn wurzel_lesbar() {
        let liste = lese_dateisysteme(true);
        let wurzel = liste
            .iter()
            .find(|d| d.pfad == "/")
            .expect("Wurzel über statvfs");
        assert!(wurzel.gesamt > wurzel.frei && wurzel.frei >= wurzel.verfuegbar);
    }
}
