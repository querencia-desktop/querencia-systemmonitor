#!/bin/bash
# Isolierte GUI-Prüfung des Querencia Systemmonitors (G2-S). Nur Wegwerfprofil,
# eigener headless labwc, eigener D-Bus; die laufende Sitzung bleibt unberührt.
#   bash werkzeug/gui-pruefung.sh [szenario.sh] [release|debug]
set -euo pipefail
E=$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)
SZ=${1:-szenario.sh}
PROFILART=${2:-release}
PROFIL=$E/gui-test/profil
[ -d "$E/gui-test" ] && [ "$(dirname "$PROFIL")" = "$E/gui-test" ] && rm -rf "$PROFIL"
mkdir -p "$PROFIL/data/applications" "$E/nachweis/gui" "$E/laufzeit"
cp "$E/data/org.querencia.Systemmonitor.desktop" "$PROFIL/data/applications/"
cp -r "$E/data/icons" "$PROFIL/data/icons"
SM_BIN=$E/target/$PROFILART/querencia-systemmonitor
[ -x "$SM_BIN" ] || { echo "Binary fehlt: $SM_BIN" >&2; exit 2; }
cat >"$PROFIL/env.sh" <<X
SM_BIN=$SM_BIN
Q=$E/werkzeug/bin/qeingabe
QA=$E/werkzeug/bin/qausgabe
OUT=$E/nachweis/gui
X
python3 -I "$E/werkzeug/isoliert.py" "$PROFIL" "$E/werkzeug/$SZ" "${SM_ZEITLIMIT:-300}"
cp "$PROFIL"/logs/*.log "$E/nachweis/gui/" 2>/dev/null || true
# Reste eines abgebrochenen Laufs: FUSE-Einhängepunkt des Dokumentportals lösen
for r in "$E"/laufzeit/qed-runtime-*; do
    [ -d "$r" ] || continue
    fusermount3 -u -z "$r/doc" 2>/dev/null || true
    rm -rf "$r"
done
[ -f "$E/nachweis/gui/ergebnis.txt" ] && cat "$E/nachweis/gui/ergebnis.txt" || true
