//! Stick pilots. Each sortie calls the same `fly` the game does.
//! Nothing is moved by teleporting the airframe or planting a blast.

use agb::display::tiled::AffineBackground;
use agb::fixnum::num;
use agb::input::{Button, ButtonController, ButtonState};

use super::*;

const MAN: u8 = 0;
const TRUCK: u8 = 1;
const TANK: u8 = 2;
const BUNKER: u8 = 3;
const FOE: u8 = 4;
const ROOF: u8 = 5;
const CACHE: u8 = 6;

#[derive(Clone, Copy)]
enum Style {
    /// Drop in from a hover, low enough that the round leaves the nose.
    Deck,
    /// Stay up and lead the fall.
    Cruise,
    /// Spotter. The mark has to sit where the target will be.
    Mark,
    /// Kamikaze. The airframe is the round.
    Ram,
    /// One mine at a time, from the cache into the hatch.
    Haul,
    /// The height gap, hands off the stick.
    Gap,
    /// Bullets only.
    Gun,
}

struct Aim {
    x: i32,
    y: i32,
    vx: i32,
    vy: i32,
    kind: u8,
}

struct Report {
    clear: bool,
    stars: u8,
    gap: &'static str,
    left: i32,
    why: &'static str,
    age: i32,
    spent: i16,
}

pub(crate) fn survey(floor: &mut AffineBackground) -> (u32, u32) {
    let mut cleared = 0u32;
    let mut starred = 0u32;
    for level in 0..LEVEL_COUNT {
        let styles = styles_for(level);
        let mut any = false;
        let mut full = false;
        for style in styles {
            let report = play(floor, level, *style);
            agb::println!(
                "{:02} {} {} {} {}* {} left {} {} spent {} age {}",
                level + 1,
                card_name(level),
                style_name(*style),
                if report.clear { "clear" } else { "FAIL" },
                report.stars,
                report.gap,
                report.left,
                report.why,
                report.spent,
                report.age
            );
            any |= report.clear;
            full |= report.stars == 3;
        }
        if any {
            cleared |= 1 << level;
        }
        if full {
            starred |= 1 << level;
        }
    }
    agb::println!("cleared {}/{}", cleared.count_ones(), LEVEL_COUNT);
    agb::println!("starred {}/{}", starred.count_ones(), LEVEL_COUNT);
    (cleared, starred)
}

/// The card's first pilot flies `frames`. `each` sees the world after every frame.
pub(crate) fn fly_for(
    floor: &mut AffineBackground,
    level: u8,
    frames: i32,
    each: &mut dyn FnMut(&mut World, &mut AffineBackground),
) -> World {
    let style = styles_for(level)[0];
    let mut world = World::preview(floor, level);
    let mut idle = || {};
    while !world.boot_ready() {
        world.boot_step(floor, 24, &mut idle);
    }
    let mut input = ButtonController::new();
    let mut still = 0i16;
    for _ in 0..frames {
        if world.targets <= 0 || world.crashed || world.again {
            break;
        }
        let state = stick(&world, style, &mut still);
        input.update_with_state(state);
        world.fly(&input, floor);
        world.maintain_actors();
        each(&mut world, floor);
    }
    world
}

fn styles_for(level: u8) -> &'static [Style] {
    match task_of(level) {
        Task::Gate => &[Style::Gap],
        Task::Fpv | Task::Nest | Task::Dart => &[Style::Ram],
        Task::Guide | Task::Column | Task::Rush | Task::Push | Task::Bend => &[Style::Mark],
        Task::Fetch => &[Style::Haul],
        Task::Swarm => &[Style::Gun],
        _ => &[Style::Deck, Style::Cruise],
    }
}

fn play(floor: &mut AffineBackground, level: u8, style: Style) -> Report {
    let mut world = World::preview(floor, level);
    let mut idle = || {};
    while !world.boot_ready() {
        world.boot_step(floor, 24, &mut idle);
    }
    let mut input = ButtonController::new();
    let mut still = 0i16;
    let extra = if matches!(task_of(level), Task::Fetch) {
        2500
    } else {
        1800
    };
    let limit = time_budget(level) + extra;
    for _ in 0..limit {
        if world.targets <= 0 || world.crashed || world.again {
            break;
        }
        let state = stick(&world, style, &mut still);
        input.update_with_state(state);
        world.fly(&input, floor);
        world.maintain_actors();
    }
    let why = if world.targets <= 0 {
        "ok"
    } else if world.again {
        "drones"
    } else if world.crashed {
        match world.why {
            Why::Cell => "cell",
            Why::Hull => "hull",
            Why::Truck => "truck",
            Why::Line => "line",
            Why::Base => "base",
            Why::None => "crash",
        }
    } else {
        "time"
    };
    Report {
        clear: world.targets <= 0 && !world.crashed,
        stars: if world.targets <= 0 {
            star_count(earned_marks(&world))
        } else {
            0
        },
        gap: star_gap(&world),
        left: world.targets.max(0),
        why,
        age: world.age,
        spent: world.spent,
    }
}

