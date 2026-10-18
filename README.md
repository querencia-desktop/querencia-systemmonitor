# Querencia Systemmonitor

Systemüberwachung für Querencia Desktop nach dem Vorbild der GNOME-2-
Systemüberwachung, geschrieben in Rust mit GTK 4 (gtk-rs 0.11), ohne
libadwaita und ohne eigene Titelleiste – labwc zeichnet den klassischen
Rahmen. Alle Daten kommen direkt aus `/proc` und `statvfs`; keine
Zusatz-Crates außer `libc` für vier Systemaufrufe (`src/sys.rs`).

> **Status: isoliert geprüft (G2-S, 0.1.0), nicht installiert.** Live-
> Integration nur nach Freigabe des Besitzers über
> `integration/systemmonitor-integrieren.sh`.

## Funktionen

| | |
|---|---|
| Prozesse | Prozessname, Benutzer, PID, % CPU, Speicher (RSS), Nice, Zustand; Sortierung je Spalte, Filter Alle / Eigene / Aktive, Suche (Name, PID, Benutzer, Strg+F); Aktualisierung 1–10 s (Standard 2 s) ohne Flackern, Auswahl und Scrollposition bleiben |
| Eingriffe | Beenden (SIGTERM, Entf/Strg+E), Abwürgen (SIGKILL, Strg+K) mit Rückfrage und Prozessname; Priorität ändern (Strg+N; ohne Root nur senken, sonst Fehlermeldung); Eigenschaften (Alt+Eingabe, Doppelklick): Befehlszeile, Arbeitsverzeichnis, Startzeit, geöffnete Dateien. Fremde Prozesse nur anzeigen |
| Ressourcen | Verlauf der letzten 60 s: CPU gesamt und je Kern (Palette, Orange zuerst), Speicher und Auslagerung, Netzwerk empfangen/gesendet in KiB/s mit Summen; GtkDrawingArea/cairo, Messung jede Sekunde |
| Dateisysteme | Gerät, Einhängepunkt, Typ, Gesamt, Frei, Verfügbar, Belegt mit Balken; tmpfs/overlay/squashfs ausgeblendet, zuschaltbar |
| Rahmen | Menüs Überwachung · Bearbeiten · Ansicht · Hilfe, Kontextmenü, Statuszeile (CPU, Speicher, Prozesse), Info mit Querencia-Q; `~/.config/querencia/systemmonitor.ini` merkt Fenstergröße, Reiter, Intervall, Filter, Sortierung, Spalten |

App-ID `org.querencia.Systemmonitor`, Desktop-Datei `Name=Systemüberwachung`,
`Categories=GTK;System;Monitor;`. **Hinweis Panel:** Das Live-Panel zeigt im
Menü „System“ nur Einträge mit `Settings;`; mit diesen Kategorien erscheint
der Systemmonitor unter **Anwendungen → Systemwerkzeuge** (siehe Bericht).

## Bauen, testen

[BUILDING.md](BUILDING.md), [TESTING.md](TESTING.md).

## Lizenz

GPL-3.0-or-later ([LICENSE](LICENSE)). Eigener Code; aus der GNOME-
Systemüberwachung wurde kein Code übernommen. Logo: offizielles Querencia-
Logo C2 (`data/logo/`, Prüfsummen in `SHA256SUMS.herkunft`).
