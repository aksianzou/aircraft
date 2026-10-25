#!/usr/bin/env python3
"""Voxel models rendered to the cartridge's sprite sheets.

Each figure is a small grid of materials. A sheet is that grid seen from the
headings and heights the game draws it at: one orthographic ray per pixel, no
smoothing, so the pixels stay crisp. Light comes from the upper left, every
material has a lit, a plain and a shaded tone, and the silhouette gets a dark
rim. Same model, same light, every angle.

    python3 tools/voxel.py            # write every sheet into gfx/
    python3 tools/voxel.py --preview  # contact sheet of each, 4x, into target/

Only the standard library is used, as in gen_gfx.py.
"""

import math
import struct
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
GFX = ROOT / "gfx"

# Facing for each of the game's eight octants. Octant 0 faces the camera,
# 2 faces screen-right, 4 shows the rear, 6 faces screen-left.
OCTANT_YAW = [0, 45, 90, 135, 180, 225, 270, 315]


def hexcolour(text):
    return tuple(int(text[i : i + 2], 16) for i in (0, 2, 4))


class Material:
    """Three tones: lit (top and the light's side), plain, shaded."""

    def __init__(self, lit, plain, shade):
        self.tones = (hexcolour(lit), hexcolour(plain), hexcolour(shade))


class Model:
    """Voxels on an integer grid. x is across, y is along (forward is +y),
    z is up. Boxes are half-open: x0 <= x < x1."""

    def __init__(self, width, length, height):
        self.size = (width, length, height)
        self.cells = {}

    def box(self, x0, y0, z0, x1, y1, z1, material):
        for x in range(x0, x1):
            for y in range(y0, y1):
                for z in range(z0, z1):
                    self.cells[(x, y, z)] = material
        return self

    def wheel(self, x0, x1, cy, cz, radius, material, hub=None):
        """A disc across x, centred at (cy, cz) in the side plane."""
        for x in range(x0, x1):
            for y in range(int(cy - radius) - 1, int(cy + radius) + 2):
                for z in range(int(cz - radius) - 1, int(cz + radius) + 2):
                    d = math.hypot(y + 0.5 - cy, z + 0.5 - cz)
                    if d <= radius:
                        inner = hub is not None and d <= radius * 0.35 and x in (x0, x1 - 1)
                        self.cells[(x, y, z)] = hub if inner else material
        return self

    def carve(self, x0, y0, z0, x1, y1, z1):
        for x in range(x0, x1):
            for y in range(y0, y1):
                for z in range(z0, z1):
                    self.cells.pop((x, y, z), None)
        return self

    def paint(self, x0, y0, z0, x1, y1, z1, material):
        """Recolour voxels that are already there."""
        for x in range(x0, x1):
            for y in range(y0, y1):
                for z in range(z0, z1):
                    if (x, y, z) in self.cells:
                        self.cells[(x, y, z)] = material
        return self


def rotate(v, yaw):
    """Turn a model-space vector by yaw degrees about z."""
    a = math.radians(yaw)
    x, y, z = v
    return (x * math.cos(a) - y * math.sin(a), x * math.sin(a) + y * math.cos(a), z)