fn star_count(bits: u8) -> u8 {
    (bits & 1) + ((bits >> 1) & 1) + ((bits >> 2) & 1)
}

fn star_gap(world: &World) -> &'static str {
    if world.targets > 0 || world.crashed {
        return "-";
    }
    match earned_marks(world) {
        7 => "3",
        6 => "no-time",
        5 => "no-pack",
        3 => "no-spare",
        4 => "no-time-pack",
        2 => "no-time-spare",
        1 => "no-pack-spare",
        _ => "no-stars",
    }
}

fn stick(world: &World, style: Style, still: &mut i16) -> ButtonState {
    let mut press = ButtonState::empty();
    let aim = nearest(world);
    match style {
        Style::Gap => gap(world, &mut press),
        Style::Haul => haul(world, &mut press),
        Style::Ram => {
            if let Some(aim) = aim.as_ref() {
                let dist = dist2_px(world.x, world.y, aim.x, aim.y);
                let frames = (isqrt64(i64::from(dist)) * 256 / 400).clamp(1, 180) as i32;
                let x = aim.x + aim.vx * frames;
                let y = aim.y + aim.vy * frames;
                face(world, x, y, &mut press);
                dive_onto(world, x, y, &mut press);
            }
        }
        Style::Mark => {
            if let Some(aim) = aim.as_ref() {
                let wait = i32::from(GUIDE_WAIT) + 30;
                let lx = aim.x + aim.vx * wait;
                let ly = aim.y + aim.vy * wait;
                face(world, lx, ly, &mut press);
                sight_pitch(world, lx, ly, &mut press);
                let (sin_r, cos_r) = world.heading();
                let (nx, ny, _) = world.nose_ground(sin_r, cos_r);
                if world.guide == 0 && world.salvo == 0 && dist2_px(nx, ny, lx, ly) <= 18 * 18 {
                    tap(&mut press, Button::B);
                }
            }
        }
        Style::Gun => pursuit(world, aim.as_ref(), 17, &mut press),
        Style::Deck | Style::Cruise => {
            if let Some(aim) = aim.as_ref() {
                if aim.kind == FOE {
                    pursuit(world, Some(aim), 17, &mut press);
                } else if matches!(style, Style::Deck) {
                    deck(world, aim, still, &mut press);
                } else {
                    cruise(world, aim, &mut press);
                }
            }
        }
    }
    // A has to be a tap. Holding it only counts once.
    if matches!(style, Style::Deck | Style::Cruise)
        && world.mode == Mode::Quad
        && world.age % 2 == 0
        && aim
            .as_ref()
            .is_some_and(|aim| world.arm != arm_for(world, aim.kind))
        && !matches!(task_of(world.level), Task::Swarm)
        && let Some(aim) = aim.as_ref()
        && arm_for(world, aim.kind) != world.arm
    {
        tap(&mut press, Button::A);
    }
    press
}

