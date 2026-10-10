use agb::{
    display::{
        AffineMatrix, HEIGHT, Palette16, Priority, Rgb, Rgb15, WIDTH, WinIn,
        object::{
            AffineMode, GraphicsMode, Object, ObjectAffine, PaletteVram, PaletteVramSingle, Sprite,
            SpriteVram,
        },
        tiled::AffineBackgroundId,
    },
    fixnum::{Num, num, rect, vec2},
    interrupt::{self, Interrupt},
};

use crate::backgrounds;
use crate::sprites;
use crate::world::*;

pub(crate) fn mask_view(
    frame: &mut agb::display::GraphicsFrame,
    ground_id: AffineBackgroundId,
    sky_h: i32,
    cockpit: bool,
    night: bool,
    wash: bool,
) {
    if !cockpit {
        {
            let sky = frame.windows().win_in(WinIn::Win0).enable_objects();
            if night || wash {
                sky.enable_blending();
            }
            sky.set_pos(rect(vec2(0, 0), vec2(WIDTH, sky_h)));
        }
        let outside = frame
            .windows()
            .win_out()
            .enable_background(ground_id)
            .enable_objects();
        if night {
            outside.enable_blending();
        }
        return;
    }
    let view_bot = VIEW_Y + VIEW_H;
    let sky_bot = sky_h.clamp(VIEW_Y, view_bot);
    let sky_span = sky_bot - VIEW_Y;
    if sky_span > 0 {
        let sky = frame.windows().win_in(WinIn::Win1).enable_objects();
        if night || wash {
            sky.enable_blending();
        }
        sky.set_pos(rect(vec2(VIEW_X, VIEW_Y), vec2(VIEW_W, sky_span)));
    }
    let ground_span = view_bot - sky_bot;
    if ground_span > 0 {
        let ground = frame
            .windows()
            .win_in(WinIn::Win0)
            .enable_objects()
            .enable_background(ground_id);
        if night || wash {
            ground.enable_blending();
        }
        ground.set_pos(rect(vec2(VIEW_X, sky_bot), vec2(VIEW_W, ground_span)));
    }
    let outside = frame.windows().win_out().enable_objects();
    if night {
        outside.enable_blending();
    }
}

pub(crate) fn clear_ground_map() {
    unsafe {
        let map = 0x0600_C000 as *mut u8;
        for cell in 0..(128 * 128) {
            map.add(cell).write_volatile(T_FLOOR + 1);
        }
    }
}

/// BG2 control: priority 1, screenblock 24, wrap, size 128×128.
pub(crate) fn arm_ground() {
    unsafe {
        (0x0400_000C as *mut u16).write_volatile(0xF801);
    }
}

/// Drop the ground layer at once. Map writes during a scanline tear the
/// picture, so a load hides the floor before it touches the table.
pub(crate) fn hide_ground() {
    unsafe {
        let disp = 0x0400_0000 as *mut u16;
        let cur = disp.read_volatile();
        disp.write_volatile(cur & !(1 << 10));
    }
}

/// One card for every mission. The field loads behind it, so changing
/// sortie does not swap the picture for another splash.
pub(crate) fn draw_select(
    frame: &mut agb::display::GraphicsFrame,
    level: u8,
    marks: u8,
    can_left: bool,
    can_right: bool,
    age: i32,
    sensor: Sensor,
    glass: Option<u8>,
) {
    match sensor {
        Sensor::Night => {
            show(frame, sprites::MOON.sprite(0), 176, 4);
            for (x, y) in [(12, 8), (36, 22), (168, 28), (200, 14), (148, 6)] {
                show(frame, sprites::STAR.sprite(0), x, y);
            }
        }
        Sensor::Day | Sensor::Winter => {
            show(frame, sprites::CLOUD.sprite(0), 8, 16);
            show(frame, sprites::CLOUD.sprite(1), 148, 8);
            show(frame, sprites::SUN.sprite(0), 176, 4);
        }
        Sensor::Heat | Sensor::Ir => {}
    }
    draw_digits(frame, level as i32 + 1, 2, 112, 48);
    let (top, bottom) = mission_title(level);
    show_lockup(frame, top, bottom, 62);
    show_stars(frame, marks, 84);
    if let Some(which) = glass {
        show(frame, sprites::GLASS.sprite(which as usize), 104, 104);
    }
    let step = (age / 6).rem_euclid(6);
    let nudge = if step < 4 { step } else { 6 - step };
    if can_left {
        show_flip(frame, sprites::ARROW.sprite(0), 4 - nudge, 66, true, false);
    }
    if can_right {
        show_flip(frame, sprites::ARROW.sprite(0), 220 + nudge, 66, false, false);
    }
    draw_load_land(frame, 0);
}

fn mission_title(level: u8) -> (&'static [u8], &'static [u8]) {
    match task_of(level) {
        Task::Officer => (b"OFFICER", b"HUNT"),
        Task::Trench => (b"TRENCH", b"LINE"),
        Task::Hq => (b"HOUSE", b"BOMB"),
        Task::Convoy => (b"KILL", b"TRUCK"),
        Task::Night => (b"NIGHT", b"SIGHT"),
        Task::Heat => (b"HEAT", b"SCAN"),
        Task::Gate => (b"THE", b"GATE"),
        Task::Guide => (b"FIRE", b"MARK"),
        Task::Column => (b"COLUMN", b"MARK"),
        Task::Armor => (b"ARMOR", b"HUNT"),
        Task::Pill => (b"PILL", b"BOX"),
        Task::Fpv => (b"LAST", b"DIVE"),
        Task::Lane => (b"THE", b"LANE"),
        Task::Rush => (b"ON", b"FOOT"),
        Task::Flare => (b"NIGHT", b"LANE"),
        Task::Nest => (b"OPEN", b"NEST"),
        Task::Dart => (b"HEAT", b"DART"),
        Task::Push => (b"THE", b"PUSH"),
        Task::Skirt => (b"PAST", b"GUNS"),
        Task::Bend => (b"LOW", b"ROAD"),
        Task::Fetch if haul_card(level) => (b"LONG", b"HAUL"),
        Task::Fetch => (b"ONE", b"CHARGE"),
        Task::Swarm if defend_card(level) => (b"THE", b"CROWS"),
        Task::Swarm => (b"THE", b"SWARM"),
        Task::Cover => (b"COVER", b"FIRE"),
    }
}

/// Both words on one line, with a gap, so the card is a title and not a stack.
fn show_lockup(
    frame: &mut agb::display::GraphicsFrame,
    top: &[u8],
    bottom: &[u8],
    y: i32,
) {
    let measure = |text: &[u8]| -> i32 {
        text.iter()
            .map(|&ch| if ch == b' ' { 8 } else { 15 })
            .sum()
    };
    let mut x = (WIDTH - (measure(top) + 10 + measure(bottom))) / 2;
    for (index, text) in [top, bottom].into_iter().enumerate() {
        for &ch in text {
            if ch != b' ' {
                show(frame, sprites::FONT.sprite(glyph(ch)), x - 1, y);
            }
            x += if ch == b' ' { 8 } else { 15 };
        }
        if index == 0 {
            x += 10;
        }
    }
}

/// The outline face. Cells are 16 wide; the letter itself sits inside that.
pub(crate) fn show_face(frame: &mut agb::display::GraphicsFrame, text: &[u8], y: i32) {
    let mut width = 0;
    for &ch in text {
        width += if ch == b' ' { 8 } else { 14 };
    }
    let mut x = (WIDTH - width) / 2;
    for &ch in text {
        let adv = if ch == b' ' { 8 } else { 14 };
        if ch != b' ' {
            show(frame, sprites::FONT.sprite(glyph(ch)), x - 1, y);
        }
        x += adv;
    }
}

fn show_stars(frame: &mut agb::display::GraphicsFrame, marks: u8, y: i32) {
    let x = (WIDTH - 56) / 2;
    for index in 0..3 {
        let which = if marks & (1 << index) != 0 { 1 } else { 0 };
        show(frame, sprites::MEDAL.sprite(which), x + index * 20, y);
    }
}

fn draw_load_land(frame: &mut agb::display::GraphicsFrame, pan: i32) {
    let scroll = pan / 3;
    let spacing = 28;
    let first = scroll.div_euclid(spacing) * spacing;
    let mut x = first - scroll - spacing;
    while x < WIDTH {
        let n = (hash((x + scroll) / spacing, 1) % 8) as usize;
        let lift = (hash((x + scroll) / spacing, 3) % 4) as i32;
        show_haze(frame, sprites::HORIZON.sprite(n), x, 146 - lift, false);
        x += spacing;
    }
}

#[allow(dead_code)]
pub(crate) fn mission_card(level: u8) -> &'static Sprite {
    match task_of(level) {
        Task::Trench => sprites::LVL_TRENCH.sprite(0),
        Task::Officer => sprites::LVL_OFFICER.sprite(0),
        Task::Night => sprites::LVL_NIGHT.sprite(0),
        Task::Heat => sprites::LVL_HEAT.sprite(0),
        Task::Gate => sprites::LVL_GATE.sprite(0),
        Task::Fpv => sprites::LVL_FPV.sprite(0),
        Task::Convoy => sprites::LVL_CONVOY.sprite(0),
        Task::Hq => sprites::LVL_HQ.sprite(0),
        Task::Guide => sprites::LVL_GUIDE.sprite(0),
        Task::Armor => sprites::LVL_ARMOR.sprite(0),
        Task::Pill => sprites::LVL_PILL.sprite(0),
        Task::Column => sprites::LVL_COLUMN.sprite(0),
        Task::Rush => sprites::LVL_RUSH.sprite(0),
        Task::Push => sprites::LVL_PUSH.sprite(0),
        Task::Lane => sprites::LVL_LANE.sprite(0),
        Task::Flare => sprites::LVL_FLARE.sprite(0),
        Task::Nest => sprites::LVL_NEST.sprite(0),
        Task::Dart => sprites::LVL_DART.sprite(0),
        Task::Bend => sprites::LVL_BEND.sprite(0),
        Task::Skirt => sprites::LVL_SKIRT.sprite(0),
        Task::Fetch if haul_card(level) => sprites::LVL_HAUL.sprite(0),
        Task::Fetch => sprites::LVL_LIFT.sprite(0),
        Task::Swarm if defend_card(level) => sprites::LVL_CROWS.sprite(0),
        Task::Swarm => sprites::LVL_SWARM.sprite(0),
        Task::Cover => sprites::LVL_COVER.sprite(0),
    }
}

#[allow(dead_code)]
pub(crate) fn mission_help(level: u8) -> &'static Sprite {
    match task_of(level) {
        Task::Fpv => sprites::HELP_FPV.sprite(0),
        Task::Gate => sprites::HELP_GATE.sprite(0),
        Task::Convoy => sprites::HELP_CONVOY.sprite(0),
        Task::Heat => sprites::HELP_HEAT.sprite(0),
        Task::Guide => sprites::HELP_GUIDE.sprite(0),
        Task::Column => sprites::HELP_COLUMN.sprite(0),
        Task::Rush => sprites::HELP_RUSH.sprite(0),
        Task::Push => sprites::HELP_PUSH.sprite(0),
        Task::Lane => sprites::HELP_LANE.sprite(0),
        Task::Flare => sprites::HELP_FLARE.sprite(0),
        Task::Nest => sprites::HELP_NEST.sprite(0),
        Task::Dart => sprites::HELP_DART.sprite(0),
        Task::Bend => sprites::HELP_BEND.sprite(0),
        Task::Skirt => sprites::HELP_SKIRT.sprite(0),
        Task::Fetch if haul_card(level) => sprites::HELP_HAUL.sprite(0),
        Task::Fetch => sprites::HELP_LIFT.sprite(0),
        Task::Swarm if defend_card(level) => sprites::HELP_CROWS.sprite(0),
        Task::Swarm => sprites::HELP_SWARM.sprite(0),
        Task::Cover => sprites::HELP_COVER.sprite(0),
        Task::Armor => sprites::HELP_ARMOR.sprite(0),
        Task::Pill => sprites::HELP_PILL.sprite(0),
        _ => sprites::HELP_QUAD.sprite(0),
    }
}

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
pub(crate) fn sky_point(yaw: Num<i32, 8>, azimuth: i32, elev: i32, horizon: i32) -> Option<(i32, i32)> {
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
    (
        (dx as i64 * 8 / len) as i32,
        (dy as i64 * 8 / len) as i32,
    )
}

pub(crate) fn draw_sun(frame: &mut agb::display::GraphicsFrame, world: &World, horizon: i32, cockpit: bool) {
    let Some((x, y)) = sky_point(world.yaw, SUN_AZIMUTH, SUN_ELEV, horizon) else {
        return;
    };
    show_sky(frame, sprites::SUN.sprite(0), x - 16, y - 16, 32, 32, cockpit, horizon, None);
}

pub(crate) fn draw_moon(frame: &mut agb::display::GraphicsFrame, world: &World, horizon: i32, cockpit: bool) {
    let Some((x, y)) = sky_point(world.yaw, MOON_AZIMUTH, MOON_ELEV, horizon) else {
        return;
    };
    // The tube clips the disc. Craters are gone; it is one hot phosphor spot.
    show_sky(
        frame,
        sprites::MOON.sprite(0),
        x - 16,
        y - 16,
        32,
        32,
        cockpit,
        horizon,
        nvg_lamp(),
    );
}

