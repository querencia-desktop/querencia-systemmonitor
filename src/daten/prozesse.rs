//! `/proc/<pid>/`: Prozessliste, CPU-Anteil, Eigenschaften.

use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

/// Felder aus `/proc/<pid>/stat` (Nummern wie in proc(5)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatZeile {
    pub pid: i32,
    /// `comm`, höchstens 15 Zeichen, ohne Klammern.
    pub comm: String,
    pub zustand: char,
    pub ppid: i32,
    pub utime: u64,
    pub stime: u64,
    pub nice: i32,
    pub threads: u32,
    /// Start in Takten nach dem Systemstart.
    pub starttime: u64,
    pub rss_seiten: u64,
}

/// Zerlegt eine `stat`-Zeile. Der Name steht in Klammern und darf Leerzeichen
/// und Klammern enthalten, deshalb wird an der letzten `)` getrennt.
pub fn parse_stat_zeile(text: &str) -> Option<StatZeile> {
    let auf = text.find('(')?;
    let zu = text.rfind(')')?;
    let pid = text[..auf].trim().parse().ok()?;
    let comm = text[auf + 1..zu].to_string();
    let rest: Vec<&str> = text[zu + 1..].split_ascii_whitespace().collect();
    // rest[0] ist Feld 3 (state); Feld n liegt bei rest[n - 3].
    let feld = |n: usize| rest.get(n - 3).copied();
    Some(StatZeile {
        pid,
        comm,
        zustand: feld(3)?.chars().next()?,
        ppid: feld(4)?.parse().ok()?,
        utime: feld(14)?.parse().ok()?,
        stime: feld(15)?.parse().ok()?,
        nice: feld(19)?.parse().ok()?,
        threads: feld(20)?.parse().ok()?,
        starttime: feld(22)?.parse().ok()?,
        rss_seiten: feld(24)?.parse().ok()?,
    })
}

/// Deutscher Zustandstext zu einem `stat`-Zustandsbuchstaben.
pub fn zustand_text(zustand: char) -> &'static str {
    match zustand {
        'R' => "Läuft",
        'S' => "Schläft",
        'D' => "Wartet (E/A)",
        'Z' => "Zombie",
        'T' => "Angehalten",
        't' => "Wird verfolgt",
        'I' => "Leerlauf",
        'X' | 'x' => "Beendet",
        _ => "Unbekannt",
    }
}

/// CPU-Anteil eines Prozesses in Prozent wie die GNOME-Systemüberwachung
/// (ohne „Solaris-Modus“): bezogen auf einen Kern, also bis `kerne` × 100.
pub fn cpu_prozent(prozess_takte: u64, gesamt_takte: u64, kerne: usize) -> f64 {
    if gesamt_takte == 0 {
        return 0.0;
    }
    let kerne = kerne.max(1) as f64;
    (prozess_takte as f64 * 100.0 * kerne / gesamt_takte as f64).clamp(0.0, 100.0 * kerne)
}

/// Anzeigename: `comm` ist auf 15 Zeichen gekürzt; wenn der Dateiname aus
/// der Befehlszeile damit beginnt und länger ist, gilt der volle Name.
pub fn prozessname(comm: &str, cmdline: Option<&str>) -> String {
    if let Some(erstes) = cmdline.and_then(|c| c.split('\0').next()) {
        let datei = erstes.rsplit('/').next().unwrap_or(erstes);
        if datei.len() > comm.len() && datei.starts_with(comm) {
            return datei.to_string();
        }
    }
    comm.to_string()
}

/// Startzeitpunkt in Sekunden seit der Epoche.
pub fn startzeit(starttime_takte: u64, btime: u64, takt: u64) -> u64 {
    btime + starttime_takte / takt.max(1)
}

/// Benutzernamen aus `/etc/passwd`.
#[derive(Debug, Default)]
pub struct Benutzer {
    namen: HashMap<u32, String>,
}

impl Benutzer {
    pub fn aus_passwd(text: &str) -> Self {
        let namen = text
            .lines()
            .filter_map(|z| {
                let mut f = z.split(':');
                let name = f.next()?;
                let uid = f.nth(1)?.parse().ok()?;
                Some((uid, name.to_string()))
            })
            .collect();
        Self { namen }
    }

    pub fn laden() -> Self {
        Self::aus_passwd(&fs::read_to_string("/etc/passwd").unwrap_or_default())
    }

    pub fn name(&self, uid: u32) -> String {
        self.namen
            .get(&uid)
            .cloned()
            .unwrap_or_else(|| uid.to_string())
    }
}