fn deck(world: &World, aim: &Aim, still: &mut i16, press: &mut ButtonState) {
    let n = fall_frames(world.alt, -((110 * world.pitch.sin().to_raw()) >> 8));
    let lx = aim.x + aim.vx * n;
    let ly = aim.y + aim.vy * n;
    let dist = dist2_px(world.x, world.y, lx, ly);
    let body = dist2_px(world.x, world.y, aim.x, aim.y);
    let alt = world.alt >> 8;
    // Only the trucks that can end the card. A pacing truck still wants the
    // short stop that clears the lane.
    let driving = aim.kind == TRUCK
        && matches!(
            task_of(world.level),
            Task::Convoy | Task::Drift | Task::Cover
        );
    // Hover slide is slower than a driving truck, so these are caught at
    // cruise speed and stopped only once the lead is already under the nose.
    let lane = matches!(task_of(world.level), Task::Lane);
    let chasing = lane || driving;
    let hatch = aim.kind == TANK;
    let guns = aa_guns(task_of(world.level));
    let watched = !guns.is_empty() && aa_covers(guns, aim.x, aim.y);
    let in_guns = !guns.is_empty() && aa_covers(guns, world.x, world.y);
    // The mud men are two drops beside the truck. Anywhere else, infantry
    // shoots back under twenty.
    let low_men = aim.kind == MAN && matches!(task_of(world.level), Task::Drift);
    let post = matches!(task_of(world.level), Task::Hearth) && aim.kind == MAN;
    let safe = if lane || (watched && !hatch) || (hatch && in_guns) {
        22
    } else if hatch
        && body > 200 * 200
        && (!guns.is_empty() || matches!(task_of(world.level), Task::Thaw))
    {
        // High over the lane, and over the mud. The dive starts close to the tank.
        22
    } else if post {
        // The watch fires under 20. The pass stays above it.
        22
    } else if hatch {
        9
    } else if low_men {
        10
    } else if matches!(aim.kind, MAN | TRUCK) && !driving {
        20
    } else {
        10
    };
    let band = if post {
        21..=24
    } else if safe >= 20 {
        18..=24
    } else {
        8..=13
    };
    let slop = drop_slop(aim.kind, alt);
    let reach = if low_men {
        36
    } else if matches!(aim.kind, BUNKER | ROOF | TANK | MAN | TRUCK) {
        72
    } else {
        32
    };
    let (enter, stay) = (reach, reach + 24);
    let here = if driving || hatch { body } else { dist };
    let hatch_window = (8..=12).contains(&alt);
    let close = if chasing {
        dist <= slop * slop
    } else if hatch {
        hatch_window && body <= if world.hover { 80 * 80 } else { 48 * 48 }
    } else if world.hover {
        here <= stay * stay
    } else {
        here <= enter * enter
    };
    let want_hover = world.mode == Mode::Quad && close && band.contains(&alt);
    if world.hover != want_hover && world.mode == Mode::Quad {
        tap(press, Button::Select);
    }
    let on_top = if hatch {
        world.hover && hatch_window && body <= HATCH * HATCH
    } else {
        world.hover && dist <= slop * slop
    };
    let (sx, sy) = if hatch { (aim.x, aim.y) } else { (lx, ly) };
    if world.hover && !on_top {
        slide(world, sx, sy, hatch || aim.kind == MAN, press);
    } else if !world.hover {
        // A goal truck dead ahead is caught on its lead. Aiming past that
        // point flies over it and the turn never comes back inside the blast.
        let (tx, ty) = if lane {
            intercept(world, lx, ly, aim.vx, aim.vy)
        } else {
            (sx, sy)
        };
        face(world, tx, ty, press);
        let descend = here <= 140 * 140 || aim.kind == TANK;
        pitch_toward(world, if descend { safe } else { safe.max(14) }, press);
    }
    // The escort is faster than a hover. The bomb has to leave while the
    // quad is still flying, or the truck is gone before the nose is steady.
    if matches!(task_of(world.level), Task::Cover)
        && aim.kind == TRUCK
        && !world.hover
        && (8..=14).contains(&alt)
        && world.arm == ARM_BOMB
        && !will_turn(world, aim, n)
    {
        let (sin_r, cos_r) = world.heading();
        let ivx = (i64::from(sin_r) * 400) >> 8;
        let ivy = -((i64::from(cos_r) * 400) >> 8);
        let ix = world.x.wrapping_add((ivx as i32).wrapping_mul(n));
        let iy = world.y.wrapping_add((ivy as i32).wrapping_mul(n));
        if dist2_px(ix, iy, lx, ly) <= slop * slop {
            tap(press, Button::B);
        }
    }
    // A moving stick puts the round off the mark. Release, then drop.
    // A truck that will turn before the bomb lands is left alone.
    // A mine or a grenade only counts the first frame of a held button.
    if on_top {
        *still = still.saturating_add(1);
        let snap = matches!(task_of(world.level), Task::Cover);
        let charge = world.arm == ARM_MINE;
        let thaw = matches!(task_of(world.level), Task::Thaw);
        // Two hull mines finish a tank. A later pulse on the mud leaves
        // while those two are still falling. The other armour cards need
        // that later pulse: the first two do not always land.
        let due = if charge {
            if thaw {
                *still == 3 || *still == 40
            } else {
                *still == 3 || (*still > 3 && *still % 80 == 0)
            }
        } else {
            *still == 3
        };
        if (snap || due) && !will_turn(world, aim, n) {
            tap(press, Button::B);
        }
    } else {
        *still = 0;
    }
}