/// Sixteen fixed lights. A few of the dim ones blink, so the field is not a stamp.
pub(crate) fn draw_stars(frame: &mut agb::display::GraphicsFrame, world: &World, horizon: i32, cockpit: bool) {
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
        if dim && hash(world.age >> 4, index as i32) % 6 == 0 {
            continue;
        }
        let Some((x, y)) = sky_point(world.yaw, azimuth, elev, horizon) else {
            continue;
        };
        let frame_i = if dim { 0 } else { 1 };
        show_sky(
            frame,
            sprites::STAR.sprite(frame_i),
            x - 4,
            y - 4,
            8,
            8,
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
        let mut ghost = Object::new(SpriteVram::from(sprites::STAR.sprite(1)));
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
        SpriteVram::from(sprites::FLASH.sprite(0)),
        AffineMatrix::from_scale(vec2(inv, inv)).into(),
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
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    cockpit: bool,
    horizon: i32,
    tint: Option<PaletteVram>,
) {
    if y + h / 2 > horizon || x > WIDTH || y > HEIGHT || x + w < 0 || y + h < 0 {
        return;
    }
    // The whole sprite stays inside the glass. A disc on the bezel paints over the instruments.
    if cockpit && (x < VIEW_X || y < VIEW_Y || x + w > VIEW_X + VIEW_W || y + h > VIEW_Y + VIEW_H)
    {
        return;
    }
    let mut object = Object::new(SpriteVram::from(sprite));
    if let Some(palette) = tint {
        object.set_palette(palette);
    }
    object.set_pos((x, y)).set_priority(Priority::P0).show(frame);
}

pub(crate) fn draw_skyline(frame: &mut agb::display::GraphicsFrame, scroll: i32, sky_h: i32, cockpit: bool) {
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
    let count = if alt_px >= 24 { 6 } else if alt_px >= 12 { 4 } else { 3 };
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

/// The displayed map is 128 tiles, 1024 pixels. The hardware wraps that, but a
/// negative reference does not wrap: it shows the sky colour as one scanline.
/// Two extra rings keep every visible line positive without moving the tile.
pub(crate) fn ring_space(sub: i32) -> i32 {
    const RING: i32 = VIEW * 8 * 256;
    sub.rem_euclid(RING) + RING * 2
}

pub(crate) struct Nadir(core::cell::UnsafeCell<i32>);
unsafe impl Sync for Nadir {}
pub(crate) static NADIR: Nadir = Nadir(core::cell::UnsafeCell::new(0));

pub(crate) fn set_nadir(scale: i32) {
    unsafe { *NADIR.0.get() = scale }
}

pub(crate) fn nadir_on() -> i32 {
    unsafe { *NADIR.0.get() }
}

/// World pixels per screen pixel while the glass looks straight down.
pub(crate) fn nadir_scale(alt: i32) -> i32 {
    ((alt >> 8).max(8) / 8).clamp(1, 4)
}

/// Flat glass. Scanline 88, pixel 120 is the ground under the airframe.
pub(crate) fn floor_nadir(
    origin_x: Num<i32, 8>,
    origin_y: Num<i32, 8>,
    sin: Num<i32, 8>,
    cos: Num<i32, 8>,
    scanline: i32,
    scale: i32,
) -> FloorLine {
    let step_x = (cos.to_raw() * scale).clamp(-32767, 32767) as i16;
    let step_y = (sin.to_raw() * scale).clamp(-32767, 32767) as i16;
    let right = Num::<i32, 8>::new(-120 * scale);
    let ahead = Num::<i32, 8>::new((88 - scanline) * scale);
    let horizontal = origin_x + right * cos + ahead * sin;
    let vertical = origin_y + right * sin - ahead * cos;
    FloorLine {
        a: step_x,
        b: 0,
        c: step_y,
        d: 0,
        x: horizontal.to_raw(),
        y: vertical.to_raw(),
    }
}

pub(crate) fn floor_line(
    origin_x: Num<i32, 8>,
    origin_y: Num<i32, 8>,
    height: Num<i32, 8>,
    sin: Num<i32, 8>,
    cos: Num<i32, 8>,
    scanline: i32,
    horizon: i32,
) -> (Num<i32, 8>, Num<i32, 8>, Num<i32, 8>, Num<i32, 8>) {
    let rel = (scanline - horizon).max(1);
    let mut lambda = height * Num::new(FLOOR_ZOOM) / Num::new(rel);
    if lambda > num!(8) {
        lambda = num!(8);
    }
    // One perspective for the whole line. A straight road then stays a
    // straight line on screen. Changing the distance without the scale bends it.
    let along = lambda * Num::new(FOCAL);
    let lcf = lambda * cos;
    let lsf = lambda * sin;
    // Screen centre looks along (sin, -cos). Pixel 0 is the left edge.
    let horizontal = origin_x - lcf * Num::new(WIDTH / 2) + sin * along;
    let vertical = origin_y - lsf * Num::new(WIDTH / 2) - cos * along;
    (horizontal, vertical, lcf, lsf)
}

/// One scanline of the mode 7 floor, in register units (8.8 fixed point).
/// Layout matches BG2PA..BG2Y so a DMA can copy a line in one burst.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct FloorLine {
    pub(crate) a: i16,
    pub(crate) b: i16,
    pub(crate) c: i16,
    pub(crate) d: i16,
    pub(crate) x: i32,
    pub(crate) y: i32,
}

pub(crate) const FLAT_LINE: FloorLine = FloorLine {
    a: 0x100,
    b: 0,
    c: 0,
    d: 0,
    x: 0,
    y: 0,
};

pub(crate) struct FloorBuf {
    pub(crate) front: core::cell::UnsafeCell<usize>,
    pub(crate) lines: core::cell::UnsafeCell<[[FloorLine; 162]; 2]>,
}

unsafe impl Sync for FloorBuf {}

#[unsafe(link_section = ".iwram")]
pub(crate) static FLOOR: FloorBuf = FloorBuf {
    front: core::cell::UnsafeCell::new(0),
    lines: core::cell::UnsafeCell::new([[FLAT_LINE; 162]; 2]),
};

pub(crate) fn write_floor_line(line: &FloorLine) {
    unsafe {
        (0x0400_0020 as *mut u16).write_volatile(line.a as u16);
        (0x0400_0022 as *mut u16).write_volatile(line.b as u16);
        (0x0400_0024 as *mut u16).write_volatile(line.c as u16);
        (0x0400_0026 as *mut u16).write_volatile(line.d as u16);
        (0x0400_0028 as *mut i32).write_volatile(line.x);
        (0x0400_002C as *mut i32).write_volatile(line.y);
    }
}

/// Four words, destination reloads, source walks forward, repeat on HBlank.
pub(crate) const FLOOR_DMA: u32 = 4
    | (0b11 << 21)
    | (1 << 25)
    | (1 << 26)
    | (0b10 << 28)
    | (1 << 31);

/// The sound mixer takes an interrupt every frame. An HBlank interrupt then
/// misses lines and the ground sticks. DMA0 copies the line in hardware.
pub(crate) fn install_floor() {
    let stop = unsafe {
        interrupt::add_interrupt_handler(Interrupt::VBlank, |_| {
            // Before the first line. A palette written on a live scanline
            // stripes the whole floor.
            flush_look();
            (0x0400_00BA as *mut u16).write_volatile(0);
        })
    };
    let start = unsafe {
        interrupt::add_interrupt_handler(Interrupt::VCounter, |_| {
            arm_floor_dma();
        })
    };
    core::mem::forget(stop);
    core::mem::forget(start);
    unsafe {
        let stat = 0x0400_0004 as *mut u16;
        let cur = stat.read_volatile();
        stat.write_volatile((cur & 0x00FF) | (1 << 5));
    }
}

pub(crate) fn arm_floor_dma() {
    unsafe {
        let front = *FLOOR.front.get();
        let lines = &(*FLOOR.lines.get())[front];
        write_floor_line(&lines[0]);
        (0x0400_00B0 as *mut u32).write_volatile(lines.as_ptr().add(1) as u32);
        (0x0400_00B4 as *mut u32).write_volatile(0x0400_0020);
        (0x0400_00B8 as *mut u32).write_volatile(FLOOR_DMA);
    }
}

pub(crate) fn publish_floor() {
    unsafe {
        let front = FLOOR.front.get();
        *front ^= 1;
        write_floor_line(&(*FLOOR.lines.get())[*front][0]);
    }
}

pub(crate) fn build_floor(cam_x: i32, cam_y: i32, alt: i32, yaw: Num<i32, 8>, horizon: i32) {
    let height = Num::<i32, 8>::from_raw(alt);
    let origin_x = Num::<i32, 8>::from_raw(cam_x);
    let origin_y = Num::<i32, 8>::from_raw(cam_y);
    let cos = yaw.cos();
    let sin = yaw.sin();
    let down = nadir_on();
    unsafe {
        let back = *FLOOR.front.get() ^ 1;
        let buf = &mut (*FLOOR.lines.get())[back];
        for scanline in 0..HEIGHT {
            // Absolute position for this line. pb and pd stay 0: the beam
            // must not take an extra step after we write the point.
            buf[scanline as usize] = if down > 0 {
                floor_nadir(origin_x, origin_y, sin, cos, scanline, down)
            } else {
                let (horizontal, vertical, lcf, lsf) =
                    floor_line(origin_x, origin_y, height, sin, cos, scanline, horizon);
                FloorLine {
                    a: narrow(lcf).to_raw(),
                    b: 0,
                    c: narrow(lsf).to_raw(),
                    d: 0,
                    x: horizontal.to_raw(),
                    y: vertical.to_raw(),
                }
            };
        }
        buf[160] = buf[159];
        buf[161] = buf[159];
    }
}

pub(crate) fn narrow(value: Num<i32, 8>) -> Num<i16, 8> {
    let limit = Num::<i32, 8>::new(100);
    let value = if value > limit {
        limit
    } else if value < -limit {
        -limit
    } else {
        value
    };
    value.try_change_base().unwrap_or(Num::new(0))
}

pub(crate) fn project(
    cam_x: i32,
    cam_y: i32,
    cam_z: i32,
    sin_r: i32,
    cos_r: i32,
    horizon: i32,
    world_x: i32,
    world_y: i32,
    world_z: i32,
) -> Option<(i32, i32)> {
    let dx = delta(world_x, cam_x);
    let dy = delta(world_y, cam_y);
    let fwd = ((dx * sin_r) - (dy * cos_r)) >> 8;
    let right = ((dx * cos_r) + (dy * sin_r)) >> 8;
    if let scale = nadir_on()
        && scale > 0
    {
        let sx = WIDTH / 2 + (right >> 8) / scale;
        // Height is the same quantity the forward glass uses. Looking down,
        // it lifts the sprite off the map point instead of dropping it into
        // the crosshair.
        let sy = 88 - (fwd >> 8) / scale - (world_z >> 8) / scale;
        if !(-32..WIDTH + 32).contains(&sx) || !(-32..HEIGHT + 32).contains(&sy) {
            return None;
        }
        return Some((sx, sy));
    }
    if fwd < 192 {
        return None;
    }
    let sy = horizon + (cam_z - world_z) * FOCAL * FLOOR_ZOOM / fwd;
    let sx = WIDTH / 2 + right * FOCAL / fwd;
    // Feet may sit below the glass while the body is still in it.
    if !(-32..WIDTH + 32).contains(&sx) || !(-32..HEIGHT + 160).contains(&sy) {
        return None;
    }
    Some((sx, sy))
}

/// Top of the ground window. Above this the floor background is not on.
pub(crate) fn ground_line(horizon: i32, alt: i32, cockpit: bool) -> i32 {
    let floor = sky_bottom(horizon, alt);
    if cockpit {
        floor.clamp(VIEW_Y, VIEW_Y + VIEW_H)
    } else {
        floor.max(0)
    }
}

pub(crate) fn marks_on_ground(sy: i32, ground_y: i32, cockpit: bool) -> bool {
    cockpit && ground_y <= VIEW_Y || sy - 26 >= ground_y
}

/// Screen height of a body, in the same scale as one mode 7 scanline.
fn ground_span(drop: i32, alt: i32, world_h: i32) -> i32 {
    let alt_px = (alt >> 8).max(1);
    let scale = nadir_on();
    let span = if scale > 0 {
        world_h * 2 / scale
    } else {
        world_h * drop / (alt_px * FLOOR_ZOOM)
    };
    span.max(2)
}

/// Feet may leave the bottom of the glass. The body stays while any of it
/// is still on the picture, and the roof may stand up into the sky.
fn actor_on_glass(sx: i32, sy: i32, span: i32, floor: i32, cockpit: bool) -> bool {
    if cockpit && (sx < VIEW_X || sx >= VIEW_X + VIEW_W) {
        return false;
    }
    if sy < floor {
        return false;
    }
    let view_bot = if cockpit { VIEW_Y + VIEW_H } else { HEIGHT };
    sy - span < view_bot
}

/// Feet stay on the ground point. Past the native size the double-size sprite
/// takes over, and its centre is still the middle of the art.
fn place_actor(
    sx: i32,
    sy: i32,
    native_w: i32,
    native_h: i32,
    display_h: i32,
    foot: i32,
    top: bool,
) -> (i32, i32, AffineMode) {
    let double = display_h > native_h;
    let mode = if double {
        AffineMode::AffineDouble
    } else {
        AffineMode::Affine
    };
    let half_x = if double { native_w } else { native_w / 2 };
    let half_y = if double { native_h } else { native_h / 2 };
    let center_y = if top {
        sy
    } else {
        sy - (foot - native_h / 2) * display_h / native_h
    };
    (sx - half_x, center_y - half_y, mode)
}

fn show_figure(
    frame: &mut agb::display::GraphicsFrame,
    sprite: &'static Sprite,
    sx: i32,
    sy: i32,
    native_w: i32,
    native_h: i32,
    ink: i32,
    foot: i32,
    span: i32,
    top: bool,
    sensor: Sensor,
    recolor: bool,
) {
    let display = (span.max(1) * native_h / ink.max(1)).clamp(4, native_h * 2);
    let inv = Num::<i32, 8>::new(native_h) / Num::new(display.max(1));
    let (x, y, mode) = place_actor(sx, sy, native_w, native_h, display, foot, top);
    if x < -native_w * 2 || y < -native_h * 2 || x > WIDTH || y > HEIGHT {
        return;
    }
    let mut object = ObjectAffine::new(SpriteVram::from(sprite), actor_matrix(inv).into(), mode);
    // Only a sheet baked from its own colours. The shared body palette
    // maps a truck's indices onto someone else's ramp, and the model breaks.
    if let Some(palette) = sheet_palette(sprite, sensor) {
        object.set_palette(palette);
    }
    let _ = recolor;
    let _ = body_palette;
    object
        .set_pos((x, y))
        .set_priority(Priority::P0)
        .show(frame);
}

/// Front, right, rear, left. The close sheets are drawn in that order.
fn four_view(oct: usize) -> usize {
    match level_octant(oct) {
        2 => 1,
        4 => 2,
        6 => 3,
        _ => 0,
    }
}

/// Front, side and rear only. Diagonal headings share the nearer pure side.
pub(crate) fn level_octant(oct: usize) -> usize {
    match oct & 7 {
        1 | 2 | 3 => 2,
        5 | 6 | 7 => 6,
        other => other,
    }
}

/// Eight headings around the object, then the same eight from a steeper dive.
pub(crate) fn heading_octant(fwd: i32, right: i32) -> usize {
    // +x faces screen-right, +y faces the camera.
    let x = right;
    let y = -fwd;
    let ax = x.abs();
    let ay = y.abs();
    if ay.saturating_mul(2) < ax {
        if x >= 0 { 2 } else { 6 }
    } else if ax.saturating_mul(2) < ay {
        if y >= 0 { 0 } else { 4 }
    } else if x >= 0 && y >= 0 {
        1
    } else if x >= 0 {
        3
    } else if y < 0 {
        5
    } else {
        7
    }
}

pub(crate) fn projection(pitch: Num<i32, 8>, fx: i32, fy: i32, sin_r: i32, cos_r: i32) -> usize {
    let fwd = fx * sin_r - fy * cos_r;
    let right = fx * cos_r + fy * sin_r;
    let oct = heading_octant(fwd, right);
    if nadir_on() > 0 {
        return 8 + oct;
    }
    // The overhead sheet was spun with an affine matrix. That rotation is
    // around the corner, so the figure came out upside down and mostly empty.
    // Past a clear nose-down, use the upright high-angle views instead.
    let sp = pitch.sin().to_raw();
    if sp >= 100 { 8 + oct } else { oct }
}

pub(crate) fn actor_matrix(inv: Num<i32, 8>) -> AffineMatrix<Num<i32, 8>> {
    AffineMatrix::from_scale(vec2(inv, inv))
}

pub(crate) fn show_soldier(
    frame: &mut agb::display::GraphicsFrame,
    soldier: &Soldier,
    sx: i32,
    sy: i32,
    horizon: i32,
    alt: i32,
    pitch: Num<i32, 8>,
    sin_r: i32,
    cos_r: i32,
    sensor: Sensor,
    yaw: Num<i32, 8>,
) {
    let drop = (sy - horizon).max(1);
    let span = ground_span(drop, alt, 16);
    let (fx, fy) = if soldier.dir == 0 {
        (0, -1)
    } else {
        (soldier.dir.signum(), 0)
    };
    let frame_index = projection(pitch, fx, fy, sin_r, cos_r);
    let bob = ((soldier.step / 8) & 1) as i32;
    let sy = sy - bob;
    // Overhead stays the small stamp. A close pass uses the tall figure,
    // with a roof plane and a shadow side, once the stamp would only swell.
    if detail_on() && frame_index < 8 && span >= 22 {
        show_figure(
            frame,
            sprites::SOLDIER_NEAR.sprite(four_view(frame_index)),
            sx,
            sy,
            32,
            64,
            46,
            61,
            span,
            false,
            sensor,
            true,
        );
    } else {
        show_figure(
            frame,
            sprites::SOLDIER.sprite(frame_index),
            sx,
            sy,
            32,
            32,
            18,
            26,
            span,
            false,
            sensor,
            true,
        );
    }
    show_shadow(frame, sx, sy, span, false, 26, 4, yaw, horizon, sensor, 8);
}

pub(crate) fn show_truck(
    frame: &mut agb::display::GraphicsFrame,
    truck: &Truck,
    sx: i32,
    sy: i32,
    horizon: i32,
    alt: i32,
    pitch: Num<i32, 8>,
    sin_r: i32,
    cos_r: i32,
    sensor: Sensor,
    yaw: Num<i32, 8>,
) {
    let drop = (sy - horizon).max(1);
    let span = ground_span(drop, alt, 12);
    let (fx, fy) = if truck.vertical {
        (0, truck.dir.signum())
    } else {
        (truck.dir.signum(), 0)
    };
    // The flight camera only uses the level front, side and rear. A three-quarter
    // frame puts the cab up the screen and the truck looks reared. The plan
    // sheet is the overhead picture, and only while the horizon is gone.
    let _ = pitch;
    let fwd = fx * sin_r - fy * cos_r;
    let right = fx * cos_r + fy * sin_r;
    let oct = heading_octant(fwd, right);
    let top = nadir_on() > 0 && horizon < 12;
    let (sprite, native_w, native_h, ink, foot) = if top {
        // Diagonal frames run further into the 32 tile than the side ones.
        let ink = if oct & 1 == 1 { 28 } else { 23 };
        (sprites::TRUCK_TOP.sprite(oct), 32, 32, ink, 25)
    } else if detail_on() && span >= 14 {
        (sprites::TRUCK_NEAR.sprite(four_view(oct)), 64, 32, 21, 29)
    } else {
        (sprites::TRUCK.sprite(level_octant(oct)), 32, 32, 9, 25)
    };
    show_figure(
        frame, sprite, sx, sy, native_w, native_h, ink, foot, span, top, sensor, true,
    );
    show_shadow(frame, sx, sy, span, top, 25, 5, yaw, horizon, sensor, 8);
}

pub(crate) fn show_tank(
    frame: &mut agb::display::GraphicsFrame,
    tank: &Tank,
    sx: i32,
    sy: i32,
    horizon: i32,
    alt: i32,
    sin_r: i32,
    cos_r: i32,
    sensor: Sensor,
    yaw: Num<i32, 8>,
) {
    let drop = (sy - horizon).max(1);
    let span = ground_span(drop, alt, 12);
    let (fx, fy) = if tank.vertical {
        (0, tank.dir.signum())
    } else {
        (tank.dir.signum(), 0)
    };
    let fwd = fx * sin_r - fy * cos_r;
    let right = fx * cos_r + fy * sin_r;
    let oct = heading_octant(fwd, right);
    let top = nadir_on() > 0 && horizon < 12;
    let (sprite, native_w, native_h, ink, foot) = if top {
        let ink = if oct & 1 == 1 { 32 } else { 27 };
        (sprites::TANK_TOP.sprite(oct), 32, 32, ink, 25)
    } else if detail_on() && span >= 14 {
        (sprites::TANK_NEAR.sprite(four_view(oct)), 64, 32, 21, 27)
    } else {
        (sprites::TANK.sprite(level_octant(oct)), 32, 32, 8, 25)
    };
    show_figure(
        frame, sprite, sx, sy, native_w, native_h, ink, foot, span, top, sensor, true,
    );
    show_shadow(frame, sx, sy, span, top, 25, 7, yaw, horizon, sensor, 8);
}

pub(crate) fn show_bunker(
    frame: &mut agb::display::GraphicsFrame,
    bunker: &Bunker,
    sx: i32,
    sy: i32,
    horizon: i32,
    alt: i32,
    cam_x: i32,
    cam_y: i32,
    sin_r: i32,
    cos_r: i32,
    sensor: Sensor,
    yaw: Num<i32, 8>,
) {
    let drop = (sy - horizon).max(1);
    let span = ground_span(drop, alt, 14);
    let dx = cam_x - bunker.x;
    let dy = cam_y - bunker.y;
    let ax = dx.abs();
    let ay = dy.abs();
    let fx = if ax * 2 > ay { dx.signum() } else { 0 };
    let fy = if ay * 2 > ax { dy.signum() } else { 0 };
    let (fx, fy) = if fx == 0 && fy == 0 { (0, 1) } else { (fx, fy) };
    let fwd = fx * sin_r - fy * cos_r;
    let right = fx * cos_r + fy * sin_r;
    let oct = heading_octant(fwd, right);
    let top = nadir_on() > 0 && horizon < 12;
    let (sprite, native_w, native_h, ink, foot) = if top {
        (sprites::BUNKER_TOP.sprite(oct), 32, 32, 24, 25)
    } else if detail_on() && span >= 12 {
        (sprites::BUNKER_NEAR.sprite(four_view(oct)), 64, 32, 22, 29)
    } else {
        (sprites::BUNKER.sprite(oct), 32, 32, 8, 25)
    };
    show_figure(
        frame, sprite, sx, sy, native_w, native_h, ink, foot, span, top, sensor, true,
    );
    show_shadow(frame, sx, sy, span, top, 25, 6, yaw, horizon, sensor, 8);
}

/// Flat contact mark. It sits off the feet, away from the sun or the moon.
/// `lift` is 8 on the ground and longer while the body is in the air.
pub(crate) fn show_shadow(
    frame: &mut agb::display::GraphicsFrame,
    sx: i32,
    sy: i32,
    body: i32,
    top: bool,
    baseline: i32,
    span: i32,
    yaw: Num<i32, 8>,
    horizon: i32,
    sensor: Sensor,
    lift: i32,
) {
    // Straight down, the plan already covers the ground a vehicle stands on.
    if top {
        return;
    }
    let _ = baseline;
    let cx = sx;
    let foot = sy;
    let w = (body * span / 8).clamp(6, 40);
    let h = (w / 3).clamp(3, 5);
    let inv_x = Num::<i32, 8>::new(32) / Num::new(w);
    let inv_y = Num::<i32, 8>::new(16) / Num::new(h);
    let (ox, oy) = shadow_cast(yaw, horizon, sensor);
    let lift = lift.clamp(8, 22);
    let x = cx - w / 2 + ox * lift / 8;
    let y = foot - h / 2 + oy * lift / 8;
    if x < -32 || y < -16 || x > WIDTH || y > HEIGHT {
        return;
    }
    ObjectAffine::new(
        SpriteVram::from(sprites::SHADOW.sprite(0)),
        AffineMatrix::from_scale(vec2(inv_x, inv_y)).into(),
        AffineMode::Affine,
    )
    .set_pos((x, y))
    .set_priority(Priority::P0)
    .show(frame);
}

/// Three red pips. They shrink as the target is hit.
pub(crate) fn show_life(
    frame: &mut agb::display::GraphicsFrame,
    sx: i32,
    sy: i32,
    hp: i8,
    max: i8,
) {
    if hp <= 0 || max <= 0 {
        return;
    }
    let n = ((hp as i32) * 3 + max as i32 - 1) / max as i32;
    let n = n.clamp(1, 3);
    // Overhead the body sits on the point. The side-view lift leaves the pips in empty grass.
    let lift = if nadir_on() > 0 { 8 } else { 26 };
    for index in 0..n {
        show(frame, sprites::DOT.sprite(0), sx - 8 + index * 4, sy - lift);
    }
}

pub(crate) fn draw_pause(frame: &mut agb::display::GraphicsFrame, menu: u8) {
    let resume = if menu == 0 {
        sprites::PAUSE_RESUME_ON.sprite(0)
    } else {
        sprites::PAUSE_RESUME.sprite(0)
    };
    let exit = if menu == 1 {
        sprites::PAUSE_EXIT_ON.sprite(0)
    } else {
        sprites::PAUSE_EXIT.sprite(0)
    };
    show(frame, resume, 88, 48);
    show(frame, exit, 88, 88);
}

fn glyph(ch: u8) -> usize {
    match ch {
        b'A'..=b'Z' => 1 + (ch - b'A') as usize,
        b'0'..=b'9' => 27 + (ch - b'0') as usize,
        b'a' => 37,
        b'e' => 38,
        b'g' => 39,
        b'm' => 40,
        b's' => 41,
        _ => 0,
    }
}

/// A role, smaller than the name under it. Same index order as the name face.
pub(crate) fn show_role(frame: &mut agb::display::GraphicsFrame, text: &[u8], y: i32) {
    let mut width = 0;
    for &ch in text {
        width += if ch == b' ' { 3 } else { 5 };
    }
    let mut x = (WIDTH - width) / 2;
    for &ch in text {
        let adv = if ch == b' ' { 3 } else { 5 };
        if ch != b' ' && (-8..HEIGHT).contains(&y) && (-8..WIDTH).contains(&x) {
            show(frame, sprites::ROLE.sprite(glyph(ch)), x, y);
        }
        x += adv;
    }
}

/// One small line, centred. Every letter is the same 5x7 on the same baseline.
pub(crate) fn show_small(frame: &mut agb::display::GraphicsFrame, text: &[u8], y: i32) {
    let mut width = 0;
    for &ch in text {
        width += if ch == b' ' { 4 } else { 6 };
    }
    let mut x = (WIDTH - width) / 2;
    for &ch in text {
        let adv = if ch == b' ' { 4 } else { 6 };
        if ch != b' ' && (-8..HEIGHT).contains(&y) && (-8..WIDTH).contains(&x) {
            show(frame, sprites::SMALL.sprite(glyph(ch)), x, y);
        }
        x += adv;
    }
}

/// Studio card. The stone settles, then the whole lockup, on one centre line.
pub(crate) fn draw_studio(frame: &mut agb::display::GraphicsFrame, age: i32) {
    let t = age.min(12);
    let dy = (12 - t) * (12 - t) / 8;
    show(frame, sprites::MARBLE.sprite(0), 104, 2 + dy);
    show_small(frame, b"2026", 146);
    if age >= 14 {
        show_face(frame, b"MARBLE ZOO", 40);
        show_face(frame, b"games", 58);
        show(frame, sprites::WORD.sprite(0), 88, 80);
    }
}

/// Quads and one wing, seen from above, drifting behind the roll.
fn draw_credit_craft(frame: &mut agb::display::GraphicsFrame, age: i32) {
    let lanes: [(i32, i32, i32, bool); 4] = [
        (10, 12, 36, false),
        (120, 48, 24, true),
        (40, 96, 44, false),
        (180, 132, 30, false),
    ];
    for (shift, y, speed, wing) in lanes {
        let x = (age * speed / 40 + shift) % 300 - 40;
        let bob = ((age / 20 + shift) & 3) - 1;
        if wing {
            show(frame, sprites::GERAN.sprite(1), x, y + bob);
        } else {
            show(frame, sprites::FOE.sprite(1), x, y + bob);
        }
    }
}

struct Person {
    first: &'static [u8],
    last: &'static [u8],
}

struct Credit {
    role: &'static [u8],
    who: &'static [&'static Person],
}

