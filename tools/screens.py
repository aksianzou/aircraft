#!/usr/bin/env python3
"""Enlarge the guide's pictures from target/screens into docs/screens.

    cargo test --release --features screens   # draws them, 240x160
    python3 tools/screens.py                   # doubles them, no smoothing

The test runner writes a picture it does not have yet and reports a failure
for it; delete target/screens first to draw them all again.
"""

import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "target" / "screens"
TARGET = ROOT / "docs" / "screens"
ZOOM = 2


def read_png(path):
    data = path.read_bytes()
    pos, idat = 8, b""
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos : pos + 4])
        tag = data[pos + 4 : pos + 8]
        body = data[pos + 8 : pos + 8 + length]
        pos += 12 + length
        if tag == b"IHDR":
            width, height, depth, kind = struct.unpack(">IIBB", body[:10])
            assert depth == 8 and kind in (2, 6), "8-bit RGB or RGBA only"
        elif tag == b"IDAT":
            idat += body
    channels = 3 if kind == 2 else 4
    raw = zlib.decompress(idat)
    stride = width * channels
    rows, prev, i = [], bytearray(stride), 0
    for _ in range(height):
        kind_filter = raw[i]
        i += 1
        line = bytearray(raw[i : i + stride])
        i += stride
        for x in range(stride):
            a = line[x - channels] if x >= channels else 0
            b = prev[x]
            c = prev[x - channels] if x >= channels else 0
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
        rows.append([bytes(line[x * channels : x * channels + 3]) for x in range(width)])
        prev = line
    return rows


def write_png(path, rows):
    raw = b"".join(b"\x00" + b"".join(row) for row in rows)

    def chunk(tag, body):
        return struct.pack(">I", len(body)) + tag + body + struct.pack(">I", zlib.crc32(tag + body) & 0xFFFFFFFF)

    header = struct.pack(">IIBBBBB", len(rows[0]), len(rows), 8, 2, 0, 0, 0)
    path.write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


def main():
    TARGET.mkdir(parents=True, exist_ok=True)
    for source in sorted(SOURCE.glob("*.png")):
        rows = read_png(source)
        big = [[p for p in row for _ in range(ZOOM)] for row in rows for _ in range(ZOOM)]
        write_png(TARGET / source.name, big)
        print(source.name)


if __name__ == "__main__":
    main()