fn cruise(world: &World, aim: &Aim, press: &mut ButtonState) {
    let want = if aa_guns(task_of(world.level)).is_empty() {
        (world.pad_alt >> 8).clamp(12, 16)
    } else {
        22
    };
    pitch_toward(world, want, press);
    let n = fall_frames(world.alt, -((110 * world.pitch.sin().to_raw()) >> 8));
    let speed = 400;
    let lx = aim.x + aim.vx * n;
    let ly = aim.y + aim.vy * n;
    let (sin_r, cos_r) = world.heading();
    let ivx = (i64::from(sin_r) * i64::from(speed)) >> 8;
    let ivy = -((i64::from(cos_r) * i64::from(speed)) >> 8);
    let ix = world.x + (ivx as i32) * n;
    let iy = world.y + (ivy as i32) * n;
    let lead = n * speed / 256 + 48;
    let dx = i64::from(delta(world.x, lx));
    let dy = i64::from(delta(world.y, ly));
    let len = isqrt64(dx * dx + dy * dy).max(1);
    if len < i64::from(lead) << 8 {
        let back = i64::from(lead) << 8;
        let sx = lx.wrapping_add((dx * back / len) as i32);
        let sy = ly.wrapping_add((dy * back / len) as i32);
        face(world, sx, sy, press);
        return;
    }
    face(world, lx, ly, press);
    let level = world.pitch > num!(-0.03) && world.pitch < num!(0.03);
    let alt = world.alt >> 8;
    if level && alt > 8 && dist2_px(ix, iy, lx, ly) <= drop_slop(aim.kind, alt).pow(2) {
        if world.arm == ARM_GUN {
            if world.man_under_sight().is_some() {
                press_hold(press, Button::B);
            }
        } else {
            tap(press, Button::B);
        }
    }
}

fn pursuit(world: &World, aim: Option<&Aim>, want_alt: i32, press: &mut ButtonState) {
    if world.alt >> 8 < 8 {
        tap(press, Button::Up);
        return;
    }
    let Some(aim) = aim else {
        pitch_toward(world, want_alt, press);
        return;
    };
    if defend_card(world.level) {
        let home = dist2_px(world.x, world.y, world.pad_x, world.pad_y);
        if home > 24 * 24 {
            want_hover(world, false, press);
            pitch_toward(world, world.pad_alt >> 8, press);
            face(world, world.pad_x, world.pad_y, press);
            return;
        }
        want_hover(world, true, press);
    }
    if aim.kind == FOE {
        let floor = if defend_card(world.level) {
            20
        } else if matches!(task_of(world.level), Task::Cover) {
            22
        } else {
            19
        };
        let alt = world.alt >> 8;
        if alt < floor
            || alt > floor + 4
            || (matches!(task_of(world.level), Task::Cover) && alt <= floor + 1)
        {
            pitch_toward(world, floor, press);
        }
        // The escorts sit on the truck's height. Climb clear of them first.
        if matches!(task_of(world.level), Task::Cover) && alt < floor {
            return;
        }
        let on = aim_flat(world, aim.x, aim.y, foe_z(world, aim), floor, press);
        if on || world.foe_under_sight().is_some() {
            press_hold(press, Button::B);
        }
        return;
    }
    let on = aim_glass(world, aim.x, aim.y, foe_z(world, aim), press);
    if on || world.foe_under_sight().is_some() || world.man_under_sight().is_some() {
        press_hold(press, Button::B);
    }
}

fn gap(world: &World, press: &mut ButtonState) {
    if world.lesson < 2 {
        let (mx, my) = world.drill_mark();
        let alt = world.alt >> 8;
        let dist = dist2_px(world.x, world.y, mx, my);
        if dist > 16 * 16 || !(10..=16).contains(&alt) {
            want_hover(world, false, press);
            face(world, mx, my, press);
            pitch_toward(world, 12, press);
        } else if !world.hover {
            want_hover(world, true, press);
        } else {
            tap(press, Button::B);
        }
        return;
    }
    if world.hover {
        want_hover(world, false, press);
        return;
    }
    face(world, world.gate_x(), world.gate_y(), press);
    pitch_toward(world, 12, press);
}

fn haul(world: &World, press: &mut ButtonState) {
    let alt = world.alt >> 8;
    if !world.cargo {
        let Some((cx, cy)) = nearest_cache(world) else {
            return;
        };
        let (sx, sy) = stand_off(world, cx, cy);
        let dist = dist2_px(world.x, world.y, sx, sy);
        if dist > 4 * 4 {
            transit(world, alt, sx, sy, 22, press);
        } else if alt > 1 {
            // Dropping through a crown ends the haul. Slide clear first.
            if alt <= TREE_TOP + 2 && world.ground_at(tile_of(world.x), tile_of(world.y)) == T_TREE
            {
                want_hover(world, true, press);
                if world.hover {
                    slide(world, sx, sy, false, press);
                }
            } else {
                want_hover(world, false, press);
                tap(press, Button::Down);
            }
        } else if world.age % 2 == 0 {
            want_hover(world, false, press);
            tap(press, Button::B);
        }
        return;
    }
    let Some(aim) = nearest(world) else {
        return;
    };
    let dist = dist2_px(world.x, world.y, aim.x, aim.y);
    if dist > 4 * 4 {
        transit(world, alt, aim.x, aim.y, 22, press);
    } else if alt > 9 {
        want_hover(world, false, press);
        tap(press, Button::Down);
    } else if alt < 8 {
        want_hover(world, false, press);
        tap(press, Button::Up);
    } else {
        want_hover(world, true, press);
        if world.hover && world.age % 2 == 0 {
            tap(press, Button::B);
        }
    }
}