static ANDREI: Person = Person {
    first: b"ANDREI",
    last: b"KSIANZOU",
};
static ALINA: Person = Person {
    first: b"ALINA",
    last: b"VELIHAYEVA",
};
static VITALI: Person = Person {
    first: b"VITALI",
    last: b"KSIANZOU",
};
static ALEX: Person = Person {
    first: b"ALEX",
    last: b"KORSHUNAU",
};

/// One line per post. A name lives here once, and the roll repeats it.
static STAFF: &[Credit] = &[
    Credit {
        role: b"DIRECTOR",
        who: &[&ANDREI],
    },
    Credit {
        role: b"PRODUCER",
        who: &[&VITALI],
    },
    Credit {
        role: b"GAME DESIGN",
        who: &[&ANDREI],
    },
    Credit {
        role: b"LEVEL DESIGN",
        who: &[&ANDREI],
    },
    Credit {
        role: b"MAIN PROGRAM",
        who: &[&ANDREI],
    },
    Credit {
        role: b"PROGRAM",
        who: &[&ALEX],
    },
    Credit {
        role: b"CHARACTER DESIGN",
        who: &[&ALINA],
    },
    Credit {
        role: b"GRAPHIC DESIGN",
        who: &[&ALINA],
    },
    Credit {
        role: b"MUSIC",
        who: &[&ANDREI],
    },
    Credit {
        role: b"SOUND EFFECTS",
        who: &[&ANDREI],
    },
];

