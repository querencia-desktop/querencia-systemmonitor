# Änderungen

## 0.1.2 – 2026-10-20

Dunkle Auswahlschrift (CSS-Welle, passend zum GTK-Thema Querencia-Klassisch,
Entscheidung 20.10.):

- Menüleiste, Menüeinträge und die ausgewählte Zeile der Prozessliste
  schreiben `#1a1a1a` statt `#ffffff` auf den orangen Verlauf. Kontrast
  6,2:1 (unten, `#f07746`) bis 8,5:1 (oben, `#f6a179`); mit Weiß waren es
  2,8:1 bzw. 2,0:1. Verläufe und alles Übrige unverändert; Texte und
  Katalog unverändert.

## 0.1.1 – 2026-10-20

Englisch und Deutsch über GNU gettext (G2-AH, Muster Querencia Editor 0.2.0):

- Texte englisch im Code, Deutsch aus `po/de.po` mit dem bisherigen
  Wortlaut: Menüs nach gnome-system-monitor 2.x (`_Monitor`, `_Edit`,
  `_View`, `_Help`), Reiter, Spalten, Statuszeile, Rückfragen, Dialoge
  „Change Priority“ und „Properties“, Prozesszustände, Fehlermeldungen,
  Ressourcen-Legenden, Desktop-Datei.
- Intervallmenü mit Pluralformen („1 Sekunde“, „2 Sekunden“).
- Datumsformat im Eigenschaften-Dialog je Sprache (de `TT.MM.JJJJ`,
  en `JJJJ-MM-TT`); Zahlen und Dezimalkomma unverändert.
- Enthält das eigene Programmsymbol (G2-V, unten), live seit 19.10.
- Doku nach ENTSCHEIDUNGEN Nr. 17 (`docs/building.md`, `docs/testing.md`,
  Arbeitsregeln), Integrationswerkzeuge nach
  `integration/werkzeug/`, Lizenzkopf (Nr. 14 Q5).

## unveröffentlicht – 2026-10-19 (G2-V)

Eigenes Programmsymbol statt des mit dem Editor byte-gleichen Logo-Platzhalters
(QD-034): Bildschirm mit orangefarbener Lastkurve, PNG 16–256 und SVG ohne
48x48; Fenstersymbol `org.querencia.Systemmonitor-fenster` (GResource) ist
dasselbe SVG.

## 0.1.0 – 2026-10-18 (G2-S)

Erste Fassung: Reiter Prozesse, Ressourcen, Dateisysteme; Beenden/Abwürgen
mit Rückfrage; Priorität; Eigenschaften; Einstellungen in
`~/.config/querencia/systemmonitor.ini`; isoliert geprüft.