/// Ein Prozess, wie die Liste ihn zeigt.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Prozess {
    pub pid: i32,
    pub name: String,
    pub uid: u32,
    pub benutzer: String,
    pub zustand: char,
    /// utime + stime in Takten.
    pub takte: u64,
    pub nice: i32,
    pub rss: u64,
    pub starttime: u64,
    pub threads: u32,
}

/// Liest alle Prozesse (nur Hauptthreads, keine Threads unter `task/`).
/// Prozesse, die während des Lesens verschwinden, werden übergangen.
pub fn lese_prozesse(seitengroesse: u64, benutzer: &Benutzer) -> Vec<Prozess> {
    let Ok(eintraege) = fs::read_dir("/proc") else {
        return Vec::new();
    };
    let mut liste = Vec::with_capacity(512);
    for eintrag in eintraege.flatten() {
        let Some(pid) = eintrag
            .file_name()
            .to_str()
            .and_then(|n| n.parse::<i32>().ok())
        else {
            continue;
        };
        if let Some(p) = lese_prozess(pid, seitengroesse, benutzer) {
            liste.push(p);
        }
    }
    liste
}

fn lese_prozess(pid: i32, seitengroesse: u64, benutzer: &Benutzer) -> Option<Prozess> {
    let verzeichnis = Path::new("/proc").join(pid.to_string());
    let stat = parse_stat_zeile(&fs::read_to_string(verzeichnis.join("stat")).ok()?)?;
    let uid = fs::metadata(&verzeichnis).ok()?.uid();
    let cmdline = fs::read(verzeichnis.join("cmdline")).ok();
    let cmdline = cmdline.as_deref().and_then(|b| std::str::from_utf8(b).ok());
    Some(Prozess {
        pid,
        name: prozessname(&stat.comm, cmdline),
        uid,
        benutzer: benutzer.name(uid),
        zustand: stat.zustand,
        takte: stat.utime + stat.stime,
        nice: stat.nice,
        rss: stat.rss_seiten.saturating_mul(seitengroesse),
        starttime: stat.starttime,
        threads: stat.threads,
    })
}

/// Eigenschaften eines Prozesses für den Dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Eigenschaften {
    pub befehlszeile: String,
    pub arbeitsverzeichnis: Option<String>,
    /// Geöffnete Dateien (Ziel der Dateideskriptoren) oder der Grund,
    /// warum sie nicht lesbar sind.
    pub dateien: Result<Vec<String>, String>,
    pub start: Option<u64>,
}

/// `cmdline` ist NUL-getrennt; Kernelthreads haben keine.
pub fn befehlszeile(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let teile: Vec<&str> = text.split('\0').filter(|t| !t.is_empty()).collect();
    teile.join(" ")
}