/// End roll for the last sortie. Roles are the short titles of that era.
pub(crate) fn draw_credits(frame: &mut agb::display::GraphicsFrame, age: i32) {
    let mut end = 16i32;
    for post in STAFF {
        end += 8 + post.who.len() as i32 * 18 + 6;
    }
    end += 24;
    // The year rests just above the bottom.
    let scroll = (age / 4).min(32 + end);
    let base = 160 - scroll;
    show_small(frame, b"DRONE", base);
    let mut y = 16i32;
    for post in STAFF {
        show_role(frame, post.role, base + y);
        y += 8;
        for person in post.who {
            show_small(frame, person.first, base + y);
            y += 9;
            show_small(frame, person.last, base + y);
            y += 9;
        }
        y += 6;
    }
    y += 4;
    show_small(frame, b"MARBLE ZOO", base + y);
    show_small(frame, b"games", base + y + 10);
    show_small(frame, b"2026", base + y + 20);
    draw_credit_craft(frame, age);
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
    let mut object = Object::new(SpriteVram::from(sprite));
    if let Some(glass) = sheet_set(sensor) {
        if let Some(palette) = glass.flame {
            object.set_palette(palette);
        }
    }
    object
        .set_pos((x, y))
        .set_priority(Priority::P0)
        .show(frame);
}

pub(crate) fn show(frame: &mut agb::display::GraphicsFrame, sprite: &'static Sprite, x: i32, y: i32) {
    if x < -64 || y < -32 || x > WIDTH || y > HEIGHT {
        return;
    }
    Object::new(SpriteVram::from(sprite))
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
    Object::new(SpriteVram::from(sprite))
        .set_pos((x, y))
        .set_priority(Priority::P0)
        .set_graphics_mode(GraphicsMode::AlphaBlending)
        .show(frame);
}

pub(crate) fn draw_digits(
    frame: &mut agb::display::GraphicsFrame,
    mut value: i32,
    count: i32,
    x: i32,
    y: i32,
) {
    if value < 0 {
        value = 0;
    }
    for index in (0..count).rev() {
        let digit = value % 10;
        show(
            frame,
            sprites::DIGITS.sprite(digit as usize),
            x + index * 8,
            y,
        );
        value /= 10;
    }
}

/// Three stars under a cleared card. Gold is earned. Grey is still open.
pub(crate) fn show_ribbon(
    frame: &mut agb::display::GraphicsFrame,
    marks: u8,
    x: i32,
    y: i32,
) {
    for index in 0..3 {
        let which = if marks & (1 << index) != 0 { 1 } else { 0 };
        show(frame, sprites::MEDAL.sprite(which), x + index * 20, y);
    }
}

pub(crate) fn in_view(sx: i32, sy: i32, cockpit: bool) -> bool {
    !cockpit || (sx >= VIEW_X && sx < VIEW_X + VIEW_W && sy >= VIEW_Y && sy < VIEW_Y + VIEW_H)
}

/// Fixed crosshair, attitude indicator on the left, radar on the right, and
/// the dark columns that hide the edges of the floor.
pub(crate) fn draw_cockpit(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    sin_r: i32,
    cos_r: i32,
) {
    draw_sight(frame, world);
    draw_gauges(frame, world);

    let inst_y = 112;
    let pitch_shift = (-world.pitch.to_raw() / 6).clamp(-8, 8);
    let bank_raw = world.bank.to_raw();
    let bar = if bank_raw > 10 {
        4
    } else if bank_raw > 4 {
        3
    } else if bank_raw < -10 {
        0
    } else if bank_raw < -4 {
        1
    } else {
        2
    };
    show(frame, sprites::ADI_WINGS.sprite(0), 0, inst_y + 8);
    show(
        frame,
        sprites::ADI_BAR.sprite(bar),
        0,
        inst_y + 8 + pitch_shift,
    );

    let radar_x = WIDTH - 32;
    draw_radar_goals(frame, world, sin_r, cos_r, radar_x, inst_y);
    show_dot(frame, 1, radar_x, inst_y, 0, 0);
    show(frame, sprites::RADAR.sprite(0), radar_x, inst_y);

    for step in 0..5 {
        let y = VIEW_Y + step * 32;
        show(frame, sprites::PANEL.sprite(0), 0, y);
        show(frame, sprites::PANEL.sprite(0), WIDTH - 32, y);
    }
    draw_static(frame, world);
}

/// The plus sits on the same pixel as the middle of the brackets.
/// FPV looks along two arms at a mine hung in front of the lens.
pub(crate) fn draw_sight(frame: &mut agb::display::GraphicsFrame, world: &World) {
    let view_cx = VIEW_X + VIEW_W / 2;
    let view_cy = VIEW_Y + VIEW_H / 2;
    if world.mode == Mode::Fpv {
        draw_fpv_rig(frame, world, view_cx, view_cy);
        return;
    }
    if world.mode == Mode::Guide {
        draw_guide_glass(frame, world, view_cx, view_cy);
    }
    // Overhead, the nose point is often off the glass. The plus marks the
    // ground under the airframe, which is the centre of that picture.
    show(frame, sprites::RETICLE.sprite(0), view_cx - 8, view_cy - 8);
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
    Object::new(SpriteVram::from(sprite))
        .set_pos((x, y))
        .set_hflip(hflip)
        .set_vflip(vflip)
        .set_priority(Priority::P0)
        .show(frame);
}

pub(crate) fn draw_guide_glass(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    view_cx: i32,
    view_cy: i32,
) {
    let mark = sprites::BRACKET.sprite(0);
    show_flip(frame, mark, VIEW_X + 4, VIEW_Y + 4, false, false);
    show_flip(frame, mark, VIEW_X + VIEW_W - 12, VIEW_Y + 4, true, false);
    show_flip(frame, mark, VIEW_X + 4, VIEW_Y + VIEW_H - 12, false, true);
    show_flip(
        frame,
        mark,
        VIEW_X + VIEW_W - 12,
        VIEW_Y + VIEW_H - 12,
        true,
        true,
    );

    let (sin_r, cos_r) = world.heading();
    let dx = 0i32;
    let dy = -(64 << 8);
    let fwd = ((dx * sin_r) - (dy * cos_r)) >> 8;
    let right = ((dx * cos_r) + (dy * sin_r)) >> 8;
    let mag = isqrt64((fwd as i64) * (fwd as i64) + (right as i64) * (right as i64)).max(1) as i32;
    let nx = (view_cx + right * 36 / mag - 4).clamp(VIEW_X + 16, VIEW_X + VIEW_W - 24);
    let ny = (view_cy - fwd * 36 / mag - 4).clamp(VIEW_Y + 16, VIEW_Y + VIEW_H - 24);
    show(frame, sprites::NORTH.sprite(0), nx, ny);
}

/// The designator is a spot on the ground, not a patch of tiles. While the
/// sight is free it sits under the crosshair. After the mark it stays there.
fn draw_lase(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    sin_r: i32,
    cos_r: i32,
    horizon: i32,
    cockpit: bool,
) {
    if world.mode != Mode::Guide || world.falling {
        return;
    }
    let (x, y) = if world.guide > 0 || world.salvo > 0 {
        (world.mark_x, world.mark_y)
    } else {
        let (nx, ny, _) = world.nose_ground(sin_r, cos_r);
        (nx, ny)
    };
    let Some((sx, sy)) = project(
        world.x, world.y, world.alt, sin_r, cos_r, horizon, x, y, 0,
    ) else {
        return;
    };
    if !in_view(sx, sy, cockpit) {
        return;
    }
    if (world.age / 3) % 2 == 0 {
        show(frame, sprites::SPARK.sprite(0), sx - 4, sy - 4);
    }
    let gap = 6;
    let mark = sprites::BRACKET.sprite(0);
    show_flip(frame, mark, sx - gap - 8, sy - gap - 8, false, false);
    show_flip(frame, mark, sx + gap, sy - gap - 8, true, false);
    show_flip(frame, mark, sx - gap - 8, sy + gap, false, true);
    show_flip(frame, mark, sx + gap, sy + gap, true, true);
}

/// Two thin leads from the lower corners to the mine on the crosshair.
pub(crate) fn draw_fpv_rig(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    view_cx: i32,
    view_cy: i32,
) {
    let blasting = world.booms.iter().any(|boom| {
        boom.alive && dist2_px(world.x, world.y, boom.x, boom.y) < 64 * 64
    });
    if blasting {
        if let Some(boom) = world.booms.iter().find(|boom| boom.alive) {
            show_fit(
                frame,
                sprites::BOOM.sprite(boom_sheet(boom.frame)),
                view_cx,
                VIEW_Y + VIEW_H - 28,
                16,
                32,
            );
        }
    }
    draw_wire(frame, VIEW_X + 4, VIEW_Y + VIEW_H - 8, view_cx - 4, view_cy);
    draw_wire(
        frame,
        VIEW_X + VIEW_W - 12,
        VIEW_Y + VIEW_H - 8,
        view_cx + 4,
        view_cy,
    );
    if !blasting {
        show(frame, sprites::MINE.sprite(0), view_cx - 4, view_cy - 4);
    }
}

