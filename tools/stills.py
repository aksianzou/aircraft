#!/usr/bin/env python3
"""The briefing pictures of the stage cards, cut from the first thirty.

gfx/stills.png is one 64x64 picture per card, in card-number order, all in
the same fifteen colours. A stage card shows both of its stages: the first
picture above a diagonal, the second below it, split by a pale line. A scout
stage puts the corners of its marks on the first half, as the glass draws
them. Only colours the sheet already has are used, so the cartridge's
palettes do not change.

    python3 tools/stills.py

It always starts again from the first thirty, so running it twice is safe.
"""

import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SHEET = ROOT / "gfx" / "stills.png"
SIDE = 64
OLD = 30

# Card 31 to 40 on the card number line, as (first stage, second stage,
# scout marks on the first). The numbers are pictures of the first thirty.
CARDS = [
    (4, 3, False),  # the convoy's truck, then its house
    (21, 5, True),  # three marks, then the night trucks
    (18, 9, False),  # the guns, then the tanks
    (15, 10, True),  # three marks, then the bunkers
    (18, 5, False),  # the guns, then the trucks at night
    (7, 13, False),  # parked trucks, then the crews running
    (20, 9, False),  # one tank, then two more
    (26, 29, True),  # three marks in the snow, then the men
    (27, 28, False),  # the snow road, then the tanks on the mud
    (21, 14, False),  # the mine field, then the tank at night
]

# Where the corners sit on a scout half, in pixels of the picture.
MARKS = [(12, 14), (34, 8), (10, 36)]


def read_rgba(path):
    data = path.read_bytes()
    pos, idat = 8, b""
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos : pos + 4])
        tag = data[pos + 4 : pos + 8]
        body = data[pos + 8 : pos + 8 + length]
        pos += 12 + length
        if tag == b"IHDR":
            width, height, depth, kind = struct.unpack(">IIBB", body[:10])
            assert depth == 8 and kind == 6, "8-bit RGBA only"
        elif tag == b"IDAT":
            idat += body
    raw = zlib.decompress(idat)
    stride = width * 4
    rows, prev, i = [], bytearray(stride), 0
    for _ in range(height):
        kind_filter = raw[i]
        i += 1
        line = bytearray(raw[i : i + stride])
        i += stride
        for x in range(stride):
            a = line[x - 4] if x >= 4 else 0
            b = prev[x]
            c = prev[x - 4] if x >= 4 else 0
            if kind_filter == 1:
                line[x] = (line[x] + a) & 255
            elif kind_filter == 2:
                line[x] = (line[x] + b) & 255
            elif kind_filter == 3:
                line[x] = (line[x] + (a + b) // 2) & 255
            elif kind_filter == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[x] = (line[x] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append([bytes(line[x * 4 : x * 4 + 4]) for x in range(width)])
        prev = line
    return rows


def write_rgba(path, rows):
    raw = b"".join(b"\x00" + b"".join(row) for row in rows)

    def chunk(tag, body):
        return struct.pack(">I", len(body)) + tag + body + struct.pack(">I", zlib.crc32(tag + body) & 0xFFFFFFFF)

    header = struct.pack(">IIBBBBB", len(rows[0]), len(rows), 8, 6, 0, 0, 0)
    path.write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


def brightness(pixel):
    return pixel[0] * 3 + pixel[1] * 6 + pixel[2]


def main():
    rows = read_rgba(SHEET)
    assert len(rows) == SIDE and len(rows[0]) >= OLD * SIDE, "the first thirty are missing"
    rows = [row[: OLD * SIDE] for row in rows]
    colours = {pixel for row in rows for pixel in row if pixel[3] == 255}
    pale = max(colours, key=brightness)
    dark = min(colours, key=brightness)

    def frame(index):
        return [row[index * SIDE : (index + 1) * SIDE] for row in rows]

    for first, second, scout in CARDS:
        top, bottom = frame(first), frame(second)
        still = []
        for y in range(SIDE):
            line = []
            for x in range(SIDE):
                # The line runs from the top right to the bottom left.
                edge = x + y - (SIDE - 1)
                if edge == 0:
                    line.append(pale)
                elif edge == 1:
                    line.append(dark)
                elif edge < 0:
                    line.append(top[y][x])
                else:
                    line.append(bottom[y][x])
            still.append(line)
        if scout:
            for mx, my in MARKS:
                for cx, cy, sx, sy in ((mx, my, 1, 1), (mx + 8, my, -1, 1), (mx, my + 8, 1, -1), (mx + 8, my + 8, -1, -1)):
                    for step in range(3):
                        still[cy][cx + sx * step] = pale
                        still[cy + sy * step][cx] = pale
        for y in range(SIDE):
            rows[y].extend(still[y])
    write_rgba(SHEET, rows)
    print(f"stills.png {len(rows[0]) // SIDE} pictures")


if __name__ == "__main__":
    main()
