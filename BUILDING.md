# Bauen

Ubuntu 26.04: `cargo`, `rustc` (≥ 1.92, Ubuntu 1.93.1), `pkgconf`,
`libgtk-4-dev`. Alle Crates liegen im lokalen Cargo-Cache (`--offline`).

```bash
cargo build --release --locked --offline
```

Auf `entwicklungs-vm` (viele parallele Sessions): `CARGO_BUILD_JOBS=2`, unter
`nice`, vorher `df -h /`, danach `target/`-Zwischendateien aufräumen.

**Symbolgrößen:** bewusst **ohne** `48x48`. Liegt ein Symbol sowohl in
`48x48` als auch in einem Scalable-Verzeichnis, stürzt GTK 4.22 beim
Schließen von Fenstern ab (doppelt freigegebener xdg-toplevel-icon-Puffer,
). Das Fenstersymbol ist ein
eigener SVG-Name aus der eingebetteten Ressource
(`data/resource/systemmonitor.gresource`, neu erzeugen mit
`werkzeug/ressourcen.sh`).

Integrationsvorschlag (Binary, Prüfsummen, Menüeintrag, Symbole) erzeugt
`werkzeug/integration-vorbereiten.sh` nach `integration/vorschlag/`.