def view_basis(yaw, elevation):
    """Screen right, screen up and the viewing direction, in model space.

    The camera looks along -y in its own frame (so a model facing +y faces it)
    and is raised by `elevation` degrees, looking down."""
    e = math.radians(elevation)
    right = (1.0, 0.0, 0.0)
    up = (0.0, -math.sin(e), math.cos(e))
    toward = (0.0, -math.cos(e), -math.sin(e))  # from camera into the scene
    # Turning the model by yaw is turning the camera the other way.
    return tuple(rotate(v, yaw) for v in (right, up, toward))


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def cast(model, origin, direction):
    """First voxel hit by a ray, and the face it entered. Amanatides-Woo DDA."""
    w, l, h = model.size
    lo, hi = (-2, -2, -2), (w + 2, l + 2, h + 2)
    # Clip the ray to the box around the model.
    t0, t1 = -1e9, 1e9
    for axis in range(3):
        if abs(direction[axis]) < 1e-9:
            if origin[axis] < lo[axis] or origin[axis] > hi[axis]:
                return None
            continue
        a = (lo[axis] - origin[axis]) / direction[axis]
        b = (hi[axis] - origin[axis]) / direction[axis]
        t0, t1 = max(t0, min(a, b)), min(t1, max(a, b))
    if t0 > t1:
        return None
    t = t0 + 1e-6
    pos = [origin[i] + direction[i] * t for i in range(3)]
    cell = [math.floor(p) for p in pos]
    step = [1 if d > 0 else -1 for d in direction]
    delta = [abs(1 / d) if abs(d) > 1e-9 else 1e9 for d in direction]
    side = []
    for i in range(3):
        if abs(direction[i]) < 1e-9:
            side.append(1e9)
        elif direction[i] > 0:
            side.append((cell[i] + 1 - pos[i]) * delta[i])
        else:
            side.append((pos[i] - cell[i]) * delta[i])
    face = None
    for _ in range(400):
        hit = model.cells.get(tuple(cell))
        if hit is not None:
            normal = [0, 0, 0]
            if face is not None:
                normal[face] = -step[face]
            return hit, tuple(normal), cell[2]
        face = min(range(3), key=lambda i: side[i])
        if side[face] > t1 - t0 + 2:
            return None
        cell[face] += step[face]
        side[face] += delta[face]
    return None


# Light from the upper left of the screen, a little toward the viewer.
LIGHT_SCREEN = (-0.55, 0.75, 0.35)


def shade(normal, basis, height):
    """`height` is how far up the model the face sits, 0 to 1."""
    right, up, toward = basis
    lx, ly, lz = LIGHT_SCREEN
    light = tuple(lx * right[i] + ly * up[i] - lz * toward[i] for i in range(3))
    k = dot(normal, light)
    if normal[2] > 0.5:
        # Roofs step down with the model, so from overhead a turret stands
        # above its hull and a tarp above the cab.
        if height >= 0.75:
            return 0
        return 1 if height >= 0.4 else 2
    # The lit tone is for walls squarely into the light: on a wall turned
    # half toward it, it reads as faded paint.
    if k > 0.75:
        return 0
    if k > -0.1:
        return 1
    return 2


def render(model, yaw, elevation, scale, cell, foot, rim):
    """One frame. `scale` is pixels per voxel, `foot` the row of the ground
    under the model's centre. Returns rows of RGBA tuples (None is clear)."""
    cw, ch = cell
    basis = view_basis(yaw, elevation)
    right, up, toward = basis
    w, l, h = model.size
    centre = (w / 2, l / 2, 0.0)
    image = [[None] * cw for _ in range(ch)]
    tones = [[None] * cw for _ in range(ch)]
    for py in range(ch):
        for px in range(cw):
            u = (px + 0.5 - cw / 2) / scale
            v = (foot - (py + 0.5)) / scale
            point = tuple(centre[i] + u * right[i] + v * up[i] for i in range(3))
            origin = tuple(point[i] - toward[i] * 100 for i in range(3))
            hit = cast(model, origin, toward)
            if hit is None:
                continue
            material, normal, z = hit
            tone = shade(normal, basis, (z + 1) / h)
            image[py][px] = material
            tones[py][px] = tone
    out = [[None] * cw for _ in range(ch)]
    for py in range(ch):
        for px in range(cw):
            material = image[py][px]
            if material is None:
                continue
            colour = material.tones[tones[py][px]]
            edge = any(
                not (0 <= py + dy < ch and 0 <= px + dx < cw) or image[py + dy][px + dx] is None
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))
            )
            if edge and rim is not None:
                colour = rim
            out[py][px] = colour
    return out


