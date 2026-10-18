#!/bin/bash
# Läuft IN der isolierten labwc-Sitzung (isoliert.py). Prüft den Systemmonitor
# mit virtuellen Eingaben (qeingabe), Bildschirmfotos (grim) und /proc.
# Fensterlage: 820 px breit, mittig auf dem 1280×720-Ausgang (labwc zentriert);
# die Höhe hängt vom Inhalt ab, deshalb wird die Lage gemessen (lage.py).
set -u
. "$HMP04_PROFILE/env.sh"
mkdir -p "$OUT"
R=$OUT/ergebnis.txt; : >"$R"
ok() { echo "PASS $*" | tee -a "$R"; }
nok() { echo "FAIL $*" | tee -a "$R"; }
blk() { echo "BLOCKED $*" | tee -a "$R"; }
. "$(dirname "$0")/eingabe.sh"
shot() { grim "$OUT/$1.png"; echo "--- $(date +%T.%N | cut -c1-12) Bild $1" >>"$LOG"; }
running() { pgrep -f "^$SM_BIN" >/dev/null; }
# Fensterlage messen: OBEN = Oberkante Titelleiste, UNTEN = Unterkante Fenster.
lage() {
    grim -t ppm -g "640,0 1x720" "$OUT/lage.ppm"
    read -r OBEN UNTEN < <(python3 -I "$(dirname "$0")/lage.py" "$OUT/lage.ppm")
    rm -f "$OUT/lage.ppm"
    MENU=$((OBEN + 42)); TABS=$((OBEN + 75)); KOPF=$((OBEN + 154)); ZEILE1=$((OBEN + 178)); SCHALTER=$((UNTEN - 42))
    echo "Fensterlage: oben $OBEN, unten $UNTEN (Menü y=$MENU, Reiter y=$TABS, Kopfzeile y=$KOPF, erste Zeile y=$ZEILE1)" | tee -a "$R"
}
INI=$XDG_CONFIG_HOME/querencia/systemmonitor.ini
LOG=$OUT/systemmonitor.log

# Eigener Testprozess, der beendet werden darf (Kind dieser Shell, isoliert).
sleep 12345 &
SLEEP=$!
sleep 7654 &
SLEEP2=$!

# 1 Start
"$SM_BIN" >"$LOG" 2>&1 &
sleep 4
running && ok "Start (Fenster 820 px breit, Reiter Prozesse)" || nok "Start"
q "move 1270 710" "wait 300"; shot 01-start
lage
[ "$OBEN" -gt 0 ] && [ "$UNTEN" -gt "$OBEN" ] && ok "Fensterlage gemessen" || nok "Fensterlage nicht messbar"

# 2 Sortieren: Kopfzeile PID (aufsteigend), dann noch einmal (absteigend), dann Speicher
q "move 660 $KOPF" "click" "wait 1500"; shot 02-sortiert-pid
q "click" "wait 1500"; shot 03-sortiert-pid-absteigend
q "move 815 $KOPF" "click" "wait 1500"; shot 04-sortiert-speicher
ok "Sortieren per Kopfzeile (Bilder 02–04; Sortierung wird in der INI geprüft)"

# 3 Filter über Menü Ansicht: Eigene Prozesse (Tastaturnavigation im Menü)
q "move 481 $MENU" "click" "wait 1200"; shot 05-menue-ansicht
# Nach dem Klick ist der erste Eintrag fokussiert: ein Down = „Eigene Prozesse“
q "key Down" "wait 300"; shot 06-menue-ansicht-eigene
q "key Return" "wait 1500"; shot 07-filter-eigene
ok "Filter „Eigene Prozesse“ gewählt (Bild 07; Filter wird in der INI geprüft)"

# 4 Suchen: Strg+F, PID des Testprozesses eintippen
q "chord ctrl f" "wait 500"
for z in $(echo "$SLEEP" | sed 's/./& /g'); do q "key $z"; done
q "wait 1500"; shot 08-suche-pid
ok "Suche nach PID $SLEEP (Bild 08)"

# 5 Testprozess beenden: Zeile anklicken, Entf -> Rückfrage; erst Abbrechen, dann Beenden
q "move 400 $ZEILE1" "click" "wait 600" "key Delete" "wait 1200"; shot 09-rueckfrage-beenden
q "key Escape" "wait 800"
kill -0 "$SLEEP" 2>/dev/null && ok "Rückfrage abgebrochen: sleep $SLEEP lebt" || nok "Abbrechen hat den Prozess beendet"
q "key Delete" "wait 1200" "key Return" "wait 1500"; shot 10-nach-beenden
if kill -0 "$SLEEP" 2>/dev/null; then nok "sleep $SLEEP lebt nach Beenden"; kill -9 "$SLEEP"; else ok "Testprozess sleep $SLEEP mit Rückfrage beendet (SIGTERM)"; fi
wait "$SLEEP" 2>/dev/null
running && ok "Programm läuft nach dem Beenden-Dialog weiter" || nok "Programm nach Dialog weg"

