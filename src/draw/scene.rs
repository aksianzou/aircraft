//! What stands on the field: the eye, the figures, their shadows, the woods.

use agb::{
    display::{
        HEIGHT, Priority, WIDTH,
        object::{AffineMode, Object, ObjectAffine, Sprite},
    },
    fixnum::Num,
};

use crate::sprites;

use super::*;

/// Where the picture is taken from this frame: the airframe, its heading, and
/// the horizon line. Everything drawn on the field goes through this.
#[derive(Clone, Copy)]
pub(crate) struct Eye {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) alt: i32,
    pub(crate) yaw: Num<i32, 8>,
    pub(crate) pitch: Num<i32, 8>,
    pub(crate) sin_r: i32,
    pub(crate) cos_r: i32,
    pub(crate) horizon: i32,
}

impl Eye {
    pub(crate) fn new(world: &World, horizon: i32) -> Self {
        let (sin_r, cos_r) = world.heading();
        Self {
            x: world.x,
            y: world.y,
            alt: world.alt,
            yaw: world.yaw,
            pitch: world.pitch,
            sin_r,
            cos_r,
            horizon,
        }
    }

    /// A point of the world on the screen, or `None` off the glass.
    pub(crate) fn project(&self, world_x: i32, world_y: i32, world_z: i32) -> Option<(i32, i32)> {
        self.ground_point(world_x, world_y, world_z, false)
    }