/// Between the cache and the hatch the quad stays above rifles and crowns.
fn transit(world: &World, alt: i32, x: i32, y: i32, cruise: i32, press: &mut ButtonState) {
    if alt < cruise {
        want_hover(world, false, press);
        tap(press, Button::Up);
    } else {
        want_hover(world, true, press);
        if world.hover {
            slide(world, x, y, true, press);
        }
    }
}

/// A cache on a trunk cannot be landed on. Fourteen pixels still closes the claws.
fn stand_off(world: &World, x: i32, y: i32) -> (i32, i32) {
    // Ten pixels still closes the claws. Farther than that and the grab misses.
    let tries = [(10, 0), (-10, 0), (0, 10), (0, -10), (8, 8), (0, 0)];
    for (dx, dy) in tries {
        let sx = x.wrapping_add(dx << 8);
        let sy = y.wrapping_add(dy << 8);
        if world.ground_at(tile_of(sx), tile_of(sy)) != T_TREE {
            return (sx, sy);
        }
    }
    (x.wrapping_add(10 << 8), y)
}

fn want_hover(world: &World, hover: bool, press: &mut ButtonState) {
    if world.hover != hover && world.mode == Mode::Quad {
        tap(press, Button::Select);
    }
}

fn face(world: &World, x: i32, y: i32, press: &mut ButtonState) {
    let (sin_r, cos_r) = world.heading();
    let dx = delta(x, world.x) as i64;
    let dy = delta(y, world.y) as i64;
    let cross = sin_r as i64 * dy + cos_r as i64 * dx;
    let dot = sin_r as i64 * dx - cos_r as i64 * dy;
    let band = (dot.abs() / 10).max(256 * 256);
    if cross > band {
        tap(press, Button::Right);
    } else if cross < -band {
        tap(press, Button::Left);
    }
}

fn slide(world: &World, x: i32, y: i32, tight: bool, press: &mut ButtonState) {
    let (sin_r, cos_r) = world.heading();
    let dx = delta(x, world.x) as i64;
    let dy = delta(y, world.y) as i64;
    let along = sin_r as i64 * dx - cos_r as i64 * dy;
    let strafe = cos_r as i64 * dx + sin_r as i64 * dy;
    // A four-pixel dead band parks the nose just outside a hatch.
    let band = if tight { 256 * 256 } else { 4 * 256 * 256 };
    if along > band {
        tap(press, Button::Up);
    } else if along < -band {
        tap(press, Button::Down);
    }
    if strafe > band {
        tap(press, Button::Right);
    } else if strafe < -band {
        tap(press, Button::Left);
    }
}

/// Nose down looks closer. Nose up looks further along the ground.
fn sight_pitch(world: &World, x: i32, y: i32, press: &mut ButtonState) {
    let (sin_r, cos_r) = world.heading();
    let (nx, ny, _) = world.nose_ground(sin_r, cos_r);
    let nose =
        sin_r as i64 * i64::from(delta(nx, world.x)) - cos_r as i64 * i64::from(delta(ny, world.y));
    let lead =
        sin_r as i64 * i64::from(delta(x, world.x)) - cos_r as i64 * i64::from(delta(y, world.y));
    let band = 8 * 256 * 256;
    if lead > nose + band {
        tap(press, Button::Up);
    } else if lead + band < nose {
        tap(press, Button::Down);
    }
}

fn dive_onto(world: &World, x: i32, y: i32, press: &mut ButtonState) {
    let dist = dist2_px(world.x, world.y, x, y);
    if dist > 36 * 36 {
        pitch_toward(world, (world.pad_alt >> 8).clamp(8, 14), press);
    } else if world.alt >> 8 > 4 {
        tap(press, Button::Down);
    }
}

/// Climb when low, and level the nose before a dive carries through the crown.
fn pitch_toward(world: &World, want: i32, press: &mut ButtonState) {
    let alt = world.alt >> 8;
    if alt < want || (alt <= want + 1 && world.pitch > num!(0.02)) {
        tap(press, Button::Up);
    } else if alt > want + 2 && world.pitch < num!(0.04) {
        tap(press, Button::Down);
    }
}

/// Put the picture on the crosshair. True when it is already there.
fn aim_glass(world: &World, x: i32, y: i32, z: i32, press: &mut ButtonState) -> bool {
    let eye = Eye::new(world, world.horizon());
    let Some((sx, sy)) = eye.project(x, y, z) else {
        face(world, x, y, press);
        return false;
    };
    let dx = sx - agb::display::WIDTH / 2;
    let dy = sy - (VIEW_Y + VIEW_H / 2);
    if dx > 3 {
        tap(press, Button::Right);
    } else if dx < -3 {
        tap(press, Button::Left);
    }
    if dy > 3 {
        tap(press, Button::Down);
    } else if dy < -3 {
        tap(press, Button::Up);
    }
    dx * dx + dy * dy <= 6 * 6
}

