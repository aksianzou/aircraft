//! Above the horizon: the sun, the moon, the stars, the clouds and the skyline.

use agb::{
    display::{
        HEIGHT, Priority, WIDTH,
        object::{AffineMode, GraphicsMode, Object, ObjectAffine, PaletteVram, Sprite},
    },
    fixnum::Num,
};

use crate::sprites;

use super::*;

/// Horizon sprites and clouds blend into the sky and the ground, so the
/// skyline edge is a haze instead of a hard cut. HUD sprites stay opaque.
/// A positive bloom whites out the glass and then lets it back.
pub(crate) fn arm_haze(bloom: i32) {
    unsafe {
        if bloom > 0 {
            // The glass window is the only place blending is on, so the
            // instruments outside it stay sharp while the picture clips white.
            (0x0400_0050 as *mut u16).write_volatile(0x00B4);
            (0x0400_0054 as *mut u16).write_volatile(bloom.min(16) as u16);
        } else {
            // OBJ over backdrop and BG2, standard alpha. 10/16 sprite, 8/16 behind.
            (0x0400_0050 as *mut u16).write_volatile(0x2450);
            (0x0400_0052 as *mut u16).write_volatile(0x080A);
        }
    }
}

/// A distant backdrop only. It does not follow the field. Clouds and mist are
/// submitted first so they sit in front of the skyline and soften its edge.
/// The sun and the moon are behind that haze. Stars are the furthest layer.
pub(crate) fn draw_horizon(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    horizon: i32,
    sky_h: i32,
    cockpit: bool,
    sensor: Sensor,
) {
    if horizon < 12 {
        return;
    }
    let pan = world.yaw.to_raw();
    let alt_px = (world.alt >> 8).max(1);
    match sensor {
        Sensor::Night => {
            draw_moon(frame, world, horizon, cockpit);
            draw_stars(frame, world, horizon, cockpit);
        }
        Sensor::Day | Sensor::Winter => {
            draw_clouds(frame, pan / 3, sky_h, alt_px, cockpit);
            draw_mist(frame, pan / 2, sky_h, alt_px, cockpit);
            draw_skyline(frame, pan, sky_h, cockpit);
            draw_sun(frame, world, horizon, cockpit);
        }
        Sensor::Heat | Sensor::Ir => {}
    }
}

/// Where a fixed sky heading lands. `None` when it is behind the glass or
/// below the horizon.
pub(crate) fn sky_point(
    yaw: Num<i32, 8>,
    azimuth: i32,
    elev: i32,
    horizon: i32,
) -> Option<(i32, i32)> {
    let mut delta = azimuth - yaw.to_raw();
    delta = delta.rem_euclid(256);
    if delta > 128 {
        delta -= 256;
    }
    let ang = Num::<i32, 8>::new(delta) / Num::new(256);
    let ahead = ang.cos().to_raw();
    if ahead < 48 {
        return None;
    }
    let side = ang.sin().to_raw();
    let x = WIDTH / 2 + side * FOCAL / ahead;
    let y = horizon - elev;
    if y > horizon {
        return None;
    }
    Some((x, y))
}

/// Screen step from a body toward the ground, away from the sun or the moon.
/// Length is 8 pixels. The same direction is used for every shadow in the frame.
pub(crate) fn shadow_cast(yaw: Num<i32, 8>, horizon: i32, sensor: Sensor) -> (i32, i32) {
    let (azimuth, elev) = match sensor {
        Sensor::Night => (MOON_AZIMUTH, MOON_ELEV),
        Sensor::Day | Sensor::Heat | Sensor::Ir | Sensor::Winter => (SUN_AZIMUTH, SUN_ELEV),
    };
    let mut delta = azimuth - yaw.to_raw();
    delta = delta.rem_euclid(256);
    if delta > 128 {
        delta -= 256;
    }
    let ang = Num::<i32, 8>::new(delta) / Num::new(256);
    let ahead = ang.cos().to_raw();
    let side = ang.sin().to_raw();
    let ahead_d = if ahead.abs() < 16 {
        if ahead < 0 { -16 } else { 16 }
    } else {
        ahead
    };
    let lx = WIDTH / 2 + side * FOCAL / ahead_d;
    let ly = horizon - elev;
    let dx = WIDTH / 2 - lx;
    let dy = horizon + 12 - ly;
    let len = isqrt64(dx as i64 * dx as i64 + dy as i64 * dy as i64).max(1);
    ((dx as i64 * 8 / len) as i32, (dy as i64 * 8 / len) as i32)
}

pub(crate) fn draw_sun(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    horizon: i32,
    cockpit: bool,
) {
    let Some((x, y)) = sky_point(world.yaw, SUN_AZIMUTH, SUN_ELEV, horizon) else {
        return;
    };
    show_sky(
        frame,
        sprites::SUN.sprite(0),
        (x - 16, y - 16),
        (32, 32),
        cockpit,
        horizon,
        None,
    );
}

