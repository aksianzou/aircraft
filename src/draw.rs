use agb::{
    display::{
        AffineMatrix, HEIGHT, Priority, WIDTH,
        object::{
            AffineMatrixObject, AffineMode, GraphicsMode, Object, ObjectAffine, Sprite, SpriteVram,
        },
    },
    fixnum::{Num, vec2},
};

use crate::sprites;
use crate::world::*;

mod cards;
mod cockpit;
mod floor;
mod glass;
mod scene;
mod sky;

pub(crate) use cards::*;
pub(crate) use cockpit::*;
pub(crate) use floor::*;
pub(crate) use glass::*;
pub(crate) use scene::*;
pub(crate) use sky::*;

/// Sheets already looked up this frame. agb finds a sheet's tiles and
/// palette through two table lookups, about 550 cycles, and a frame repeats
/// the digits, the panels and the crowns many times. Emptied before every
/// frame, so it never keeps a sheet or a palette the picture is not showing.
struct Sheets([Option<(usize, SpriteVram)>; SHEET_SLOTS]);

const SHEET_SLOTS: usize = 64;

struct SheetCell(core::cell::UnsafeCell<Sheets>);
unsafe impl Sync for SheetCell {}

static SHEETS: SheetCell = SheetCell(core::cell::UnsafeCell::new(Sheets(
    [const { None }; SHEET_SLOTS],
)));

pub(crate) fn vram(sprite: &'static Sprite) -> SpriteVram {
    let key = sprite as *const Sprite as usize;
    let sheets = unsafe { &mut *SHEETS.0.get() };
    let slot = ((key as u32 >> 2).wrapping_mul(0x9E37_79B1) >> 26) as usize;
    if let Some((held, handle)) = &sheets.0[slot]
        && *held == key
    {
        return handle.clone();
    }
    let handle = SpriteVram::from(sprite);
    sheets.0[slot] = Some((key, handle.clone()));
    handle
}

/// Start a frame with nothing held.
pub(crate) fn forget_sheets() {
    let sheets = unsafe { &mut *SHEETS.0.get() };
    for slot in &mut sheets.0 {
        *slot = None;
    }
}

/// One matrix per scale. Figures at the same size share it, and a frame
/// has room for only 32.
struct Scales([Option<(i32, i32, AffineMatrixObject)>; SCALE_SLOTS]);

const SCALE_SLOTS: usize = 32;

struct ScaleCell(core::cell::UnsafeCell<Scales>);
unsafe impl Sync for ScaleCell {}

static SCALES: ScaleCell = ScaleCell(core::cell::UnsafeCell::new(Scales(
    [const { None }; SCALE_SLOTS],
)));

pub(crate) fn scale_matrix(inv_x: Num<i32, 8>, inv_y: Num<i32, 8>) -> AffineMatrixObject {
    let (x, y) = (inv_x.to_raw(), inv_y.to_raw());
    let scales = unsafe { &mut *SCALES.0.get() };
    let slot = ((x as u32).wrapping_mul(31) ^ y as u32) as usize % SCALE_SLOTS;
    if let Some((held_x, held_y, matrix)) = &scales.0[slot]
        && *held_x == x
        && *held_y == y
    {
        return matrix.clone();
    }
    let matrix: AffineMatrixObject = AffineMatrix::from_scale(vec2(inv_x, inv_y)).into();
    scales.0[slot] = Some((x, y, matrix.clone()));
    matrix
}

fn show_flame(
    frame: &mut agb::display::GraphicsFrame,
    sprite: &'static Sprite,
    x: i32,
    y: i32,
    sensor: Sensor,
) {
    if x < -64 || y < -32 || x > WIDTH || y > HEIGHT {
        return;
    }
    let mut object = Object::new(vram(sprite));
    if let Some(palette) = dye(sprite, sensor) {
        object.set_palette(palette);
    }
    object
        .set_pos((x, y))
        .set_priority(Priority::P0)
        .show(frame);
}

fn show_cold(frame: &mut agb::display::GraphicsFrame, sprite: &'static Sprite, x: i32, y: i32) {
    if x < -64 || y < -32 || x > WIDTH || y > HEIGHT {
        return;
    }
    let mut object = Object::new(vram(sprite));
    if let Some(palette) = glass(glass_now(), Tone::Plant) {
        object.set_palette(palette);
    }
    object
        .set_pos((x, y))
        .set_priority(Priority::P0)
        .show(frame);
}

pub(crate) fn show(
    frame: &mut agb::display::GraphicsFrame,
    sprite: &'static Sprite,
    x: i32,
    y: i32,
) {
    if x < -64 || y < -32 || x > WIDTH || y > HEIGHT {
        return;
    }
    Object::new(vram(sprite))
        .set_pos((x, y))
        .set_priority(Priority::P0)
        .show(frame);
}

pub(crate) fn show_haze(
    frame: &mut agb::display::GraphicsFrame,
    sprite: &'static Sprite,
    x: i32,
    y: i32,
    cockpit: bool,
) {
    if x < -64 || y < -32 || x > WIDTH || y > HEIGHT {
        return;
    }
    // Keep haze inside the view so it does not share scanlines with the frame.
    if cockpit && (x < VIEW_X || y < VIEW_Y || x + 32 > VIEW_X + VIEW_W || y + 16 > VIEW_Y + VIEW_H)
    {
        return;
    }
    Object::new(vram(sprite))
        .set_pos((x, y))
        .set_priority(Priority::P0)
        .set_graphics_mode(GraphicsMode::AlphaBlending)
        .show(frame);
}

