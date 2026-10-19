# Testen

```bash
cargo fmt --all --check
cargo clippy --all-targets --offline -- -D warnings
cargo test --offline                      # 32 Tests ohne Display: /proc-Parser
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

Integrationsskript in Umleitungspfaden: `bash werkzeug/integration-probe.sh`.
