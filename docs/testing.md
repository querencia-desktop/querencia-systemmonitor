# Testen

```bash
cargo fmt --all --check
cargo clippy --all-targets --offline -- -D warnings
python3 werkzeug/i18n.py pruefen         # Katalog vollständig, .pot aktuell
cargo test --offline                      # Tests ohne Display: /proc-Parser
                                          # mit Beispieldateien (tests/daten/),
                                          # Einheiten, CPU-Prozent, INI, sys.rs
```

GUI nur isoliert (eigener headless labwc 0.9.8, eigener D-Bus, eigenes
XDG-Profil, virtuelle Eingaben mit `qeingabe`; nie die laufende Sitzung):

```bash
cd werkzeug && gcc -O2 -o bin/qeingabe qeingabe.c bin/virtual-keyboard-unstable-v1-protocol.c \
    bin/wlr-virtual-pointer-unstable-v1-protocol.c -Ibin $(pkg-config --cflags --libs wayland-client xkbcommon) && \
    gcc -O2 -o bin/qausgabe qausgabe.c bin/wlr-output-management-unstable-v1-protocol.c -Ibin \
    $(pkg-config --cflags --libs wayland-client) && cd ..
bash werkzeug/gui-pruefung.sh            # szenario.sh gegen target/release, Bilder nach nachweis/gui/
bash werkzeug/gui-pruefung.sh szenario-rauch.sh debug
```

`qeingabe`/`qausgabe` verweigern außerhalb eines `qed-runtime-*`-Ordners.
Beendet werden im Szenario nur eigene `sleep`-Testprozesse. Nach einem
Abbruch: Reste unter `laufzeit/` (FUSE-Portal) löst `gui-pruefung.sh` beim
nächsten Lauf.

Integrationsskripte (nur Entwicklungsumgebung) haben eigene Proben unter
`integration/werkzeug/`.

Sprache: Die GUI-Prüfung läuft wie die Sitzung mit `LANG=en_US.UTF-8` und
`LANGUAGE=de` (`QSM_LANGUAGE=en` für Englisch). Ab 0.1.1 braucht die
geprüfte Binary den Katalog in `<Ordner der Binary>/share/locale`, sonst ist
sie englisch.
