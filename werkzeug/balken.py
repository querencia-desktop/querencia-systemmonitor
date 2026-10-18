#!/usr/bin/env python3
"""Oberkante des Bildlaufleisten-Griffs in einem PPM-Ausschnitt (grim -t ppm).
Aufruf: balken.py DATEI.ppm  -> gibt die Zeile (0-basiert) des ersten dunklen
Pixels aus, -1 wenn keines. Dunkel = Mittelwert der Kanäle < 200."""
import sys

def main() -> int:
    with open(sys.argv[1], "rb") as f:
        daten = f.read()
    # P6\n<w> <h>\n255\n
    teile = daten.split(maxsplit=4)
    w, h = int(teile[1]), int(teile[2])
    pixel = teile[4][-w * h * 3:]
    for y in range(h):
        zeile = pixel[y * w * 3:(y + 1) * w * 3]
        if any(sum(zeile[x * 3:x * 3 + 3]) / 3 < 200 for x in range(w)):
            print(y)
            return 0
    print(-1)
    return 0

if __name__ == "__main__":
    sys.exit(main())
