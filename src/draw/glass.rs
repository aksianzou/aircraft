//! The glass: how night, heat, infrared and snow repaint the field and the figures.

use agb::display::{
    Palette16, Rgb, Rgb15,
    object::{PaletteVram, PaletteVramSingle, Sprite},
};

use crate::backgrounds;
use crate::sprites;

use super::*;

struct Look {
    sensor: u8,
    applied: u8,
    sky: u16,
    sky_applied: u16,
}

pub(crate) struct LookCell(core::cell::UnsafeCell<Look>);
unsafe impl Sync for LookCell {}

pub(crate) static LOOK: LookCell = LookCell(core::cell::UnsafeCell::new(Look {
    sensor: 0,
    applied: 255,
    sky: 0,
    sky_applied: 0xffff,
}));

struct Detail(core::cell::UnsafeCell<bool>);
unsafe impl Sync for Detail {}

static DETAIL: Detail = Detail(core::cell::UnsafeCell::new(false));

pub(crate) fn set_detail(on: bool) {
    unsafe { *DETAIL.0.get() = on }
}

pub(crate) fn detail_on() -> bool {
    unsafe { *DETAIL.0.get() }
}

fn sensor_tag(sensor: Sensor) -> u8 {
    match sensor {
        Sensor::Day => 0,
        Sensor::Night => 1,
        Sensor::Heat => 2,
        Sensor::Ir => 3,
        Sensor::Winter => 4,
    }
}

pub(crate) fn sensor_from(tag: u8) -> Sensor {
    match tag {
        1 => Sensor::Night,
        2 => Sensor::Heat,
        3 => Sensor::Ir,
        4 => Sensor::Winter,
        _ => Sensor::Day,
    }
}

/// Remember the glass. The vblank handler is what actually writes it.
pub(crate) fn queue_look(sensor: Sensor, sky: Rgb15) {
    unsafe {
        let look = &mut *LOOK.0.get();
        look.sensor = sensor_tag(sensor);
        look.sky = sky.0;
    }
}

pub(crate) fn flush_look() {
    unsafe {
        let look = &mut *LOOK.0.get();
        if look.applied != look.sensor {
            paint_sensor(sensor_from(look.sensor));
            look.applied = look.sensor;
        }
        if look.sky_applied != look.sky {
            let ram = 0x0500_0000 as *mut u16;
            ram.write_volatile(look.sky);
            look.sky_applied = look.sky;
        }
    }
}

/// The field through the glass. Sprites use the same four filters.
pub(crate) fn paint_sensor(sensor: Sensor) {
    unsafe {
        let ram = 0x0500_0000 as *mut u16;
        for pal in 0..backgrounds::PALETTES.len() {
            for index in 0..16 {
                let src = backgrounds::PALETTES[pal].colour(index).0;
                let out = match sensor {
                    Sensor::Night => nvg_ground(src),
                    Sensor::Heat => cold_ground(src),
                    Sensor::Ir => ir_ground(src),
                    Sensor::Day => src,
                    Sensor::Winter => winter_ground(src),
                };
                ram.add(pal * 16 + index).write_volatile(out);
            }
        }
    }
    paint_glass(sensor);
}

/// What a picture is. The sheet stays. The glass only swaps the colours.
pub(crate) enum Tone {
    Plant,
    Warm,
    Flame,
    Ash,
}

struct Held {
    plant: Option<PaletteVram>,
    warm: Option<PaletteVram>,
    flame: Option<PaletteVram>,
    ash: Option<PaletteVram>,
    plant_i: u8,
    warm_i: u8,
    flame_i: u8,
    ash_i: u8,
}

struct HeldCell(core::cell::UnsafeCell<Held>);
unsafe impl Sync for HeldCell {}
static HELD: HeldCell = HeldCell(core::cell::UnsafeCell::new(Held {
    plant: None,
    warm: None,
    flame: None,
    ash: None,
    plant_i: 0,
    warm_i: 0,
    flame_i: 0,
    ash_i: 0,
}));

