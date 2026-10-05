#!/usr/bin/env python3
"""Build the GBA tile and sprite sheets. Tile order must match src/main.rs."""

import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "gfx"


class Image:
    def __init__(self, width, height, color):
        self.width = width
        self.height = height
        self.pixels = [color for _ in range(width * height)]

    def putpixel(self, xy, color):
        x, y = xy
        self.pixels[y * self.width + x] = color

    def save(self, path):
        rows = []
        for y in range(self.height):
            row = bytearray()
            start = y * self.width
            for pixel in self.pixels[start : start + self.width]:
                row.extend(pixel)
            rows.append(bytes(row))
        write_png(Path(path), self.width, self.height, rows)


def write_png(path, width, height, rows):
    def chunk(tag, data):
        crc = zlib.crc32(tag + data) & 0xFFFFFFFF
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", crc)

    raw = b"".join(b"\x00" + row for row in rows)
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", ihdr)
    png += chunk(b"IDAT", zlib.compress(raw, 9))
    png += chunk(b"IEND", b"")
    path.write_bytes(png)
TRANSPARENT = (0, 0, 0, 0)
SKY = (88, 144, 200, 255)
WHITE = (248, 248, 248, 255)
BLACK = (8, 8, 16, 255)
YELLOW = (248, 208, 32, 255)
ORANGE = (232, 112, 24, 255)
RED = (200, 40, 32, 255)
GREEN = (48, 160, 48, 255)

# 5x7 glyphs. Keep this set small: the HUD is baked into sprites.
FONT = {
    " ": ["00000"] * 7,
    "A": ["01110", "10001", "10001", "11111", "10001", "10001", "10001"],
    "B": ["11110", "10001", "10001", "11110", "10001", "10001", "11110"],
    "C": ["01111", "10000", "10000", "10000", "10000", "10000", "01111"],
    "D": ["11110", "10001", "10001", "10001", "10001", "10001", "11110"],
    "E": ["11111", "10000", "10000", "11110", "10000", "10000", "11111"],
    "F": ["11111", "10000", "10000", "11110", "10000", "10000", "10000"],
    "G": ["01111", "10000", "10000", "10111", "10001", "10001", "01111"],
    "H": ["10001", "10001", "10001", "11111", "10001", "10001", "10001"],
    "I": ["01110", "00100", "00100", "00100", "00100", "00100", "01110"],
    "L": ["10000", "10000", "10000", "10000", "10000", "10000", "11111"],
    "M": ["10001", "11011", "10101", "10101", "10001", "10001", "10001"],
    "N": ["10001", "11001", "10101", "10011", "10001", "10001", "10001"],
    "O": ["01110", "10001", "10001", "10001", "10001", "10001", "01110"],
    "R": ["11110", "10001", "10001", "11110", "10100", "10010", "10001"],
    "S": ["01111", "10000", "10000", "01110", "00001", "00001", "11110"],
    "T": ["11111", "00100", "00100", "00100", "00100", "00100", "00100"],
    "U": ["10001", "10001", "10001", "10001", "10001", "10001", "01110"],
    "V": ["10001", "10001", "10001", "10001", "01010", "01010", "00100"],
    "W": ["10001", "10001", "10001", "10101", "10101", "11011", "10001"],
    "Y": ["10001", "10001", "01010", "00100", "00100", "00100", "00100"],
    "0": ["01110", "10001", "10011", "10101", "11001", "10001", "01110"],
    "1": ["00100", "01100", "00100", "00100", "00100", "00100", "01110"],
    "2": ["01110", "10001", "00001", "00010", "00100", "01000", "11111"],
    "3": ["11110", "00001", "00001", "01110", "00001", "00001", "11110"],
    "4": ["00010", "00110", "01010", "10010", "11111", "00010", "00010"],
    "5": ["11111", "10000", "10000", "11110", "00001", "00001", "11110"],
    "6": ["01110", "10000", "10000", "11110", "10001", "10001", "01110"],
    "7": ["11111", "00001", "00010", "00100", "01000", "01000", "01000"],
    "8": ["01110", "10001", "10001", "01110", "10001", "10001", "01110"],
    "9": ["01110", "10001", "10001", "01111", "00001", "00001", "01110"],
}


def blank(w, h, color=TRANSPARENT):
    return Image(w, h, color)


def put(img, x, y, color):
    if 0 <= x < img.width and 0 <= y < img.height:
        img.putpixel((x, y), color)


def draw_text(img, text, x, y, color, shadow=BLACK, scale=1):
    cursor = x
    for ch in text:
        glyph = FONT.get(ch, FONT[" "])
        for row, bits in enumerate(glyph):
            for col, bit in enumerate(bits):
                if bit != "1":
                    continue
                for sy in range(scale):
                    for sx in range(scale):
                        px = cursor + col * scale + sx
                        py = y + row * scale + sy
                        if shadow is not None:
                            put(img, px + scale, py + scale, shadow)
                        put(img, px, py, color)
        cursor += 6 * scale