def ink_rows(frame):
    rows = [y for y, row in enumerate(frame) if any(p is not None for p in row)]
    return (rows[0], rows[-1]) if rows else (0, -1)


def sheet(frames):
    height = len(frames[0])
    rows = []
    for y in range(height):
        row = []
        for frame in frames:
            row.extend(frame[y])
        rows.append(row)
    return rows


def write_png(path, rows):
    height, width = len(rows), len(rows[0])
    raw = bytearray()
    for row in rows:
        raw.append(0)
        for p in row:
            raw.extend(p + (255,) if p is not None else (0, 0, 0, 0))

    def chunk(tag, data):
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(bytes(raw), 9))
    png += chunk(b"IEND", b"")
    path.write_bytes(png)


def preview(path, rows, zoom=4):
    """The sheet on a grey check, enlarged, for looking at."""
    out = []
    for y, row in enumerate(rows):
        line = []
        for x, p in enumerate(row):
            if p is None:
                p = (60, 60, 70) if (x // 4 + y // 4) % 2 else (75, 75, 85)
            line.extend([p] * zoom)
        out.extend([line] * zoom)
    write_png(path, out)


# One palette for every vehicle and the bunker. Every colour here was
# already on another sheet: the image converter packs all sprites into sixteen
# palettes, and colours of their own would not fit.
RIM = hexcolour("181810")
OLIVE = ("98b0a8", "486030", "383020")
OLIVE_DEEP = "202020"
KHAKI = ("b8b090", "a07830", "705830")
GLASS_ = ("a8c8c8", "506868", "383020")
RUBBER = ("383020", "202020", "181810")
LAMP_ = ("f8d020", "f8d020", "a07830")
STEEL = ("706060", "706060", "383020")
CONCRETE = ("d8c8c0", "b8b090", "a07830")
CONCRETE_DEEP = "706060"
SAND = ("a08040", "705830", "383020")

PAINT = Material(*OLIVE)
UNDER = Material(OLIVE[2], OLIVE_DEEP, "181810")
TARP = Material(*KHAKI)
RIB = Material(KHAKI[1], KHAKI[2], OLIVE_DEEP)
GLASS = Material(*GLASS_)
TYRE = Material(*RUBBER)
HUB = Material(OLIVE[2], OLIVE_DEEP, OLIVE_DEEP)
LAMP = Material(*LAMP_)
TRACK = Material(*STEEL)
WHEEL = Material(STEEL[0], STEEL[0], STEEL[2])
CONC = Material(*CONCRETE)
SLOT = Material(CONCRETE_DEEP, "181810", "181810")
BAGS = Material(*SAND)


def truck():
    m = Model(10, 26, 12)
    m.box(1, 2, 2, 9, 24, 4, UNDER)  # chassis
    m.box(0, 1, 4, 10, 17, 6, PAINT)  # bed and its sides
    m.box(0, 1, 6, 10, 17, 10, TARP)  # canvas over the bed
    m.box(1, 1, 10, 9, 17, 11, TARP)  # its rounded top
    for y in (5, 9, 13):
        m.paint(0, y, 6, 10, y + 1, 11, RIB)
    m.box(0, 18, 4, 10, 23, 10, PAINT)  # cab
    m.box(1, 23, 4, 9, 26, 7, PAINT)  # bonnet
    m.paint(1, 22, 7, 9, 23, 9, GLASS)  # windscreen
    m.paint(0, 19, 7, 1, 22, 9, GLASS).paint(9, 19, 7, 10, 22, 9, GLASS)
    m.paint(1, 25, 5, 3, 26, 6, LAMP).paint(7, 25, 5, 9, 26, 6, LAMP)
    m.paint(3, 25, 4, 7, 26, 6, UNDER)  # grille
    for cy in (5, 10, 21):
        m.wheel(0, 2, cy, 2.2, 2.2, TYRE, HUB)
        m.wheel(8, 10, cy, 2.2, 2.2, TYRE, HUB)
    return m


def tank():
    m = Model(13, 34, 11)
    m.box(0, 3, 0, 3, 25, 4, TRACK).box(10, 3, 0, 13, 25, 4, TRACK)  # tracks
    m.box(0, 4, 4, 3, 24, 5, PAINT).box(10, 4, 4, 13, 24, 5, PAINT)  # guards
    for cy in (6, 10, 14, 18, 22):
        # Road wheels sit in the track's outer face: rubber with a steel hub.
        m.wheel(0, 1, cy, 1.9, 1.6, TYRE, WHEEL).wheel(12, 13, cy, 1.9, 1.6, TYRE, WHEEL)
    m.paint(0, 3, 0, 1, 25, 1, TRACK).paint(12, 3, 0, 13, 25, 1, TRACK)  # track shoes
    m.box(3, 3, 1, 10, 25, 5, UNDER)  # belly between the tracks
    m.box(1, 3, 5, 12, 25, 7, PAINT)  # hull deck
    m.box(2, 23, 4, 11, 26, 6, PAINT)  # glacis
    m.box(3, 8, 7, 10, 17, 9, PAINT)  # turret
    m.box(4, 9, 9, 9, 16, 10, PAINT)  # its roof
    m.paint(5, 10, 9, 8, 13, 10, UNDER)  # hatch
    m.box(5, 17, 7, 7, 34, 8, UNDER)  # barrel, two wide so it shows from overhead
    m.box(5, 31, 7, 8, 33, 8, UNDER)  # muzzle brake
    return m


def bunker():
    m = Model(18, 16, 9)
    m.box(0, 0, 0, 18, 16, 2, BAGS)  # sandbag footing
    m.box(1, 1, 2, 17, 15, 7, CONC)
    m.box(2, 2, 7, 16, 14, 8, CONC)  # chamfered roof
    m.box(3, 3, 8, 15, 13, 9, BAGS)  # bags on the roof
    for y0 in (1, 14):
        m.paint(4, y0, 4, 14, y0 + 1, 5, SLOT)  # firing slits front and back
    m.paint(0, 5, 4, 18, 11, 5, SLOT).carve(0, 5, 4, 1, 11, 5).carve(17, 5, 4, 18, 11, 5)
    for x in range(0, 18, 3):
        m.paint(x, 0, 0, x + 1, 16, 2, Material(SAND[1], SAND[2], CONCRETE_DEEP))
    return m


SKIN = Material("d8c8c0", "b89890", "a08078")
CLOTH = Material(*OLIVE)
TROUSER = Material(OLIVE[1], OLIVE[2], OLIVE_DEEP)
BOOT = Material(OLIVE_DEEP, "181810", "181810")
GUN = Material(STEEL[2], OLIVE_DEEP, "181810")


def soldier(stride=False):
    """`stride` is mid-step: the left leg forward, the right one back, the
    arms swinging the other way."""
    m = Model(8, 6, 19)
    left, right = (3, -1) if stride else (1, 1)
    m.box(1, left, 0, 3, left + 3, 1, BOOT).box(5, right, 0, 7, right + 3, 1, BOOT)
    m.box(1, left, 1, 3, left + 2, 7, TROUSER).box(5, right, 1, 7, right + 2, 7, TROUSER)
    m.box(1, 0, 7, 7, 4, 13, CLOTH)  # jacket
    m.paint(1, 0, 7, 7, 4, 8, BOOT)  # belt
    arm_l, arm_r = (-1, 3) if stride else (1, 1)
    m.box(0, arm_l, 8, 1, arm_l + 2, 13, CLOTH).box(7, arm_r, 8, 8, arm_r + 2, 13, CLOTH)  # arms
    m.box(2, 1, 13, 6, 4, 16, SKIN)  # head
    m.box(2, 0, 15, 6, 5, 17, CLOTH)  # helmet, its brim to the front
    m.box(2, 1, 17, 6, 4, 18, CLOTH)
    m.box(0, 4, 10, 8, 5, 11, GUN)  # rifle across the chest
    m.box(6, 4, 11, 8, 6, 12, GUN)
    return m


def fallen():
    """The same man on his back: what was up runs along the ground."""
    standing = soldier()
    w, l, h = standing.size
    m = Model(w, h, l)
    for (x, y, z), material in standing.cells.items():
        m.cells[(x, z, y)] = material
    return m


# name: (model, cell, frames as (octant, elevation), scale, foot)
LEVEL = 12
DIVE = 35
PLAN = 90  # straight down, as the overhead glass looks
FAR = [(o, LEVEL) for o in range(8)] + [(o, DIVE) for o in range(8)]
NEAR = [(o, LEVEL) for o in (0, 2, 4, 6)]
TOP = [(o, PLAN) for o in range(8)]

SHEETS = {
    "truck": (truck, (32, 32), FAR, 1.0, 25),
    "truck_near": (truck, (64, 32), NEAR, 1.9, 27),
    "truck_top": (truck, (32, 32), TOP, 0.95, 16),
    "tank": (tank, (32, 32), FAR, 0.8, 24),
    "tank_near": (tank, (64, 32), NEAR, 1.7, 26),
    "tank_top": (tank, (32, 32), TOP, 0.8, 16),
    "bunker": (bunker, (32, 32), FAR, 1.15, 23),
    "bunker_near": (bunker, (64, 32), NEAR, 2.2, 27),
    "bunker_top": (bunker, (32, 32), TOP, 1.1, 16),
    "soldier": (soldier, (32, 32), FAR, 1.0, 26),
    "soldier_near": (soldier, (32, 64), [(o, LEVEL) for o in (0, 2, 4, 6)], 2.45, 61),
    "soldier_top": (soldier, (32, 32), TOP, 1.0, 16),
}

# Cells after the sheet's own: [(model, octant, elevation)]. The soldier's
# are the body on the ground, then mid-step at each level heading.
TAILS = {
    "soldier": [(fallen, 1, DIVE)] + [(lambda: soldier(True), o, LEVEL) for o in range(8)],
    "soldier_near": [(lambda: soldier(True), o, LEVEL) for o in (0, 2, 4, 6)],
}


def build(name):
    make, cell, frames, scale, foot = SHEETS[name]
    model = make()
    out = [render(model, OCTANT_YAW[o], e, scale, cell, foot, RIM) for o, e in frames]
    for tail, octant, elevation in TAILS.get(name, []):
        out.append(render(tail(), OCTANT_YAW[octant], elevation, scale, cell, foot, RIM))
    return out


def metrics(frames):
    """Height of the ink in each frame, and the longest side (plan sheets)."""
    out = []
    for f in frames:
        top, bottom = ink_rows(f)
        cols = [x for x in range(len(f[0])) if any(row[x] is not None for row in f)]
        out.append((bottom - top + 1, cols[-1] - cols[0] + 1 if cols else 0, bottom))
    return out


def dust():
    """Road dust behind a moving vehicle: three 16x16 puffs, each wider and
    thinner than the last. The edge is a checker, so the puff reads as air."""
    tones = [hexcolour("d8c8c0"), hexcolour("b8b090"), hexcolour("a08040")]
    frames = []
    for radius, solid in ((4.0, 2.5), (5.5, 3.0), (7.0, 3.0)):
        cell = [[None] * 16 for _ in range(16)]
        for y in range(16):
            for x in range(16):
                d = math.hypot(x + 0.5 - 8, (y + 0.5 - 8) * 1.4)
                if d > radius or (d > solid and (x + y) % 2):
                    continue
                shade = 0 if y < 7 else (1 if y < 10 else 2)
                cell[y][x] = tones[shade]
        frames.append(cell)
    return frames


PROP = Material("706060", "706060", "383020")
LENS = Material("a8c8c8", "506868", "202020")
LED = Material("f8d020", "f8d020", "a08040")


def quad():
    """The game's own quadcopter, with a bomb under it, for the studio card."""
    m = Model(40, 40, 15)
    m.box(13, 12, 5, 27, 28, 10, PAINT)  # body
    m.box(14, 13, 10, 26, 27, 11, PAINT)  # its lid
    m.paint(17, 27, 8, 23, 28, 9, LED)  # nose light
    m.box(17, 27, 3, 23, 30, 6, UNDER).paint(19, 29, 4, 21, 30, 5, LENS)  # camera
    for index, (cx, cy) in enumerate(((8, 8), (32, 8), (8, 32), (32, 32))):
        for t in range(0, 21):
            x = round(20 + (cx - 20) * t / 20)
            y = round(20 + (cy - 20) * t / 20)
            m.box(x - 1, y - 1, 8, x + 1, y + 1, 9, UNDER)  # arm
        m.box(cx - 2, cy - 2, 7, cx + 2, cy + 2, 11, UNDER)  # motor
        # Two blades caught mid-turn, each motor at its own angle.
        angle = math.radians(20 + 50 * index)
        for step in range(-14, 15):
            x = round(cx + step / 2 * math.cos(angle))
            y = round(cy + step / 2 * math.sin(angle))
            m.cells[(x, y, 11)] = PROP
        lx = cx + (3 if cx < 20 else -3)
        ly = cy + (3 if cy < 20 else -3)
        m.box(lx, ly, 2, lx + 1, ly + 1, 7, UNDER)  # leg
    m.box(17, 15, 1, 23, 25, 5, TRACK)  # the bomb
    m.box(19, 12, 2, 21, 15, 4, UNDER)  # its fins
    return m


SKY_BANDS = ["7090a8", "98b0c0", "a0b0c0", "a8c0d8", "b8d0d8"]
FIELD = ["486030", "383020", "705830"]


def title():
    """256x192, cut into twelve 64x64 cells, read across and then down."""
    width, height, horizon = 256, 192, 116
    rows = []
    for y in range(height):
        row = []
        for x in range(width):
            if y < horizon:
                # Bands down the sky, each blending into the next on a checker.
                pos = y * (len(SKY_BANDS) - 1) / horizon
                band = int(pos)
                mix = pos - band
                pick = band + (1 if (x + y) % 2 and mix > 0.5 else 0)
                row.append(hexcolour(SKY_BANDS[min(pick, len(SKY_BANDS) - 1)]))
            else:
                # Long furrows across the field, closer together at the horizon.
                depth = y - horizon
                tone = FIELD[0]
                if depth % (3 + depth // 12) == 0 and (x + depth * 7) % 23 < 15:
                    tone = FIELD[1]
                row.append(hexcolour(tone))
        rows.append(row)
    hero = render(quad(), 35, 25, 4.6, (width, height), 132, RIM)
    for y in range(height):
        for x in range(width):
            if hero[y][x] is not None:
                rows[y][x] = hero[y][x]
    return rows


def main():
    look = "--preview" in sys.argv
    target = ROOT / "target" / "voxel"
    target.mkdir(parents=True, exist_ok=True)
    for name in SHEETS:
        frames = build(name)
        rows = sheet(frames)
        colours = {p for row in rows for p in row if p is not None}
        print(f"{name}: {len(colours)} colours, (ink, width, bottom) {metrics(frames)}")
        if look:
            preview(target / f"{name}.png", rows)
        else:
            write_png(GFX / f"{name}.png", rows)
    rows = sheet(dust())
    if look:
        preview(target / "dust.png", rows)
    else:
        write_png(GFX / "dust.png", rows)
    rows = title()
    colours = {p for row in rows for p in row}
    print(f"title: {len(colours)} colours")
    if look:
        preview(target / "title.png", rows, 2)
    else:
        write_png(GFX / "title.png", rows)


if __name__ == "__main__":
    main()