pub fn lese_eigenschaften(pid: i32, btime: u64, takt: u64) -> Eigenschaften {
    let verzeichnis = Path::new("/proc").join(pid.to_string());
    let befehlszeile = fs::read(verzeichnis.join("cmdline"))
        .map(|b| befehlszeile(&b))
        .unwrap_or_default();
    let arbeitsverzeichnis = fs::read_link(verzeichnis.join("cwd"))
        .ok()
        .map(|p| p.to_string_lossy().into_owned());
    let dateien = match fs::read_dir(verzeichnis.join("fd")) {
        Ok(fds) => {
            let mut ziele: Vec<(u32, String)> = fds
                .flatten()
                .filter_map(|fd| {
                    let nummer = fd.file_name().to_str()?.parse::<u32>().ok()?;
                    let ziel = fs::read_link(fd.path()).ok()?;
                    Some((nummer, ziel.to_string_lossy().into_owned()))
                })
                .collect();
            ziele.sort_unstable();
            Ok(ziele
                .into_iter()
                .map(|(n, z)| format!("{n}: {z}"))
                .collect())
        }
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            Err("Keine Berechtigung (fremder Prozess)".to_string())
        }
        Err(e) => Err(e.to_string()),
    };
    let start = fs::read_to_string(verzeichnis.join("stat"))
        .ok()
        .and_then(|t| parse_stat_zeile(&t))
        .map(|s| startzeit(s.starttime, btime, takt));
    Eigenschaften {
        befehlszeile,
        arbeitsverzeichnis,
        dateien,
        start,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stat_zeile_beispiel() {
        let s = parse_stat_zeile(include_str!("../../tests/daten/self-stat")).unwrap();
        assert_eq!(s.comm, "cat");
        assert_eq!(s.zustand, 'R');
        assert_eq!(s.nice, 0);
        assert!(s.threads >= 1);
        assert!(s.starttime > 0 && s.rss_seiten > 0);
        let p1 = parse_stat_zeile(include_str!("../../tests/daten/pid1-stat")).unwrap();
        assert_eq!(p1.pid, 1);
        assert_eq!(p1.ppid, 0);
    }

    #[test]
    fn stat_zeile_mit_klammern_und_leerzeichen_im_namen() {
        let s = parse_stat_zeile(
            "42 (Web Content (x)) S 1 42 42 0 -1 4194560 10 0 0 0 7 3 0 0 20 5 2 0 1234 5000 300 184467",
        )
        .unwrap();
        assert_eq!(s.pid, 42);
        assert_eq!(s.comm, "Web Content (x)");
        assert_eq!(s.zustand, 'S');
        assert_eq!(s.ppid, 1);
        assert_eq!((s.utime, s.stime), (7, 3));
        assert_eq!(s.nice, 5);
        assert_eq!(s.threads, 2);
        assert_eq!(s.starttime, 1234);
        assert_eq!(s.rss_seiten, 300);
        assert!(parse_stat_zeile("kaputt").is_none());
        assert!(parse_stat_zeile("1 (zu) kurz").is_none());
    }

    #[test]
    fn zustandstexte() {
        assert_eq!(zustand_text('R'), "Läuft");
        assert_eq!(zustand_text('Z'), "Zombie");
        assert_eq!(zustand_text('?'), "Unbekannt");
    }

    #[test]
    fn cpu_prozent_wie_gnome() {
        // Ein Kern vollständig belegt bei sechs Kernen: 100 %, nicht 16,7 %.
        assert!((cpu_prozent(200, 1200, 6) - 100.0).abs() < 1e-9);
        assert!((cpu_prozent(100, 1200, 6) - 50.0).abs() < 1e-9);
        assert_eq!(cpu_prozent(5, 0, 6), 0.0);
        assert!((cpu_prozent(50, 100, 1) - 50.0).abs() < 1e-9);
        assert!(
            (cpu_prozent(9999, 100, 2) - 200.0).abs() < 1e-9,
            "Deckel bei Kerne × 100"
        );
    }

    #[test]
    fn namen_und_startzeit() {
        assert_eq!(
            prozessname("querencia-syste", Some("/opt/x/querencia-systemmonitor\0")),
            "querencia-systemmonitor"
        );
        assert_eq!(prozessname("bash", Some("/bin/bash\0-l\0")), "bash");
        assert_eq!(prozessname("kworker/0:1", None), "kworker/0:1");
        assert_eq!(
            prozessname("cat", Some("python3\0script.py\0")),
            "cat",
            "fremder Name zählt nicht"
        );
        assert_eq!(startzeit(755_331, 1_700_000_000, 100), 1_700_007_553);
        assert_eq!(startzeit(100, 5, 0), 105, "Takt 0 wird abgefangen");
        assert_eq!(befehlszeile(b"/bin/sleep\x00300\x00"), "/bin/sleep 300");
        assert_eq!(befehlszeile(b""), "");
    }

    #[test]
    fn benutzer_aus_passwd() {
        let b = Benutzer::aus_passwd(
            "root:x:0:0:root:/root:/bin/bash\nuser:x:1001:1001::~:/bin/bash\nkaputt\n",
        );
        assert_eq!(b.name(0), "root");
        assert_eq!(b.name(1001), "user");
        assert_eq!(b.name(4711), "4711");
    }

    #[test]
    fn eigener_prozess_lesbar() {
        let pid = std::process::id() as i32;
        let b = Benutzer::laden();
        let liste = lese_prozesse(4096, &b);
        let ich = liste
            .iter()
            .find(|p| p.pid == pid)
            .expect("eigener Prozess in der Liste");
        assert!(ich.rss > 0);
        assert_eq!(ich.uid, fs::metadata("/proc/self").unwrap().uid());
        let e = lese_eigenschaften(pid, 1_700_000_000, 100);
        assert!(e.befehlszeile.contains("querencia"), "{}", e.befehlszeile);
        assert!(e.arbeitsverzeichnis.is_some());
        assert!(e.dateien.as_ref().is_ok_and(|d| !d.is_empty()));
        assert!(e.start.is_some());
        let fremd = lese_eigenschaften(1, 1_700_000_000, 100);
        assert!(fremd.dateien.is_err(), "PID 1 gehört root");
    }
}