/// Take a palette before the pictures load. Every entry is part of the mark,
/// so the bank cannot be a digit or a medal that happens to share one colour.
fn claim_ramp(stamp: u16) -> Option<(PaletteVram, u8)> {
    let mut colours = [Rgb15(0); 16];
    for (index, colour) in colours.iter_mut().enumerate().skip(1) {
        *colour = Rgb15(stamp.wrapping_add((index as u16) << 4));
    }
    let palette = PaletteVramSingle::try_allocate_new(&Palette16::new(colours)).ok()?;
    let bank = (0..16).find(|&bank| unsafe {
        let ram = 0x0500_0200 as *const u16;
        (1..16).all(|index| ram.add(bank * 16 + index).read_volatile() == colours[index].0)
    })?;
    Some((palette.palette(), bank as u8))
}

/// Four palettes of our own, taken before the pictures load. A filter writes
/// only these banks. The digit, sun, and medal sheets stay as they were drawn.
pub(crate) fn reserve_glass() {
    unsafe {
        let held = &mut *HELD.0.get();
        if held.plant.is_some() {
            return;
        }
        if let Some((palette, bank)) = claim_ramp(0x4001) {
            held.plant = Some(palette);
            held.plant_i = bank;
        }
        if let Some((palette, bank)) = claim_ramp(0x4002) {
            held.warm = Some(palette);
            held.warm_i = bank;
        }
        if let Some((palette, bank)) = claim_ramp(0x4003) {
            held.flame = Some(palette);
            held.flame_i = bank;
        }
        if let Some((palette, bank)) = claim_ramp(0x4004) {
            held.ash = Some(palette);
            held.ash_i = bank;
            paint_bank(bank, soot);
        }
    }
}

/// The banks a filter repaints. Their colours follow the glass, not the sheet.
#[cfg(test)]
pub(crate) fn glass_banks() -> [u8; 4] {
    let held = unsafe { &*HELD.0.get() };
    [held.plant_i, held.warm_i, held.flame_i, held.ash_i]
}

fn paint_bank(bank: u8, map: impl Fn(u16) -> u16) {
    unsafe {
        let ram = 0x0500_0200 as *mut u16;
        let base = bank as usize * 16;
        ram.add(base).write_volatile(0);
        for index in 1..16 {
            let y = index as u16;
            let grey = y | (y << 5) | (y << 10);
            ram.add(base + index).write_volatile(map(grey));
        }
    }
}

pub(crate) fn paint_glass(sensor: Sensor) {
    reserve_glass();
    if matches!(sensor, Sensor::Day | Sensor::Winter) {
        return;
    }
    let (plant, warm, flame) = unsafe {
        let held = &*HELD.0.get();
        (
            held.plant.as_ref().map(|_| held.plant_i),
            held.warm.as_ref().map(|_| held.warm_i),
            held.flame.as_ref().map(|_| held.flame_i),
        )
    };
    if let Some(bank) = plant {
        paint_bank(bank, |colour| tone_colour(sensor, Tone::Plant, colour));
    }
    if let Some(bank) = warm {
        paint_bank(bank, |colour| tone_colour(sensor, Tone::Warm, colour));
    }
    if let Some(bank) = flame {
        paint_bank(bank, |colour| tone_colour(sensor, Tone::Flame, colour));
    }
}

pub(crate) fn glass(sensor: Sensor, tone: Tone) -> Option<PaletteVram> {
    reserve_glass();
    unsafe {
        let held = &*HELD.0.get();
        if matches!(tone, Tone::Ash) {
            return held.ash.clone();
        }
        if matches!(sensor, Sensor::Day | Sensor::Winter) {
            return None;
        }
        match tone {
            Tone::Plant => held.plant.clone(),
            Tone::Warm => held.warm.clone(),
            Tone::Flame => held.flame.clone(),
            Tone::Ash => held.ash.clone(),
        }
    }
}

pub(crate) fn dye(sprite: &'static Sprite, sensor: Sensor) -> Option<PaletteVram> {
    glass(sensor, tone_of(sprite)?)
}