def fill_rect(img, x, y, w, h, color):
    for yy in range(y, y + h):
        for xx in range(x, x + w):
            put(img, xx, yy, color)


def tile_floor(img, ox, oy, dark=False):
    base = (16, 56, 24, 255) if dark else (32, 80, 28, 255)
    spec = (48, 96, 32, 255) if dark else (72, 120, 40, 255)
    for y in range(8):
        for x in range(8):
            color = spec if (x * 3 + y * 5) % 7 == 0 else base
            put(img, ox + x, oy + y, color)


def tile_dirt(img, ox, oy):
    for y in range(8):
        for x in range(8):
            color = (104, 72, 40, 255) if (x + y) % 3 == 0 else (136, 96, 48, 255)
            put(img, ox + x, oy + y, color)


def tile_road(img, ox, oy):
    fill_rect(img, ox, oy, 8, 8, (104, 80, 48, 255))
    fill_rect(img, ox, oy + 2, 8, 1, (64, 48, 24, 255))
    fill_rect(img, ox, oy + 5, 8, 1, (64, 48, 24, 255))


def tile_water(img, ox, oy):
    for y in range(8):
        for x in range(8):
            color = (56, 104, 184, 255) if (x + y) % 4 == 0 else (32, 72, 160, 255)
            put(img, ox + x, oy + y, color)


def tile_tree(img, ox, oy):
    tile_floor(img, ox, oy, True)
    disc(img, ox + 3, oy + 3, 3, (8, 48, 16, 255))
    disc(img, ox + 3, oy + 3, 2, (24, 88, 28, 255))
    put(img, ox + 3, oy + 6, (88, 56, 24, 255))
    put(img, ox + 4, oy + 6, (88, 56, 24, 255))


def tile_trench(img, ox, oy):
    fill_rect(img, ox, oy, 8, 8, (48, 36, 20, 255))
    fill_rect(img, ox, oy + 2, 8, 4, (16, 12, 8, 255))


def tile_lip(img, ox, oy):
    fill_rect(img, ox, oy, 8, 8, (112, 88, 40, 255))
    fill_rect(img, ox, oy + 5, 8, 3, (24, 16, 8, 255))


def tile_crater(img, ox, oy):
    fill_rect(img, ox, oy, 8, 8, (72, 56, 40, 255))
    fill_rect(img, ox + 2, oy + 2, 4, 4, (24, 16, 16, 255))
    put(img, ox + 1, oy + 1, (120, 96, 64, 255))
    put(img, ox + 6, oy + 6, (120, 96, 64, 255))


def tile_mine(img, ox, oy):
    tile_dirt(img, ox, oy)
    put(img, ox + 3, oy + 2, RED)
    put(img, ox + 4, oy + 2, RED)
    put(img, ox + 2, oy + 3, RED)
    put(img, ox + 3, oy + 3, YELLOW)
    put(img, ox + 4, oy + 3, YELLOW)
    put(img, ox + 5, oy + 3, RED)
    put(img, ox + 3, oy + 4, RED)
    put(img, ox + 4, oy + 4, RED)


def tile_sky(img, ox, oy):
    fill_rect(img, ox, oy, 8, 8, SKY)


def build_terrain():
    img = blank(88, 8, SKY)
    painters = [
        tile_sky,
        lambda i, x, y: tile_floor(i, x, y, False),
        lambda i, x, y: tile_floor(i, x, y, True),
        tile_dirt,
        tile_road,
        tile_water,
        tile_tree,
        tile_crater,
        tile_mine,
        tile_trench,
        tile_lip,
    ]
    for index, paint in enumerate(painters):
        paint(img, index * 8, 0)
    img.save(ROOT / "terrain.png")


def disc(img, cx, cy, r, color):
    for y in range(cy - r, cy + r + 1):
        for x in range(cx - r, cx + r + 1):
            if (x - cx) ** 2 + (y - cy) ** 2 <= r * r:
                put(img, x, y, color)