/// Same picture, but the nose is not allowed to dive through `floor`.
fn aim_flat(world: &World, x: i32, y: i32, z: i32, floor: i32, press: &mut ButtonState) -> bool {
    let eye = Eye::new(world, world.horizon());
    let Some((sx, sy)) = eye.project(x, y, z) else {
        face(world, x, y, press);
        return false;
    };
    let dx = sx - agb::display::WIDTH / 2;
    let dy = sy - (VIEW_Y + VIEW_H / 2);
    let alt = world.alt >> 8;
    if dx > 3 {
        tap(press, Button::Right);
    } else if dx < -3 {
        tap(press, Button::Left);
    }
    if dy > 3 && alt > floor + 1 {
        tap(press, Button::Down);
    } else if dy < -3 && alt < floor + 4 {
        tap(press, Button::Up);
    }
    dx * dx + dy * dy <= 6 * 6
}

fn foe_z(world: &World, aim: &Aim) -> i32 {
    if aim.kind != FOE {
        return 0;
    }
    world
        .foes
        .iter()
        .find(|foe| foe.alive && dist2_px(foe.x, foe.y, aim.x, aim.y) <= 4)
        .map(|foe| foe.z)
        .unwrap_or(0)
}

fn nearest(world: &World) -> Option<Aim> {
    if let Some(aim) = fleeing(world) {
        return Some(aim);
    }
    let mut best: Option<(i32, Aim)> = None;
    let task = task_of(world.level);
    if matches!(task, Task::Hq)
        && let Some((x, y)) = roof(world)
    {
        keep(&mut best, world, x, y, 0, 0, ROOF);
    }
    if matches!(task, Task::Fetch) && !world.cargo {
        if let Some((x, y)) = nearest_cache(world) {
            keep(&mut best, world, x, y, 0, 0, CACHE);
        }
        return best.map(|(_, aim)| aim);
    }
    for soldier in &world.soldiers {
        if soldier.alive && man_counts(task, soldier) {
            let (vx, vy) = man_vel(world, soldier);
            // A pair shares a tile. Aiming at one of them flips every frame
            // and the grenade never leaves.
            let (x, y) = if matches!(task, Task::Pits) {
                (tile_mid(soldier.home_tx), tile_mid(soldier.home_ty))
            } else {
                (soldier.x, soldier.y)
            };
            keep(&mut best, world, x, y, vx, vy, MAN);
        }
    }
    for truck in &world.trucks {
        if truck.alive && truck_counts(task, truck) {
            let (vx, vy) = truck_vel(world, truck);
            keep(&mut best, world, truck.x, truck.y, vx, vy, TRUCK);
        }
    }
    for tank in &world.tanks {
        if tank.alive && tank.objective && armor_counts(task) {
            let step = ground_step(tank.dir);
            let (vx, vy) = if tank.vertical { (0, step) } else { (step, 0) };
            keep(&mut best, world, tank.x, tank.y, vx, vy, TANK);
        }
    }
    for bunker in &world.bunkers {
        if bunker.alive && bunker.objective && matches!(task, Task::Pill | Task::Skirt) {
            keep(&mut best, world, bunker.x, bunker.y, 0, 0, BUNKER);
        }
    }
    if matches!(task, Task::Swarm | Task::Cover) {
        for foe in &world.foes {
            if !foe.alive {
                continue;
            }
            if defend_card(world.level) {
                // The one about to reach the pad, not the one nearest the nose.
                let dist = dist2_px(world.pad_x, world.pad_y, foe.x, foe.y);
                let aim = Aim {
                    x: foe.x,
                    y: foe.y,
                    vx: 0,
                    vy: 0,
                    kind: FOE,
                };
                if best.as_ref().is_none_or(|(near, _)| dist < *near) {
                    best = Some((dist, aim));
                }
            } else {
                keep(&mut best, world, foe.x, foe.y, 0, 0, FOE);
            }
        }
    }
    best.map(|(_, aim)| aim)
}

/// A truck that is running out of road is the only thing that matters.
fn fleeing(world: &World) -> Option<Aim> {
    let task = task_of(world.level);
    let line = match task {
        Task::Convoy | Task::Drift | Task::Cover => 80,
        _ => return None,
    };
    for truck in &world.trucks {
        if truck.alive && truck.goal && tile_of(truck.y) <= line {
            let (vx, vy) = truck_vel(world, truck);
            return Some(Aim {
                x: truck.x,
                y: truck.y,
                vx,
                vy,
                kind: TRUCK,
            });
        }
    }
    None
}