pub(crate) fn ir_ground(colour: u16) -> u16 {
    let (r, g, b) = channels(colour);
    let mut y = luma(colour);
    if b > r + 2 && b > g {
        y /= 4;
    } else if r > g && r > b {
        y = (y + 3).min(28);
    }
    let lit = if y < 6 { 3 } else { (5 + y * 2 / 3).min(24) };
    let v = lit as u16;
    v | (v << 5) | (v << 10)
}

/// Dirty snow, brown mud, and a sulfur crust. Trenches and burned ground stay dark.
pub(crate) fn winter_ground(colour: u16) -> u16 {
    let (r, g, b) = channels(colour);
    if r == 0 && g == 0 && b == 0 {
        return 0;
    }
    let y = luma(colour);
    let pack = |r: i32, g: i32, b: i32| -> u16 {
        let r = r.clamp(0, 31) as u16;
        let g = g.clamp(0, 31) as u16;
        let b = b.clamp(0, 31) as u16;
        r | (g << 5) | (b << 10)
    };
    // Ponds go yellow: sulfur water, not ice. The pale sky tile is a patch of white snow.
    if b > r + 2 && b > g {
        if r >= 12 && b - g <= 4 {
            return pack(31, 31, 30);
        }
        let v = 14 + y / 3;
        return pack(v, v - 4, v / 5);
    }
    // Grass is snow. The lighter blades are white, the rest a warm grey.
    if g > r && g >= b {
        if y >= 14 {
            return pack(31, 31, 30);
        }
        let v = 22 + y / 2;
        return pack(v, v, v - 2);
    }
    // The trench and the burned mud stay dark enough to read.
    if y < 6 {
        return pack(y / 2 + 1, y / 2 + 1, y / 4);
    }
    // Light earth is a sulfur crust. The rest stays brown mud.
    if y >= 16 {
        let v = 18 + (y - 16) / 2;
        return pack(v, v * 3 / 4, v / 5);
    }
    let v = 10 + y / 2;
    pack(v, v * 2 / 3, v / 3)
}

pub(crate) fn cold_ground(colour: u16) -> u16 {
    let y = luma(colour);
    // Dirt and roads are not heat. A little shade keeps the field from going flat black.
    let v = (1 + y / 12).min(4) as u16;
    (v / 4) | ((v / 2) << 5) | (v << 10)
}

pub(crate) fn nvg_ground(colour: u16) -> u16 {
    let (r, g, b) = channels(colour);
    let y = luma(colour);
    let mut lit = (9 + y * 2 / 3).clamp(8, 26);
    if b > r + 2 && b > g {
        lit = (lit - 5).max(6);
    } else if g >= r && g >= b {
        // Crowns were falling into the black end of the glass.
        lit = (lit + 6).min(24);
    } else if r > g && r > b {
        lit = (lit + 2).min(26);
    }
    let green = lit as u16;
    (green / 7) | (green << 5) | ((green / 6) << 10)
}

fn on_sheet(sprite: &'static Sprite, tag: &agb::display::object::Tag) -> bool {
    let sprites = tag.sprites();
    let start = sprites.as_ptr() as usize;
    let end = start + core::mem::size_of_val(sprites);
    let at = sprite as *const Sprite as usize;
    at >= start && at < end
}

/// A tree is cold. It sits in the same dark as the ground, not in daylight green.
fn cold_plant(colour: u16) -> u16 {
    let y = luma(colour);
    let v = (1 + y / 16).min(3) as u16;
    (v / 3) | ((v / 2) << 5) | (v << 10)
}

/// A blast in heat is a dark red. The night glass uses phosphor instead, so
/// this ramp has no green to bleed into.
fn flame_heat(colour: u16) -> u16 {
    let y = luma(colour);

    (5 + y / 4).clamp(5, 12) as u16
}

/// Infrared fire is white. The burst keeps its shape; the yellow stays in daylight.
fn flame_ir(_colour: u16) -> u16 {
    31 | (31 << 5) | (31 << 10)
}

