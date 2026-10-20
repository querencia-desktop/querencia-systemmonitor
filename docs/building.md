# Bauen

Ubuntu 26.04: `cargo`, `rustc` (≥ 1.92, Ubuntu 1.93.1), `pkgconf`,
`libgtk-4-dev`. Alle Crates liegen im lokalen Cargo-Cache (`--offline`).

```bash
cargo build --release --locked --offline
```

Auf der Entwicklungs-VM (viele parallele Sessions): gemeinsamer Build-Lock,
`CARGO_BUILD_JOBS` klein halten, unter `nice`, vorher `df -h /`, danach
`cargo clean`. Für Pakete und Anhänge mit
`RUSTFLAGS="--remap-path-prefix=$HOME=~"` bauen.

**Übersetzungen (ab 0.1.1):** Die Binary sucht
`<lang>/LC_MESSAGES/querencia-systemmonitor.mo` in
`<Ordner der Binary>/share/locale`, dann `<Ordner der Binary>/../share/locale`,
zuletzt `/usr/share/locale`:

```bash
msgfmt -c -o share/locale/de/LC_MESSAGES/querencia-systemmonitor.mo po/de.po
python3 werkzeug/i18n.py mo <ziel>
```

Ohne Katalog ist die Oberfläche englisch.

**Symbolgrößen:** bewusst **ohne** `48x48`. Liegt ein Symbol sowohl in
`48x48` als auch in einem Scalable-Verzeichnis, stürzt GTK 4.22 beim
Schließen von Fenstern ab (doppelt freigegebener xdg-toplevel-icon-Puffer,
geprüft im Oktober 2026). Das Fenstersymbol ist ein
eigener SVG-Name aus der eingebetteten Ressource
(`data/resource/systemmonitor.gresource`, neu erzeugen mit
`werkzeug/ressourcen.sh`).

**Integration (nur auf der Entwicklungs-VM, nicht Teil der Veröffentlichung):**
Skripte und Vorlagen unter `integration/` (feste Pfade der VM) stellen eine
Fassung in die Querencia-Sitzung der VM (unveränderliche Ablage
`~/opt/querencia/systemmonitor/<sha16>/`, Link, Starter). Der Ordner wird vor
der Veröffentlichung herausgefiltert (ENTSCHEIDUNGEN Nr. 13).