fn keep(best: &mut Option<(i32, Aim)>, world: &World, x: i32, y: i32, vx: i32, vy: i32, kind: u8) {
    let dist = dist2_px(world.x, world.y, x, y);
    if best.as_ref().is_none_or(|(near, _)| dist < *near) {
        *best = Some((dist, Aim { x, y, vx, vy, kind }));
    }
}

fn roof(world: &World) -> Option<(i32, i32)> {
    let mut sx = 0i64;
    let mut sy = 0i64;
    let mut n = 0i64;
    for ty in 16..=17 {
        for tx in 22..=24 {
            if world.ground_at(tx, ty) == T_CRATER {
                continue;
            }
            sx += i64::from(tile_mid(tx));
            sy += i64::from(tile_mid(ty));
            n += 1;
        }
    }
    if n == 0 {
        None
    } else {
        Some(((sx / n) as i32, (sy / n) as i32))
    }
}

fn nearest_cache(world: &World) -> Option<(i32, i32)> {
    let mut best: Option<(i32, i32, i32)> = None;
    for cache in &world.caches {
        if !cache.alive {
            continue;
        }
        let dist = dist2_px(world.x, world.y, cache.x, cache.y);
        if best.is_none_or(|(near, _, _)| dist < near) {
            best = Some((dist, cache.x, cache.y));
        }
    }
    best.map(|(_, x, y)| (x, y))
}

fn man_counts(task: Task, soldier: &Soldier) -> bool {
    match task {
        Task::Officer => soldier.officer,
        Task::Hearth
        | Task::Trench
        | Task::Pits
        | Task::Grove
        | Task::Thaw
        | Task::Drift
        | Task::Fpv
        | Task::Nest
        | Task::Rush
        | Task::Push => soldier.objective,
        _ => false,
    }
}

fn truck_counts(task: Task, truck: &Truck) -> bool {
    match task {
        Task::Convoy | Task::Cover | Task::Drift => truck.goal,
        Task::Night
        | Task::Heat
        | Task::Guide
        | Task::Column
        | Task::Lane
        | Task::Dart
        | Task::Bend
        | Task::Push => truck.objective,
        _ => false,
    }
}

fn armor_counts(task: Task) -> bool {
    matches!(task, Task::Armor | Task::Flare | Task::Fetch | Task::Thaw)
}

fn man_vel(world: &World, soldier: &Soldier) -> (i32, i32) {
    if soldier.objective && matches!(task_of(world.level), Task::Rush | Task::Push) {
        (0, -rush_step(world.level))
    } else if matches!(
        task_of(world.level),
        Task::Fpv | Task::Nest | Task::Grove | Task::Thaw | Task::Hearth | Task::Pits
    ) {
        (0, 0)
    } else {
        (ground_step(soldier.dir), 0)
    }
}

fn truck_vel(world: &World, truck: &Truck) -> (i32, i32) {
    if truck.objective && matches!(task_of(world.level), Task::Push) {
        return (0, -rush_step(world.level));
    }
    let guide = world.mode == Mode::Guide;
    let step = drive_step(guide, truck.dir);
    if truck.vertical { (0, step) } else { (step, 0) }
}

fn arm_for(world: &World, kind: u8) -> u8 {
    match kind {
        FOE => ARM_GUN,
        BUNKER | ROOF => {
            if world.he > 0 {
                ARM_HE
            } else {
                ARM_BOMB
            }
        }
        TRUCK => ARM_BOMB,
        TANK => {
            if world.mines > 0 {
                ARM_MINE
            } else {
                ARM_BOMB
            }
        }
        _ => {
            if world.vog > 0 {
                ARM_VOG
            } else if world.bombs > 0 {
                ARM_BOMB
            } else if world.he > 0 {
                ARM_HE
            } else {
                ARM_GUN
            }
        }
    }
}

/// The patrol reverses inside this many pixels. A bomb still in the air
/// when that happens lands on empty road.
fn will_turn(world: &World, aim: &Aim, fall: i32) -> bool {
    if aim.kind != TRUCK {
        return false;
    }
    let Some(truck) = world
        .trucks
        .iter()
        .find(|truck| truck.alive && truck.x == aim.x && truck.y == aim.y)
    else {
        return false;
    };
    if truck.goal || truck.dir == 0 {
        return false;
    }
    let home = if truck.vertical {
        truck.home_ty * 8 + 4
    } else {
        truck.home_tx * 8 + 4
    };
    let here = if truck.vertical {
        truck.y >> 8
    } else {
        truck.x >> 8
    };
    let sign = if truck.dir > 0 { 1 } else { -1 };
    let room = 48 - (here - home) * sign;
    let pace = aim.vx.abs().max(aim.vy.abs());
    room < pace * fall / 256 + 8
}