/// One ramp for every glass. Ash is filled once and never follows the sensor.
fn tone_colour(sensor: Sensor, tone: Tone, colour: u16) -> u16 {
    match (sensor, tone) {
        (_, Tone::Ash) => soot(colour),
        (Sensor::Night, _) => nvg_body(colour),
        (Sensor::Ir, Tone::Flame) => flame_ir(colour),
        (Sensor::Ir, _) => ir_body(colour),
        (_, Tone::Plant) => cold_plant(colour),
        (_, Tone::Flame) => flame_heat(colour),
        (_, Tone::Warm) => heat_body(colour),
    }
}

fn soot(colour: u16) -> u16 {
    let y = luma(colour);
    let v = (if y % 3 == 0 { 2 } else { 1 }) as u16;
    v | (v << 5) | (v << 10)
}

fn on_list(sprite: &'static Sprite, tags: &[&agb::display::object::Tag]) -> bool {
    tags.iter().any(|tag| on_sheet(sprite, tag))
}

/// Sheets already drawn for one glass, such as the grey pines, are absent here
/// and keep the colours in the picture.
fn tone_of(sprite: &'static Sprite) -> Option<Tone> {
    if on_list(
        sprite,
        &[&sprites::GROVE, &sprites::GROVE_NEAR, &sprites::BUSH],
    ) {
        Some(Tone::Plant)
    } else if on_list(
        sprite,
        &[
            &sprites::SOLDIER,
            &sprites::SOLDIER_NEAR,
            &sprites::TANK,
            &sprites::TANK_NEAR,
            &sprites::TANK_TOP,
            &sprites::TRUCK,
            &sprites::TRUCK_NEAR,
            &sprites::TRUCK_TOP,
            &sprites::BUNKER,
            &sprites::BUNKER_NEAR,
            &sprites::BUNKER_TOP,
            &sprites::HQ,
            &sprites::HQ_NEAR,
            &sprites::FOE,
            &sprites::GERAN,
        ],
    ) {
        Some(Tone::Warm)
    } else if on_list(sprite, &[&sprites::BOOM, &sprites::SPARK, &sprites::FLASH]) {
        Some(Tone::Flame)
    } else {
        None
    }
}

pub(crate) fn heat_body(colour: u16) -> u16 {
    let y = luma(colour);
    // A hot ramp, not four flat bands, so a roof and a wheel stay different.
    let hot = (6 + y).clamp(6, 31) as u16;
    let g = (hot / 4).min(16);
    hot | (g << 5) | (1 << 10)
}

pub(crate) fn ir_body(colour: u16) -> u16 {
    let y = luma(colour);
    let v = (22 + y / 3).clamp(20, 31) as u16;
    v | (v << 5) | (v << 10)
}

/// One hot phosphor. Every index of the moon and the stars lands on it, so the
/// disc blooms instead of keeping its daylight blue.
pub(crate) fn nvg_lamp() -> Option<PaletteVram> {
    unsafe {
        let slot = &mut *NVG_LAMP.0.get();
        if slot.is_none() {
            let hot = Rgb::new(40, 248, 48).to_rgb15();
            let mut colours = [Rgb15(0); 16];
            colours[1..].fill(hot);
            let palette = PaletteVramSingle::try_allocate_new(&Palette16::new(colours)).ok()?;
            *slot = Some(palette.palette());
        }
        slot.clone()
    }
}

pub(crate) struct LampHold(core::cell::UnsafeCell<Option<PaletteVram>>);
unsafe impl Sync for LampHold {}
pub(crate) static NVG_LAMP: LampHold = LampHold(core::cell::UnsafeCell::new(None));

pub(crate) fn nvg_body(colour: u16) -> u16 {
    let y = luma(colour);
    let green = (18 + y / 2).clamp(16, 31) as u16;
    (green / 6) | (green << 5) | ((green / 5) << 10)
}

pub(crate) fn channels(colour: u16) -> (i32, i32, i32) {
    let r = (colour & 31) as i32;
    let g = ((colour >> 5) & 31) as i32;
    let b = ((colour >> 10) & 31) as i32;
    (r, g, b)
}

pub(crate) fn luma(colour: u16) -> i32 {
    let (r, g, b) = channels(colour);
    (r * 3 + g * 6 + b) / 10
}

pub(crate) fn glass_now() -> Sensor {
    unsafe { sensor_from((*LOOK.0.get()).sensor) }
}
