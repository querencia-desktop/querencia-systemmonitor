#!/bin/bash
# Probe des Integrationsskripts in Umleitungspfaden (nichts Live):
# pruefen → aktivieren → pruefen → aktivieren (idempotent) → zurueck → pruefen.
#   bash werkzeug/integration-probe.sh   (Ergebnis nach nachweis/integration-probe.log)
set -u
E=$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)
P=${1:-$E/gui-test/integration-probe}
rm -rf "$P"; mkdir -p "$P/opt/bin" "$P/apps" "$P/icons" "$P/state"
export G2S_OPT=$P/opt G2S_APPS=$P/apps G2S_ICONS=$P/icons G2S_STATE=$P/state
S=$E/integration/systemmonitor-integrieren.sh
L=$E/nachweis/integration-probe.log; : >"$L"
n=0; f=0
lauf() { # Erwarteter Exit, Befehl
    local soll=$1; shift
    "$@" >>"$L" 2>&1; local ist=$?
    n=$((n + 1))
    if [ "$ist" = "$soll" ]; then echo "PASS $* (Exit $ist)" | tee -a "$L"; else echo "FAIL $* (Exit $ist, erwartet $soll)" | tee -a "$L"; f=$((f + 1)); fi
}
lauf 0 bash "$S" pruefen
lauf 0 bash "$S" aktivieren
[ -L "$P/opt/bin/querencia-systemmonitor" ] && [ -f "$P/apps/org.querencia.Systemmonitor.desktop" ] && [ -f "$P/icons/scalable/apps/org.querencia.Systemmonitor.svg" ] && [ ! -e "$P/icons/48x48" ] &&
    echo "PASS Link, Menüeintrag, Symbole ohne 48x48 angelegt" | tee -a "$L" || { echo "FAIL Dateien nach aktivieren" | tee -a "$L"; f=$((f + 1)); }
n=$((n + 1))
"$P/opt/bin/querencia-systemmonitor" --version >/dev/null 2>&1; echo "HINW Programm über Link aufrufbar (Exit $?)" | tee -a "$L"
lauf 0 bash "$S" pruefen
lauf 0 bash "$S" aktivieren
lauf 0 bash "$S" zurueck
[ ! -e "$P/opt/bin/querencia-systemmonitor" ] && [ ! -e "$P/apps/org.querencia.Systemmonitor.desktop" ] && [ -z "$(find "$P/icons" -type f)" ] && [ -d "$P"/opt/systemmonitor/* ] &&
    echo "PASS zurueck: alles entfernt, Ablage bleibt" | tee -a "$L" || { echo "FAIL Dateien nach zurueck" | tee -a "$L"; f=$((f + 1)); }
n=$((n + 1))
lauf 0 bash "$S" zurueck
# Fremder Zustand wird verweigert
echo fremd >"$P/apps/org.querencia.Systemmonitor.desktop"
lauf 2 bash "$S" aktivieren
rm -f "$P/apps/org.querencia.Systemmonitor.desktop"
grep -c . "$P/state/panel-rueckfall.txt" >/dev/null && echo "HINW Journal: $(wc -l <"$P/state/panel-rueckfall.txt") Einträge" | tee -a "$L"
echo "Ergebnis Integrationsprobe: $((n - f)) PASS, $f FAIL" | tee -a "$L"
