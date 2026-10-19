#!/bin/bash
# Füllt integration/vorschlag/ aus dem Release-Build und trägt die Prüfsummen
# in integration/systemmonitor-integrieren.sh ein. Ändert nichts Live.
#   bash werkzeug/integration-vorbereiten.sh
set -euo pipefail
E=$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)
V=$E/integration/vorschlag
BIN=$E/target/release/querencia-systemmonitor
[ -x "$BIN" ] || { echo "Release-Binary fehlt: $BIN" >&2; exit 2; }
rm -rf "$V" && mkdir -p "$V/icons/hicolor"
cp "$BIN" "$V/querencia-systemmonitor"
for d in "$E"/data/icons/hicolor/*/; do
    g=$(basename "$d"); [ "$g" = 48x48 ] && continue
    mkdir -p "$V/icons/hicolor/$g/apps" && cp "$d"apps/org.querencia.Systemmonitor.* "$V/icons/hicolor/$g/apps/"
done
# Menüeintrag mit absolutem Programmpfad (Link in ~/opt/querencia/bin,
# anderswo über QUERENCIA_BIN)
ZIELBIN=${QUERENCIA_BIN:-$HOME/opt/querencia/bin}
sed -e "s#^Exec=querencia-systemmonitor#Exec=$ZIELBIN/querencia-systemmonitor#" \
    -e "s#^TryExec=querencia-systemmonitor#TryExec=$ZIELBIN/querencia-systemmonitor#" \
    -e '2i # Live-Vorschlag G2-S (Querencia-Werkstatt, 2026-10-18); Rückfall: bash ~/projects/querencia-systemmonitor/integration/systemmonitor-integrieren.sh zurueck' \
    "$E/data/org.querencia.Systemmonitor.desktop" >"$V/org.querencia.Systemmonitor.desktop"
desktop-file-validate "$V/org.querencia.Systemmonitor.desktop"
(cd "$V" && sha256sum querencia-systemmonitor org.querencia.Systemmonitor.desktop >SHA256SUMS && find icons -type f | LC_ALL=C sort | xargs sha256sum >>SHA256SUMS)
PROG=$(sha256sum "$V/querencia-systemmonitor" | cut -d' ' -f1)
DESK=$(sha256sum "$V/org.querencia.Systemmonitor.desktop" | cut -d' ' -f1)
S=$E/integration/systemmonitor-integrieren.sh
sed -i -e "s#^PROG_NEU=.*#PROG_NEU=$PROG#" -e "s#^DESK_NEU=.*#DESK_NEU=$DESK#" "$S"
bash -n "$S"
echo "Vorschlag: $V"; cat "$V/SHA256SUMS"