# 6 Zweiter Testprozess: Eigenschaften (Alt+Return), Priorität (Strg+N) erhöhen -> Fehler, senken -> ok
q "chord ctrl f" "chord ctrl a"
for z in $(echo "$SLEEP2" | sed 's/./& /g'); do q "key $z"; done
q "wait 1500" "move 400 $ZEILE1" "click" "wait 600" "chord alt Return" "wait 1500"; shot 11-eigenschaften
q "key Escape" "wait 800" "key Down" "wait 500"
running && ok "Eigenschaften-Dialog geöffnet und mit Tastatur geschlossen (Bild 11)" || nok "Absturz nach Eigenschaften-Dialog"
q "chord ctrl n" "wait 1200"; shot 12-prioritaet-dialog
q "key Tab" "key Left" "key Left" "key Left" "key Left" "key Left" "wait 300" "chord alt n" "wait 1200"; shot 13-prioritaet-erhoehen-fehler
q "key Escape" "wait 800" "key Right" "key Right" "key Right" "key Right" "key Right" "key Right" "key Right" "key Right" "key Right" "key Right" "wait 300" "chord alt n" "wait 1500"; shot 14-prioritaet-gesenkt
NICE=$(cut -d' ' -f19 "/proc/$SLEEP2/stat" 2>/dev/null)
[ "$NICE" = 5 ] && ok "Priorität gesenkt: Nice von sleep $SLEEP2 ist $NICE" || nok "Nice von sleep $SLEEP2 ist „$NICE“, erwartet 5"
running && ok "Programm läuft nach den Prioritätsdialogen weiter" || nok "Absturz nach Prioritätsdialog"
# Kontextmenü per Rechtsklick
q "move 400 $ZEILE1" "rclick" "wait 1200"; shot 15-kontextmenue
q "key Escape" "wait 500"
# Suche leeren, Filter zurück auf Alle (Ansicht -> erster Eintrag)
q "chord ctrl f" "chord ctrl a" "key BackSpace" "wait 500"
q "move 481 $MENU" "click" "wait 1000" "key Return" "wait 1200"
# Fremder Prozess (kthreadd, root): nur anzeigen, Knopf und Entf wirkungslos
q "chord ctrl f" "key k" "key t" "key h" "key r" "key e" "key a" "key d" "key d" "wait 1200" "move 400 $ZEILE1" "click" "wait 500" "key Delete" "wait 1000"; shot 15b-fremder-prozess
running && ok "Fremder Prozess ausgewählt: Entf ohne Wirkung, Programm läuft (Bild 15b)" || nok "Absturz bei fremdem Prozess"
q "chord ctrl f" "chord ctrl a" "key BackSpace" "wait 500"
# Scrollposition bleibt bei CPU-Sortierung stehen: 5 Raststufen nach unten, Griff zweimal messen
q "move 730 $KOPF" "click" "wait 800" "click" "wait 800" "move 640 400" "scroll 5" "wait 2500"
GRIFF="1038,$((OBEN + 170)) 3x$((UNTEN - OBEN - 260))"
grim -t ppm -g "$GRIFF" "$OUT/griff-a.ppm"; shot 15c-scroll-a
sleep 4.5
grim -t ppm -g "$GRIFF" "$OUT/griff-b.ppm"; shot 15d-scroll-b
GA=$(python3 -I "$(dirname "$0")/balken.py" "$OUT/griff-a.ppm"); GB=$(python3 -I "$(dirname "$0")/balken.py" "$OUT/griff-b.ppm")
# Toleranz 10 px: der Griff wandert minimal, wenn sich die Prozesszahl ändert.
if [ "$GA" -ge 0 ] && [ "$GB" -ge 0 ] && [ $((GA - GB)) -le 10 ] && [ $((GB - GA)) -le 10 ]; then ok "Scrollposition stabil bei CPU-Sortierung (Griff bei $GA/$GB px)"; else nok "Scrollposition driftet (Griff bei $GA/$GB px)"; fi
rm -f "$OUT/griff-a.ppm" "$OUT/griff-b.ppm"
q "move 380 $KOPF" "click" "wait 800"

# 7 Reiter Ressourcen: zwei Bilder im Abstand, Graphen laufen
q "move 390 $TABS" "click" "wait 2500"; shot 16-ressourcen-a
sleep 6; shot 17-ressourcen-b
if cmp -s "$OUT/16-ressourcen-a.png" "$OUT/17-ressourcen-b.png"; then nok "Graphen unverändert nach 6 s"; else ok "Graphen laufen (Bilder 16/17 unterscheiden sich)"; fi