def build_sprites():
    reticle = blank(16, 16)
    for i in range(5):
        put(reticle, 1 + i, 1, WHITE)
        put(reticle, 1, 1 + i, WHITE)
        put(reticle, 10 + i, 1, WHITE)
        put(reticle, 14, 1 + i, WHITE)
        put(reticle, 1 + i, 14, WHITE)
        put(reticle, 1, 10 + i, WHITE)
        put(reticle, 10 + i, 14, WHITE)
        put(reticle, 14, 10 + i, WHITE)
    put(reticle, 8, 7, YELLOW)
    put(reticle, 7, 8, YELLOW)
    put(reticle, 8, 8, YELLOW)
    put(reticle, 9, 8, YELLOW)
    put(reticle, 8, 9, YELLOW)
    reticle.save(ROOT / "reticle.png")

    bomb = blank(8, 8)
    disc(bomb, 3, 4, 2, (32, 32, 32, 255))
    put(bomb, 3, 1, ORANGE)
    put(bomb, 4, 0, YELLOW)
    bomb.save(ROOT / "bomb.png")

    he = blank(8, 8)
    disc(he, 3, 4, 3, ORANGE)
    disc(he, 3, 4, 1, YELLOW)
    he.save(ROOT / "he.png")

    mine = blank(8, 8)
    disc(mine, 3, 4, 2, (40, 40, 48, 255))
    for dx, dy in ((0, -3), (0, 3), (-3, 0), (3, 0), (-2, -2), (2, -2), (-2, 2), (2, 2)):
        put(mine, 3 + dx, 4 + dy, RED)
    put(mine, 3, 4, YELLOW)
    mine.save(ROOT / "mine.png")

    boom = blank(48, 16)
    disc(boom, 8, 8, 2, YELLOW)
    disc(boom, 24, 8, 5, ORANGE)
    disc(boom, 24, 8, 2, WHITE)
    disc(boom, 40, 8, 7, RED)
    disc(boom, 40, 8, 4, YELLOW)
    disc(boom, 40, 8, 1, WHITE)
    boom.save(ROOT / "boom.png")

    truck = blank(16, 8)
    fill_rect(truck, 2, 2, 12, 4, (48, 72, 40, 255))
    fill_rect(truck, 3, 3, 4, 2, (160, 176, 96, 255))
    put(truck, 1, 2, BLACK)
    put(truck, 1, 5, BLACK)
    put(truck, 14, 2, BLACK)
    put(truck, 14, 5, BLACK)
    truck.save(ROOT / "truck.png")

    soldier = blank(8, 8)
    put(soldier, 3, 1, (24, 48, 24, 255))
    put(soldier, 4, 1, (24, 48, 24, 255))
    fill_rect(soldier, 2, 2, 4, 2, (72, 88, 40, 255))
    fill_rect(soldier, 2, 4, 4, 2, (120, 104, 56, 255))
    put(soldier, 2, 6, (40, 32, 16, 255))
    put(soldier, 5, 6, (40, 32, 16, 255))
    put(soldier, 6, 3, (32, 32, 32, 255))
    soldier.save(ROOT / "soldier.png")

    digits = blank(80, 8)
    for index, ch in enumerate("0123456789"):
        draw_text(digits, ch, index * 8 + 1, 0, WHITE, BLACK)
    digits.save(ROOT / "digits.png")

    labels = {
        "lbl_score": "SCORE",
        "lbl_bomb": "BOMB",
        "lbl_he": "FUGAS",
        "lbl_mine": "MINE",
        "lbl_alt": "ALT",
    }
    for name, text in labels.items():
        img = blank(32, 8)
        draw_text(img, text, 0, 0, WHITE, BLACK)
        img.save(ROOT / f"{name}.png")

    title = blank(64, 32)
    draw_text(title, "DRONE", 2, 8, YELLOW, BLACK, scale=2)
    title.save(ROOT / "title.png")

    help_quad = blank(64, 32)
    draw_text(help_quad, "PAD MOVE", 2, 1, WHITE, BLACK)
    draw_text(help_quad, "L R TURN", 2, 9, WHITE, BLACK)
    draw_text(help_quad, "A BOMB", 2, 17, YELLOW, BLACK)
    draw_text(help_quad, "START", 2, 25, GREEN, BLACK)
    help_quad.save(ROOT / "help_quad.png")

    help_fpv = blank(64, 32)
    draw_text(help_fpv, "PAD NOSE", 2, 1, WHITE, BLACK)
    draw_text(help_fpv, "L R GAS", 2, 9, WHITE, BLACK)
    draw_text(help_fpv, "A BOMB", 2, 17, YELLOW, BLACK)
    draw_text(help_fpv, "START", 2, 25, GREEN, BLACK)
    help_fpv.save(ROOT / "help_fpv.png")

    woods = blank(64, 32)
    draw_text(woods, "WOODS", 2, 0, GREEN, BLACK, scale=2)
    draw_text(woods, "HOVER", 2, 16, WHITE, BLACK, scale=2)
    woods.save(ROOT / "lvl_woods.png")

    ditch = blank(64, 32)
    draw_text(ditch, "DITCH", 2, 0, (160, 120, 48, 255), BLACK, scale=2)
    draw_text(ditch, "HOVER", 2, 16, WHITE, BLACK, scale=2)
    ditch.save(ROOT / "lvl_ditch.png")

    raid = blank(64, 32)
    draw_text(raid, "FPV", 14, 0, ORANGE, BLACK, scale=2)
    draw_text(raid, "RAID", 8, 16, WHITE, BLACK, scale=2)
    raid.save(ROOT / "lvl_fpv.png")

    crash = blank(64, 32)
    draw_text(crash, "CRASH", 4, 4, RED, BLACK, scale=2)
    crash.save(ROOT / "crash.png")

    win = blank(64, 32)
    draw_text(win, "CLEAR", 4, 4, GREEN, BLACK, scale=2)
    win.save(ROOT / "win.png")


def main():
    ROOT.mkdir(parents=True, exist_ok=True)
    build_terrain()
    build_sprites()


if __name__ == "__main__":
    main()