pub(crate) fn draw_moon(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    horizon: i32,
    cockpit: bool,
) {
    let Some((x, y)) = sky_point(world.yaw, MOON_AZIMUTH, MOON_ELEV, horizon) else {
        return;
    };
    // The tube clips the disc. Craters are gone; it is one hot phosphor spot.
    show_sky(
        frame,
        sprites::MOON.sprite(0),
        (x - 16, y - 16),
        (32, 32),
        cockpit,
        horizon,
        nvg_lamp(),
    );
}

/// Sixteen fixed lights. A few of the dim ones blink, so the field is not a stamp.
pub(crate) fn draw_stars(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    horizon: i32,
    cockpit: bool,
) {
    // Headings stay inside the forward glass on a northbound run. A few sit
    // higher, so they only come in when the nose comes up.
    const STARS: [(i32, i32); 16] = [
        (4, 12),
        (14, 26),
        (24, 16),
        (34, 32),
        (44, 20),
        (252, 18),
        (242, 30),
        (232, 14),
        (222, 28),
        (212, 22),
        (8, 36),
        (248, 34),
        (18, 10),
        (238, 12),
        (30, 40),
        (226, 38),
    ];
    for (index, (azimuth, elev)) in STARS.into_iter().enumerate() {
        let dim = index % 3 != 0;
        if dim && hash(world.age >> 4, index as i32).is_multiple_of(6) {
            continue;
        }
        let Some((x, y)) = sky_point(world.yaw, azimuth, elev, horizon) else {
            continue;
        };
        let frame_i = if dim { 0 } else { 1 };
        show_sky(
            frame,
            sprites::STAR.sprite(frame_i),
            (x - 4, y - 4),
            (8, 8),
            cockpit,
            horizon,
            nvg_lamp(),
        );
    }
}

/// Lens wash on the glass, in front of the field. The moon does it properly.
/// The sun only flashes if you put it on the crosshair.
pub(crate) fn draw_glass_glare(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    horizon: i32,
    sensor: Sensor,
) {
    let lamp = match sensor {
        Sensor::Night => nvg_lamp(),
        Sensor::Day | Sensor::Heat | Sensor::Ir | Sensor::Winter => None,
    };
    let (azimuth, elev, reach, hard) = match sensor {
        Sensor::Night => (MOON_AZIMUTH, MOON_ELEV, 40, 26),
        Sensor::Day | Sensor::Winter => (SUN_AZIMUTH, SUN_ELEV, 16, 10),
        Sensor::Heat | Sensor::Ir => return,
    };
    let Some((x, y)) = sky_point(world.yaw, azimuth, elev, horizon) else {
        return;
    };
    let cx = VIEW_X + VIEW_W / 2;
    let cy = VIEW_Y + VIEW_H / 2;
    let dx = x - cx;
    let dy = y - cy;
    let dist = dx * dx + dy * dy;
    if dist < hard * hard {
        // A modest white disc. The flash art is a hard circle, so this stays small.
        show_veil(frame, x, y, 2, lamp.clone());
    } else if dist < reach * reach {
        show_veil(frame, x, y, 4, lamp.clone());
    } else {
        return;
    }
    if !matches!(sensor, Sensor::Night) {
        return;
    }
    // One ghost across the glass, on the line from the moon through the sight.
    // It sits on the picture, including over the ground, so it is not a sky sprite.
    let gx = cx + (cx - x) / 2 - 4;
    let gy = cy + (cy - y) / 2 - 4;
    if gx >= VIEW_X && gy >= VIEW_Y && gx + 8 <= VIEW_X + VIEW_W && gy + 8 <= VIEW_Y + VIEW_H {
        let mut ghost = Object::new(vram(sprites::STAR.sprite(1)));
        if let Some(palette) = lamp {
            ghost.set_palette(palette);
        }
        ghost
            .set_pos((gx, gy))
            .set_priority(Priority::P0)
            .set_graphics_mode(GraphicsMode::AlphaBlending)
            .show(frame);
    }
}

pub(crate) fn show_veil(
    frame: &mut agb::display::GraphicsFrame,
    cx: i32,
    cy: i32,
    denom: i32,
    tint: Option<PaletteVram>,
) {
    // Inverse scale: denom 2 draws the 64 circle at 32px, in the top-left of the box.
    let vis = 64 / denom;
    let x = cx - vis / 2;
    let y = cy - vis / 2;
    if x < VIEW_X || y < VIEW_Y || x + 64 > VIEW_X + VIEW_W || y + 64 > VIEW_Y + VIEW_H {
        return;
    }
    let inv = Num::<i32, 8>::new(denom);
    let mut veil = ObjectAffine::new(
        vram(sprites::FLASH.sprite(0)),
        scale_matrix(inv, inv),
        AffineMode::Affine,
    );
    if let Some(palette) = tint {
        veil.set_palette(palette);
    }
    veil.set_pos((x, y))
        .set_priority(Priority::P0)
        .set_graphics_mode(GraphicsMode::AlphaBlending)
        .show(frame);
}

