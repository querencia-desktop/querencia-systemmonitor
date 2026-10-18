#!/usr/bin/env python3
"""Fensterlage im isolierten labwc: liest eine 1 Pixel breite PPM-Spalte
(grim -t ppm -g "X,0 1xH") und gibt "OBEN UNTEN" aus – erste und letzte
Zeile, die nicht Hintergrund (schwarz) ist."""
import sys

def main() -> int:
    with open(sys.argv[1], "rb") as f:
        daten = f.read()
    teile = daten.split(maxsplit=4)
    w, h = int(teile[1]), int(teile[2])
    pixel = teile[4][-w * h * 3:]
    zeilen = [y for y in range(h) if sum(pixel[y * w * 3:(y + 1) * w * 3]) > 60 * w]
    if not zeilen:
        print("-1 -1")
        return 1
    print(zeilen[0], zeilen[-1])
    return 0

if __name__ == "__main__":
    sys.exit(main())
