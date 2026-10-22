# Querencia Systemmonitor

Systemüberwachung für alle, die Querencia Desktop benutzen: Prozesse,
Ressourcenverlauf und Dateisysteme in einem Fenster. Teil von
[Querencia Desktop](https://querencialinux.org) – einer
klassischen Desktopumgebung im GNOME-2-Stil für Wayland (labwc, Rust, GTK 4).
Installiert wird es zusammen mit Querencia Desktop aus der APT-Quelle;
Anleitung auf <https://querencialinux.org>.

*English summary:* A system monitor (processes, resources, file systems)
modelled on the GNOME 2 System Monitor. Part of Querencia Desktop, a classic
GNOME-2-style desktop for Wayland. Alpha software; documentation is in German.

> **Alpha.** Fremde Prozesse werden nur angezeigt, eine Priorität lässt sich
> ohne Root nur senken. Keine Sicherheits- oder Datenverlustgarantie.

## Zweck

| | |
|---|---|
| Prozesse | Prozessname, Benutzer, PID, % CPU, Speicher (RSS), Nice, Zustand; Sortierung je Spalte, Filter Alle / Eigene / Aktive, Suche (Name, PID, Benutzer, Strg+F); Aktualisierung 1–10 s (Standard 2 s) ohne Flackern, Auswahl und Scrollposition bleiben |
| Eingriffe | Beenden (SIGTERM, Entf/Strg+E), Abwürgen (SIGKILL, Strg+K) mit Rückfrage und Prozessname; Priorität ändern (Strg+N; ohne Root nur senken); Eigenschaften (Alt+Eingabe, Doppelklick): Befehlszeile, Arbeitsverzeichnis, Startzeit, geöffnete Dateien. Fremde Prozesse nur anzeigen |
| Ressourcen | Verlauf der letzten 60 s: CPU gesamt und je Kern, Speicher und Auslagerung, Netzwerk empfangen/gesendet in KiB/s mit Summen; Messung jede Sekunde |
| Dateisysteme | Gerät, Einhängepunkt, Typ, Gesamt, Frei, Verfügbar, Belegt mit Balken; tmpfs/overlay/squashfs ausgeblendet, zuschaltbar |
| Rahmen | Menüs Überwachung · Bearbeiten · Ansicht · Hilfe, Kontextmenü, Statuszeile, Info-Dialog; `~/.config/querencia/systemmonitor.ini` merkt Fenstergröße, Reiter, Intervall, Filter, Sortierung, Spalten |

Alle Daten kommen direkt aus `/proc` und `statvfs`; außer gtk4-rs nur `libc`
für vier Systemaufrufe (`src/sys.rs`). Bewusst **nicht** Teil dieses
Programms: Dienste verwalten, Festplatten einhängen, Verlauf über die letzten
60 s hinaus.

Anwendungskennung (App-ID): `org.querencia.Systemmonitor` · Binary:
`querencia-systemmonitor` · Debian-Paket: `querencia-systemmonitor`

| Datei | Inhalt |
|---|---|
| `src/daten/` | Parser für `/proc` (CPU, Speicher, Netz, Prozesse, Einhängungen), Einheiten; Unit-Tests mit Beispieldateien aus `tests/daten/` |
| `src/prozessliste.rs`, `src/ressourcen.rs`, `src/dateisystemliste.rs` | die drei Reiter |
| `src/fenster.rs`, `src/dialoge.rs`, `src/einstellungen.rs` | Fenster, Menüs, Kürzel, Rückfragen, INI-Datei |
| `data/` | Desktop-Datei, Symbole (PNG 16–256 und SVG, ohne 48×48), Logo, eingebettete Ressource |
| `werkzeug/` | isolierte GUI-Prüfung (siehe docs/testing.md) |

## Stand

| | |
|---|---|
| Version | 0.1.5 (Tag `systemmonitor/0.1.5`; Info-Dialog mit Alpha-Hinweis, Website, Copyright und Lizenz, Hilfe-Menü mit Online-Hilfe und „Fehler melden“); dunkle Darstellung seit 0.1.3, Diagrammfarben hell mit mindestens 3:1 seit 0.1.4 |
| Letzte Prüfung | 22.10.2026 (0.1.5): fmt und Clippy sauber, 40 Unit-Tests, `i18n.py pruefen`; isoliert de/en 1024×768: Online-Hilfe, „Fehler melden“ (je über die Zugriffstaste) und Info-Dialog. 21.10.2026 (0.1.4): isolierte GUI-Regression gegen 0.1.3 hell und dunkel (8 Läufe, 96 Prüfpunkte), Kontrastmessung der Diagrammfarben |
| Bekannte Lücken | siehe „Zweck“ und „Alpha“ |
| Getestet auf | Ubuntu 26.04, labwc 0.9.8, GTK 4.22, Rust 1.93 |
| Nicht getestet | andere Distributionen, HiDPI, Mehrmonitor |

Änderungen: [CHANGELOG.md](CHANGELOG.md).

## Bauen

Voraussetzungen (Ubuntu 26.04):

```bash
sudo apt install build-essential pkg-config rustc cargo libgtk-4-dev
```

```bash
cargo build --release --locked
```

Das Ergebnis liegt unter `target/release/querencia-systemmonitor`.
Ausführliche Hinweise (Toolchain-Version, Offline-Bau, Symbole):
[docs/building.md](docs/building.md).

## Prüfen

```bash
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
python3 werkzeug/i18n.py pruefen
```

Diese Befehle laufen ohne Bildschirm; die ersten drei sind auch die CI
(`.github/workflows/ci.yml`), die po-Prüfung braucht gettext und läuft lokal. Die **GUI-Prüfung** läuft in einer
abgeschotteten Sitzung (eigener labwc ohne Bildschirm, eigener D-Bus, eigenes
`XDG_RUNTIME_DIR` und XDG-Profil), niemals in der laufenden Desktopsitzung.
Die Eingabehilfen `qeingabe`/`qausgabe` werden vorher aus `werkzeug/`
übersetzt (Befehl in docs/testing.md):

```bash
bash werkzeug/gui-pruefung.sh
```

Fälle, Ablauf und Belege: [docs/testing.md](docs/testing.md). Arbeitsweise insgesamt:
[CONTRIBUTING.md](CONTRIBUTING.md).

## Sprachen

Die Texte sind englisch im Code (msgid) und deutsch in `po/de.po` (GNU
gettext); das Englische folgt gnome-system-monitor 2.x. Die Sprache folgt
`LANGUAGE`/`LANG`; ohne deutschen Katalog erscheint Englisch. Zahlen behalten
das Dezimalkomma. po-Pflege mit `werkzeug/i18n.py` (`pot`, `update`,
`desktop`, `mo`, `pruefen`).

## Vorbild und Abgrenzung

Vorbild für Bedienung und Aussehen ist **die Systemüberwachung aus GNOME 2**.
Querencia übernimmt davon die Bedienideen (Reiter, Menüs, Kürzel, Anordnung),
**keinen Quellcode, keine Texte, keine Grafiken**. Das Projekt ist mit der
GNOME-Systemüberwachung, GNOME und deren Autoren weder verbunden noch von
ihnen unterstützt.

Abweichungen vom Vorbild und Dinge, die bewusst fehlen:

- GTK 4 statt GTK 2, Fensterrahmen vom Compositor (labwc), kein libadwaita.
- Keine Dienste- und Laufwerksverwaltung.
- Das Querencia-Panel zeigt im Menü „System“ nur Einstellungen; der
  Systemmonitor erscheint unter **Anwendungen → Systemwerkzeuge**.

Fremdbestandteile: Rust-Abhängigkeiten (gtk4-rs und Umfeld, `libc`), Liste mit
`cargo metadata --locked`; dynamisch gelinkt: GTK 4 und seine
Systembibliotheken. In `werkzeug/bin/` liegen von `wayland-scanner` erzeugte
Protokolldateien für die Prüfwerkzeuge mit eigenem Vermerk:
`wlr-output-management-unstable-v1-*` (© 2019 Purism SPC, HPND-artige
Erlaubnis, siehe Dateikopf), `wlr-virtual-pointer-unstable-v1-*`
(© 2019 Josef Gajdusek, MIT, siehe Dateikopf) und
`virtual-keyboard-unstable-v1-*` (gekürzte Fassung des wlroots-Protokolls,
© 2008–2011 Kristian Høgsberg, © 2010–2013 Intel Corporation, © 2012–2013
Collabora, Ltd., © 2018 Purism SPC, MIT, siehe Dateikopf);
dieselbe Erlaubnis gilt für `werkzeug/virtual-keyboard-unstable-v1.xml`.

## Lizenz

Programm: GPL-3.0-or-later, siehe [LICENSE](LICENSE). Copyright 2026
Querencia Desktop contributors; Urheberhinweis in [COPYRIGHT](COPYRIGHT). Dateien mit eigenem Lizenzvermerk (die
Protokolldateien in `werkzeug/bin/`, siehe oben) behalten diesen.

Name und Logo „Querencia“ sind keine freien Inhalte: [MARKE.md](MARKE.md).

## Fehler melden

Fehler, Wünsche und Fragen zu Querencia gehen an die gemeinsame Meldestelle:
<https://github.com/querencia-desktop/querencia/issues/new/choose>.
Sicherheitslücken bitte nicht öffentlich melden, siehe [SECURITY.md](SECURITY.md).