pub(crate) fn show_sky(
    frame: &mut agb::display::GraphicsFrame,
    sprite: &'static Sprite,
    (x, y): (i32, i32),
    (w, h): (i32, i32),
    cockpit: bool,
    horizon: i32,
    tint: Option<PaletteVram>,
) {
    if y + h / 2 > horizon || x > WIDTH || y > HEIGHT || x + w < 0 || y + h < 0 {
        return;
    }
    // The whole sprite stays inside the glass. A disc on the bezel paints over the instruments.
    if cockpit && (x < VIEW_X || y < VIEW_Y || x + w > VIEW_X + VIEW_W || y + h > VIEW_Y + VIEW_H) {
        return;
    }
    let mut object = Object::new(vram(sprite));
    if let Some(palette) = tint {
        object.set_palette(palette);
    }
    object
        .set_pos((x, y))
        .set_priority(Priority::P0)
        .show(frame);
}

pub(crate) fn draw_skyline(
    frame: &mut agb::display::GraphicsFrame,
    scroll: i32,
    sky_h: i32,
    cockpit: bool,
) {
    // A dive has no sky band, so the silhouettes stay off the screen.
    if sky_h < 24 {
        return;
    }
    let spacing = 96;
    let first = scroll.div_euclid(spacing) * spacing;
    // 16px tall. The drawing sits on the bottom edge, on the horizon.
    let y = sky_h - 15;
    for step in 0..4 {
        let slot = first + (step - 1) * spacing;
        let n = (hash(slot.div_euclid(spacing), 1) % 8) as usize;
        let lift = (hash(slot.div_euclid(spacing), 3) % 3) as i32;
        show_haze(
            frame,
            sprites::HORIZON.sprite(n),
            slot - scroll,
            y - lift,
            cockpit,
        );
    }
}

pub(crate) fn draw_clouds(
    frame: &mut agb::display::GraphicsFrame,
    scroll: i32,
    sky_h: i32,
    alt_px: i32,
    cockpit: bool,
) {
    // Stay above the mist bank so the two layers do not share a scanline.
    let room = sky_h - 36;
    if room < 20 {
        return;
    }
    let count = if alt_px >= 24 {
        6
    } else if alt_px >= 12 {
        4
    } else {
        3
    };
    let spacing = 80;
    let first = scroll.div_euclid(spacing) * spacing;
    let top = if cockpit { VIEW_Y + 2 } else { 4 };
    let bottom = if cockpit {
        room.min(VIEW_Y + VIEW_H - 18)
    } else {
        room
    };
    if bottom <= top + 8 {
        return;
    }
    let span = (bottom - top).max(1) as u32;
    for step in 0..count {
        let slot = first + (step - 2) * spacing;
        let n = hash(slot.div_euclid(spacing), 9 + step);
        let y = top + (n % span) as i32;
        show_haze(
            frame,
            sprites::CLOUD.sprite((n % 2) as usize),
            slot - scroll,
            y,
            cockpit,
        );
    }
}

/// A pale bank on the horizon. It covers the cut between the sky and the
/// silhouettes so the skyline does not read as a pasted shape.
pub(crate) fn draw_mist(
    frame: &mut agb::display::GraphicsFrame,
    scroll: i32,
    sky_h: i32,
    alt_px: i32,
    cockpit: bool,
) {
    if sky_h < 28 {
        return;
    }
    let count = 3;
    let spacing = 72;
    let first = scroll.div_euclid(spacing) * spacing;
    let y = sky_h - 14 - (alt_px / 12).min(6);
    for step in 0..count {
        let slot = first + (step - 1) * spacing;
        let n = hash(slot.div_euclid(spacing), 5);
        let frame_i = 2 + (n % 2) as usize;
        let bob = (n % 3) as i32;
        show_haze(
            frame,
            sprites::CLOUD.sprite(frame_i),
            slot - scroll,
            y - bob,
            cockpit,
        );
    }
}

pub(crate) fn sky_bottom(horizon: i32, alt: i32) -> i32 {
    let alt_px = (alt >> 8).max(1);
    let skip = (alt_px * FLOOR_ZOOM * 25 / REACH).max(12);
    (horizon + skip).clamp(horizon + 8, HEIGHT).max(0)
}
