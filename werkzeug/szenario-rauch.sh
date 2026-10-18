#!/bin/bash
# Rauchtest IN der isolierten labwc-Sitzung: starten, Bild, Reiter, beenden.
set -u
. "$HMP04_PROFILE/env.sh"
mkdir -p "$OUT"
. "$(dirname "$0")/eingabe.sh"
shot() { grim "$OUT/$1.png"; }
"$SM_BIN" >"$OUT/rauch.log" 2>&1 &
sleep 4
q "move 1270 710" "wait 300"; shot rauch-01-prozesse
q "chord ctrl q" "wait 1500"
pgrep -f "^$SM_BIN" >/dev/null && echo "RAUCH: läuft noch" | tee "$OUT/rauch-ergebnis.txt" || echo "RAUCH: beendet" | tee "$OUT/rauch-ergebnis.txt"
cat "$OUT/rauch.log"
touch "$HMP04_PROFILE/logs/FERTIG"
