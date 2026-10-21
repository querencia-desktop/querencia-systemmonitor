# Querencia Systemmonitor

Systemüberwachung für Querencia Desktop nach dem Vorbild der GNOME-2-
Systemüberwachung, geschrieben in Rust mit GTK 4 (gtk-rs 0.11), ohne
libadwaita und ohne eigene Titelleiste – labwc zeichnet den klassischen
Rahmen. Alle Daten kommen direkt aus `/proc` und `statvfs`; keine
Zusatz-Crates außer `libc` für vier Systemaufrufe (`src/sys.rs`).

> **Alpha.** Fremde Prozesse werden nur angezeigt. Keine Sicherheits- oder
> Datenverlustgarantie.

## Stand

| | |
|---|---|
| Version | 0.1.3 (Zweig `v0.1.3`, Integration `systemmonitor/0.1.3-int`; dunkeltauglich über Themenfarben, hell unverändert; live bis zur Freigabe 0.1.2) |
| Auf der Entwicklungs-VM | live seit 20.10.2026, nur über die Skripte unter `integration/` nach Freigabe |
| Letzte Prüfung | 21.10.2026 (0.1.3): fmt und Clippy sauber, 38/38 Unit-Tests, Integrationsprobe 128/0, isolierte GUI-Regression hell/dunkel 28 Läufe, 348/0 (Arbeitsordner `~/projects/querencia-systemmonitor-013/nachweis/`) |
| Getestet auf | Ubuntu 26.04, labwc 0.9.8, GTK 4.22 |

Änderungen je Version: [CHANGELOG.md](CHANGELOG.md).

## Funktionen

| | |
|---|---|
| Prozesse | Prozessname, Benutzer, PID, % CPU, Speicher (RSS), Nice, Zustand; Sortierung je Spalte, Filter Alle / Eigene / Aktive, Suche (Name, PID, Benutzer, Strg+F); Aktualisierung 1–10 s (Standard 2 s) ohne Flackern, Auswahl und Scrollposition bleiben |
| Eingriffe | Beenden (SIGTERM, Entf/Strg+E), Abwürgen (SIGKILL, Strg+K) mit Rückfrage und Prozessname; Priorität ändern (Strg+N; ohne Root nur senken, sonst Fehlermeldung); Eigenschaften (Alt+Eingabe, Doppelklick): Befehlszeile, Arbeitsverzeichnis, Startzeit, geöffnete Dateien. Fremde Prozesse nur anzeigen |
| Ressourcen | Verlauf der letzten 60 s: CPU gesamt und je Kern (Palette, Orange zuerst), Speicher und Auslagerung, Netzwerk empfangen/gesendet in KiB/s mit Summen; GtkDrawingArea/cairo, Messung jede Sekunde |
| Dateisysteme | Gerät, Einhängepunkt, Typ, Gesamt, Frei, Verfügbar, Belegt mit Balken; tmpfs/overlay/squashfs ausgeblendet, zuschaltbar |
| Rahmen | Menüs Überwachung · Bearbeiten · Ansicht · Hilfe, Kontextmenü, Statuszeile (CPU, Speicher, Prozesse), Info mit Querencia-Q; `~/.config/querencia/systemmonitor.ini` merkt Fenstergröße, Reiter, Intervall, Filter, Sortierung, Spalten |

App-ID `org.querencia.Systemmonitor`, Desktop-Datei `Name=System Monitor`
(`Name[de]=Systemüberwachung`),
`Categories=GTK;System;Monitor;`. **Hinweis Panel:** Das Live-Panel zeigt im
Menü „System“ nur Einträge mit `Settings;`; mit diesen Kategorien erscheint
der Systemmonitor unter **Anwendungen → Systemwerkzeuge** (siehe Bericht).

## Bauen und Prüfen

`cargo build --release --locked`, danach `cargo fmt --all --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked` und `python3 werkzeug/i18n.py pruefen`.
Voraussetzungen, Katalog, Symbole, Integration und die isolierte GUI-Prüfung: [docs/building.md](docs/building.md), [docs/testing.md](docs/testing.md).

## Sprachen

Seit 0.1.1 sind die Texte englisch im Code (msgid) und deutsch in
`po/de.po` (GNU gettext, Muster Querencia Editor 0.2.0); der deutsche
Wortlaut ist der bisherige, das Englische folgt gnome-system-monitor 2.x.
Die Sprache folgt `LANGUAGE`/`LANG`; ohne deutschen Katalog erscheint
Englisch. Zahlen behalten das Dezimalkomma. po-Pflege mit
`werkzeug/i18n.py` (`pot`, `update`, `desktop`, `mo`, `pruefen`).

## Lizenz

GPL-3.0-or-later ([LICENSE](LICENSE), reiner Lizenztext). Copyright 2026
Querencia Desktop contributors; Urheberhinweis in
[COPYRIGHT](COPYRIGHT). Eigener Code; aus der GNOME-
Systemüberwachung wurde kein Code übernommen. Logo: offizielles Querencia-
Logo C2 (`data/logo/`, Prüfsummen in `SHA256SUMS.herkunft`).

Eigene Vermerke: `werkzeug/virtual-keyboard-unstable-v1.xml` (gekürzte Kopie
des Wayland-Protokolls, Copyright Kristian Høgsberg u. a., MIT-artige Lizenz
im Dateikopf) und die mit `wayland-scanner` erzeugten Dateien unter
`werkzeug/bin/` (Lizenz der jeweiligen Protokolle); nur für das Prüfwerkzeug.