# 8 Reiter Dateisysteme, Schalter „Alle Dateisysteme“
q "move 515 $TABS" "click" "wait 1500"; shot 18-dateisysteme
q "move 420 $SCHALTER" "click" "wait 1500"; shot 19-dateisysteme-alle
[ "$(grep -c . /proc/mounts)" -gt 3 ] && ok "Dateisysteme: Schalter „Alle“ (Bild 19 zeigt tmpfs/squashfs)" || true
q "click" "wait 800"
ok "Dateisysteme angezeigt (Bild 18: nur Geräte)"

# 9 Info-Dialog über Menü Hilfe, mit Tastaturfokus schließen, danach Taste (Symbolabsturz-Muster)
q "move 542 $MENU" "click" "wait 1000" "key Return" "wait 1200"; shot 20-info
q "key Escape" "wait 800" "key Down" "key Down" "wait 800"
running && ok "Info-Dialog geöffnet und mit Escape geschlossen, Taste danach: kein Absturz (Bild 20)" || nok "Absturz nach Info-Dialog"
# Info noch einmal per Maus schließen (Schließen-Knopf hat den Fokus: Return)
q "move 542 $MENU" "click" "wait 1000" "key Return" "wait 1200" "key Return" "wait 800" "key Down" "wait 500"
running && ok "Info-Dialog mit Return geschlossen: kein Absturz" || nok "Absturz nach Info-Dialog (Return)"

# 10 Zurück auf Prozesse, Sortierung CPU (für die INI-Prüfung), dann Spalte Nice ausblenden (Ansicht -> Spalten -> Nice)
q "move 282 $TABS" "click" "wait 800" "move 730 $KOPF" "click" "wait 1200"; shot 21-sortiert-cpu
q "move 481 $MENU" "click" "wait 1000" "key Down" "key Down" "key Down" "key Down" "key Right" "wait 600"; shot 22-menue-spalten
q "key Down" "key Down" "key Down" "key Down" "key Return" "wait 1200"; shot 23-ohne-nice

# 11 Beenden mit Strg+Q, INI prüfen
q "chord ctrl q" "wait 2000"
running && nok "Programm läuft nach Strg+Q weiter" || ok "Beenden mit Strg+Q"
if [ -f "$INI" ]; then
    cp "$INI" "$OUT/systemmonitor-nach-lauf.ini"
    grep -q '^Sortierung=cpu' "$INI" && grep -q '^Filter=alle' "$INI" && grep -q '^Spalten=' "$INI" && ! grep '^Spalten=' "$INI" | grep -q 'nice' &&
        ok "INI geschrieben: Sortierung=cpu, Filter=alle, Spalte Nice ausgeblendet ($INI)" || nok "INI-Inhalt unerwartet: $(tr '\n' ' ' <"$INI")"
else nok "keine INI unter $INI"; fi

# 12 Fenstergrößen 1024×768 und 1920×1080 (Ausgang umstellen, Größe in der INI)
for groesse in 1024x768 1920x1080; do
    B=${groesse%x*}; H=${groesse#*x}
    "$QA" "$groesse" >>"$OUT/qausgabe.log" 2>&1; sleep 1
    printf '[Fenster]\nBreite=%s\nHoehe=%s\nMaximiert=false\n' "$B" "$H" >"$INI"
    "$SM_BIN" >>"$LOG" 2>&1 &
    sleep 4
    q "move 10 10" "wait 300"; shot "24-fenster-$groesse"
    q "move $((B/2)) 105" "wait 200"
    running && ok "Fenster $groesse gestartet (Bild 24-$groesse)" || nok "Start bei $groesse"
    q "chord ctrl q" "wait 2000"
done
"$QA" 1280x720 >>"$OUT/qausgabe.log" 2>&1

# 13 Zweite Instanz holt das Fenster nach vorn (eine Instanz)
"$SM_BIN" >>"$LOG" 2>&1 &
sleep 3
"$SM_BIN" >>"$LOG" 2>&1
sleep 1
[ "$(pgrep -c -f "^$SM_BIN")" = 1 ] && ok "eine Instanz trotz zweitem Aufruf" || nok "mehrere Instanzen"
q "chord ctrl q" "wait 1500"

kill "$SLEEP2" 2>/dev/null; wait "$SLEEP2" 2>/dev/null
grep -i -E 'critical|Gtk-WARNING' "$LOG" | head -20 >"$OUT/gtk-meldungen.txt"
[ -s "$OUT/gtk-meldungen.txt" ] && echo "HINW GTK-Meldungen: $(wc -l <"$OUT/gtk-meldungen.txt")" | tee -a "$R"
echo "Ergebnis: $(grep -c '^PASS' "$R") PASS, $(grep -c '^FAIL' "$R") FAIL, $(grep -c '^BLOCKED' "$R") BLOCKED" | tee -a "$R"
touch "$HMP04_PROFILE/logs/FERTIG"