    /// `wide` keeps a point that has already been shown a little longer.
    fn ground_point(
        &self,
        world_x: i32,
        world_y: i32,
        world_z: i32,
        wide: bool,
    ) -> Option<(i32, i32)> {
        let dx = delta(world_x, self.x);
        let dy = delta(world_y, self.y);
        let fwd = ((dx * self.sin_r) - (dy * self.cos_r)) >> 8;
        let right = ((dx * self.cos_r) + (dy * self.sin_r)) >> 8;
        if let scale = nadir_on()
            && scale > 0
        {
            let sx = WIDTH / 2 + (right >> 8) / scale;
            // Height is the same quantity the forward glass uses. Looking down,
            // it lifts the sprite off the map point instead of dropping it into
            // the crosshair.
            let sy = 88 - (fwd >> 8) / scale - (world_z >> 8) / scale;
            let (x0, x1, y0, y1) = if wide {
                (-80, WIDTH + 80, -48, HEIGHT + 64)
            } else {
                (-32, WIDTH + 32, -32, HEIGHT + 32)
            };
            if !(x0..x1).contains(&sx) || !(y0..y1).contains(&sy) {
                return None;
            }
            return Some((sx, sy));
        }
        // A held crown may stand a little closer than a new pick. The near
        // plane still keeps the division off zero.
        if fwd < if wide { 64 } else { 192 } {
            return None;
        }
        let sy = self.horizon + (self.alt - world_z) * FOCAL * FLOOR_ZOOM / fwd;
        let sx = WIDTH / 2 + right * FOCAL / fwd;
        // Feet may sit below the glass while the body is still in it.
        let (x0, x1, y0, y1) = if wide {
            (-80, WIDTH + 80, -80, HEIGHT + 240)
        } else {
            (-32, WIDTH + 32, -32, HEIGHT + 160)
        };
        if !(x0..x1).contains(&sx) || !(y0..y1).contains(&sy) {
            return None;
        }
        Some((sx, sy))
    }
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

fn on_plan(horizon: i32) -> bool {
    nadir_on() > 0 && horizon < 12
}

/// One placement for every body. The feet stay on the projected ground
/// point, `span` is the painted height, and the close sheet starts at
/// `near_at` screen pixels. Overhead uses the plan sheet instead.
struct BodyScale {
    span: i32,
    plan: bool,
    near: bool,
}

fn body_scale(sy: i32, horizon: i32, alt: i32, world_h: i32, near_at: i32) -> BodyScale {
    let span = ground_span((sy - horizon).max(1), alt, world_h);
    let plan = on_plan(horizon);
    BodyScale {
        span,
        plan,
        near: !plan && detail_on() && span >= near_at,
    }
}

/// Feet may leave the bottom of the glass. The body stays while any of it
/// is still on the picture, and the roof may stand up into the sky.
fn actor_on_glass(sx: i32, sy: i32, span: i32, floor: i32, cockpit: bool) -> bool {
    if cockpit && !(VIEW_X..VIEW_X + VIEW_W).contains(&sx) {
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

/// How a sheet is drawn: its cell, the height of the ink in it, and the row
/// the feet stand on.
#[derive(Clone, Copy)]
struct Art {
    w: i32,
    h: i32,
    ink: i32,
    foot: i32,
}

/// How a figure is coloured: through the glass, or burnt out.
#[derive(Clone, Copy)]
enum Coat {
    Live(Sensor),
    Burnt,
}

impl Coat {
    fn of(sensor: Sensor, wreck: bool) -> Self {
        if wreck {
            Coat::Burnt
        } else {
            Coat::Live(sensor)
        }
    }
}

fn show_figure(
    frame: &mut agb::display::GraphicsFrame,
    sprite: &'static Sprite,
    (sx, sy): (i32, i32),
    art: Art,
    span: i32,
    top: bool,
    coat: Coat,
) {
    let Art {
        w: native_w,
        h: native_h,
        ink,
        foot,
    } = art;
    let display = (span.max(1) * native_h / ink.max(1)).clamp(4, native_h * 2);
    let inv = Num::<i32, 8>::new(native_h) / Num::new(display.max(1));
    let (x, y, mode) = place_actor(sx, sy, native_w, native_h, display, foot, top);
    if x < -native_w * 2 || y < -native_h * 2 || x > WIDTH || y > HEIGHT {
        return;
    }
    let mut object = ObjectAffine::new(vram(sprite), scale_matrix(inv, inv), mode);
    let painted = match coat {
        // Ash is the same through every glass.
        Coat::Burnt => glass(Sensor::Day, Tone::Ash),
        Coat::Live(sensor) => dye(sprite, sensor),
    };
    if let Some(palette) = painted {
        object.set_palette(palette);
    }
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
        1..=3 => 2,
        5..=7 => 6,
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

pub(crate) fn show_soldier(
    frame: &mut agb::display::GraphicsFrame,
    soldier: &Soldier,
    sx: i32,
    sy: i32,
    eye: &Eye,
    sensor: Sensor,
    wreck: bool,
) {
    let Eye {
        horizon,
        alt,
        pitch,
        sin_r,
        cos_r,
        ..
    } = *eye;
    let scale = body_scale(sy, horizon, alt, 8, 22);
    if wreck {
        // The sheet's last cell is a body on the ground, not a man still standing.
        show_figure(
            frame,
            sprites::SOLDIER.sprite(16),
            (sx, sy),
            Art {
                w: 32,
                h: 32,
                ink: 18,
                foot: 26,
            },
            scale.span,
            false,
            Coat::Burnt,
        );
        show_shadow(frame, eye, (sx, sy), scale.span * 3 / 8, false, sensor, 4);
        return;
    }
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
    if scale.near && frame_index < 8 {
        show_figure(
            frame,
            sprites::SOLDIER_NEAR.sprite(four_view(frame_index)),
            (sx, sy),
            Art {
                w: 32,
                h: 64,
                ink: 46,
                foot: 61,
            },
            scale.span,
            false,
            Coat::of(sensor, wreck),
        );
    } else {
        show_figure(
            frame,
            sprites::SOLDIER.sprite(frame_index),
            (sx, sy),
            Art {
                w: 32,
                h: 32,
                ink: 18,
                foot: 26,
            },
            scale.span,
            false,
            Coat::of(sensor, wreck),
        );
    }
    show_shadow(frame, eye, (sx, sy), scale.span * 4 / 8, false, sensor, 8);
}

pub(crate) fn show_truck(
    frame: &mut agb::display::GraphicsFrame,
    truck: &Truck,
    sx: i32,
    sy: i32,
    eye: &Eye,
    sensor: Sensor,
    wreck: bool,
) {
    let Eye {
        horizon,
        alt,
        pitch,
        sin_r,
        cos_r,
        ..
    } = *eye;
    let scale = body_scale(sy, horizon, alt, 12, 14);
    let (fx, fy) = if truck.vertical {
        (0, truck.dir.signum())
    } else {
        (truck.dir.signum(), 0)
    };
    // Level flight keeps the flat front, side and rear. A three-quarter frame
    // puts the cab up the screen and the truck looks reared. A dive uses the
    // flatter row, the same way a man does. Looking straight down, the plan
    // sheet is a roof and the near wall.
    let fwd = fx * sin_r - fy * cos_r;
    let right = fx * cos_r + fy * sin_r;
    let oct = heading_octant(fwd, right);
    let pitched = pitch.sin().to_raw() >= 100;
    let (sprite, native_w, native_h, ink, foot) = if scale.plan {
        // The ink is the long axis of that frame, so turning does not resize it.
        let ink = match oct & 7 {
            0 => 12,
            2 | 6 => 16,
            4 => 15,
            _ => 19,
        };
        (sprites::TRUCK_TOP.sprite(oct), 32, 32, ink, 25)
    } else if scale.near && !pitched {
        (sprites::TRUCK_NEAR.sprite(four_view(oct)), 64, 32, 21, 29)
    } else {
        let index = level_octant(oct) + if pitched { 8 } else { 0 };
        (sprites::TRUCK.sprite(index), 32, 32, 9, 25)
    };
    show_figure(
        frame,
        sprite,
        (sx, sy),
        Art {
            w: native_w,
            h: native_h,
            ink,
            foot,
        },
        scale.span,
        scale.plan,
        Coat::of(sensor, wreck),
    );
    show_shadow(
        frame,
        eye,
        (sx, sy),
        scale.span * 5 / 8,
        scale.plan,
        sensor,
        8,
    );
}

pub(crate) fn show_tank(
    frame: &mut agb::display::GraphicsFrame,
    tank: &Tank,
    sx: i32,
    sy: i32,
    eye: &Eye,
    sensor: Sensor,
    wreck: bool,
) {
    let Eye {
        horizon,
        alt,
        pitch,
        sin_r,
        cos_r,
        ..
    } = *eye;
    let scale = body_scale(sy, horizon, alt, 12, 14);
    let (fx, fy) = if tank.vertical {
        (0, tank.dir.signum())
    } else {
        (tank.dir.signum(), 0)
    };
    let fwd = fx * sin_r - fy * cos_r;
    let right = fx * cos_r + fy * sin_r;
    let oct = heading_octant(fwd, right);
    let pitched = pitch.sin().to_raw() >= 100;
    let (sprite, native_w, native_h, ink, foot) = if scale.plan {
        let ink = match oct & 7 {
            0 => 12,
            4 => 14,
            2 | 6 => 17,
            _ => 18,
        };
        (sprites::TANK_TOP.sprite(oct), 32, 32, ink, 25)
    } else if scale.near && !pitched {
        (sprites::TANK_NEAR.sprite(four_view(oct)), 64, 32, 21, 27)
    } else {
        let index = level_octant(oct) + if pitched { 8 } else { 0 };
        (sprites::TANK.sprite(index), 32, 32, 8, 25)
    };
    show_figure(
        frame,
        sprite,
        (sx, sy),
        Art {
            w: native_w,
            h: native_h,
            ink,
            foot,
        },
        scale.span,
        scale.plan,
        Coat::of(sensor, wreck),
    );
    show_shadow(
        frame,
        eye,
        (sx, sy),
        scale.span * 7 / 8,
        scale.plan,
        sensor,
        8,
    );
}

pub(crate) fn show_bunker(
    frame: &mut agb::display::GraphicsFrame,
    bunker: &Bunker,
    sx: i32,
    sy: i32,
    eye: &Eye,
    sensor: Sensor,
    wreck: bool,
) {
    let Eye {
        horizon,
        alt,
        pitch,
        x: cam_x,
        y: cam_y,
        sin_r,
        cos_r,
        ..
    } = *eye;
    let scale = body_scale(sy, horizon, alt, 16, 12);
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
    let pitched = pitch.sin().to_raw() >= 100;
    let (sprite, native_w, native_h, ink, foot) = if scale.plan {
        let ink = match oct & 7 {
            2 | 6 => 12,
            0 | 4 => 14,
            _ => 18,
        };
        (sprites::BUNKER_TOP.sprite(oct), 32, 32, ink, 25)
    } else if scale.near && !pitched {
        (sprites::BUNKER_NEAR.sprite(four_view(oct)), 64, 32, 22, 29)
    } else {
        let index = level_octant(oct) + if pitched { 8 } else { 0 };
        (sprites::BUNKER.sprite(index), 32, 32, 8, 25)
    };
    show_figure(
        frame,
        sprite,
        (sx, sy),
        Art {
            w: native_w,
            h: native_h,
            ink,
            foot,
        },
        scale.span,
        scale.plan,
        Coat::of(sensor, wreck),
    );
    show_shadow(
        frame,
        eye,
        (sx, sy),
        scale.span * 6 / 8,
        scale.plan,
        sensor,
        8,
    );
}

/// Flat contact mark. It sits off the feet, away from the sun or the moon.
/// `lift` is 8 on the ground and longer while the body is in the air.
/// `width` is the shadow across, in pixels, before the clamp.
pub(crate) fn show_shadow(
    frame: &mut agb::display::GraphicsFrame,
    eye: &Eye,
    (sx, sy): (i32, i32),
    width: i32,
    top: bool,
    sensor: Sensor,
    lift: i32,
) {
    let Eye { yaw, horizon, .. } = *eye;
    // Straight down, the plan already covers the ground a vehicle stands on.
    if top {
        return;
    }
    let cx = sx;
    let foot = sy;
    let w = width.clamp(6, 40);
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
        vram(sprites::SHADOW.sprite(0)),
        scale_matrix(inv_x, inv_y),
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

pub(crate) fn draw_world(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    eye: &Eye,
    cockpit: bool,
    sensor: Sensor,
) {
    let Eye { horizon, .. } = *eye;
    paint_glass(sensor);
    let floor = sky_bottom(horizon, world.alt);
    let ground_y = ground_line(horizon, world.alt, cockpit);
    draw_lase(frame, world, eye, cockpit);
    draw_drill(frame, world, eye, cockpit);
    for boom in &world.booms {
        if !boom.alive {
            continue;
        }
        let Some((sx, sy)) = eye.project(boom.x, boom.y, boom.z) else {
            continue;
        };
        if !in_view(sx, sy, cockpit) {
            continue;
        }
        // Monochrome glass, and a hatch cook-off, are the white flash then the cloud.
        if matches!(sensor, Sensor::Ir) || boom.plume {
            show_smoke(frame, sx, sy, boom.frame);
            continue;
        }
        // A bullet or a grenade is a spark. The fireball is a bomb or a shell.
        if boom.radius <= 4 {
            if boom.frame < 12 {
                show_spark(frame, sx, sy, cockpit);
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
        // including a hit that sits above the ground line. A HIMARS hit on
        // the ground still shows the fire: the spotter is usually far above it.
        let guide_hit = world.mode == Mode::Guide && boom.z <= 2 << 8;
        let far = !guide_hit && dist2 >= 80 * 80;
        let airborne = boom.z > 2 << 8;
        if far {
            if boom.frame < 16 {
                show_spark(frame, sx, sy, cockpit);
            }
        } else if sy >= floor || airborne || guide_hit {
            // The fire is the blast. A near one also washes the ground, including through night glass.
            if boom.frame < 8 {
                show_spark(frame, sx, sy, cockpit);
            }
            if dist2 < 40 * 40 || (guide_hit && boom.radius >= 12) {
                let pixels = if guide_hit { 28 } else { 40 };
                show_fit(frame, sprites::BOOM.sprite(frame_index), sx, sy, 16, pixels);
            } else {
                show_flame(
                    frame,
                    sprites::BOOM.sprite(frame_index),
                    sx - 8,
                    sy - 8,
                    sensor,
                );
            }
        }
    }
    for bit in &world.scrap {
        if !bit.alive {
            continue;
        }
        let Some((sx, sy)) = eye.project(bit.x, bit.y, bit.z) else {
            continue;
        };
        if !in_view(sx, sy, cockpit) {
            continue;
        }
        let kind = (bit.kind as usize).min(2);
        let mut object = Object::new(vram(sprites::SCRAP.sprite(kind)));
        if !matches!(sensor, Sensor::Day | Sensor::Winter)
            && let Some(palette) = glass(sensor, Tone::Ash)
        {
            object.set_palette(palette);
        }
        object
            .set_pos((sx - 8, sy - 8))
            .set_priority(Priority::P0)
            .show(frame);
    }
    for shot in &world.shots {
        if !shot.alive {
            continue;
        }
        let Some((sx, sy)) = eye.project(shot.x, shot.y, shot.z) else {
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
        let Some((sx, sy)) = eye.project(mine.x, mine.y, 0) else {
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
        let Some((sx, sy)) = eye.project(cache.x, cache.y, 0) else {
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
        let Some((sx, sy)) = eye.project(foe.x, foe.y, foe.z) else {
            continue;
        };
        let dx = delta(foe.x, world.x) >> 8;
        let dy = delta(foe.y, world.y) >> 8;
        let dz = alt_px - fz;
        let dist =
            isqrt64(dx as i64 * dx as i64 + dy as i64 * dy as i64 + dz as i64 * dz as i64).max(1);
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
        if let Some((gx, gy)) = eye.project(foe.x, foe.y, 0)
            && in_view(gx, gy, cockpit)
            && gy >= floor
        {
            let lift = 8 + (foe.z >> 8);
            // Six eighths of the 16 pixels a quad is across.
            show_shadow(frame, eye, (gx, gy), 12, false, sensor, lift);
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
        let Some((sx, sy)) = eye.project(soldier.x, soldier.y, 0) else {
            continue;
        };
        let man_px = ground_span((sy - horizon).max(1), world.alt, 8);
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
        show_soldier(frame, soldier, sx, sy, eye, sensor, false);
    }
    for truck in &world.trucks {
        if !truck.alive {
            continue;
        }
        let Some((sx, sy)) = eye.project(truck.x, truck.y, 0) else {
            continue;
        };
        let truck_px = ground_span((sy - horizon).max(1), world.alt, 12);
        if !actor_on_glass(sx, sy, truck_px, floor, cockpit) {
            continue;
        }
        if marks_on_ground(sy, ground_y, cockpit) {
            show_life(frame, sx, sy, truck.hp, HP_TRUCK);
        }
        show_truck(frame, truck, sx, sy, eye, sensor, false);
    }
    for tank in &world.tanks {
        if !tank.alive {
            continue;
        }
        let Some((sx, sy)) = eye.project(tank.x, tank.y, 0) else {
            continue;
        };
        let tank_px = ground_span((sy - horizon).max(1), world.alt, 12);
        if !actor_on_glass(sx, sy, tank_px, floor, cockpit) {
            continue;
        }
        if marks_on_ground(sy, ground_y, cockpit) {
            show_life(frame, sx, sy, tank.hp, HP_TANK);
        }
        show_tank(frame, tank, sx, sy, eye, sensor, false);
    }
    for bunker in &world.bunkers {
        if !bunker.alive {
            continue;
        }
        let Some((sx, sy)) = eye.project(bunker.x, bunker.y, 0) else {
            continue;
        };
        let bunker_px = ground_span((sy - horizon).max(1), world.alt, 16);
        if !actor_on_glass(sx, sy, bunker_px, floor, cockpit) {
            continue;
        }
        if marks_on_ground(sy, ground_y, cockpit) {
            show_life(frame, sx, sy, bunker.hp, HP_BUNKER);
        }
        show_bunker(frame, bunker, sx, sy, eye, sensor, false);
    }
    draw_husks(frame, world, eye, floor, cockpit, sensor);
    draw_gate(frame, world, eye, cockpit);
    draw_sites(frame, world, eye, cockpit);
    draw_aa_guns(frame, world, eye, cockpit, sensor);
    draw_grove(frame, world, eye, floor, cockpit, sensor);
}

fn draw_husks(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    eye: &Eye,
    floor: i32,
    cockpit: bool,
    sensor: Sensor,
) {
    let Eye { horizon, .. } = *eye;
    for husk in &world.husks {
        if !husk.alive {
            continue;
        }
        let Some((sx, sy)) = eye.project(husk.x, husk.y, 0) else {
            continue;
        };
        let tall = match husk.kind {
            HUSK_MAN => 8,
            HUSK_BUNKER => 16,
            _ => 12,
        };
        let span = ground_span((sy - horizon).max(1), world.alt, tall);
        if !actor_on_glass(sx, sy, span, floor, cockpit) {
            continue;
        }
        match husk.kind {
            HUSK_TANK => {
                let tank = Tank {
                    x: husk.x,
                    y: husk.y,
                    dir: husk.dir,
                    vertical: husk.vertical,
                    home_tx: 0,
                    home_ty: 0,
                    objective: false,
                    hp: 0,
                    cool: 0,
                    alive: true,
                };
                show_tank(frame, &tank, sx, sy, eye, sensor, true);
            }
            HUSK_BUNKER => {
                let bunker = Bunker {
                    x: husk.x,
                    y: husk.y,
                    home_tx: 0,
                    home_ty: 0,
                    objective: false,
                    hp: 0,
                    cool: 0,
                    alive: true,
                };
                show_bunker(frame, &bunker, sx, sy, eye, sensor, true);
            }
            HUSK_MAN => {
                let soldier = Soldier {
                    x: husk.x,
                    y: husk.y,
                    dir: husk.dir,
                    step: 0,
                    home_tx: 0,
                    home_ty: 0,
                    officer: false,
                    objective: false,
                    hp: 0,
                    cool: 0,
                    alive: true,
                };
                show_soldier(frame, &soldier, sx, sy, eye, sensor, true);
            }
            _ => {
                let truck = Truck {
                    x: husk.x,
                    y: husk.y,
                    dir: husk.dir,
                    vertical: husk.vertical,
                    home_tx: 0,
                    home_ty: 0,
                    goal: false,
                    objective: false,
                    hp: 0,
                    cool: 0,
                    alive: true,
                };
                show_truck(frame, &truck, sx, sy, eye, sensor, true);
            }
        }
    }
}

fn crown_spot(
    world: &World,
    eye: &Eye,
    floor: i32,
    cockpit: bool,
    tx: i32,
    ty: i32,
    keep: bool,
) -> Option<(i32, i32, i32, u8)> {
    let Eye { horizon, .. } = *eye;
    let tile = world.ground_at(tx, ty);
    if tile != T_TREE && tile != T_BUSH && tile != T_STUMP {
        return None;
    }
    let wx = tile_mid(tx);
    let wy = tile_mid(ty);
    let (sx, sy) = eye.ground_point(wx, wy, 0, keep)?;
    let plan = on_plan(horizon);
    // A man is 8, a truck 12, a bunker 16. A pine stands over them. A bush
    // stays a shrub, and a burned trunk is the stick without the crown.
    let world_h = if tile == T_STUMP {
        10
    } else if tile == T_BUSH {
        6
    } else if plan {
        10
    } else {
        28
    };
    let scale = body_scale(sy, horizon, world.alt, world_h, i32::MAX);
    let span = scale.span;
    let slack = if keep { 24 } else { 0 };
    let floor_lim = floor - slack;
    if sy < floor_lim {
        return None;
    }
    let view_bot = if cockpit { VIEW_Y + VIEW_H } else { HEIGHT };
    // The feet stay on the ground point. The crown leaves only once the
    // whole body has gone past the glass, so it does not freeze and slide.
    if sy - span >= view_bot + slack {
        return None;
    }
    // A new crown has to be standing on the glass. One already held may
    // walk off the bottom as the aircraft closes in, still at full size.
    if !keep && sy > view_bot {
        return None;
    }
    if cockpit && (sx < VIEW_X - slack || sx >= VIEW_X + VIEW_W + slack) {
        return None;
    }
    Some((sx, sy, span, tile))
}

fn fill_grove(
    world: &World,
    eye: &Eye,
    floor: i32,
    cockpit: bool,
    rows: core::ops::RangeInclusive<i32>,
    picks: &mut [(i32, i32, i32); 6],
    slots: &[CrownSlot; 6],
) {
    let cx0 = tile_of(world.x).div_euclid(3);
    let span_y = (HEIGHT - floor).max(1);
    for cy in rows {
        for cx in (cx0 - 12)..=(cx0 + 12) {
            if !hash(cx, cy.wrapping_add(17)).is_multiple_of(6) {
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
            let Some((sx, sy, _, _)) = crown_spot(world, eye, floor, cockpit, tx, ty, false) else {
                continue;
            };
            let col = if sx < WIDTH / 2 { 0 } else { 1 };
            let row = ((sy - floor) * 3 / span_y).clamp(0, 2);
            let cell = (row * 2 + col) as usize;
            let cx = if col == 0 { WIDTH / 4 } else { WIDTH * 3 / 4 };
            let cy_screen = floor + (row * 2 + 1) * span_y / 6;
            let score = (sx - cx).abs() + (sy - cy_screen).abs();
            if score < picks[cell].2 {
                picks[cell] = (tx, ty, score);
            }
        }
    }
}

fn take_picks(slots: &mut [CrownSlot; 6], picks: &[(i32, i32, i32); 6]) {
    for pick in picks {
        if pick.2 == i32::MAX {
            continue;
        }
        if let Some(slot) = slots.iter_mut().find(|slot| slot.ty == i32::MIN) {
            *slot = CrownSlot {
                tx: pick.0,
                ty: pick.1,
                sx: 0,
                sy: 0,
                span: 0,
                tile: 0,
                miss: 0,
            };
        }
    }
}

#[derive(Clone, Copy)]
struct CrownSlot {
    tx: i32,
    ty: i32,
    sx: i32,
    sy: i32,
    span: i32,
    tile: u8,
    miss: u8,
}

struct GroveHold(core::cell::UnsafeCell<[CrownSlot; 6]>);
unsafe impl Sync for GroveHold {}
static GROVE_HOLD: GroveHold = GroveHold(core::cell::UnsafeCell::new(
    [CrownSlot {
        tx: 0,
        ty: i32::MIN,
        sx: 0,
        sy: 0,
        span: 0,
        tile: 0,
        miss: 0,
    }; 6],
));

/// Last camera tile, and how far the crown search has walked from it.
/// Two rows a frame. A full pass in one frame replayed the music.
struct GroveWork {
    cam_x: i32,
    cam_y: i32,
    next: i32,
    picks: [(i32, i32, i32); 6],
}

struct GroveScan(core::cell::UnsafeCell<GroveWork>);
unsafe impl Sync for GroveScan {}
static GROVE_SCAN: GroveScan = GroveScan(core::cell::UnsafeCell::new(GroveWork {
    cam_x: i32::MIN,
    cam_y: i32::MIN,
    next: i32::MIN,
    picks: [(0, 0, i32::MAX); 6],
}));

fn grove_held(tx: i32, ty: i32, slots: &[CrownSlot; 6]) -> bool {
    slots.iter().any(|slot| slot.tx == tx && slot.ty == ty)
}

fn seat_crown(world: &World, eye: &Eye, floor: i32, cockpit: bool, slot: &mut CrownSlot) -> bool {
    let Some((sx, sy, span, tile)) = crown_spot(world, eye, floor, cockpit, slot.tx, slot.ty, true)
    else {
        return false;
    };
    slot.sx = sx;
    slot.sy = sy;
    slot.span = span;
    slot.tile = tile;
    slot.miss = 0;
    true
}

fn clear_crown(slot: &mut CrownSlot) {
    *slot = CrownSlot {
        tx: 0,
        ty: i32::MIN,
        sx: 0,
        sy: 0,
        span: 0,
        tile: 0,
        miss: 0,
    };
}

/// The 16×32 sheet cannot grow past twice its ink. Past that the ground
/// keeps enlarging and the tree looks as if it is shrinking, so the doubled
/// sheet takes over and the feet stay on the same ground point.
fn show_pine(
    frame: &mut agb::display::GraphicsFrame,
    sensor: Sensor,
    variant: usize,
    sx: i32,
    sy: i32,
    span: i32,
    stump: bool,
) {
    let ir = matches!(sensor, Sensor::Ir);
    let variant = variant % 3;
    let (sprite, native_w, native_h, ink, foot) = if stump && span > 36 {
        (
            if ir {
                sprites::GROVE_IR_NEAR.sprite(3)
            } else {
                sprites::GROVE_NEAR.sprite(3)
            },
            32,
            64,
            36,
            61,
        )
    } else if stump {
        (
            if ir {
                sprites::GROVE_IR.sprite(4)
            } else {
                sprites::GROVE.sprite(4)
            },
            16,
            32,
            18,
            30,
        )
    } else if span > 56 {
        let ink = [56, 54, 52][variant];
        (
            if ir {
                sprites::GROVE_IR_NEAR.sprite(variant)
            } else {
                sprites::GROVE_NEAR.sprite(variant)
            },
            32,
            64,
            ink,
            61,
        )
    } else {
        (
            if ir {
                sprites::GROVE_IR.sprite(variant)
            } else {
                sprites::GROVE.sprite(variant)
            },
            16,
            32,
            28,
            30,
        )
    };
    show_figure(
        frame,
        sprite,
        (sx, sy),
        Art {
            w: native_w,
            h: native_h,
            ink,
            foot,
        },
        span,
        false,
        Coat::Live(sensor),
    );
}

/// Six crowns, two across and three deep. A crown stays until it leaves
/// the glass, so a neighbour nearer the sight does not take its place.
fn draw_grove(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    eye: &Eye,
    floor: i32,
    cockpit: bool,
    sensor: Sensor,
) {
    let Eye { horizon, .. } = *eye;
    let slots = unsafe { &mut *GROVE_HOLD.0.get() };
    for slot in slots.iter_mut() {
        if slot.ty == i32::MIN {
            continue;
        }
        if seat_crown(world, eye, floor, cockpit, slot) {
            continue;
        }
        // A failed frame keeps the slot so the search does not run, but the
        // picture is not redrawn from the old point. That point would sit
        // still while the ground moved, and the tree would look like it fled.
        slot.miss = slot.miss.saturating_add(1);
        if slot.miss >= 4 {
            clear_crown(slot);
        }
    }
    let cam = (tile_of(world.x), tile_of(world.y));
    let scan = unsafe { &mut *GROVE_SCAN.0.get() };
    if slots.iter().any(|slot| slot.ty == i32::MIN) {
        if scan.cam_x != cam.0 || scan.cam_y != cam.1 {
            scan.cam_x = cam.0;
            scan.cam_y = cam.1;
            scan.next = cam.1.div_euclid(3) - 18;
            scan.picks = [(0, 0, i32::MAX); 6];
        }
        if scan.next != i32::MIN {
            let end = scan.cam_y.div_euclid(3) + 18;
            let from = scan.next;
            let to = (from + 1).min(end);
            fill_grove(
                world,
                eye,
                floor,
                cockpit,
                from..=to,
                &mut scan.picks,
                slots,
            );
            if to >= end {
                take_picks(slots, &scan.picks);
                scan.next = i32::MIN;
            } else {
                scan.next = to + 1;
            }
        }
    } else {
        scan.cam_x = i32::MIN;
        scan.cam_y = i32::MIN;
        scan.next = i32::MIN;
    }
    for slot in slots.iter_mut() {
        if slot.ty == i32::MIN || slot.span != 0 {
            continue;
        }
        if !seat_crown(world, eye, floor, cockpit, slot) {
            clear_crown(slot);
        }
    }
    // Near first, so a closer crown sits in front of the one behind it.
    let mut order = [(0i32, 0usize); 6];
    let mut count = 0;
    for (index, slot) in slots.iter().enumerate() {
        if slot.ty == i32::MIN || slot.span == 0 || slot.miss != 0 {
            continue;
        }
        order[count] = (slot.sy, index);
        count += 1;
    }
    for a in 0..count {
        for b in (a + 1)..count {
            if order[b].0 > order[a].0 {
                order.swap(a, b);
            }
        }
    }
    let plan = on_plan(horizon);
    for n in 0..count {
        let slot = slots[order[n].1];
        let tx = slot.tx;
        let ty = slot.ty;
        let sx = slot.sx;
        let sy = slot.sy;
        let span = slot.span;
        let tile = slot.tile;
        let burning = world
            .burns
            .iter()
            .any(|burn| burn.alive && burn.tx == tx && burn.ty == ty);
        if tile == T_STUMP {
            show_pine(frame, sensor, 0, sx, sy, span, true);
            continue;
        }
        if tile == T_BUSH {
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
                (sx, sy),
                Art {
                    w: 16,
                    h: 16,
                    ink: if plan { 11 } else { 10 },
                    foot: 14,
                },
                span,
                plan,
                Coat::Live(sensor),
            );
        } else if plan {
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
                frame,
                crown,
                (sx, sy),
                Art {
                    w: 16,
                    h: 32,
                    ink: 11,
                    foot: 16,
                },
                span,
                true,
                Coat::Live(sensor),
            );
        } else {
            if burning {
                show_flame(
                    frame,
                    sprites::BOOM.sprite(((world.age / 5) as usize + tx as usize) % 3),
                    sx - 8,
                    sy - span * 2 / 3 - 8,
                    sensor,
                );
            }
            show_pine(
                frame,
                sensor,
                (hash(tx, ty) % 3) as usize,
                sx,
                sy,
                span,
                false,
            );
        }
    }
}

/// Two dark posts and a lintel. The opening is the mission.
pub(crate) fn draw_gate(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    eye: &Eye,
    cockpit: bool,
) {
    if !matches!(task_of(world.level), Task::Gate) {
        return;
    }
    let gx = world.gate_x();
    let gy = world.gate_y();
    for dx in [-36, 36] {
        let Some((sx, sy)) = eye.project(gx + (dx << 8), gy, 0) else {
            continue;
        };
        if cockpit && !in_view(sx, sy, true) {
            continue;
        }
        show_cold(frame, sprites::POST.sprite(0), sx - 4, sy - 32);
    }
    let Some((sx, sy)) = eye.project(gx, gy, 16 << 8) else {
        return;
    };
    if !cockpit || in_view(sx, sy, true) {
        show_cold(frame, sprites::BEAM.sprite(0), sx - 16, sy - 4);
    }
}

/// The convoy's pale mark, and the headquarters building.
pub(crate) fn draw_sites(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    eye: &Eye,
    cockpit: bool,
) {
    let Eye { horizon, .. } = *eye;
    let task = task_of(world.level);
    if matches!(task, Task::Convoy) {
        for ty in [3, 4] {
            let gy = tile_mid(ty);
            let Some((sx, sy)) = eye.project(tile_mid(0), gy, 0) else {
                continue;
            };
            if cockpit && !in_view(sx, sy, true) {
                continue;
            }
            show_cold(frame, sprites::POST.sprite(0), sx - 4, sy - 32);
        }
    }
    if matches!(task, Task::Cover) {
        for tx in [16, 32] {
            let Some((sx, sy)) = eye.project(tile_mid(tx), tile_mid(14), 0) else {
                continue;
            };
            if cockpit && !in_view(sx, sy, true) {
                continue;
            }
            show_cold(frame, sprites::POST.sprite(0), sx - 4, sy - 32);
        }
    }
    if matches!(task, Task::Rush | Task::Push) {
        for tx in [8, 24, 40, 56] {
            let Some((sx, sy)) = eye.project(tile_mid(tx), tile_mid(14), 0) else {
                continue;
            };
            if cockpit && !in_view(sx, sy, true) {
                continue;
            }
            show_cold(frame, sprites::POST.sprite(0), sx - 4, sy - 32);
        }
    }
    if matches!(task, Task::Hq) && world.targets > 0 {
        let hx = tile_mid(23);
        let hy = (16 * 8 + 8) << 8;
        let Some((sx, sy)) = eye.project(hx, hy, 0) else {
            return;
        };
        if cockpit && !in_view(sx, sy, true) {
            return;
        }
        let scale = body_scale(sy, horizon, world.alt, 20, 16);
        let (sprite, native, ink, foot) = if scale.near {
            (sprites::HQ_NEAR.sprite(0), 64, 46, 57)
        } else {
            (sprites::HQ.sprite(0), 32, 12, 30)
        };
        show_figure(
            frame,
            sprite,
            (sx, sy),
            Art {
                w: native,
                h: native,
                ink,
                foot,
            },
            scale.span,
            false,
            Coat::Live(glass_now()),
        );
    }
}

/// The batteries themselves. Their circles are the holes in the radar lane.
pub(crate) fn draw_aa_guns(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    eye: &Eye,
    cockpit: bool,
    sensor: Sensor,
) {
    let Eye { horizon, .. } = *eye;
    let guns = aa_guns(task_of(world.level));
    if guns.is_empty() {
        return;
    }
    let floor = sky_bottom(horizon, world.alt);
    for gun in guns {
        let (x, y) = gun_at(gun);
        let Some((sx, sy)) = eye.project(x, y, 0) else {
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
        show_bunker(frame, &bunker, sx, sy, eye, sensor, false);
    }
}
