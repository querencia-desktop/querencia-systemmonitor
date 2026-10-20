//! Die einzigen Systemaufrufe ohne sicheren Rust-Wrapper in der
//! Standardbibliothek: `sysconf`, `kill`, `getpriority`/`setpriority`,
//! `statvfs`. Nur hier ist `unsafe` erlaubt; jede Funktion prüft Argumente
//! und wertet `errno` aus.
#![allow(unsafe_code)]

use std::ffi::CString;
use std::io;
use std::mem::MaybeUninit;

use crate::i18n::gettext;

/// Seitengröße in Bytes (`_SC_PAGESIZE`), 4096 als Rückfall.
pub fn seitengroesse() -> u64 {
    // SAFETY: sysconf liest nur eine Konstante.
    let wert = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if wert > 0 { wert as u64 } else { 4096 }
}

/// Takte je Sekunde (`CLK_TCK`), 100 als Rückfall.
pub fn takt() -> u64 {
    // SAFETY: sysconf liest nur eine Konstante.
    let wert = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    if wert > 0 { wert as u64 } else { 100 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    /// SIGTERM: höflich beenden.
    Beenden,
    /// SIGKILL: abwürgen.
    Abwuergen,
}

impl Signal {
    fn nummer(self) -> libc::c_int {
        match self {
            Self::Beenden => libc::SIGTERM,
            Self::Abwuergen => libc::SIGKILL,
        }
    }
}

/// Fehlertext (übersetzt) zu einem Systemfehler.
pub fn fehlertext(err: &io::Error) -> String {
    match err.raw_os_error() {
        Some(libc::EPERM) => gettext("No permission for this process."),
        Some(libc::EACCES) => gettext("Access denied."),
        Some(libc::ESRCH) => gettext("The process no longer exists."),
        _ => err.to_string(),
    }
}

/// Schickt `signal` an genau einen Prozess (`pid` > 1; nie an Gruppen oder
/// „alle“, wie `kill(0)`/`kill(-1)` es täten).
pub fn signal(pid: i32, signal: Signal) -> Result<(), String> {
    if pid <= 1 {
        return Err(gettext("This process must not be ended."));
    }
    // SAFETY: kill hat keine Speicherargumente; pid ist geprüft positiv.
    let r = unsafe { libc::kill(pid, signal.nummer()) };
    if r == 0 {
        Ok(())
    } else {
        Err(fehlertext(&io::Error::last_os_error()))
    }
}

/// Nice-Wert eines Prozesses (−20 … 19).
pub fn prioritaet(pid: i32) -> Option<i32> {
    if pid <= 0 {
        return None;
    }
    // SAFETY: getpriority hat keine Speicherargumente. −1 kann ein gültiger
    // Wert sein, deshalb wird errno vorher gelöscht und danach geprüft.
    unsafe {
        *libc::__errno_location() = 0;
        let wert = libc::getpriority(libc::PRIO_PROCESS, pid as libc::id_t);
        if wert == -1 && *libc::__errno_location() != 0 {
            None
        } else {
            Some(wert)
        }
    }
}

/// Setzt den Nice-Wert. Ohne Root lässt der Kernel nur Erhöhungen zu
/// (niedrigere Priorität); der Fehler wird in der Oberflächensprache erklärt.
pub fn prioritaet_setzen(pid: i32, nice: i32) -> Result<(), String> {
    if pid <= 0 {
        return Err(gettext("Invalid process ID."));
    }
    let nice = nice.clamp(-20, 19);
    // SAFETY: setpriority hat keine Speicherargumente; Werte sind geprüft.
    let r = unsafe { libc::setpriority(libc::PRIO_PROCESS, pid as libc::id_t, nice) };
    if r == 0 {
        return Ok(());
    }
    let err = io::Error::last_os_error();
    Err(match err.raw_os_error() {
        Some(libc::EACCES) => {
            gettext("Without root privileges the priority can only be lowered (higher nice value).")
        }
        _ => fehlertext(&err),
    })
}

/// (gesamt, frei, verfügbar) in Bytes für den Einhängepunkt `pfad`.
pub fn statvfs(pfad: &str) -> Option<(u64, u64, u64)> {
    let c = CString::new(pfad).ok()?;
    let mut puffer = MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: c ist eine gültige C-Zeichenkette, puffer hat die richtige
    // Größe; bei Erfolg (0) ist er vollständig beschrieben.
    let r = unsafe { libc::statvfs(c.as_ptr(), puffer.as_mut_ptr()) };
    if r != 0 {
        return None;
    }
    // SAFETY: r == 0, also initialisiert.
    let s = unsafe { puffer.assume_init() };
    let block = s.f_frsize;
    Some((
        s.f_blocks.saturating_mul(block),
        s.f_bfree.saturating_mul(block),
        s.f_bavail.saturating_mul(block),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn konstanten() {
        assert!(seitengroesse() >= 4096);
        assert_eq!(takt(), 100, "Linux x86_64");
    }

    #[test]
    fn eigene_prioritaet() {
        // Der Testlauf kann selbst unter `nice` stehen: gegen /proc prüfen.
        let pid = std::process::id() as i32;
        let stat = std::fs::read_to_string("/proc/self/stat").unwrap();
        let erwartet: i32 = stat
            .rsplit(')')
            .next()
            .unwrap()
            .split_whitespace()
            .nth(16)
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(prioritaet(pid), Some(erwartet));
        assert_eq!(prioritaet(-5), None);
        assert_eq!(prioritaet(i32::MAX), None, "gibt es nicht");
    }

    #[test]
    fn prioritaet_erhoehen_verweigert() {
        // Prozess 1 gehört root: EPERM (oder EACCES), niemals Erfolg.
        let e = prioritaet_setzen(1, -5).unwrap_err();
        assert!(e.contains("permission") || e.contains("root"), "{e}");
        assert!(prioritaet_setzen(0, 0).is_err());
    }

    #[test]
    fn signal_schutz() {
        assert!(signal(0, Signal::Beenden).is_err(), "nie an die Gruppe");
        assert!(signal(-1, Signal::Abwuergen).is_err(), "nie an alle");
        assert!(signal(1, Signal::Beenden).is_err(), "nie an init");
        // Ein eigener Testprozess darf beendet werden.
        let mut kind = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .unwrap();
        signal(kind.id() as i32, Signal::Beenden).unwrap();
        let status = kind.wait().unwrap();
        assert!(!status.success());
        let e = signal(i32::MAX, Signal::Beenden).unwrap_err();
        assert!(e.contains("no longer exists"), "{e}");
    }

    #[test]
    fn statvfs_wurzel() {
        let (gesamt, frei, verf) = statvfs("/").unwrap();
        assert!(gesamt > frei && frei >= verf);
        assert!(statvfs("/gibt/es/nicht").is_none());
        assert!(statvfs("mit\0null").is_none());
    }
}
