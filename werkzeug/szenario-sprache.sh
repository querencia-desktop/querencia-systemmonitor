#!/bin/bash
# Läuft IN der isolierten labwc-Sitzung (isoliert.py). Sprache des
# Querencia Systemmonitors 0.1.1 (G2-AH) in LANGUAGE (gui-pruefung.sh
# QSM_LANGUAGE): eingeblendeter Katalog (/proc/<pid>/maps), Menüs (F10 und
# Pfeiltasten), Rückfrage „Beenden“ mit Ausführung, Eigenschaften, Priorität,
# Reiter Ressourcen und Dateisysteme (Zugriffstasten der Sprache). Nur Tastatur, damit die
# Bedienung nicht von der Breite der Beschriftungen abhängt. Auflösung aus
# QSM_GROESSE (Standard 1280x720).
set -u
. "$HMP04_PROFILE/env.sh"
mkdir -p "$OUT"
R=$OUT/ergebnis.txt; : >"$R"
ok() { echo "PASS $*" | tee -a "$R"; }
nok() { echo "FAIL $*" | tee -a "$R"; }
. "$(dirname "$0")/eingabe.sh"
SPRACHE=${LANGUAGE%%:*}
SPRACHE=${SPRACHE%%_*}
shot() { grim "$OUT/$1-$SPRACHE.png"; }
running() { pgrep -f "^$SM_BIN" >/dev/null; }
LOG=$OUT/systemmonitor.log
GROESSE=${QSM_GROESSE:-1280x720}
B=${GROESSE%x*}; H=${GROESSE#*x}
"$QA" "$GROESSE" >"$OUT/qausgabe.log" 2>&1 || true
sleep 1
mkdir -p "$XDG_CONFIG_HOME/querencia"
printf '[Fenster]\nBreite=%s\nHoehe=%s\nMaximiert=false\n' $((B * 3 / 4)) $((H * 3 / 4)) >"$XDG_CONFIG_HOME/querencia/systemmonitor.ini"
echo "INFO LANG=$LANG LANGUAGE=$LANGUAGE Ausgang $GROESSE" >>"$R"

sleep 23456 &
TEST=$!

"$SM_BIN" >"$LOG" 2>&1 &
sleep 4
running && ok "Start" || nok "Start"
PID=$(pgrep -f "^$SM_BIN" | head -1)
KAT=$(grep -o '[^ ]*querencia-systemmonitor.mo' "/proc/$PID/maps" | head -1)
echo "INFO Katalog: ${KAT:-keiner}" >>"$R"
case $SPRACHE in
de | it | es | fr) [ -n "$KAT" ] && ok "$SPRACHE: Katalog eingeblendet" || nok "$SPRACHE: kein Katalog eingeblendet" ;;
*) [ -z "$KAT" ] && ok "$SPRACHE: kein Katalog eingeblendet" || nok "$SPRACHE: Katalog eingeblendet" ;;
esac
q "move 1270 710" "wait 400"; shot s1-start

# Menüs der Reihe nach
q "key F10" "wait 900"; shot s2-menue-1
q "key Right" "wait 700"; shot s3-menue-2
q "key Right" "wait 700"; shot s4-menue-3
q "key Right" "wait 700"; shot s5-menue-4
q "key Escape" "wait 400" "key Escape" "wait 600"
running && ok "Menüs geöffnet (Bilder s2–s5)" || nok "Absturz bei den Menüs"

# Eigener Testprozess: suchen, Eigenschaften, Priorität, Beenden mit Rückfrage
q "chord ctrl f" "wait 400"
for z in $(echo "$TEST" | sed 's/./& /g'); do q "key $z"; done
# Erste Zeile anklicken: Lage wie in szenario.sh (Oberkante Titelleiste
# gemessen, erste Zeile 178 px darunter), Spalte in der Bildschirmmitte.
grim -t ppm -g "$((B / 2)),0 1x$H" "$OUT/lage.ppm"
read -r OBEN _ < <(python3 -I "$(dirname "$0")/lage.py" "$OUT/lage.ppm"); rm -f "$OUT/lage.ppm"
echo "INFO Fenster oben bei $OBEN" >>"$R"
# qeingabe rechnet Zeigerkoordinaten in 1280x720 (auf den Ausgang skaliert).
ZX=$(((B / 2) * 1280 / B)); ZY=$(((OBEN + 178) * 720 / H))
q "wait 1500" "move $ZX $ZY" "click" "wait 800"
q "chord alt Return" "wait 1500"; shot s6-eigenschaften
q "key Escape" "wait 800"
q "chord ctrl n" "wait 1200"; shot s7-prioritaet
q "key Escape" "wait 800"
q "key Delete" "wait 1200"; shot s8-rueckfrage-beenden
q "key Return" "wait 1500"
if kill -0 "$TEST" 2>/dev/null; then nok "Beenden über die Rückfrage ($SPRACHE): Testprozess lebt"; kill "$TEST"; else ok "Beenden über die Rückfrage ($SPRACHE): Testprozess beendet"; fi
wait "$TEST" 2>/dev/null

# Reiter Ressourcen und Dateisysteme über die Zugriffstasten der Sprache
case $SPRACHE in de) T_RES=r; T_DAT=d ;; es) T_RES=r; T_DAT=a ;; *) T_RES=r; T_DAT=f ;; esac
q "chord alt $T_RES" "wait 2500"; shot s9-ressourcen
q "chord alt $T_DAT" "wait 1500"; shot s10-dateisysteme
running && ok "Reiter Ressourcen und Dateisysteme (Bilder s9, s10)" || nok "Absturz bei den Reitern"

q "chord ctrl q" "wait 2000"
running && nok "Strg+Q beendet nicht" || ok "Strg+Q beendet"
echo "Ergebnis $GROESSE $SPRACHE: $(grep -c '^PASS' "$R") PASS, $(grep -c '^FAIL' "$R") FAIL" | tee -a "$R"
touch "$HMP04_PROFILE/logs/FERTIG"