pub(crate) fn in_view(sx: i32, sy: i32, cockpit: bool) -> bool {
    !cockpit || ((VIEW_X..VIEW_X + VIEW_W).contains(&sx) && (VIEW_Y..VIEW_Y + VIEW_H).contains(&sy))
}

pub(crate) fn show_flip(
    frame: &mut agb::display::GraphicsFrame,
    sprite: &'static Sprite,
    x: i32,
    y: i32,
    hflip: bool,
    vflip: bool,
) {
    if x < -16 || y < -16 || x > WIDTH || y > HEIGHT {
        return;
    }
    Object::new(vram(sprite))
        .set_pos((x, y))
        .set_hflip(hflip)
        .set_vflip(vflip)
        .set_priority(Priority::P0)
        .show(frame);
}

pub(crate) fn boom_sheet(frame: i16) -> usize {
    if frame < 7 {
        0
    } else if frame < 14 {
        1
    } else {
        2
    }
}

/// A far hit is a few bright pixels, not the full fire.
pub(crate) fn show_spark(frame: &mut agb::display::GraphicsFrame, sx: i32, sy: i32, cockpit: bool) {
    let x = sx - 4;
    let y = sy - 4;
    if cockpit && (x < VIEW_X || y < VIEW_Y || x + 8 > VIEW_X + VIEW_W || y + 8 > VIEW_Y + VIEW_H) {
        return;
    }
    let mut object = Object::new(vram(sprites::SPARK.sprite(0)));
    if let Some(palette) = dye(sprites::SPARK.sprite(0), glass_now()) {
        object.set_palette(palette);
    }
    object
        .set_pos((x, y))
        .set_priority(Priority::P0)
        .show(frame);
}

fn show_marked(
    frame: &mut agb::display::GraphicsFrame,
    sprite: &'static Sprite,
    native: i32,
    sx: i32,
    sy: i32,
    pixels: i32,
    tone: Tone,
) {
    let double = pixels > native;
    let mode = if double {
        AffineMode::AffineDouble
    } else {
        AffineMode::Affine
    };
    let inv = Num::<i32, 8>::new(native) / Num::new(pixels.max(1));
    let half = if double { native } else { native / 2 };
    let x = sx - half;
    let y = sy - half;
    if x < -native * 2 || y < -native * 2 || x > WIDTH || y > HEIGHT {
        return;
    }
    let mut object = ObjectAffine::new(vram(sprite), scale_matrix(inv, inv), mode);
    if let Some(palette) = glass(Sensor::Ir, tone) {
        object.set_palette(palette);
    }
    object
        .set_pos((x, y))
        .set_priority(Priority::P0)
        .show(frame);
}

/// The hit opens white, then the same point becomes the black cloud.
fn show_smoke(frame: &mut agb::display::GraphicsFrame, sx: i32, sy: i32, age: i16) {
    if age < 8 {
        show_marked(frame, sprites::SMOKE.sprite(0), 32, sx, sy, 52, Tone::Flame);
        show_marked(frame, sprites::FLASH.sprite(0), 64, sx, sy, 64, Tone::Flame);
        return;
    }
    let (sheet, pixels) = if age < 18 { (1, 46) } else { (2, 60) };
    show_marked(
        frame,
        sprites::SMOKE.sprite(sheet),
        32,
        sx,
        sy,
        pixels,
        Tone::Ash,
    );
}

pub(crate) fn show_fit(
    frame: &mut agb::display::GraphicsFrame,
    sprite: &'static Sprite,
    cx: i32,
    cy: i32,
    src: i32,
    pixels: i32,
) {
    let inv = Num::<i32, 8>::new(src) / Num::new(pixels.max(1));
    let x = cx - src / 2;
    let y = cy - src / 2;
    if x < -64 || y < -64 || x > WIDTH || y > HEIGHT {
        return;
    }
    let mut object = ObjectAffine::new(vram(sprite), scale_matrix(inv, inv), AffineMode::Affine);
    if let Some(palette) = dye(sprite, glass_now()) {
        object.set_palette(palette);
    }
    object
        .set_pos((x, y))
        .set_priority(Priority::P0)
        .show(frame);
}

/// Fire past its 16-pixel art. The double-size box holds up to 32, and the
/// size goes in steps of four so neighbouring fires share a matrix.
pub(crate) fn show_blaze(
    frame: &mut agb::display::GraphicsFrame,
    sprite: &'static Sprite,
    sx: i32,
    sy: i32,
    pixels: i32,
) {
    let pixels = (pixels & !3).clamp(8, 32);
    let double = pixels > 16;
    let mode = if double {
        AffineMode::AffineDouble
    } else {
        AffineMode::Affine
    };
    let half = if double { 16 } else { 8 };
    let (x, y) = (sx - half, sy - half);
    if x < -32 || y < -32 || x > WIDTH || y > HEIGHT {
        return;
    }
    let inv = Num::<i32, 8>::new(16) / Num::new(pixels);
    let mut object = ObjectAffine::new(vram(sprite), scale_matrix(inv, inv), mode);
    if let Some(palette) = dye(sprite, glass_now()) {
        object.set_palette(palette);
    }
    object
        .set_pos((x, y))
        .set_priority(Priority::P0)
        .show(frame);
}

pub(crate) fn isqrt64(n: i64) -> i64 {
    if n <= 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}