pub(crate) fn draw_wire(
    frame: &mut agb::display::GraphicsFrame,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
) {
    let steps = 8;
    for i in 0..steps {
        let x = x0 + (x1 - x0) * i / steps;
        let y = y0 + (y1 - y0) * i / steps;
        if !(VIEW_X..VIEW_X + VIEW_W).contains(&x) {
            continue;
        }
        show(frame, sprites::BEAD.sprite(0), x - 3, y - 3);
    }
}

/// White circle, wider than the kill radius, flat on the ground.
fn show_blast(
    frame: &mut agb::display::GraphicsFrame,
    sx: i32,
    sy: i32,
    drop: i32,
    alt: i32,
    radius: i32,
    age: i32,
) {
    let remain = (18 - age).max(4);
    let wide = (ground_span(drop, alt, radius * 4) * remain / 18).clamp(12, 120);
    let flat = (wide * drop / 160).clamp(6, wide);
    let double = wide > 64 || flat > 64;
    let mode = if double {
        AffineMode::AffineDouble
    } else {
        AffineMode::Affine
    };
    let inv_x = Num::<i32, 8>::new(64) / Num::new(wide.max(1));
    let inv_y = Num::<i32, 8>::new(64) / Num::new(flat.max(1));
    let half = if double { 64 } else { 32 };
    let x = sx - half;
    let y = sy - half;
    if x < -128 || y < -128 || x > WIDTH || y > HEIGHT {
        return;
    }
    let mut disc = ObjectAffine::new(
        SpriteVram::from(sprites::FLASH.sprite(0)),
        AffineMatrix::from_scale(vec2(inv_x, inv_y)).into(),
        mode,
    );
    if age >= 6 {
        disc.set_graphics_mode(GraphicsMode::AlphaBlending);
    }
    disc.set_pos((x, y))
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
pub(crate) fn show_spark(
    frame: &mut agb::display::GraphicsFrame,
    sx: i32,
    sy: i32,
    cockpit: bool,
) {
    let x = sx - 4;
    let y = sy - 4;
    if cockpit
        && (x < VIEW_X || y < VIEW_Y || x + 8 > VIEW_X + VIEW_W || y + 8 > VIEW_Y + VIEW_H)
    {
        return;
    }
    show(frame, sprites::SPARK.sprite(0), x, y);
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
    ObjectAffine::new(
        SpriteVram::from(sprite),
        actor_matrix(inv).into(),
        AffineMode::Affine,
    )
    .set_pos((x, y))
    .set_priority(Priority::P0)
    .show(frame);
}

/// Red mark is the airframe, yellow is the battery, the snow icon is the radio.
pub(crate) fn draw_gauges(frame: &mut agb::display::GraphicsFrame, world: &World) {
    show(frame, sprites::BLIP.sprite(0), 4, 18);
    draw_digits(frame, world.hull.clamp(0, 100), 3, 4, 28);
    let fill = (world.hull.clamp(0, 100) * 10 / 100) as usize;
    show(frame, sprites::HULL.sprite(fill), 0, 40);
    let wep = match world.arm {
        ARM_GUN => 0,
        ARM_BOMB => 1,
        ARM_HE => 2,
        ARM_MINE => 3,
        _ => 4,
    };
    show(frame, sprites::WEP.sprite(wep), 8, 52);
    let ammo = match world.arm {
        ARM_GUN => None,
        ARM_BOMB => Some(world.bombs),
        ARM_HE => Some(world.he),
        ARM_MINE => Some(world.mines),
        _ => Some(world.vog),
    };
    if let Some(ammo) = ammo {
        draw_digits(frame, ammo, 2, 8, 70);
    }
    show(frame, sprites::BLIP.sprite(1), WIDTH - 28, 18);
    draw_digits(frame, world.battery.clamp(0, 100), 3, WIDTH - 28, 28);
    let signal = (100 - world.signal_loss() * 100 / 16).clamp(0, 100);
    show(
        frame,
        sprites::STATIC.sprite((world.age as usize / 8) % 4),
        WIDTH - 28,
        46,
    );
    draw_digits(frame, signal, 3, WIDTH - 28, 56);
}

/// Canopy crack and extra snow once the airframe is badly hit.
pub(crate) fn draw_hurt(frame: &mut agb::display::GraphicsFrame, world: &World) {
    if world.hull > 40 {
        return;
    }
    show(frame, sprites::CRACK.sprite(0), VIEW_X + 28, VIEW_Y + 24);
    if world.hull <= 20 {
        show(frame, sprites::CRACK.sprite(1), VIEW_X + 96, VIEW_Y + 64);
    }
}

pub(crate) fn draw_noise(frame: &mut agb::display::GraphicsFrame, age: i32) {
    let jx = (hash(age, 1) % 4) as i32;
    let jy = (hash(age, 3) % 4) as i32;
    let flip = (age as usize / 2) % 2;
    let xs = [VIEW_X, VIEW_X + 64, VIEW_X + VIEW_W - 64];
    let ys = [VIEW_Y, VIEW_Y + 64, VIEW_Y + VIEW_H - 64];
    for (row, y) in ys.into_iter().enumerate() {
        for (col, x) in xs.into_iter().enumerate() {
            show(
                frame,
                sprites::NOISE.sprite((flip + row + col) % 2),
                x - jx,
                y - jy,
            );
        }
    }
}

struct Look {
    sensor: u8,
    applied: u8,
    sky: u16,
    sky_applied: u16,
}

struct LookCell(core::cell::UnsafeCell<Look>);
unsafe impl Sync for LookCell {}

static LOOK: LookCell = LookCell(core::cell::UnsafeCell::new(Look {
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

fn detail_on() -> bool {
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

fn sensor_from(tag: u8) -> Sensor {
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

fn flush_look() {
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

/// The field through the glass. Temperature is not the tile's brightness:
/// grass stays cold, and only bodies are painted hot.
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

/// Snow. Dark tiles stay readable (roads, trunks, trenches). Grass goes pale.
pub(crate) fn winter_ground(colour: u16) -> u16 {
    let (r, g, b) = channels(colour);
    if b > r + 4 && b > g {
        let v = (b / 2 + 4).min(18) as u16;
        return (v / 3) | ((v / 2) << 5) | (v << 10);
    }
    let y = luma(colour);
    if y < 10 {
        let v = (y / 2 + 4).min(12) as u16;
        return v | (v << 5) | ((v + 1).min(31) << 10);
    }
    let v = (16 + y / 2).min(31) as u16;
    v | (v << 5) | ((v - 1) << 10)
}

pub(crate) fn cold_ground(colour: u16) -> u16 {
    let (r, g, b) = channels(colour);
    if b > r + 2 && b > g {
        return 1 | (1 << 5) | (7 << 10);
    }
    if r > g && r > b {
        let warm = (3 + (r - g).clamp(0, 8) / 3).min(7) as u16;
        return warm | ((warm / 3) << 5) | (1 << 10);
    }
    let shade = (1 + g / 16).min(3) as u16;
    (shade / 2) | (1 << 5) | (3 << 10)
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

pub(crate) struct BodyPals {
    pub(crate) heat: PaletteVram,
    pub(crate) night: PaletteVram,
    pub(crate) ir: PaletteVram,
}

pub(crate) struct BodyHold(core::cell::UnsafeCell<Option<BodyPals>>);
unsafe impl Sync for BodyHold {}
pub(crate) static BODY_PALS: BodyHold = BodyHold(core::cell::UnsafeCell::new(None));

struct SheetGlass {
    tank: Option<PaletteVram>,
    truck: Option<PaletteVram>,
    rig: Option<PaletteVram>,
    bunker: Option<PaletteVram>,
    pill: Option<PaletteVram>,
    tree: Option<PaletteVram>,
    flame: Option<PaletteVram>,
}

struct SheetHold(core::cell::UnsafeCell<Option<(u8, SheetGlass)>>);
unsafe impl Sync for SheetHold {}
static SHEETS: SheetHold = SheetHold(core::cell::UnsafeCell::new(None));

fn on_sheet(sprite: &'static Sprite, tag: &agb::display::object::Tag) -> bool {
    let sprites = tag.sprites();
    let start = sprites.as_ptr() as usize;
    let end = start + sprites.len() * core::mem::size_of::<Sprite>();
    let at = sprite as *const Sprite as usize;
    at >= start && at < end
}

fn bank_with(mark: Rgb, also: Rgb, absent: Option<Rgb>) -> Option<usize> {
    let needle = mark.to_rgb15().0;
    let other = also.to_rgb15().0;
    let skip = absent.map(|colour| colour.to_rgb15().0);
    unsafe {
        let ram = 0x0500_0200 as *const u16;
        for bank in 0..16 {
            let mut saw_mark = false;
            let mut saw_other = false;
            let mut saw_skip = false;
            for index in 1..16 {
                let colour = ram.add(bank * 16 + index).read_volatile();
                saw_mark |= colour == needle;
                saw_other |= colour == other;
                if let Some(skip) = skip {
                    saw_skip |= colour == skip;
                }
            }
            if saw_mark && saw_other && !saw_skip {
                return Some(bank);
            }
        }
    }
    None
}

fn bake_bank(bank: usize, map: fn(u16) -> u16) -> Palette16 {
    let mut colours = [Rgb15(0); 16];
    unsafe {
        let ram = 0x0500_0200 as *const u16;
        for index in 1..16 {
            colours[index] = Rgb15(map(ram.add(bank * 16 + index).read_volatile()));
        }
    }
    Palette16::new(colours)
}

/// A crown in heat stays cold, but light enough to read on the black field.
fn cold_plant(colour: u16) -> u16 {
    let y = luma(colour);
    let v = (7 + y / 4).clamp(7, 14) as u16;
    (v / 2) | (1 << 5) | (v << 10)
}

fn plant_map(sensor: Sensor) -> fn(u16) -> u16 {
    match sensor {
        Sensor::Night => nvg_body,
        Sensor::Ir => ir_body,
        _ => cold_plant,
    }
}

fn body_map(sensor: Sensor) -> fn(u16) -> u16 {
    match sensor {
        Sensor::Night => nvg_body,
        Sensor::Ir => ir_body,
        _ => heat_body,
    }
}

/// Fire through the glass. Night is phosphor, heat is white-hot, the grey sight is a white bloom.
fn flame_heat(colour: u16) -> u16 {
    let y = luma(colour);
    let v = (20 + y / 2).clamp(20, 31) as u16;
    let g = if y > 18 { v - 2 } else { v / 3 };
    v | (g << 5) | ((v / 3) << 10)
}

fn flame_ir(colour: u16) -> u16 {
    let y = luma(colour);
    let v = (26 + y / 5).clamp(26, 31) as u16;
    v | (v << 5) | (v << 10)
}

fn flame_map(sensor: Sensor) -> fn(u16) -> u16 {
    match sensor {
        Sensor::Night => nvg_body,
        Sensor::Ir => flame_ir,
        _ => flame_heat,
    }
}

fn take_sheet(bank: Option<usize>, map: fn(u16) -> u16) -> Option<PaletteVram> {
    let bank = bank?;
    PaletteVramSingle::try_allocate_new(&bake_bank(bank, map))
        .ok()
        .map(|palette| palette.palette())
}

fn load_sheets(sensor: Sensor) -> SheetGlass {
    // Touch the sheets so their real palettes are in object memory before the search.
    let _ = SpriteVram::from(sprites::TANK_TOP.sprite(0));
    let _ = SpriteVram::from(sprites::TRUCK.sprite(0));
    let _ = SpriteVram::from(sprites::TRUCK_NEAR.sprite(0));
    let _ = SpriteVram::from(sprites::TRUCK_TOP.sprite(0));
    let _ = SpriteVram::from(sprites::BUNKER.sprite(0));
    let _ = SpriteVram::from(sprites::BUNKER_NEAR.sprite(0));
    let _ = SpriteVram::from(sprites::BUNKER_TOP.sprite(0));
    let _ = SpriteVram::from(sprites::GROVE.sprite(0));
    let _ = SpriteVram::from(sprites::BOOM.sprite(0));
    let bodies = body_map(sensor);
    let plants = plant_map(sensor);
    let flames = flame_map(sensor);
    // The side sheet is the five earth colours. The plan adds the dark greens,
    // so a bank without them is the side truck on its own.
    let rig_bank = bank_with(
        Rgb::new(112, 88, 48),
        Rgb::new(168, 200, 200),
        Some(Rgb::new(40, 56, 32)),
    );
    let top_bank = bank_with(Rgb::new(40, 56, 32), Rgb::new(112, 88, 48), None).or(rig_bank);
    let truck = take_sheet(top_bank, bodies);
    let rig = if rig_bank.is_some() && rig_bank != top_bank {
        take_sheet(rig_bank, bodies)
    } else {
        truck.clone()
    };
    // Violet slit is only on the side sheet. The plan adds a dark green the side lacks.
    let pill_bank = bank_with(Rgb::new(56, 0, 184), Rgb::new(184, 176, 144), None);
    let dome_bank = bank_with(Rgb::new(72, 96, 48), Rgb::new(40, 56, 64), None).or(pill_bank);
    let bunker = take_sheet(dome_bank, bodies);
    let pill = if pill_bank.is_some() && pill_bank != dome_bank {
        take_sheet(pill_bank, bodies)
    } else {
        bunker.clone()
    };
    SheetGlass {
        tank: take_sheet(
            bank_with(Rgb::new(240, 240, 16), Rgb::new(224, 208, 0), None),
            bodies,
        ),
        truck,
        rig,
        bunker,
        pill,
        tree: take_sheet(
            bank_with(Rgb::new(72, 96, 48), Rgb::new(24, 24, 16), None),
            plants,
        ),
        flame: take_sheet(
            bank_with(Rgb::new(200, 40, 32), Rgb::new(248, 208, 32), None),
            flames,
        ),
    }
}

fn sheet_set(sensor: Sensor) -> Option<SheetGlass> {
    if matches!(sensor, Sensor::Day | Sensor::Winter) {
        return None;
    }
    let tag = sensor_tag(sensor);
    unsafe {
        let hold = &mut *SHEETS.0.get();
        if hold.as_ref().is_none_or(|stored| stored.0 != tag) {
            *hold = None;
            *hold = Some((tag, load_sheets(sensor)));
        }
        hold.as_ref().map(|stored| SheetGlass {
            tank: stored.1.tank.clone(),
            truck: stored.1.truck.clone(),
            rig: stored.1.rig.clone(),
            bunker: stored.1.bunker.clone(),
            pill: stored.1.pill.clone(),
            tree: stored.1.tree.clone(),
            flame: stored.1.flame.clone(),
        })
    }
}

/// Overhead plans and crowns keep their own colour indices, so the glass is
/// baked from that sheet rather than from the first sprite palette.
fn sheet_palette(sprite: &'static Sprite, sensor: Sensor) -> Option<PaletteVram> {
    let glass = sheet_set(sensor)?;
    if on_sheet(sprite, &sprites::TANK_TOP) {
        glass.tank
    } else if on_sheet(sprite, &sprites::TRUCK)
        || on_sheet(sprite, &sprites::TRUCK_NEAR)
        || on_sheet(sprite, &sprites::TRUCK_TOP)
    {
        if on_sheet(sprite, &sprites::TRUCK_TOP) {
            glass.truck
        } else {
            glass.rig
        }
    } else if on_sheet(sprite, &sprites::BUNKER_TOP) {
        glass.bunker
    } else if on_sheet(sprite, &sprites::BUNKER) || on_sheet(sprite, &sprites::BUNKER_NEAR) {
        glass.pill
    } else if on_sheet(sprite, &sprites::GROVE) || on_sheet(sprite, &sprites::BUSH) {
        glass.tree
    } else {
        None
    }
}

pub(crate) fn body_palette(sensor: Sensor) -> Option<PaletteVram> {
    let pals = unsafe {
        let slot = &mut *BODY_PALS.0.get();
        if slot.is_none() {
            let src = 0x0500_0200 as *const u16;
            if src.add(1).read_volatile() == 0 {
                return None;
            }
            let heat = PaletteVramSingle::try_allocate_new(&bake_body(Sensor::Heat)).ok()?;
            let night = PaletteVramSingle::try_allocate_new(&bake_body(Sensor::Night)).ok()?;
            let ir = PaletteVramSingle::try_allocate_new(&bake_body(Sensor::Ir)).ok()?;
            *slot = Some(BodyPals {
                heat: heat.palette(),
                night: night.palette(),
                ir: ir.palette(),
            });
        }
        slot.as_ref().unwrap()
    };
    match sensor {
        Sensor::Heat => Some(pals.heat.clone()),
        Sensor::Night => Some(pals.night.clone()),
        Sensor::Ir => Some(pals.ir.clone()),
        Sensor::Day | Sensor::Winter => None,
    }
}

pub(crate) fn bake_body(sensor: Sensor) -> Palette16 {
    let mut colours = [Rgb15(0); 16];
    unsafe {
        let src = 0x0500_0200 as *const u16;
        for index in 1..16 {
            let colour = src.add(index).read_volatile();
            colours[index] = Rgb15(match sensor {
                Sensor::Heat => heat_body(colour),
                Sensor::Night => nvg_body(colour),
                Sensor::Ir => ir_body(colour),
                Sensor::Day | Sensor::Winter => colour,
            });
        }
    }
    Palette16::new(colours)
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
            for index in 1..16 {
                colours[index] = hot;
            }
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

pub(crate) fn draw_static(frame: &mut agb::display::GraphicsFrame, world: &World) {
    let loss = world.signal_loss();
    let hurt = if world.hull <= 20 {
        6
    } else if world.hull <= 40 {
        3
    } else {
        0
    };
    let count = (loss / 2 + hurt).clamp(0, 10);
    for index in 0..count {
        let n = hash(world.age.wrapping_add(index * 17), loss + index);
        let x = VIEW_X + (n % VIEW_W as u32) as i32;
        let y = VIEW_Y + 4 + ((n >> 8) % (VIEW_H as u32 - 12)) as i32;
        show(
            frame,
            sprites::STATIC.sprite((n % 4) as usize),
            x,
            y,
        );
    }
}

/// Yellow dots are the air the guns do not own, while the drone is still in their sight.
pub(crate) fn draw_aa_lanes(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    sin_r: i32,
    cos_r: i32,
    radar_x: i32,
    inst_y: i32,
) {
    let guns = aa_guns(task_of(world.level));
    if guns.is_empty() || !guns_see(world.alt >> 8) {
        return;
    }
    let samples = [
        (0, -10),
        (8, -6),
        (-8, -6),
        (10, -1),
        (-10, -1),
        (6, 6),
        (-6, 6),
        (0, 8),
    ];
    for (rx, ry) in samples {
        let (wx, wy) = lane_world(world.x, world.y, sin_r, cos_r, rx, ry);
        if aa_covers(guns, wx, wy) {
            continue;
        }
        show_dot(frame, 1, radar_x, inst_y, rx, ry);
    }
}

/// The mission mark, clamped to the radar ring when it is farther than the scale.
pub(crate) fn draw_radar_goals(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    sin_r: i32,
    cos_r: i32,
    radar_x: i32,
    inst_y: i32,
) {
    let mut pts = [(0i32, 0i32); 3];
    let n = goal_points(world, &mut pts);
    if matches!(task_of(world.level), Task::Fetch) && !world.cargo {
        for cache in &world.caches {
            if !cache.alive {
                continue;
            }
            let (bx, by) = radar_blip(
                delta(cache.x, world.x),
                delta(cache.y, world.y),
                sin_r,
                cos_r,
            );
            show_dot(frame, 0, radar_x, inst_y, bx, by);
        }
    }
    if matches!(task_of(world.level), Task::Fetch) {
        for &(tx, ty) in charge_spots(world.level) {
            let (bx, by) = radar_blip(
                delta((tx * 8 + 4) << 8, world.x),
                delta((ty * 8 + 4) << 8, world.y),
                sin_r,
                cos_r,
            );
            show_dot(frame, 1, radar_x, inst_y, bx, by);
        }
    }
    if defend_card(world.level) {
        let (bx, by) = radar_blip(
            delta(world.pad_x, world.x),
            delta(world.pad_y, world.y),
            sin_r,
            cos_r,
        );
        show_dot(frame, 1, radar_x, inst_y, bx, by);
    }
    for foe in &world.foes {
        if !foe.alive {
            continue;
        }
        let (bx, by) = radar_blip(
            delta(foe.x, world.x),
            delta(foe.y, world.y),
            sin_r,
            cos_r,
        );
        show_dot(frame, 0, radar_x, inst_y, bx, by);
    }
    for index in 0..n {
        let (bx, by) = radar_blip(
            delta(pts[index].0, world.x),
            delta(pts[index].1, world.y),
            sin_r,
            cos_r,
        );
        show_dot(frame, 0, radar_x, inst_y, bx, by);
    }
    if matches!(task_of(world.level), Task::Convoy) {
        let mx = (0 * 8 + 4) << 8;
        let my = (4 * 8 + 4) << 8;
        let (bx, by) = radar_blip(delta(mx, world.x), delta(my, world.y), sin_r, cos_r);
        show_dot(frame, 1, radar_x, inst_y, bx, by);
    }
    if matches!(task_of(world.level), Task::Cover) {
        let mx = (24 * 8 + 4) << 8;
        let my = (14 * 8 + 4) << 8;
        let (bx, by) = radar_blip(delta(mx, world.x), delta(my, world.y), sin_r, cos_r);
        show_dot(frame, 1, radar_x, inst_y, bx, by);
    }
    if matches!(task_of(world.level), Task::Rush | Task::Push) {
        let mx = (24 * 8 + 4) << 8;
        let my = (14 * 8 + 4) << 8;
        let (bx, by) = radar_blip(delta(mx, world.x), delta(my, world.y), sin_r, cos_r);
        show_dot(frame, 1, radar_x, inst_y, bx, by);
    }
    if world.guide > 0 || world.salvo > 0 {
        let (bx, by) = radar_blip(
            delta(world.mark_x, world.x),
            delta(world.mark_y, world.y),
            sin_r,
            cos_r,
        );
        show_dot(frame, 1, radar_x, inst_y, bx, by);
    }
    draw_aa_lanes(frame, world, sin_r, cos_r, radar_x, inst_y);
}

pub(crate) fn goal_points(world: &World, pts: &mut [(i32, i32); 3]) -> usize {
    match task_of(world.level) {
        Task::Trench | Task::Officer => {
            nearest_marked(world, pts, true, false)
        }
        Task::Night | Task::Heat | Task::Guide | Task::Column => {
            nearest_marked(world, pts, false, true)
        }
        Task::Rush => nearest_marked(world, pts, true, false),
        Task::Push => nearest_marked(world, pts, true, true),
        Task::Lane | Task::Dart | Task::Bend => nearest_marked(world, pts, false, true),
        Task::Nest => nearest_marked(world, pts, true, false),
        Task::Flare | Task::Armor => {
            let mut n = 0;
            for tank in &world.tanks {
                if tank.alive && tank.objective && n < 3 {
                    pts[n] = (tank.x, tank.y);
                    n += 1;
                }
            }
            n
        }
        Task::Pill | Task::Skirt => {
            let mut n = 0;
            for bunker in &world.bunkers {
                if bunker.alive && bunker.objective && n < 3 {
                    pts[n] = (bunker.x, bunker.y);
                    n += 1;
                }
            }
            n
        }
        Task::Convoy | Task::Cover => world
            .trucks
            .iter()
            .find(|truck| truck.alive && truck.goal)
            .map(|truck| {
                pts[0] = (truck.x, truck.y);
                1
            })
            .unwrap_or(0),
        Task::Gate => {
            pts[0] = (world.gate_x(), world.gate_y());
            1
        }
        Task::Hq => {
            if world.targets <= 0 {
                return 0;
            }
            pts[0] = ((23 * 8 + 4) << 8, (16 * 8 + 8) << 8);
            1
        }
        Task::Fpv => nearest_marked(world, pts, true, false),
        Task::Fetch => {
            if world.cargo {
                let mut n = 0;
                for tank in &world.tanks {
                    if tank.alive && tank.objective && n < 3 {
                        pts[n] = (tank.x, tank.y);
                        n += 1;
                    }
                }
                return n;
            } else {
                // Every round is drawn on the dial below. Three slots cannot hold them.
                0
            }
        }
        Task::Swarm => 0,
    }
}

/// Nearest living mission soldiers, or mission trucks.
pub(crate) fn nearest_marked(
    world: &World,
    pts: &mut [(i32, i32); 3],
    soldiers: bool,
    trucks: bool,
) -> usize {
    let mut dist = [i64::MAX; 3];
    let mut n = 0usize;
    let mut remember = |x: i32, y: i32| {
        let dx = x as i64 - world.x as i64;
        let dy = y as i64 - world.y as i64;
        let d = dx * dx + dy * dy;
        if n < 3 {
            pts[n] = (x, y);
            dist[n] = d;
            n += 1;
            return;
        }
        let mut worst = 0;
        if dist[1] > dist[worst] {
            worst = 1;
        }
        if dist[2] > dist[worst] {
            worst = 2;
        }
        if d < dist[worst] {
            pts[worst] = (x, y);
            dist[worst] = d;
        }
    };
    if soldiers {
        for soldier in &world.soldiers {
            if soldier.alive && soldier.objective {
                remember(soldier.x, soldier.y);
            }
        }
    }
    if trucks {
        for truck in &world.trucks {
            if truck.alive && truck.objective {
                remember(truck.x, truck.y);
            }
        }
    }
    n
}

/// Up to three nearest trucks, or trucks and soldiers when `trucks_only` is false.
pub(crate) fn nearest(world: &World, pts: &mut [(i32, i32); 3], trucks_only: bool) -> usize {
    let mut dist = [i64::MAX; 3];
    let mut n = 0usize;
    let remember = |n: &mut usize, dist: &mut [i64; 3], pts: &mut [(i32, i32); 3], x, y| {
        let dx = x as i64 - world.x as i64;
        let dy = y as i64 - world.y as i64;
        let d = dx * dx + dy * dy;
        if *n < 3 {
            pts[*n] = (x, y);
            dist[*n] = d;
            *n += 1;
            return;
        }
        let mut worst = 0;
        if dist[1] > dist[worst] {
            worst = 1;
        }
        if dist[2] > dist[worst] {
            worst = 2;
        }
        if d < dist[worst] {
            pts[worst] = (x, y);
            dist[worst] = d;
        }
    };
    for truck in &world.trucks {
        if truck.alive {
            remember(&mut n, &mut dist, pts, truck.x, truck.y);
        }
    }
    if !trucks_only {
        for soldier in &world.soldiers {
            if soldier.alive {
                remember(&mut n, &mut dist, pts, soldier.x, soldier.y);
            }
        }
    }
    n
}

/// A 3×3 dot. The sprite's lit pixels sit in its top-left corner.
pub(crate) fn show_dot(
    frame: &mut agb::display::GraphicsFrame,
    which: usize,
    radar_x: i32,
    inst_y: i32,
    bx: i32,
    by: i32,
) {
    show(
        frame,
        sprites::DOT.sprite(which),
        radar_x + 16 + bx - 1,
        inst_y + 16 + by - 1,
    );
}

/// Ahead is up. Far marks stay on the ring, in the bearing they actually have.
pub(crate) fn radar_blip(dx: i32, dy: i32, sin_r: i32, cos_r: i32) -> (i32, i32) {
    let dx = dx as i64;
    let dy = dy as i64;
    let sin_r = sin_r as i64;
    let cos_r = cos_r as i64;
    let fwd = (dx * sin_r - dy * cos_r) >> 16;
    let right = (dx * cos_r + dy * sin_r) >> 16;
    let dist = isqrt64(fwd * fwd + right * right);
    let den = dist.max(RADAR_RANGE as i64).max(1);
    let radius = 12i64;
    let x = (right * radius / den) as i32;
    let y = (-fwd * radius / den) as i32;
    (x, y)
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

pub(crate) fn draw_hud(frame: &mut agb::display::GraphicsFrame, world: &World) {
    show(frame, sprites::LBL_SCORE.sprite(0), 0, 0);
    draw_digits(frame, world.score, 4, 32, 0);
    show(frame, sprites::LBL_ALT.sprite(0), 96, 0);
    draw_digits(frame, world.alt >> 8, 2, 128, 0);
    if world.mode == Mode::Guide && world.guide > 0 {
        let secs = ((world.guide as i32) + 59) / 60;
        draw_digits(frame, secs, 1, VIEW_X + VIEW_W / 2 - 4, VIEW_Y + 6);
    }
    if world.mode != Mode::Fpv && world.mode != Mode::Guide {
        show(frame, sprites::LBL_BOMB.sprite(0), 0, 8);
        draw_digits(frame, world.bombs, 2, 32, 8);
        show(frame, sprites::LBL_HE.sprite(0), 56, 8);
        draw_digits(frame, world.he, 2, 88, 8);
        show(frame, sprites::LBL_MINE.sprite(0), 112, 8);
        draw_digits(frame, world.mines, 2, 144, 8);
        show(frame, sprites::LBL_VOG.sprite(0), 168, 8);
        draw_digits(frame, world.vog, 2, 200, 8);
    }
    if world.mode == Mode::Fpv {
        show(frame, sprites::LBL_DRONE.sprite(0), 160, 0);
        draw_digits(frame, world.drones, 1, 176, 0);
    }
    // How many mission marks are still left.
    show(frame, sprites::DOT.sprite(0), WIDTH - 28, 72);
    draw_digits(frame, world.targets, 2, WIDTH - 28, 80);
}

pub(crate) fn draw_world(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    sin_r: i32,
    cos_r: i32,
    horizon: i32,
    cockpit: bool,
    sensor: Sensor,
) {
    let floor = sky_bottom(horizon, world.alt);
    let ground_y = ground_line(horizon, world.alt, cockpit);
    draw_lase(frame, world, sin_r, cos_r, horizon, cockpit);
    for boom in &world.booms {
        if !boom.alive {
            continue;
        }
        let Some((sx, sy)) = project(
            world.x, world.y, world.alt, sin_r, cos_r, horizon, boom.x, boom.y, boom.z,
        ) else {
            continue;
        };
        if !in_view(sx, sy, cockpit) {
            continue;
        }
        // Guidance: a white disc on the ground, wider than the kill circle,
        // squashed so it lies on the field instead of standing up as fire.
        if world.mode == Mode::Guide && boom.radius > 0 && boom.z <= 2 << 8 {
            if boom.frame < 18 && sy >= floor {
                show_blast(
                    frame,
                    sx,
                    sy,
                    (sy - horizon).max(1),
                    world.alt,
                    boom.radius as i32,
                    boom.frame as i32,
                );
            }
            continue;
        }
        let frame_index = boom_sheet(boom.frame);
        let dist2 = dist2_px(world.x, world.y, boom.x, boom.y);
        // The FPV crash is drawn on the wire frame, in front of the glass.
        if world.mode == Mode::Fpv && dist2 < 48 * 48 {
            continue;
        }
        // Past this the fire sprite is a speck. A small flash stays readable,
        // including a hit that sits above the ground line.
        let far = dist2 >= 80 * 80;
        let airborne = boom.z > 2 << 8;
        if far {
            if boom.frame < 16 {
                show_spark(frame, sx, sy, cockpit);
            }
        } else if sy >= floor || airborne {
            // The fire is the blast. A near one also washes the ground, including through night glass.
            if boom.frame < 8 {
                show_spark(frame, sx, sy, cockpit);
            }
            if dist2 < 40 * 40 {
                show_fit(frame, sprites::BOOM.sprite(frame_index), sx, sy, 16, 40);
            } else {
                show(frame, sprites::BOOM.sprite(frame_index), sx - 8, sy - 8);
            }
        }
    }
    for shot in &world.shots {
        if !shot.alive {
            continue;
        }
        let Some((sx, sy)) = project(
            world.x, world.y, world.alt, sin_r, cos_r, horizon, shot.x, shot.y, shot.z,
        ) else {
            continue;
        };
        if !in_view(sx, sy, cockpit) {
            continue;
        }
        let (sprite, ox) = match shot.kind {
            KIND_HE => (sprites::HE.sprite(0), 4),
            KIND_MINE => (sprites::MINE.sprite(0), 4),
            KIND_VOG => (sprites::VOG.sprite(0), 4),
            KIND_GUN => (sprites::ROUND.sprite(0), 1),
            KIND_FLAK | KIND_SHELL => (sprites::ROUND.sprite(1), 1),
            _ => (sprites::BOMB.sprite(0), 4),
        };
        show(frame, sprite, sx - ox, sy - ox);
    }
    for mine in &world.mine_list {
        if !mine.alive {
            continue;
        }
        if mine.arm > 0 && (mine.arm / 6) % 2 == 0 {
            continue;
        }
        let Some((sx, sy)) = project(
            world.x, world.y, world.alt, sin_r, cos_r, horizon, mine.x, mine.y, 0,
        ) else {
            continue;
        };
        if !in_view(sx, sy, cockpit) || sy < floor {
            continue;
        }
        show(frame, sprites::MINE.sprite(0), sx - 4, sy - 4);
    }
    for cache in &world.caches {
        if !cache.alive {
            continue;
        }
        let Some((sx, sy)) = project(
            world.x, world.y, world.alt, sin_r, cos_r, horizon, cache.x, cache.y, 0,
        ) else {
            continue;
        };
        if !in_view(sx, sy, cockpit) || sy < floor {
            continue;
        }
        show(frame, sprites::MINE.sprite(0), sx - 4, sy - 4);
    }
    for foe in &world.foes {
        if !foe.alive {
            continue;
        }
        let alt_px = world.alt >> 8;
        let fz = foe.z >> 8;
        // Above the airframe, or on its height, there is no picture of them.
        // The same height is a collision. Only a drone underneath is drawn.
        if fz + 2 >= alt_px {
            continue;
        }
        let Some((sx, sy)) = project(
            world.x, world.y, world.alt, sin_r, cos_r, horizon, foe.x, foe.y, foe.z,
        ) else {
            continue;
        };
        let dx = delta(foe.x, world.x) >> 8;
        let dy = delta(foe.y, world.y) >> 8;
        let dz = alt_px - fz;
        let dist = isqrt64(
            dx as i64 * dx as i64 + dy as i64 * dy as i64 + dz as i64 * dz as i64,
        )
        .max(1);
        // Neither craft is drawn past its own art, so a close pass shows the pixels.
        let (src, pixels) = if foe.wing {
            (32, (32 * 88 / dist as i32).clamp(12, 32))
        } else {
            (16, (16 * 96 / dist as i32).clamp(8, 16))
        };
        let half = pixels / 2;
        if cockpit
            && (sx - half < VIEW_X
                || sx + half >= VIEW_X + VIEW_W
                || sy - half < VIEW_Y
                || sy + half >= VIEW_Y + VIEW_H)
        {
            continue;
        }
        if let Some((gx, gy)) = project(
            world.x, world.y, world.alt, sin_r, cos_r, horizon, foe.x, foe.y, 0,
        ) {
            if in_view(gx, gy, cockpit) && gy >= floor {
                let lift = 8 + (foe.z >> 8);
                show_shadow(frame, gx, gy, 16, false, 48, 6, world.yaw, horizon, sensor, lift);
            }
        }
        let frame_i = if nadir_on() > 0 && horizon < 12 { 1 } else { 0 };
        let sprite = if foe.wing {
            sprites::GERAN.sprite(frame_i)
        } else {
            sprites::FOE.sprite(frame_i)
        };
        show_fit(frame, sprite, sx, sy, src, pixels);
    }
    for soldier in &world.soldiers {
        if !soldier.alive {
            continue;
        }
        let Some((sx, sy)) = project(
            world.x, world.y, world.alt, sin_r, cos_r, horizon, soldier.x, soldier.y, 0,
        ) else {
            continue;
        };
        let man_px = ground_span((sy - horizon).max(1), world.alt, 16);
        if !actor_on_glass(sx, sy, man_px, floor, cockpit) {
            continue;
        }
        if marks_on_ground(sy, ground_y, cockpit) {
            if soldier.officer {
                show(frame, sprites::BLIP.sprite(1), sx - 4, sy - 22);
            }
            let man_max = if soldier.officer { HP_OFFICER } else { HP_MAN };
            show_life(frame, sx, sy, soldier.hp, man_max);
        }
        show_soldier(
            frame,
            soldier,
            sx,
            sy,
            horizon,
            world.alt,
            world.pitch,
            sin_r,
            cos_r,
            sensor,
            world.yaw,
        );
    }
    for truck in &world.trucks {
        if !truck.alive {
            continue;
        }
        let Some((sx, sy)) = project(
            world.x, world.y, world.alt, sin_r, cos_r, horizon, truck.x, truck.y, 0,
        ) else {
            continue;
        };
        let truck_px = ground_span((sy - horizon).max(1), world.alt, 12);
        if !actor_on_glass(sx, sy, truck_px, floor, cockpit) {
            continue;
        }
        if marks_on_ground(sy, ground_y, cockpit) {
            show_life(frame, sx, sy, truck.hp, HP_TRUCK);
        }
        show_truck(
            frame,
            truck,
            sx,
            sy,
            horizon,
            world.alt,
            world.pitch,
            sin_r,
            cos_r,
            sensor,
            world.yaw,
        );
    }
    for tank in &world.tanks {
        if !tank.alive {
            continue;
        }
        let Some((sx, sy)) = project(
            world.x, world.y, world.alt, sin_r, cos_r, horizon, tank.x, tank.y, 0,
        ) else {
            continue;
        };
        let tank_px = ground_span((sy - horizon).max(1), world.alt, 12);
        if !actor_on_glass(sx, sy, tank_px, floor, cockpit) {
            continue;
        }
        if marks_on_ground(sy, ground_y, cockpit) {
            show_life(frame, sx, sy, tank.hp, HP_TANK);
        }
        show_tank(
            frame,
            tank,
            sx,
            sy,
            horizon,
            world.alt,
            sin_r,
            cos_r,
            sensor,
            world.yaw,
        );
    }
    for bunker in &world.bunkers {
        if !bunker.alive {
            continue;
        }
        let Some((sx, sy)) = project(
            world.x, world.y, world.alt, sin_r, cos_r, horizon, bunker.x, bunker.y, 0,
        ) else {
            continue;
        };
        let bunker_px = ground_span((sy - horizon).max(1), world.alt, 14);
        if !actor_on_glass(sx, sy, bunker_px, floor, cockpit) {
            continue;
        }
        if marks_on_ground(sy, ground_y, cockpit) {
            show_life(frame, sx, sy, bunker.hp, HP_BUNKER);
        }
        show_bunker(
            frame,
            bunker,
            sx,
            sy,
            horizon,
            world.alt,
            world.x,
            world.y,
            sin_r,
            cos_r,
            sensor,
            world.yaw,
        );
    }
    draw_gate(frame, world, sin_r, cos_r, horizon, cockpit);
    draw_sites(frame, world, sin_r, cos_r, horizon, cockpit);
    draw_aa_guns(frame, world, sin_r, cos_r, horizon, cockpit, sensor);
    draw_grove(frame, world, sin_r, cos_r, horizon, floor, cockpit, sensor);
}

fn crown_spot(
    world: &World,
    sin_r: i32,
    cos_r: i32,
    horizon: i32,
    floor: i32,
    cockpit: bool,
    tx: i32,
    ty: i32,
    keep: bool,
) -> Option<(i32, i32, i32, u8)> {
    let tile = world.ground_at(tx, ty);
    if tile != T_TREE && tile != T_BUSH {
        return None;
    }
    let wx = (tx * 8 + 4) << 8;
    let wy = (ty * 8 + 4) << 8;
    let (sx, sy) = project(
        world.x, world.y, world.alt, sin_r, cos_r, horizon, wx, wy, 0,
    )?;
    let tall = if tile == T_BUSH { 8 } else { 20 };
    let span = ground_span((sy - horizon).max(1), world.alt, tall);
    let slack = if keep { 16 } else { 0 };
    let floor_lim = floor - slack;
    if sy < floor_lim {
        return None;
    }
    let view_bot = if cockpit { VIEW_Y + VIEW_H } else { HEIGHT };
    if sy - span >= view_bot + slack {
        return None;
    }
    if cockpit && (sx < VIEW_X - slack || sx >= VIEW_X + VIEW_W + slack) {
        return None;
    }
    Some((sx, sy, span, tile))
}

fn fill_grove(
    world: &World,
    sin_r: i32,
    cos_r: i32,
    horizon: i32,
    floor: i32,
    slots: &mut [(i32, i32); 6],
) {
    let tx0 = tile_of(world.x);
    let ty0 = tile_of(world.y);
    let cx0 = tx0.div_euclid(3);
    let cy0 = ty0.div_euclid(3);
    let mut picks = [(0i32, 0i32, i32::MAX); 6];
    let span_y = (HEIGHT - floor).max(1);
    for cy in (cy0 - 18)..=(cy0 + 18) {
        for cx in (cx0 - 12)..=(cx0 + 12) {
            if hash(cx, cy.wrapping_add(17)) % 6 != 0 {
                continue;
            }
            let tx = cx * 3 + 1;
            let ty = cy * 3 + 1;
            if grove_held(tx, ty, slots) {
                continue;
            }
            let grown = terrain(world.level, tx, ty);
            if grown != T_TREE && grown != T_BUSH {
                continue;
            }
            let wx = (tx * 8 + 4) << 8;
            let wy = (ty * 8 + 4) << 8;
            let rel_x = delta(wx, world.x);
            let rel_y = delta(wy, world.y);
            let fwd = ((rel_x as i64 * sin_r as i64) - (rel_y as i64 * cos_r as i64)) >> 8;
            if fwd < 256 {
                continue;
            }
            let right = ((rel_x as i64 * cos_r as i64) + (rel_y as i64 * sin_r as i64)) >> 8;
            if right.abs() * (FOCAL as i64) > fwd * 150 {
                continue;
            }
            let sy = horizon as i64
                + (world.alt as i64) * (FOCAL as i64) * (FLOOR_ZOOM as i64) / fwd;
            if sy < floor as i64 || sy > (HEIGHT + 80) as i64 {
                continue;
            }
            let sx = (WIDTH as i64) / 2 + right * (FOCAL as i64) / fwd;
            let col = if sx < (WIDTH as i64) / 2 { 0 } else { 1 };
            let row = (((sy as i32 - floor) * 3) / span_y).clamp(0, 2);
            let cell = (row * 2 + col) as usize;
            let cx = if col == 0 { WIDTH / 4 } else { WIDTH * 3 / 4 };
            let cy = floor + (row * 2 + 1) * span_y / 6;
            let score = (sx as i32 - cx).abs() + (sy as i32 - cy).abs();
            if score < picks[cell].2 {
                picks[cell] = (tx, ty, score);
            }
        }
    }
    for pick in picks {
        if pick.2 == i32::MAX {
            continue;
        }
        if crown_spot(
            world, sin_r, cos_r, horizon, floor, false, pick.0, pick.1, false,
        )
        .is_none()
        {
            continue;
        }
        if let Some(slot) = slots.iter_mut().find(|slot| slot.1 == i32::MIN) {
            *slot = (pick.0, pick.1);
        }
    }
}

struct GroveHold(core::cell::UnsafeCell<[(i32, i32); 6]>);
unsafe impl Sync for GroveHold {}
static GROVE_HOLD: GroveHold = GroveHold(core::cell::UnsafeCell::new([(0, i32::MIN); 6]));

fn grove_held(tx: i32, ty: i32, slots: &[(i32, i32); 6]) -> bool {
    slots.iter().any(|slot| slot.0 == tx && slot.1 == ty)
}

/// Six crowns, two across and three deep. A crown stays until it leaves
/// the glass, so a neighbour nearer the sight does not take its place.
fn draw_grove(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    sin_r: i32,
    cos_r: i32,
    horizon: i32,
    floor: i32,
    cockpit: bool,
    sensor: Sensor,
) {
    let slots = unsafe { &mut *GROVE_HOLD.0.get() };
    for slot in slots.iter_mut() {
        if slot.1 == i32::MIN {
            continue;
        }
        if crown_spot(world, sin_r, cos_r, horizon, floor, cockpit, slot.0, slot.1, true).is_none()
        {
            *slot = (0, i32::MIN);
        }
    }
    if slots.iter().any(|slot| slot.1 == i32::MIN) {
        fill_grove(world, sin_r, cos_r, horizon, floor, slots);
    }
    // Near first, so a closer crown sits in front of the one behind it.
    let mut order = [(0i32, 0usize); 6];
    let mut count = 0;
    for (index, slot) in slots.iter().enumerate() {
        if slot.1 == i32::MIN {
            continue;
        }
        let Some((_, sy, _, _)) =
            crown_spot(world, sin_r, cos_r, horizon, floor, cockpit, slot.0, slot.1, true)
        else {
            continue;
        };
        order[count] = (sy, index);
        count += 1;
    }
    for a in 0..count {
        for b in (a + 1)..count {
            if order[b].0 > order[a].0 {
                order.swap(a, b);
            }
        }
    }
    for n in 0..count {
        let (tx, ty) = slots[order[n].1];
        let Some((sx, sy, _, tile)) =
            crown_spot(world, sin_r, cos_r, horizon, floor, cockpit, tx, ty, true)
        else {
            continue;
        };
        let burning = world
            .burns
            .iter()
            .any(|burn| burn.alive && burn.tx == tx && burn.ty == ty);
        let plan = nadir_on() > 0 && horizon < 12;
        if tile == T_BUSH {
            let span = ground_span((sy - horizon).max(1), world.alt, 8);
            if burning {
                let fy = if plan { sy - span / 2 } else { sy - span - 4 };
                show_flame(
                    frame,
                    sprites::BOOM.sprite(((world.age / 5) as usize + tx as usize) % 3),
                    sx - 8,
                    fy,
                    sensor,
                );
            }
            let bush_sheet = if matches!(sensor, Sensor::Ir) {
                sprites::BUSH_IR.sprite((hash(tx, ty) % 2) as usize)
            } else {
                sprites::BUSH.sprite((hash(tx, ty) % 2) as usize)
            };
            show_figure(
                frame,
                bush_sheet,
                sx,
                sy,
                16,
                16,
                if plan { 11 } else { 10 },
                14,
                span,
                plan,
                sensor,
                false,
            );
        } else if plan {
            let span = ground_span((sy - horizon).max(1), world.alt, 10);
            if burning {
                show_flame(
                    frame,
                    sprites::BOOM.sprite(((world.age / 5) as usize + tx as usize) % 3),
                    sx - 8,
                    sy - span / 2,
                    sensor,
                );
            }
            let crown = if matches!(sensor, Sensor::Ir) {
                sprites::GROVE_IR.sprite(3)
            } else {
                sprites::GROVE.sprite(3)
            };
            show_figure(
                frame, crown, sx, sy, 16, 32, 11, 16, span, true, sensor, false,
            );
        } else {
            let span = ground_span((sy - horizon).max(1), world.alt, 20);
            if burning {
                show_flame(
                    frame,
                    sprites::BOOM.sprite(((world.age / 5) as usize + tx as usize) % 3),
                    sx - 8,
                    sy - span * 2 / 3 - 8,
                    sensor,
                );
            }
            let tree_sheet = if matches!(sensor, Sensor::Ir) {
                sprites::GROVE_IR.sprite((hash(tx, ty) % 3) as usize)
            } else {
                sprites::GROVE.sprite((hash(tx, ty) % 3) as usize)
            };
            show_figure(
                frame,
                tree_sheet,
                sx,
                sy,
                16,
                32,
                28,
                30,
                span,
                false,
                sensor,
                false,
            );
        }
    }
}

/// Two dark posts and a lintel. The opening is the mission.
pub(crate) fn draw_gate(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    sin_r: i32,
    cos_r: i32,
    horizon: i32,
    cockpit: bool,
) {
    if !matches!(task_of(world.level), Task::Gate) {
        return;
    }
    let gx = world.gate_x();
    let gy = world.gate_y();
    for dx in [-36, 36] {
        let Some((sx, sy)) = project(
            world.x, world.y, world.alt, sin_r, cos_r, horizon, gx + (dx << 8), gy, 0,
        ) else {
            continue;
        };
        if cockpit && !in_view(sx, sy, true) {
            continue;
        }
        show(frame, sprites::POST.sprite(0), sx - 4, sy - 32);
    }
    let Some((sx, sy)) = project(
        world.x, world.y, world.alt, sin_r, cos_r, horizon, gx, gy, 16 << 8,
    ) else {
        return;
    };
    if !cockpit || in_view(sx, sy, true) {
        show(frame, sprites::BEAM.sprite(0), sx - 16, sy - 4);
    }
}

/// The convoy's pale mark, and the headquarters building.
pub(crate) fn draw_sites(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    sin_r: i32,
    cos_r: i32,
    horizon: i32,
    cockpit: bool,
) {
    let task = task_of(world.level);
    if matches!(task, Task::Convoy) {
        for ty in [3, 4] {
            let gy = (ty * 8 + 4) << 8;
            let Some((sx, sy)) = project(
                world.x,
                world.y,
                world.alt,
                sin_r,
                cos_r,
                horizon,
                (0 * 8 + 4) << 8,
                gy,
                0,
            ) else {
                continue;
            };
            if cockpit && !in_view(sx, sy, true) {
                continue;
            }
            show(frame, sprites::POST.sprite(0), sx - 4, sy - 32);
        }
    }
    if matches!(task, Task::Cover) {
        for tx in [16, 32] {
            let Some((sx, sy)) = project(
                world.x,
                world.y,
                world.alt,
                sin_r,
                cos_r,
                horizon,
                (tx * 8 + 4) << 8,
                (14 * 8 + 4) << 8,
                0,
            ) else {
                continue;
            };
            if cockpit && !in_view(sx, sy, true) {
                continue;
            }
            show(frame, sprites::POST.sprite(0), sx - 4, sy - 32);
        }
    }
    if matches!(task, Task::Rush | Task::Push) {
        for tx in [8, 24, 40, 56] {
            let Some((sx, sy)) = project(
                world.x,
                world.y,
                world.alt,
                sin_r,
                cos_r,
                horizon,
                (tx * 8 + 4) << 8,
                (14 * 8 + 4) << 8,
                0,
            ) else {
                continue;
            };
            if cockpit && !in_view(sx, sy, true) {
                continue;
            }
            show(frame, sprites::POST.sprite(0), sx - 4, sy - 32);
        }
    }
    if matches!(task, Task::Hq) && world.targets > 0 {
        let hx = (23 * 8 + 4) << 8;
        let hy = (16 * 8 + 8) << 8;
        let Some((sx, sy)) = project(
            world.x, world.y, world.alt, sin_r, cos_r, horizon, hx, hy, 0,
        ) else {
            return;
        };
        if cockpit && !in_view(sx, sy, true) {
            return;
        }
        let drop = (sy - horizon).max(1);
        let span = ground_span(drop, world.alt, 20);
        let (sprite, native, ink, foot) = if detail_on() && span >= 16 {
            (sprites::HQ_NEAR.sprite(0), 64, 46, 57)
        } else {
            (sprites::HQ.sprite(0), 32, 12, 30)
        };
        show_figure(
            frame, sprite, sx, sy, native, native, ink, foot, span, false, Sensor::Day, false,
        );
    }
}

/// The batteries themselves. Their circles are the holes in the radar lane.
pub(crate) fn draw_aa_guns(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    sin_r: i32,
    cos_r: i32,
    horizon: i32,
    cockpit: bool,
    sensor: Sensor,
) {
    let guns = aa_guns(task_of(world.level));
    if guns.is_empty() {
        return;
    }
    let floor = sky_bottom(horizon, world.alt);
    for gun in guns {
        let (x, y) = gun_at(gun);
        let Some((sx, sy)) = project(
            world.x, world.y, world.alt, sin_r, cos_r, horizon, x, y, 0,
        ) else {
            continue;
        };
        let gun_px = ground_span((sy - horizon).max(1), world.alt, 14);
        if !actor_on_glass(sx, sy, gun_px, floor, cockpit) {
            continue;
        }
        let bunker = Bunker {
            x,
            y,
            home_tx: gun.tx,
            home_ty: gun.ty,
            objective: false,
            hp: 1,
            cool: 0,
            alive: true,
        };
        show_bunker(
            frame,
            &bunker,
            sx,
            sy,
            horizon,
            world.alt,
            world.x,
            world.y,
            sin_r,
            cos_r,
            sensor,
            world.yaw,
        );
    }
}
