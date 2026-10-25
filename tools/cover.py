#!/usr/bin/env python3
"""The repository's cover, cover.png, made from the guide's own pictures.

The studio card large on the left, five sorties around it: day, night, heat,
guidance and snow. Run tools/screens.py's capture first (see that script).

    python3 tools/cover.py
"""

from pathlib import Path

from screens import ROOT, read_png, write_png

SOURCE = ROOT / "target" / "screens"
GAP = 16
BACK = bytes((18, 18, 26))
FRAME = bytes((96, 104, 120))

# (picture, zoom, x, y)
# The bottom row lines up with the third tile on the right, and the card sits
# in the middle of the space above it.
ROW = GAP * 3 + 320
LAYOUT = [
    ("studio", 2, GAP, (ROW - 320) // 2),
    ("gate", 1, GAP * 2 + 480, GAP),
    ("night", 1, GAP * 2 + 480, GAP * 2 + 160),
    ("heat", 1, GAP * 2 + 480, ROW),
    ("guidance", 1, GAP, ROW),
    ("winter", 1, GAP * 2 + 240, ROW),
]


def main():
    width = GAP * 3 + 480 + 240
    height = GAP * 4 + 480
    canvas = [[BACK] * width for _ in range(height)]
    for name, zoom, ox, oy in LAYOUT:
        rows = read_png(SOURCE / f"{name}.png")
        h, w = len(rows) * zoom, len(rows[0]) * zoom
        for y in range(-1, h + 1):
            for x in range(-1, w + 1):
                if 0 <= y < h and 0 <= x < w:
                    canvas[oy + y][ox + x] = rows[y // zoom][x // zoom]
                else:
                    canvas[oy + y][ox + x] = FRAME
    write_png(ROOT / "cover.png", canvas)
    print(f"cover.png {width}x{height}")


if __name__ == "__main__":
    main()