fn drop_slop(kind: u8, alt_px: i32) -> i32 {
    let radius = match kind {
        TANK => 5,
        TRUCK => 10,
        BUNKER => 16,
        ROOF => 12,
        MAN => 8,
        _ => 8,
    };
    let scatter = match kind {
        TANK => alt_px / 5,
        BUNKER | ROOF => alt_px / 4,
        MAN => alt_px / 8,
        _ => alt_px / 5,
    };
    (radius - scatter).max(3)
}

/// Where the truck will be when a cruise at 400 meets it.
fn intercept(world: &World, lx: i32, ly: i32, vx: i32, vy: i32) -> (i32, i32) {
    let px = i64::from(delta(lx, world.x));
    let py = i64::from(delta(ly, world.y));
    let vtx = i64::from(vx);
    let vty = i64::from(vy);
    let speed = 400i64;
    let a = vtx * vtx + vty * vty - speed * speed;
    let b = 2 * (px * vtx + py * vty);
    let c = px * px + py * py;
    let disc = b * b - 4 * a * c;
    if a == 0 || disc < 0 {
        return (lx, ly);
    }
    let root = isqrt64(disc);
    let t1 = (-b - root) / (2 * a);
    let t2 = (-b + root) / (2 * a);
    let t = [t1, t2].into_iter().filter(|t| *t > 0).min().unwrap_or(0);
    (
        lx.wrapping_add((vtx * t) as i32),
        ly.wrapping_add((vty * t) as i32),
    )
}

fn fall_frames(alt: i32, vz0: i32) -> i32 {
    let mut z = alt;
    let mut vz = vz0;
    let mut n = 0;
    while z > 0 && n < 300 {
        vz -= GRAVITY;
        z += vz;
        n += 1;
    }
    n.max(1)
}

fn tap(press: &mut ButtonState, button: Button) {
    *press = *press | button;
}

fn press_hold(press: &mut ButtonState, button: Button) {
    *press = *press | button;
}

fn style_name(style: Style) -> &'static str {
    match style {
        Style::Deck => "deck",
        Style::Cruise => "cruise",
        Style::Mark => "mark",
        Style::Ram => "ram",
        Style::Haul => "haul",
        Style::Gap => "gap",
        Style::Gun => "gun",
    }
}

fn card_name(level: u8) -> &'static str {
    match level {
        0 => "GATE",
        1 => "OFFICER",
        2 => "TRENCH",
        3 => "HOUSE",
        4 => "TRUCK",
        5 => "NIGHT",
        6 => "HEAT",
        7 => "MARK",
        8 => "COLUMN",
        9 => "ARMOR",
        10 => "PILL",
        11 => "DIVE",
        12 => "LANE",
        13 => "FOOT",
        14 => "FLARE",
        15 => "NEST",
        16 => "DART",
        17 => "PUSH",
        18 => "SKIRT",
        19 => "BEND",
        20 => "CHARGE",
        21 => "HAUL",
        22 => "SWARM",
        23 => "CROWS",
        24 => "COVER",
        25 => "PITS",
        26 => "WOOD",
        27 => "ROAD",
        28 => "IRON",
        29 => "POST",
        _ => "?",
    }
}

#[test_case]
fn campaign_sorties(_gba: &mut agb::Gba) {
    let mut floor = AffineBackground::new(
        agb::display::Priority::P1,
        agb::display::tiled::AffineBackgroundSize::Background64x64,
        agb::display::tiled::AffineBackgroundWrapBehaviour::Wrap,
    );
    let (cleared, starred) = survey(&mut floor);
    // Cards a stick pilot has already cleared. A miss here is a regression.
    // The other lines in the log are the sorts the pilot does not finish.
    let proved = (1 << 0)
        | (1 << 1)
        | (1 << 2)
        | (1 << 3)
        | (1 << 4)
        | (1 << 5)
        | (1 << 6)
        | (1 << 7)
        | (1 << 8)
        | (1 << 9)
        | (1 << 10)
        | (1 << 11)
        | (1 << 12)
        | (1 << 13)
        | (1 << 14)
        | (1 << 15)
        | (1 << 16)
        | (1 << 17)
        | (1 << 18)
        | (1 << 19)
        | (1 << 20)
        | (1 << 21)
        | (1 << 22)
        | (1 << 23)
        | (1 << 24)
        | (1 << 25)
        | (1 << 26)
        | (1 << 27)
        | (1 << 28)
        | (1 << 29);
    let missed = proved & !cleared;
    assert!(missed == 0, "regressed cards {missed:#x}");
    let short = proved & !starred;
    assert!(short == 0, "short of 3 stars {short:#x}");
}
