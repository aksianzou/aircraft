//! The operator's panel: the readouts, the sight, the radar and the static.

use agb::display::WIDTH;

use crate::sprites;

use super::*;

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

fn draw_dashes(frame: &mut agb::display::GraphicsFrame, count: i32, x: i32, y: i32) {
    for index in 0..count {
        show(frame, sprites::DIGITS.sprite(10), x + index * 8, y);
    }
}

fn draw_readout(
    frame: &mut agb::display::GraphicsFrame,
    value: i32,
    count: i32,
    x: i32,
    y: i32,
    blind: bool,
) {
    if blind {
        draw_dashes(frame, count, x, y);
    } else {
        draw_digits(frame, value, count, x, y);
    }
}

/// Three stars under a cleared card. Gold is earned. Grey is still open.
pub(crate) fn show_ribbon(frame: &mut agb::display::GraphicsFrame, marks: u8, x: i32, y: i32) {
    for index in 0..3 {
        let which = if marks & (1 << index) != 0 { 1 } else { 0 };
        show(frame, sprites::MEDAL.sprite(which), x + index * 20, y);
    }
}

/// Fixed crosshair, attitude indicator on the left, radar on the right, and
/// the dark columns that hide the edges of the floor.
pub(crate) fn draw_cockpit(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    eye: &Eye,
    blind: bool,
) {
    draw_sight(frame, world);
    draw_gauges(frame, world, blind);

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
    draw_radar_goals(frame, world, eye, radar_x, inst_y);
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
pub(crate) fn draw_lase(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    eye: &Eye,
    cockpit: bool,
) {
    let Eye { sin_r, cos_r, .. } = *eye;
    if world.mode != Mode::Guide || world.falling {
        return;
    }
    let (x, y) = if world.guide > 0 || world.salvo > 0 {
        (world.mark_x, world.mark_y)
    } else {
        let (nx, ny, _) = world.nose_ground(sin_r, cos_r);
        (nx, ny)
    };
    let Some((sx, sy)) = eye.project(x, y, 0) else {
        return;
    };
    if !in_view(sx, sy, cockpit) {
        return;
    }
    let smoking = world.booms.iter().any(|boom| boom.alive);
    if !smoking && (world.age / 3) % 2 == 0 {
        show(frame, sprites::SPARK.sprite(0), sx - 4, sy - 4);
    }
    let gap = 6;
    let mark = sprites::BRACKET.sprite(0);
    show_flip(frame, mark, sx - gap - 8, sy - gap - 8, false, false);
    show_flip(frame, mark, sx + gap, sy - gap - 8, true, false);
    show_flip(frame, mark, sx - gap - 8, sy + gap, false, true);
    show_flip(frame, mark, sx + gap, sy + gap, true, true);
}

/// The practice mark on the first card, until the bomb leaves the rail.
pub(crate) fn draw_drill(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    eye: &Eye,
    cockpit: bool,
) {
    if !matches!(task_of(world.level), Task::Gate) || world.lesson >= 2 || world.falling {
        return;
    }
    let (x, y) = world.drill_mark();
    let Some((sx, sy)) = eye.project(x, y, 0) else {
        return;
    };
    if !in_view(sx, sy, cockpit) {
        return;
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
    let blasting = world
        .booms
        .iter()
        .any(|boom| boom.alive && dist2_px(world.x, world.y, boom.x, boom.y) < 64 * 64);
    if blasting && let Some(boom) = world.booms.iter().find(|boom| boom.alive) {
        show_fit(
            frame,
            sprites::BOOM.sprite(boom_sheet(boom.frame)),
            view_cx,
            VIEW_Y + VIEW_H - 28,
            16,
            32,
        );
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

/// Red mark is the airframe, yellow is the battery, the snow icon is the radio.
pub(crate) fn draw_gauges(frame: &mut agb::display::GraphicsFrame, world: &World, blind: bool) {
    show(frame, sprites::BLIP.sprite(0), 4, 18);
    draw_readout(frame, world.hull.clamp(0, 100), 3, 4, 28, blind);
    let fill = if blind {
        0
    } else {
        (world.hull.clamp(0, 100) * 10 / 100) as usize
    };
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
        draw_readout(frame, ammo, 2, 8, 70, blind);
    }
    show(frame, sprites::BLIP.sprite(1), WIDTH - 28, 18);
    draw_readout(frame, world.battery.clamp(0, 100), 3, WIDTH - 28, 28, blind);
    let signal = (100 - world.signal_loss() * 100 / 16).clamp(0, 100);
    show(
        frame,
        sprites::STATIC.sprite((world.age as usize / 8) % 4),
        WIDTH - 28,
        46,
    );
    draw_readout(frame, signal, 3, WIDTH - 28, 56, blind);
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

pub(crate) fn draw_noise(frame: &mut agb::display::GraphicsFrame, age: i32, sealed: bool) {
    let jx = if sealed { 0 } else { (hash(age, 1) % 4) as i32 };
    let jy = if sealed { 0 } else { (hash(age, 3) % 4) as i32 };
    let flip = (age as usize / 2) % 2;
    // The snow is bright. A dark lip sits on its top edge so those pixels
    // cannot climb into the instrument bar.
    if sealed {
        let mut x = VIEW_X;
        while x < VIEW_X + VIEW_W {
            show(frame, sprites::LIP.sprite(0), x, VIEW_Y);
            x += 32;
        }
    }
    // Three tiles cover the glass edge to edge. Jitter is only for the short flash.
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
        show(frame, sprites::STATIC.sprite((n % 4) as usize), x, y);
    }
}

/// Yellow dots are the air the guns do not own, while the drone is still in their sight.
pub(crate) fn draw_aa_lanes(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    eye: &Eye,
    radar_x: i32,
    inst_y: i32,
) {
    let Eye { sin_r, cos_r, .. } = *eye;
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
    eye: &Eye,
    radar_x: i32,
    inst_y: i32,
) {
    let Eye { sin_r, cos_r, .. } = *eye;
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
                delta(tile_mid(tx), world.x),
                delta(tile_mid(ty), world.y),
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
        let (bx, by) = radar_blip(delta(foe.x, world.x), delta(foe.y, world.y), sin_r, cos_r);
        show_dot(frame, 0, radar_x, inst_y, bx, by);
    }
    for &(px, py) in &pts[..n] {
        let (bx, by) = radar_blip(delta(px, world.x), delta(py, world.y), sin_r, cos_r);
        show_dot(frame, 0, radar_x, inst_y, bx, by);
    }
    if matches!(task_of(world.level), Task::Convoy) {
        let mx = tile_mid(0);
        let my = tile_mid(4);
        let (bx, by) = radar_blip(delta(mx, world.x), delta(my, world.y), sin_r, cos_r);
        show_dot(frame, 1, radar_x, inst_y, bx, by);
    }
    if matches!(task_of(world.level), Task::Cover) {
        let mx = tile_mid(24);
        let my = tile_mid(14);
        let (bx, by) = radar_blip(delta(mx, world.x), delta(my, world.y), sin_r, cos_r);
        show_dot(frame, 1, radar_x, inst_y, bx, by);
    }
    if matches!(task_of(world.level), Task::Rush | Task::Push) {
        let mx = tile_mid(24);
        let my = tile_mid(14);
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
    draw_aa_lanes(frame, world, eye, radar_x, inst_y);
}

pub(crate) fn goal_points(world: &World, pts: &mut [(i32, i32); 3]) -> usize {
    match task_of(world.level) {
        Task::Trench | Task::Officer | Task::Pits | Task::Grove | Task::Hearth => {
            nearest_marked(world, pts, true, false)
        }
        Task::Night | Task::Heat | Task::Guide | Task::Column => {
            nearest_marked(world, pts, false, true)
        }
        Task::Rush => nearest_marked(world, pts, true, false),
        Task::Push => nearest_marked(world, pts, true, true),
        Task::Lane | Task::Dart | Task::Bend => nearest_marked(world, pts, false, true),
        Task::Nest => nearest_marked(world, pts, true, false),
        Task::Flare | Task::Armor | Task::Thaw => {
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
        Task::Convoy | Task::Cover | Task::Drift => world
            .trucks
            .iter()
            .find(|truck| truck.alive && truck.goal)
            .map(|truck| {
                pts[0] = (truck.x, truck.y);
                1
            })
            .unwrap_or(0),
        Task::Gate => {
            pts[0] = if world.lesson < 2 {
                world.drill_mark()
            } else {
                (world.gate_x(), world.gate_y())
            };
            1
        }
        Task::Hq => {
            if world.targets <= 0 {
                return 0;
            }
            pts[0] = (tile_mid(23), (16 * 8 + 8) << 8);
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
                n
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

pub(crate) fn draw_hud(frame: &mut agb::display::GraphicsFrame, world: &World, blind: bool) {
    show(frame, sprites::LBL_SCORE.sprite(0), 0, 0);
    draw_readout(frame, world.score, 4, 32, 0, blind);
    show(frame, sprites::LBL_ALT.sprite(0), 96, 0);
    draw_readout(frame, world.alt >> 8, 2, 128, 0, blind);
    if world.mode == Mode::Guide && world.guide > 0 {
        let secs = ((world.guide as i32) + 59) / 60;
        draw_readout(frame, secs, 1, VIEW_X + VIEW_W / 2 - 4, VIEW_Y + 6, blind);
    }
    // The charge in hand and what is left of it sit in the left column.
    // The rest show as A cycles to them, so the top stays one quiet line.
    if world.mode == Mode::Fpv {
        show(frame, sprites::LBL_DRONE.sprite(0), 160, 0);
        draw_readout(frame, world.drones, 1, 176, 0, blind);
    }
    if matches!(task_of(world.level), Task::Gate) && world.targets > 0 {
        let line: &[u8] = match world.lesson {
            0 => b"SELECT OVER MARK",
            1 => b"B DROP THE BOMB",
            _ => b"FLY THE GAP",
        };
        show_small(frame, line, 148);
    }
    // How many mission marks are still left.
    show(frame, sprites::DOT.sprite(0), WIDTH - 28, 72);
    draw_readout(frame, world.targets, 2, WIDTH - 28, 80, blind);
}
