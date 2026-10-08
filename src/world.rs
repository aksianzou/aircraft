use agb::display::tiled::AffineBackground;
use agb::display::WIDTH;
use agb::fixnum::{Num, num};
use agb::input::{Button, ButtonController, Tri};

use crate::backgrounds;
use alloc::vec;
use alloc::vec::Vec;

use crate::draw::{isqrt64, project};

pub(crate) const SFX_GUN: u8 = 1;
pub(crate) const SFX_LINK: u8 = 2;
pub(crate) const SFX_BOOM: u8 = 4;
pub(crate) const SFX_MARK: u8 = 8;
pub(crate) const SFX_FALL: u8 = 16;
pub(crate) const SFX_WHISTLE: u8 = 32;

/// Affine window kept around the plane. The world itself is generated, not stored.
pub(crate) const VIEW: i32 = 128;
/// Half-width of the tiles guaranteed to match the seed. A wider view would
/// wrap onto the wrong district, so the fog starts before that distance.
pub(crate) const SPAN: i32 = 60;
pub(crate) const REGION: i32 = 64;
pub(crate) const MAX_MARKS: usize = 128;
pub(crate) const FOCAL: i32 = 160;
/// Sky headings, in the same 8.8 turns as yaw. The moon is due north, so a
/// level run keeps it above the glass and a look up puts it on the crosshair.
/// The sun sits off that line.
pub(crate) const MOON_AZIMUTH: i32 = 0;
pub(crate) const SUN_AZIMUTH: i32 = 16;
pub(crate) const MOON_ELEV: i32 = 24;
pub(crate) const SUN_ELEV: i32 = 22;
/// Same factor the mode 7 floor uses, so a world point lands on its scanline.
/// Low enough that a moderate altitude still shows a field, not a ribbon.
pub(crate) const FLOOR_ZOOM: i32 = 5;
pub(crate) const GRAVITY: i32 = 2;
/// Rise over run, in eighths. A tank barrel stays near the level, so a machine
/// sitting above the turret is outside the arc. Flak points up.
pub(crate) const TANK_SLOPE: i32 = 3;
pub(crate) const FLAK_SLOPE: i32 = 48;
/// Mode 7 sits in this rectangle. The margins are the instrument frame, so the
/// far corners of the map stay off the picture.
pub(crate) const VIEW_X: i32 = 32;
pub(crate) const VIEW_Y: i32 = 16;
pub(crate) const VIEW_W: i32 = 176;
pub(crate) const VIEW_H: i32 = 144;
/// Radar radius, in world pixels. About three dozen tiles.
pub(crate) const RADAR_RANGE: i32 = 28 * 8;

pub(crate) const T_SKY: u8 = 0;
pub(crate) const T_FLOOR: u8 = 1;
pub(crate) const T_FLOOR2: u8 = 2;
pub(crate) const T_DIRT: u8 = 3;
pub(crate) const T_ROAD: u8 = 4;
pub(crate) const T_WATER: u8 = 5;
pub(crate) const T_TREE: u8 = 6;
pub(crate) const T_CRATER: u8 = 7;
/// Parked with the sheet. The mortar round is a sprite and bursts when it lands.
pub(crate) const T_MINE: u8 = 8;
pub(crate) const T_TRENCH: u8 = 9;
pub(crate) const T_LIP: u8 = 10;
pub(crate) const T_FOG: u8 = 11;
/// Extra grass, already parked in VRAM after the fog tile. They break up the flat field.
pub(crate) const T_GRASS_L: u8 = 12;
pub(crate) const T_GRASS_D: u8 = 13;
pub(crate) const T_TUFT: u8 = 14;
pub(crate) const T_BUSH: u8 = 15;
pub(crate) const T_PATCH: u8 = 16;

pub(crate) const LEVEL_COUNT: u8 = 25;
/// A battery sees the drone only inside its circle, and only under this height.
/// At this height the airframe leaves every gun's sight.
pub(crate) const AA_HIDE: i32 = 20;

pub(crate) const KIND_BOMB: u8 = 0;
pub(crate) const KIND_HE: u8 = 1;
/// Mortar round. A tank dies when any charge but a bullet lands in the hatch.
pub(crate) const KIND_MINE: u8 = 2;
pub(crate) const KIND_GUN: u8 = 3;
pub(crate) const KIND_FLAK: u8 = 4;
pub(crate) const KIND_SHELL: u8 = 5;
/// Underbarrel grenade. Infantry only.
pub(crate) const KIND_VOG: u8 = 6;

pub(crate) const ARM_GUN: u8 = 0;
pub(crate) const ARM_BOMB: u8 = 1;
pub(crate) const ARM_HE: u8 = 2;
pub(crate) const ARM_MINE: u8 = 3;
pub(crate) const ARM_VOG: u8 = 4;
/// World pixels from the turret centre. The hatch is the dark square in the top view.
pub(crate) const HATCH: i32 = 6;
/// Beside the hull. Wide enough that a normal bomb drop counts.
pub(crate) const TANK_NEAR: i32 = 28;
/// Bombs beside the hull that finish a tank. The hatch is still one shot.
pub(crate) const BOMB_HULL_HITS: i8 = 3;

/// How many blasts beside the hull finish a full tank. Zero leaves the hull alone.
pub(crate) fn hull_hits(kind: u8) -> i8 {
    match kind {
        KIND_BOMB => BOMB_HULL_HITS,
        KIND_MINE => 2,
        _ => 0,
    }
}

pub(crate) const HP_MAN: i8 = 8;
pub(crate) const HP_OFFICER: i8 = 12;
pub(crate) const HP_TRUCK: i8 = 22;
pub(crate) const HP_TANK: i8 = 36;
pub(crate) const HP_BUNKER: i8 = 28;

pub(crate) const MAX_SHOTS: usize = 12;
pub(crate) const MAX_MINES: usize = 8;
pub(crate) const MAX_BOOMS: usize = 6;
pub(crate) const MAX_TRUCKS: usize = 4;
pub(crate) const MAX_TANKS: usize = 3;
pub(crate) const MAX_BUNKERS: usize = 3;
pub(crate) const MAX_SOLDIERS: usize = 8;
pub(crate) const MAX_FOES: usize = 8;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    /// Opening cards: the name, then the author.
    Splash,
    Title,
    Fly,
    /// Held flight. Resume continues it. Exit returns to the level card.
    Pause,
    Crash,
    Win,
    /// The last card is done.
    Done,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Quad,
    /// Fly the opening. Weapons are off; missing the gap returns you to the pad.
    Gate,
    Fpv,
    /// Spotter. The sight marks a point, then the salvo arrives.
    Guide,
}

/// Which line sits under CRASH.
#[derive(Clone, Copy)]
pub(crate) enum Why {
    None,
    Cell,
    Hull,
    Truck,
    Line,
    Base,
}

/// What ends the sortie. Each card is a different task and a different way to fly.
pub(crate) enum Task {
    /// Bomb a marked stretch of trench.
    Trench,
    /// The marked man is the only target.
    Officer,
    /// Dark sortie through a night sight. Only trucks count.
    Night,
    /// The same ground, seen as heat. Hot trucks count.
    Heat,
    /// Pass the gap at the right height.
    Gate,
    /// Kamikaze. Each impact spends a drone.
    Fpv,
    /// A truck is driving to a mark. Stop it before it arrives.
    Convoy,
    /// Bomb the marked building until it is gone.
    Hq,
    /// Mark a point through the sight. Artillery arrives on a count.
    Guide,
    /// Three tanks, seen through the night sight.
    Armor,
    /// Three bunkers, seen as heat. They fire when you come down.
    Pill,
    /// The spotter again. The trucks are moving, so the mark has to lead them.
    Column,
    /// Men on foot running for a base. Stop them before they arrive.
    Rush,
    /// Trucks and men pushing for the same base.
    Push,
    /// Day strike. Under 20 a gun sees you only in its circle. At 20 it loses you.
    Lane,
    /// Night sight, tanks, and the same low lane between the guns.
    Flare,
    /// Kamikaze along the lane. The men stand in the open gaps.
    Nest,
    /// Kamikaze through the heat sight. Trucks sit in the open gaps.
    Dart,
    /// Spotter holds one height, under the line where the guns go blind, so the lane is the road.
    Bend,
    /// Heat sight. The bunkers sit past the lane.
    Skirt,
    /// Small quad. It holds one charge and slides. The glass can look down or ahead.
    Fetch,
    /// Hostile quads. Only the bullet brings one down.
    Swarm,
    /// One truck for the posts, two quads riding above it. The bullet takes the quads, a bomb the truck.
    Cover,
}

/// What the glass shows. The side instruments stay in their own colours.
#[derive(Clone, Copy)]
pub(crate) enum Sensor {
    Day,
    Night,
    Heat,
    /// Active infrared illuminator: grey-green, with blooms on a blast.
    Ir,
}

pub(crate) fn sensor_of(level: u8) -> Sensor {
    match task_of(level) {
        Task::Night | Task::Armor | Task::Flare => Sensor::Night,
        Task::Heat | Task::Pill | Task::Dart | Task::Skirt => Sensor::Heat,
        Task::Guide | Task::Column | Task::Rush | Task::Push | Task::Bend => Sensor::Ir,
        _ => Sensor::Day,
    }
}

pub(crate) fn task_of(level: u8) -> Task {
    // One campaign, in order. The cards are not a menu.
    match level % LEVEL_COUNT {
        0 => Task::Trench,
        1 => Task::Officer,
        2 => Task::Convoy,
        3 => Task::Hq,
        4 => Task::Night,
        5 => Task::Heat,
        6 => Task::Gate,
        7 => Task::Fpv,
        8 => Task::Guide,
        9 => Task::Armor,
        10 => Task::Pill,
        11 => Task::Column,
        12 => Task::Lane,
        13 => Task::Rush,
        14 => Task::Flare,
        15 => Task::Nest,
        16 => Task::Dart,
        17 => Task::Push,
        18 => Task::Skirt,
        19 => Task::Bend,
        20 | 21 => Task::Fetch,
        22 | 23 => Task::Swarm,
        24 => Task::Cover,
        _ => Task::Guide,
    }
}

/// The second mine card. The rounds and the tanks sit farther from the pad.
pub(crate) fn haul_card(level: u8) -> bool {
    matches!(task_of(level), Task::Fetch) && level % LEVEL_COUNT == 21
}

/// The second drone card. The quads fly at the pad.
pub(crate) fn defend_card(level: u8) -> bool {
    matches!(task_of(level), Task::Swarm) && level % LEVEL_COUNT == 23
}

#[derive(Clone, Copy)]
pub(crate) struct Shot {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) z: i32,
    pub(crate) vx: i32,
    pub(crate) vy: i32,
    pub(crate) vz: i32,
    pub(crate) kind: u8,
    pub(crate) life: i16,
    pub(crate) alive: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct Mine {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) arm: i16,
    pub(crate) alive: bool,
}

/// One mortar round lying in the grass. The quad can carry a single one.
#[derive(Clone, Copy)]
pub(crate) struct Cache {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) alive: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct Boom {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) z: i32,
    pub(crate) frame: i16,
    pub(crate) alive: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct Truck {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) dir: i32,
    pub(crate) vertical: bool,
    pub(crate) home_tx: i32,
    pub(crate) home_ty: i32,
    /// This is the truck that has to be stopped.
    pub(crate) goal: bool,
    /// This truck is one of the mission marks.
    pub(crate) objective: bool,
    pub(crate) hp: i8,
    pub(crate) cool: u8,
    pub(crate) alive: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct Tank {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) dir: i32,
    pub(crate) vertical: bool,
    pub(crate) home_tx: i32,
    pub(crate) home_ty: i32,
    pub(crate) objective: bool,
    pub(crate) hp: i8,
    pub(crate) cool: u8,
    pub(crate) alive: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct Bunker {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) home_tx: i32,
    pub(crate) home_ty: i32,
    pub(crate) objective: bool,
    pub(crate) hp: i8,
    pub(crate) cool: u8,
    pub(crate) alive: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct Soldier {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) dir: i32,
    pub(crate) step: u8,
    pub(crate) home_tx: i32,
    pub(crate) home_ty: i32,
    pub(crate) officer: bool,
    /// This soldier is one of the mission marks.
    pub(crate) objective: bool,
    pub(crate) hp: i8,
    pub(crate) cool: u8,
    pub(crate) alive: bool,
}

/// Hostile aircraft. A quad chases. A wing flies slower and reads from farther away.
#[derive(Clone, Copy)]
pub(crate) struct Foe {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) z: i32,
    pub(crate) home_x: i32,
    pub(crate) home_y: i32,
    /// Frames until another same-height bump can land.
    pub(crate) cool: i16,
    /// Delta wing. Wider than a quad, so it stays a shape on the horizon.
    pub(crate) wing: bool,
    /// `1` flies south, `-1` flies north. A wing never sits on a point.
    pub(crate) heading: i8,
    pub(crate) alive: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct Mark {
    pub(crate) tx: i32,
    pub(crate) ty: i32,
    pub(crate) tile: u8,
    pub(crate) quiet: bool,
    pub(crate) used: bool,
}

pub(crate) struct World {
    pub(crate) marks: Vec<Mark>,
    pub(crate) mark_len: usize,
    pub(crate) view_tx: i32,
    pub(crate) view_ty: i32,
    pub(crate) spawn_row: u8,
    pub(crate) level: u8,
    pub(crate) mode: Mode,
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) alt: i32,
    pub(crate) pad_x: i32,
    pub(crate) pad_y: i32,
    pub(crate) pad_alt: i32,
    pub(crate) drones: i32,
    pub(crate) vx: i32,
    pub(crate) vy: i32,
    pub(crate) yaw: Num<i32, 8>,
    pub(crate) pitch: Num<i32, 8>,
    /// Display only. Follows the yaw stick so the attitude indicator banks.
    pub(crate) bank: Num<i32, 8>,
    pub(crate) bombs: i32,
    pub(crate) he: i32,
    pub(crate) mines: i32,
    pub(crate) vog: i32,
    /// How long the quad stick has been held, and which way. Speed grows with it.
    pub(crate) stick_hold: i16,
    pub(crate) stick_key: i8,
    /// What B fires. A steps to the next one.
    pub(crate) arm: u8,
    /// Airframe, 0..=100. A close blast spends it.
    pub(crate) hull: i32,
    /// Flight time, 0..=100. Empty drops the link.
    pub(crate) battery: i32,
    /// Frames spent past the radio. Turning back spends it down.
    pub(crate) blackout: i16,
    pub(crate) age: i32,
    pub(crate) score: i32,
    pub(crate) targets: i32,
    pub(crate) fire_cd: i16,
    pub(crate) recover: i16,
    pub(crate) crashed: bool,
    pub(crate) why: Why,
    /// The last FPV airframe is gone. The same mission starts over.
    pub(crate) again: bool,
    /// The radio is gone. The nose drops until the airframe hits.
    pub(crate) falling: bool,
    /// Quad only. Holds height, looks straight down, and the stick slides.
    pub(crate) hover: bool,
    /// The one mortar round a small quad can carry. Fetch missions only.
    pub(crate) cargo: bool,
    pub(crate) caches: [Cache; 6],
    /// Frames until the marked salvo. Zero means the sight is free.
    pub(crate) guide: i16,
    pub(crate) salvo: i8,
    pub(crate) salvo_cd: i8,
    pub(crate) mark_x: i32,
    pub(crate) mark_y: i32,
    /// Tile the ground frame is built around. `i32::MIN` means there is no frame.
    pub(crate) guide_cx: i32,
    pub(crate) guide_cy: i32,
    pub(crate) guide_frame_n: u8,
    /// Previous tile under each cell of the ground frame, so the square can leave.
    pub(crate) guide_frame: [(i32, i32, u8); 16],
    /// IR bloom after a blast. Counts down to the plain illuminator picture.
    pub(crate) bloom: i16,
    /// Gun, drop and blast cues for this frame.
    pub(crate) sfx: u8,
    pub(crate) shots: [Shot; MAX_SHOTS],
    pub(crate) mine_list: [Mine; MAX_MINES],
    pub(crate) booms: [Boom; MAX_BOOMS],
    pub(crate) trucks: [Truck; MAX_TRUCKS],
    pub(crate) tanks: [Tank; MAX_TANKS],
    pub(crate) bunkers: [Bunker; MAX_BUNKERS],
    pub(crate) soldiers: [Soldier; MAX_SOLDIERS],
    pub(crate) foes: [Foe; MAX_FOES],
}

/// mGBA keeps SRAM only when the ROM contains this tag.

pub(crate) fn wrap_coord(value: i32) -> i32 {
    value
}

pub(crate) fn wrap_yaw(yaw: &mut Num<i32, 8>) {
    if *yaw >= num!(1) {
        *yaw -= num!(1);
    }
    if *yaw < num!(0) {
        *yaw += num!(1);
    }
}

pub(crate) fn delta(a: i32, b: i32) -> i32 {
    a.wrapping_sub(b)
}

pub(crate) fn tile_of(sub: i32) -> i32 {
    (sub >> 8).div_euclid(8)
}

pub(crate) fn tri_value(tri: Tri) -> i32 {
    match tri {
        Tri::Negative => -1,
        Tri::Zero => 0,
        Tri::Positive => 1,
    }
}

pub(crate) fn hash(x: i32, y: i32) -> u32 {
    let mut n = x
        .wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263));
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    n as u32
}

pub(crate) fn start_tile(level: u8) -> (i32, i32) {
    match task_of(level) {
        // South of the marks: a stretch of open ground, still inside the radio.
        Task::Trench | Task::Officer | Task::Hq => (24, 90),
        Task::Night | Task::Heat => (24, 78),
        Task::Convoy => (8, 70),
        Task::Gate | Task::Fpv => (24, 92),
        Task::Guide | Task::Column | Task::Armor => (24, 90),
        Task::Rush | Task::Push => (24, 64),
        Task::Lane | Task::Flare | Task::Nest | Task::Dart | Task::Bend | Task::Skirt => (22, 88),
        Task::Pill => (24, 86),
        Task::Fetch => (24, 90),
        Task::Swarm => (24, 90),
        // South of the truck, facing the posts.
        Task::Cover => (24, 78),
    }
}

pub(crate) struct AaGun {
    pub(crate) tx: i32,
    pub(crate) ty: i32,
    pub(crate) radius: i32,
}

/// Three gates. The open side shifts right, then back left.
pub(crate) const LANE_GUNS: [AaGun; 6] = [
    AaGun { tx: -4, ty: 78, radius: 104 },
    AaGun { tx: 48, ty: 78, radius: 104 },
    AaGun { tx: 6, ty: 56, radius: 120 },
    AaGun { tx: 54, ty: 56, radius: 104 },
    AaGun { tx: -6, ty: 34, radius: 104 },
    AaGun { tx: 42, ty: 34, radius: 104 },
];

pub(crate) fn aa_guns(task: Task) -> &'static [AaGun] {
    match task {
        Task::Lane | Task::Flare | Task::Nest | Task::Dart | Task::Bend | Task::Skirt => {
            &LANE_GUNS
        }
        _ => &[],
    }
}

pub(crate) fn gun_at(gun: &AaGun) -> (i32, i32) {
    ((gun.tx * 8 + 4) << 8, (gun.ty * 8 + 4) << 8)
}

/// The drone is in sight under `AA_HIDE`. At that height every gun loses it.
pub(crate) fn guns_see(alt_px: i32) -> bool {
    alt_px < AA_HIDE
}

pub(crate) fn aa_covers(guns: &[AaGun], x: i32, y: i32) -> bool {
    guns.iter().any(|gun| {
        let (gx, gy) = gun_at(gun);
        dist2_px(x, y, gx, gy) <= gun.radius * gun.radius
    })
}

/// Radar dial point back into the world. Ahead on the dial is negative `ry`.
pub(crate) fn lane_world(cam_x: i32, cam_y: i32, sin_r: i32, cos_r: i32, rx: i32, ry: i32) -> (i32, i32) {
    let fwd = -ry * RADAR_RANGE / 12;
    let right = rx * RADAR_RANGE / 12;
    let dx = (fwd * sin_r + right * cos_r) >> 8;
    let dy = (right * sin_r - fwd * cos_r) >> 8;
    (cam_x + (dx << 8), cam_y + (dy << 8))
}

/// One battlefield everywhere. Roads tie the districts together. Trenches
/// and a few copses sit in the grass. Water is a pond, not a scatter of puddles.
/// The skyline is not this map.
pub(crate) fn terrain(level: u8, tx: i32, ty: i32) -> u8 {
    let (pad_x, pad_y) = start_tile(level);
    if (tx - pad_x).abs() <= 3 && (ty - pad_y).abs() <= 3 {
        return T_FLOOR;
    }
    // A charge pad: concrete square, dark tile in the middle. The quad sits there.
    if charge_spots(level)
        .iter()
        .any(|&(cx, cy)| (tx - cx).abs() <= 1 && (ty - cy).abs() <= 1)
    {
        return if charge_spots(level)
            .iter()
            .any(|&(cx, cy)| tx == cx && ty == cy)
        {
            T_ROAD
        } else {
            T_FLOOR
        };
    }
    let lx = tx.rem_euclid(REGION);
    let ly = ty.rem_euclid(REGION);
    if lx <= 1 || ly <= 1 {
        return T_ROAD;
    }
    if lx == 2 || ly == 2 {
        return T_DIRT;
    }
    let n = hash(tx, ty.wrapping_add(level as i32 * 13));
    let gap = lx % 14 == 7;
    if !gap && (ly == 20 || ly == 36) {
        return T_LIP;
    }
    if !gap && (ly == 21 || ly == 37) {
        return T_TRENCH;
    }
    if pond(tx, ty) {
        return T_WATER;
    }
    // Woods share a 3×3 cell, so trees stand together instead of peppering the field.
    let wx = tx.div_euclid(3);
    let wy = ty.div_euclid(3);
    if hash(wx, wy.wrapping_add(17)) % 28 == 0 {
        return if n % 5 == 0 { T_BUSH } else { T_TREE };
    }
    if n % 29 == 0 {
        return T_TUFT;
    }
    if n % 53 == 0 {
        return T_PATCH;
    }
    // One tone for a 4×4 patch. Neighbouring tiles stay in the same grass.
    let tone = hash(tx.div_euclid(4), ty.div_euclid(4).wrapping_add(level as i32 * 13));
    match tone % 5 {
        0 => T_GRASS_D,
        1 => T_GRASS_L,
        2 => T_FLOOR2,
        _ => T_FLOOR,
    }
}

/// A pond a few tiles across, clear of the road grid and the trench lines.
pub(crate) fn pond(tx: i32, ty: i32) -> bool {
    let span = 20;
    let cx = tx.div_euclid(span);
    let cy = ty.div_euclid(span);
    if hash(cx, cy.wrapping_add(41)) % 5 != 0 {
        return false;
    }
    let ox = 6 + (hash(cx, cy) % 8) as i32;
    let oy = 6 + (hash(cx, cy.wrapping_add(2)) % 8) as i32;
    let mid_x = cx * span + ox;
    let mid_y = cy * span + oy;
    let rx = mid_x.rem_euclid(REGION);
    let ry = mid_y.rem_euclid(REGION);
    if rx < 8 || rx > REGION - 8 || ry < 8 || ry > REGION - 8 {
        return false;
    }
    if (17..=24).contains(&ry) || (33..=40).contains(&ry) {
        return false;
    }
    let dx = tx - mid_x;
    let dy = ty - mid_y;
    let radius = 3 + (hash(cx, cy.wrapping_add(5)) % 2) as i32;
    let dist = dx * dx + dy * dy;
    if dist > radius * radius {
        return false;
    }
    !(dist + 3 >= radius * radius && hash(tx, ty) % 3 == 0)
}

pub(crate) fn dist2_px(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    let dx = delta(ax, bx) >> 8;
    let dy = delta(ay, by) >> 8;
    dx * dx + dy * dy
}

pub(crate) fn dead_shot() -> Shot {
    Shot {
        x: 0,
        y: 0,
        z: 0,
        vx: 0,
        vy: 0,
        vz: 0,
        kind: 0,
        life: 0,
        alive: false,
    }
}

pub(crate) fn dead_mine() -> Mine {
    Mine {
        x: 0,
        y: 0,
        arm: 0,
        alive: false,
    }
}

pub(crate) fn dead_cache() -> Cache {
    Cache {
        x: 0,
        y: 0,
        alive: false,
    }
}

/// Rounds to pick up. The later card keeps them farther from the pad.
/// The last three sit well south of the tanks, on open grass.
pub(crate) fn cache_spots(level: u8) -> [(i32, i32); 6] {
    if haul_card(level) {
        [(14, 78), (34, 76), (24, 70), (6, 96), (42, 96), (24, 112)]
    } else {
        [(18, 82), (30, 82), (24, 76), (8, 96), (40, 96), (24, 108)]
    }
}

/// Where the mine quad can sit and fill the battery. Clear of the tanks.
pub(crate) fn charge_spots(level: u8) -> &'static [(i32, i32)] {
    if haul_card(level) {
        &[(24, 56), (24, 102)]
    } else if matches!(task_of(level), Task::Fetch) {
        &[(24, 62), (24, 102)]
    } else {
        &[]
    }
}

/// Where those rounds have to land. The trucks stay put.
pub(crate) fn mortar_spots(level: u8) -> [(i32, i32); 3] {
    if haul_card(level) {
        [(10, 40), (24, 34), (40, 42)]
    } else {
        [(16, 52), (26, 46), (36, 50)]
    }
}

pub(crate) fn dead_boom() -> Boom {
    Boom {
        x: 0,
        y: 0,
        z: 0,
        frame: 0,
        alive: false,
    }
}

pub(crate) fn dead_mark() -> Mark {
    Mark {
        tx: 0,
        ty: 0,
        tile: T_FLOOR,
        quiet: false,
        used: false,
    }
}

pub(crate) fn dead_truck() -> Truck {
    Truck {
        x: 0,
        y: 0,
        dir: 0,
        vertical: false,
        home_tx: 0,
        home_ty: 0,
        goal: false,
        objective: false,
        hp: 0,
        cool: 0,
        alive: false,
    }
}

pub(crate) fn dead_tank() -> Tank {
    Tank {
        x: 0,
        y: 0,
        dir: 0,
        vertical: false,
        home_tx: 0,
        home_ty: 0,
        objective: false,
        hp: 0,
        cool: 0,
        alive: false,
    }
}

pub(crate) fn dead_bunker() -> Bunker {
    Bunker {
        x: 0,
        y: 0,
        home_tx: 0,
        home_ty: 0,
        objective: false,
        hp: 0,
        cool: 0,
        alive: false,
    }
}

pub(crate) fn dead_soldier() -> Soldier {
    Soldier {
        x: 0,
        y: 0,
        dir: 0,
        step: 0,
        home_tx: 0,
        home_ty: 0,
        officer: false,
        objective: false,
        hp: 0,
        cool: 0,
        alive: false,
    }
}

pub(crate) fn dead_foe() -> Foe {
    Foe {
        x: 0,
        y: 0,
        z: 0,
        home_x: 0,
        home_y: 0,
        cool: 0,
        wing: false,
        heading: -1,
        alive: false,
    }
}

/// The later slots on an air card are delta wings, not quads.
pub(crate) fn foe_wing(level: u8, index: usize) -> bool {
    if defend_card(level) {
        index >= 6
    } else if matches!(task_of(level), Task::Swarm) {
        index >= 4
    } else if matches!(task_of(level), Task::Cover) {
        index >= 2
    } else {
        false
    }
}

/// Hostile quads. Only the later cards have them. `i32::MIN` means that slot is empty.
pub(crate) fn foe_spots(level: u8) -> [(i32, i32); MAX_FOES] {
    let placed: &[(i32, i32)] = if defend_card(level) {
        &[
            (14, 68),
            (34, 62),
            (8, 56),
            (40, 52),
            (20, 44),
            (28, 36),
            (16, 24),
            (32, 24),
        ]
    } else if matches!(task_of(level), Task::Swarm) {
        &[(14, 72), (34, 72), (18, 56), (30, 56), (8, 40), (40, 40)]
    } else if matches!(task_of(level), Task::Cover) {
        // Quads flank the truck. The wings sit farther ahead, where a quad vanishes.
        &[(18, 36), (30, 36), (8, 28), (40, 28)]
    } else {
        &[]
    };
    let mut spots = [(i32::MIN, i32::MIN); MAX_FOES];
    for (index, spot) in placed.iter().enumerate() {
        spots[index] = *spot;
    }
    spots
}

/// Frames from the mark until the first rocket.
pub(crate) const GUIDE_WAIT: i16 = 240;

/// 0 on the first card, 16 on card 22. Cards after that stay at 16.
pub(crate) fn heat(level: u8) -> i32 {
    (level as i32).min(21) * 16 / 21
}

/// How fast a rush or a push walks north. 24 on the first card, 40 on the
/// last. A flat 48 reached the line before a bad mark could be corrected.
pub(crate) fn rush_step(level: u8) -> i32 {
    24 + heat(level)
}

/// Facing stays in `dir`. The step is a crawl so a mark cannot leave the radio.
pub(crate) fn ground_step(dir: i32) -> i32 {
    if dir > 0 {
        18
    } else if dir < 0 {
        -18
    } else {
        0
    }
}

impl World {
    pub(crate) fn new(bg: &mut AffineBackground, level: u8, pump: &mut dyn FnMut()) -> Self {
        let level = level % LEVEL_COUNT;
        let task = task_of(level);
        let mode = match task {
            Task::Fpv | Task::Nest | Task::Dart => Mode::Fpv,
            Task::Gate => Mode::Gate,
            Task::Guide | Task::Column | Task::Rush | Task::Push | Task::Bend => Mode::Guide,
            _ => Mode::Quad,
        };
        let (tx, ty) = start_tile(level);
        let alt = match task {
            Task::Gate => 12,
            Task::Fpv => 10,
            Task::Night | Task::Heat | Task::Armor | Task::Pill => 14,
            Task::Lane | Task::Flare | Task::Nest | Task::Dart | Task::Bend | Task::Skirt => 11,
            Task::Guide | Task::Column | Task::Rush | Task::Push => 16,
            // Already above the crows, so they are in the glass from the first frame.
            Task::Swarm if defend_card(level) => 22,
            _ => 12,
        };
        let start_x = (tx * 8 + 4) << 8;
        let start_y = (ty * 8 + 4) << 8;
        let start_alt = alt << 8;
        let fetch = matches!(task, Task::Fetch);
        let bare = mode == Mode::Fpv || mode == Mode::Guide || fetch;
        let mut world = Self {
            marks: vec![dead_mark(); MAX_MARKS],
            mark_len: 0,
            view_tx: tx,
            view_ty: ty,
            spawn_row: 0,
            level,
            mode,
            x: start_x,
            y: start_y,
            alt: start_alt,
            pad_x: start_x,
            pad_y: start_y,
            pad_alt: start_alt,
            drones: match task {
                // Pairs share one blast, so six frames is two misses.
                Task::Fpv => 6,
                // Four nests, one spare. The dart keeps a single spare.
                Task::Nest => 5,
                Task::Dart => 4,
                _ => 0,
            },
            vx: 0,
            vy: 0,
            yaw: num!(0),
            pitch: num!(0),
            bank: num!(0),
            bombs: if bare { 0 } else { 40 },
            he: if bare { 0 } else { 8 },
            mines: if bare { 0 } else { 8 },
            vog: if bare { 0 } else { 16 },
            stick_hold: 0,
            stick_key: 0,
            arm: if mode == Mode::Fpv || matches!(task, Task::Swarm | Task::Cover) {
                ARM_GUN
            } else if fetch {
                ARM_MINE
            } else {
                ARM_BOMB
            },
            hull: 100,
            battery: 100,
            blackout: 0,
            age: 0,
            score: 0,
            targets: match task {
                Task::Trench => 6,
                Task::Officer => 1,
                Task::Night | Task::Heat | Task::Armor | Task::Pill | Task::Column => 3,
                Task::Gate => 1,
                Task::Fpv => 8,
                Task::Convoy => 1,
                Task::Hq => 6,
                Task::Guide => 3,
                Task::Rush => 6,
                Task::Push => 9,
                Task::Lane | Task::Flare | Task::Dart | Task::Bend | Task::Skirt => 3,
                Task::Nest => 4,
                Task::Fetch => 3,
                Task::Swarm => {
                    if defend_card(level) {
                        8
                    } else {
                        6
                    }
                }
                // Two quads, two wings, and the truck. Each has to fall.
                Task::Cover => 5,
            },
            fire_cd: 0,
            recover: 0,
            crashed: false,
            why: Why::None,
            again: false,
            falling: false,
            hover: fetch,
            cargo: false,
            caches: [dead_cache(); 6],
            guide: 0,
            salvo: 0,
            salvo_cd: 0,
            mark_x: 0,
            mark_y: 0,
            guide_cx: i32::MIN,
            guide_cy: i32::MIN,
            guide_frame_n: 0,
            guide_frame: [(0, 0, 0); 16],
            bloom: 0,
            sfx: 0,
            shots: [dead_shot(); MAX_SHOTS],
            mine_list: [dead_mine(); MAX_MINES],
            booms: [dead_boom(); MAX_BOOMS],
            trucks: [dead_truck(); MAX_TRUCKS],
            tanks: [dead_tank(); MAX_TANKS],
            bunkers: [dead_bunker(); MAX_BUNKERS],
            soldiers: [dead_soldier(); MAX_SOLDIERS],
            foes: [dead_foe(); MAX_FOES],
        };
        world.park(bg);
        world.refill(pump);
        world.seat_objective();
        world.seat_foes();
        if matches!(task, Task::Convoy) {
            world.paint(bg, 0, 4, T_FLOOR);
            world.paint(bg, 0, 3, T_FLOOR);
        }
        world.spawn_nearby(pump);
        world
    }

    pub(crate) fn gate_x(&self) -> i32 {
        (24 * 8 + 4) << 8
    }

    pub(crate) fn gate_y(&self) -> i32 {
        (6 * 8 + 4) << 8
    }

    pub(crate) fn seat_foes(&mut self) {
        for (index, (tx, ty)) in foe_spots(self.level).into_iter().enumerate() {
            if tx == i32::MIN {
                continue;
            }
            let x = (tx * 8 + 4) << 8;
            let y = (ty * 8 + 4) << 8;
            let wing = foe_wing(self.level, index);
            self.foes[index] = Foe {
                x,
                y,
                // Wings sit lower, so they are already in the glass when a quad is not.
                z: if wing { 8 << 8 } else { 12 << 8 },
                home_x: x,
                home_y: y,
                cool: 0,
                wing,
                heading: -1,
                alive: true,
            };
        }
    }

    pub(crate) fn seat_objective(&mut self) {
        let task = task_of(self.level);
        match task {
            Task::Trench => {
                for (index, tx) in [22, 23, 24, 25, 26, 27].into_iter().enumerate() {
                    self.soldiers[index] = Soldier {
                        x: (tx * 8 + 4) << 8,
                        y: (21 * 8 + 4) << 8,
                        dir: 80,
                        step: 0,
                        home_tx: tx,
                        home_ty: 21,
                        officer: false,
                        objective: true,
                        hp: HP_MAN,
                        cool: 40,
                        alive: true,
                    };
                }
            }
            Task::Officer => {
                self.soldiers[0] = Soldier {
                    x: (24 * 8 + 4) << 8,
                    y: (21 * 8 + 4) << 8,
                    dir: 80,
                    step: 0,
                    home_tx: 24,
                    home_ty: 21,
                    officer: true,
                    objective: true,
                    hp: HP_OFFICER,
                    cool: 40,
                    alive: true,
                };
            }
            Task::Night | Task::Heat => {
                let row = if matches!(task, Task::Heat) { 12 } else { 0 };
                let spots = if matches!(task, Task::Heat) {
                    [6, 22, 38]
                } else {
                    [8, 24, 40]
                };
                for (index, tx) in spots.into_iter().enumerate() {
                    self.trucks[index] = Truck {
                        x: (tx * 8 + 4) << 8,
                        y: (row * 8 + 4) << 8,
                        dir: 150,
                        vertical: false,
                        home_tx: tx,
                        home_ty: row,
                        goal: false,
                        objective: true,
                        hp: HP_TRUCK,
                        cool: 30,
                        alive: true,
                    };
                }
            }
            Task::Cover => {
                // Northbound between the posts. The quads ride above this truck.
                self.trucks[0] = Truck {
                    x: (24 * 8 + 4) << 8,
                    y: (40 * 8 + 4) << 8,
                    dir: -32,
                    vertical: true,
                    home_tx: 24,
                    home_ty: 40,
                    goal: true,
                    objective: true,
                    hp: HP_TRUCK,
                    cool: 20,
                    alive: true,
                };
            }
            Task::Convoy => {
                // Northbound on the west road, toward the pale mark.
                self.trucks[0] = Truck {
                    x: (0 * 8 + 4) << 8,
                    y: (14 * 8 + 4) << 8,
                    dir: -32,
                    vertical: true,
                    home_tx: 0,
                    home_ty: 14,
                    goal: true,
                    objective: true,
                    hp: HP_TRUCK,
                    cool: 20,
                    alive: true,
                };
            }
            Task::Guide | Task::Column => {
                let spots = [10, 26, 42];
                let moving = matches!(task, Task::Column);
                for (index, tx) in spots.into_iter().enumerate() {
                    let row = if moving { 64 } else { 20 };
                    self.trucks[index] = Truck {
                        x: (tx * 8 + 4) << 8,
                        y: (row * 8 + 4) << 8,
                        dir: if moving { 80 } else { 0 },
                        vertical: false,
                        home_tx: tx,
                        home_ty: row,
                        goal: false,
                        objective: true,
                        hp: HP_TRUCK,
                        cool: 30,
                        alive: true,
                    };
                }
            }
            Task::Armor => {
                for (index, tx) in [12, 28, 44].into_iter().enumerate() {
                    self.tanks[index] = Tank {
                        x: (tx * 8 + 4) << 8,
                        y: (64 * 8 + 4) << 8,
                        dir: 80,
                        vertical: false,
                        home_tx: tx,
                        home_ty: 64,
                        objective: true,
                        hp: HP_TANK,
                        cool: 30,
                        alive: true,
                    };
                }
            }
            Task::Pill => {
                for (index, tx) in [16, 28, 40].into_iter().enumerate() {
                    self.bunkers[index] = Bunker {
                        x: (tx * 8 + 4) << 8,
                        y: (21 * 8 + 4) << 8,
                        home_tx: tx,
                        home_ty: 21,
                        objective: true,
                        hp: HP_BUNKER,
                        cool: 20,
                        alive: true,
                    };
                }
            }
            Task::Rush => {
                let spots = [(16, 41), (17, 40), (31, 43), (32, 42), (46, 40), (47, 41)];
                for (index, (tx, ty)) in spots.into_iter().enumerate() {
                    self.soldiers[index] = Soldier {
                        x: (tx * 8 + 4) << 8,
                        y: (ty * 8 + 4) << 8,
                        dir: 0,
                        step: 0,
                        home_tx: tx,
                        home_ty: ty,
                        officer: false,
                        objective: true,
                        hp: HP_MAN,
                        cool: 40,
                        alive: true,
                    };
                }
            }
            Task::Push => {
                // Three clumps. A mark on the truck reaches the two men beside it.
                // The east clump is nearest the posts, the centre one is last.
                let trucks = [(14, 52), (30, 56), (46, 48)];
                let men = [
                    (15, 52),
                    (14, 51),
                    (31, 56),
                    (30, 55),
                    (45, 48),
                    (46, 47),
                ];
                for (index, (tx, ty)) in trucks.into_iter().enumerate() {
                    self.trucks[index] = Truck {
                        x: (tx * 8 + 4) << 8,
                        y: (ty * 8 + 4) << 8,
                        dir: -80,
                        vertical: true,
                        home_tx: tx,
                        home_ty: ty,
                        goal: false,
                        objective: true,
                        hp: HP_TRUCK,
                        cool: 30,
                        alive: true,
                    };
                }
                for (index, (tx, ty)) in men.into_iter().enumerate() {
                    self.soldiers[index] = Soldier {
                        x: (tx * 8 + 4) << 8,
                        y: (ty * 8 + 4) << 8,
                        dir: 0,
                        step: 0,
                        home_tx: tx,
                        home_ty: ty,
                        officer: false,
                        objective: true,
                        hp: HP_MAN,
                        cool: 40,
                        alive: true,
                    };
                }
            }
            Task::Fpv => {
                // Four pairs, far enough apart that one blast cannot clear two.
                let groups = [(16, 36), (32, 36), (16, 24), (32, 24)];
                let mut slot = 0;
                for (gx, gy) in groups {
                    for (ox, oy) in [(0, 0), (12, 0)] {
                        let px = gx * 8 + 4 + ox;
                        let py = gy * 8 + 4 + oy;
                        self.soldiers[slot] = Soldier {
                            x: px << 8,
                            y: py << 8,
                            dir: 0,
                            step: 0,
                            home_tx: px.div_euclid(8),
                            home_ty: py.div_euclid(8),
                            officer: false,
                            objective: true,
                            hp: HP_MAN,
                            cool: 40,
                            alive: true,
                        };
                        slot += 1;
                    }
                }
            }
            Task::Lane | Task::Dart | Task::Bend => {
                let spots = [(14, 24), (24, 22), (32, 24)];
                for (index, (tx, ty)) in spots.into_iter().enumerate() {
                    self.trucks[index] = Truck {
                        x: (tx * 8 + 4) << 8,
                        y: (ty * 8 + 4) << 8,
                        dir: 0,
                        vertical: false,
                        home_tx: tx,
                        home_ty: ty,
                        goal: false,
                        objective: true,
                        hp: HP_TRUCK,
                        cool: 30,
                        alive: true,
                    };
                }
            }
            Task::Flare => {
                let spots = [(14, 24), (24, 22), (32, 24)];
                for (index, (tx, ty)) in spots.into_iter().enumerate() {
                    self.tanks[index] = Tank {
                        x: (tx * 8 + 4) << 8,
                        y: (ty * 8 + 4) << 8,
                        dir: 0,
                        vertical: false,
                        home_tx: tx,
                        home_ty: ty,
                        objective: true,
                        hp: HP_TANK,
                        cool: 30,
                        alive: true,
                    };
                }
            }
            Task::Nest => {
                let spots = [(22, 84), (32, 62), (28, 50), (18, 36)];
                for (index, (tx, ty)) in spots.into_iter().enumerate() {
                    self.soldiers[index] = Soldier {
                        x: (tx * 8 + 4) << 8,
                        y: (ty * 8 + 4) << 8,
                        dir: 0,
                        step: 0,
                        home_tx: tx,
                        home_ty: ty,
                        officer: false,
                        objective: true,
                        hp: HP_MAN,
                        cool: 40,
                        alive: true,
                    };
                }
            }
            Task::Fetch => {
                for (index, (tx, ty)) in cache_spots(self.level).into_iter().enumerate() {
                    self.caches[index] = Cache {
                        x: (tx * 8 + 4) << 8,
                        y: (ty * 8 + 4) << 8,
                        alive: true,
                    };
                }
                for (index, (tx, ty)) in mortar_spots(self.level).into_iter().enumerate() {
                    self.tanks[index] = Tank {
                        x: (tx * 8 + 4) << 8,
                        y: (ty * 8 + 4) << 8,
                        dir: 0,
                        vertical: false,
                        home_tx: tx,
                        home_ty: ty,
                        objective: true,
                        hp: HP_TANK,
                        cool: 40,
                        alive: true,
                    };
                }
            }
            Task::Skirt => {
                let spots = [(14, 24), (24, 22), (32, 24)];
                for (index, (tx, ty)) in spots.into_iter().enumerate() {
                    self.bunkers[index] = Bunker {
                        x: (tx * 8 + 4) << 8,
                        y: (ty * 8 + 4) << 8,
                        home_tx: tx,
                        home_ty: ty,
                        objective: true,
                        hp: HP_BUNKER,
                        cool: 20,
                        alive: true,
                    };
                }
            }
            _ => {}
        }
    }

    pub(crate) fn hq_goal(&self, tx: i32, ty: i32) -> bool {
        matches!(task_of(self.level), Task::Hq) && (22..=24).contains(&tx) && (16..=17).contains(&ty)
    }

    /// A kill counts only when it is this card's objective.
    pub(crate) fn claim(&mut self, officer: bool, truck: bool, goal: bool, objective: bool) {
        let counts = match task_of(self.level) {
            Task::Officer => officer,
            Task::Trench => objective && !truck,
            Task::Night | Task::Heat | Task::Guide | Task::Column | Task::Lane | Task::Dart
            | Task::Bend => truck && objective,
            Task::Convoy | Task::Cover => goal,
            Task::Fpv => objective,
            Task::Rush => objective && !truck,
            Task::Nest => objective && !truck,
            Task::Push => objective,
            Task::Gate | Task::Hq | Task::Armor | Task::Pill | Task::Flare | Task::Skirt
            | Task::Fetch | Task::Swarm => false,
        };
        if counts && self.targets > 0 {
            self.targets -= 1;
            self.sfx |= SFX_MARK;
        }
    }

    pub(crate) fn claim_armor(&mut self) {
        if matches!(task_of(self.level), Task::Armor | Task::Flare | Task::Fetch)
            && self.targets > 0
        {
            self.targets -= 1;
            self.sfx |= SFX_MARK;
        }
    }

    pub(crate) fn claim_pill(&mut self) {
        if matches!(task_of(self.level), Task::Pill | Task::Skirt) && self.targets > 0 {
            self.targets -= 1;
            self.sfx |= SFX_MARK;
        }
    }

    /// Hold time, not the glass, sets how fast the quad slides.
    pub(crate) fn quad_creep(&mut self, pitch_in: Tri, yaw_in: Tri) -> i32 {
        let along = -tri_value(pitch_in);
        let strafe = tri_value(yaw_in);
        let key = (along + strafe * 3) as i8;
        if key == 0 || key != self.stick_key {
            self.stick_hold = 0;
            self.stick_key = key;
        }
        if key != 0 && self.stick_hold < 200 {
            self.stick_hold += 2;
        }
        16 + self.stick_hold as i32
    }

    /// One mortar round at a time. B over a round takes it. B again drops it
    /// into the hatch.
    pub(crate) fn fetch_tick(&mut self, input: &ButtonController) {
        self.arm = ARM_MINE;
        self.mines = if self.cargo { 1 } else { 0 };
        if self.signal_loss() >= 16 || !input.is_just_pressed(Button::B) {
            return;
        }
        if self.cargo {
            // Released from the ground, the round detonates in the claws.
            if (self.alt >> 8) > 8 && self.try_shot(KIND_MINE) {
                self.cargo = false;
            }
            return;
        }
        // The claws close on the ground, not from cruise height.
        if (self.alt >> 8) > 1 {
            return;
        }
        if let Some(slot) = self
            .caches
            .iter()
            .position(|cache| cache.alive && dist2_px(self.x, self.y, cache.x, cache.y) <= 16 * 16)
        {
            self.caches[slot].alive = false;
            self.cargo = true;
            self.sfx |= SFX_MARK;
        }
    }

    pub(crate) fn ground_at(&self, tx: i32, ty: i32) -> u8 {
        for mark in &self.marks[..self.mark_len] {
            if mark.tx == tx && mark.ty == ty {
                return mark.tile;
            }
        }
        terrain(self.level, tx, ty)
    }

    pub(crate) fn is_quiet(&self, tx: i32, ty: i32) -> bool {
        self.marks[..self.mark_len]
            .iter()
            .any(|mark| mark.quiet && mark.tx == tx && mark.ty == ty)
    }

    pub(crate) fn evict_mark(&self) -> usize {
        let cx = tile_of(self.x);
        let cy = tile_of(self.y);
        let mut best = 0;
        let mut best_d = -1;
        let mut any_best = 0;
        let mut any_d = -1;
        for (index, mark) in self.marks[..self.mark_len].iter().enumerate() {
            let distance = (mark.tx - cx).abs() + (mark.ty - cy).abs();
            if !mark.quiet && distance > best_d {
                best_d = distance;
                best = index;
            }
            if distance > any_d {
                any_d = distance;
                any_best = index;
            }
        }
        if best_d >= 0 { best } else { any_best }
    }

    pub(crate) fn insert_mark(&mut self, tx: i32, ty: i32, tile: u8, quiet: bool) {
        if let Some(mark) = self.marks[..self.mark_len]
            .iter_mut()
            .find(|mark| mark.tx == tx && mark.ty == ty)
        {
            mark.tile = tile;
            mark.quiet |= quiet;
            return;
        }
        let (slot, restore) = if self.mark_len < MAX_MARKS {
            let slot = self.mark_len;
            self.mark_len += 1;
            (slot, None)
        } else {
            let slot = self.evict_mark();
            (slot, Some((self.marks[slot].tx, self.marks[slot].ty)))
        };
        self.marks[slot] = Mark {
            tx,
            ty,
            tile,
            quiet,
            used: true,
        };
        // The displaced scar must leave the map. Only if that cell is still
        // the one stored in the ring — a cell 128 tiles away shares the slot.
        if let Some((ox, oy)) = restore {
            if (ox - self.view_tx).abs() <= SPAN && (oy - self.view_ty).abs() <= SPAN {
                self.blit(ox, oy);
            }
        }
    }

    pub(crate) fn mark_quiet(&mut self, tx: i32, ty: i32) {
        if let Some(mark) = self.marks[..self.mark_len]
            .iter_mut()
            .find(|mark| mark.tx == tx && mark.ty == ty)
        {
            mark.quiet = true;
            return;
        }
        let tile = terrain(self.level, tx, ty);
        self.insert_mark(tx, ty, tile, true);
    }

    pub(crate) fn blit(&self, tx: i32, ty: i32) {
        let hx = tx.rem_euclid(VIEW) as usize;
        let hy = ty.rem_euclid(VIEW) as usize;
        // Park loaded tileset slot 0 into hardware index 1, and so on.
        let hw = self.ground_at(tx, ty) + 1;
        unsafe {
            (0x0600_C000 as *mut u8)
                .add(hy * 128 + hx)
                .write_volatile(hw);
        }
    }

    /// Keep every ground tile in VRAM. The 64×64 loader is not what is shown.
    pub(crate) fn park(&mut self, bg: &mut AffineBackground) {
        let last = (T_FOG as u16 + 7).max(T_MINE as u16);
        for tile in (T_SKY as u16)..=last {
            bg.set_tile((tile as i32, 0), &backgrounds::TERRAIN.tiles, tile);
        }
    }

    /// The window is thousands of tiles. `pump` keeps the music buffer full
    /// while that write runs, so a mission change does not replay a slice.
    pub(crate) fn refill(&mut self, pump: &mut dyn FnMut()) {
        let tx = tile_of(self.x);
        let ty = tile_of(self.y);
        self.view_tx = tx;
        self.view_ty = ty;
        pump();
        let mut rows = 0u32;
        for y in (ty - SPAN)..=(ty + SPAN) {
            for x in (tx - SPAN)..=(tx + SPAN) {
                self.blit(x, y);
            }
            rows += 1;
            if rows & 1 == 0 {
                pump();
            }
        }
    }

    /// Slide the window by the columns and rows the plane just entered.
    pub(crate) fn sync_ground(&mut self, pump: &mut dyn FnMut()) {
        let tx = tile_of(self.x);
        let ty = tile_of(self.y);
        if (tx - self.view_tx).abs() > 8 || (ty - self.view_ty).abs() > 8 {
            self.refill(pump);
            return;
        }
        while self.view_tx != tx {
            let dir = if tx > self.view_tx { 1 } else { -1 };
            self.view_tx += dir;
            let col = self.view_tx + dir * SPAN;
            for y in (self.view_ty - SPAN)..=(self.view_ty + SPAN) {
                self.blit(col, y);
            }
        }
        while self.view_ty != ty {
            let dir = if ty > self.view_ty { 1 } else { -1 };
            self.view_ty += dir;
            let row = self.view_ty + dir * SPAN;
            for x in (self.view_tx - SPAN)..=(self.view_tx + SPAN) {
                self.blit(x, row);
            }
        }
    }

    pub(crate) fn occupied(&self, tx: i32, ty: i32) -> bool {
        self.soldiers
            .iter()
            .any(|soldier| soldier.alive && soldier.home_tx == tx && soldier.home_ty == ty)
            || self
                .trucks
                .iter()
                .any(|truck| truck.alive && truck.home_tx == tx && truck.home_ty == ty)
            || self
                .tanks
                .iter()
                .any(|tank| tank.alive && tank.home_tx == tx && tank.home_ty == ty)
            || self
                .bunkers
                .iter()
                .any(|bunker| bunker.alive && bunker.home_tx == tx && bunker.home_ty == ty)
    }

    pub(crate) fn try_spawn_soldier(&mut self, tx: i32, ty: i32) {
        if self.occupied(tx, ty) {
            return;
        }
        let Some(slot) = self.soldiers.iter_mut().find(|soldier| !soldier.alive) else {
            return;
        };
        *slot = Soldier {
            x: (tx * 8 + 4) << 8,
            y: (ty * 8 + 4) << 8,
            dir: if hash(tx, ty) & 1 == 0 { 80 } else { -80 },
            step: 0,
            home_tx: tx,
            home_ty: ty,
            officer: false,
            objective: false,
            hp: HP_MAN,
            cool: 50,
            alive: true,
        };
    }

    pub(crate) fn try_spawn_truck(&mut self, tx: i32, ty: i32, vertical: bool) {
        if self.occupied(tx, ty) {
            return;
        }
        let Some(slot) = self.trucks.iter_mut().find(|truck| !truck.alive) else {
            return;
        };
        *slot = Truck {
            x: (tx * 8 + 4) << 8,
            y: (ty * 8 + 4) << 8,
            dir: if hash(tx, ty) & 1 == 0 { 150 } else { -150 },
            vertical,
            home_tx: tx,
            home_ty: ty,
            goal: false,
            objective: false,
            hp: HP_TRUCK,
            cool: 40,
            alive: true,
        };
    }

    pub(crate) fn try_spawn_tank(&mut self, tx: i32, ty: i32, vertical: bool) {
        if self.occupied(tx, ty) {
            return;
        }
        let Some(slot) = self.tanks.iter_mut().find(|tank| !tank.alive) else {
            return;
        };
        *slot = Tank {
            x: (tx * 8 + 4) << 8,
            y: (ty * 8 + 4) << 8,
            dir: if hash(tx, ty) & 1 == 0 { 120 } else { -120 },
            vertical,
            home_tx: tx,
            home_ty: ty,
            objective: false,
            hp: HP_TANK,
            cool: 30,
            alive: true,
        };
    }

    pub(crate) fn try_spawn_bunker(&mut self, tx: i32, ty: i32) {
        if self.occupied(tx, ty) {
            return;
        }
        let Some(slot) = self.bunkers.iter_mut().find(|bunker| !bunker.alive) else {
            return;
        };
        *slot = Bunker {
            x: (tx * 8 + 4) << 8,
            y: (ty * 8 + 4) << 8,
            home_tx: tx,
            home_ty: ty,
            objective: false,
            hp: HP_BUNKER,
            cool: 20,
            alive: true,
        };
    }

    pub(crate) fn consider_tile(&mut self, tx: i32, ty: i32) {
        let seeded = terrain(self.level, tx, ty);
        if seeded == T_TRENCH && hash(tx, ty) % 6 == 1 {
            if self.ground_at(tx, ty) == T_TRENCH && !self.is_quiet(tx, ty) {
                self.try_spawn_soldier(tx, ty);
            }
        } else if seeded == T_TRENCH && hash(tx, ty) % 11 == 2 && !self.is_quiet(tx, ty) {
            self.try_spawn_bunker(tx, ty);
        } else if seeded == T_ROAD && !self.is_quiet(tx, ty) {
            let lx = tx.rem_euclid(REGION);
            let ly = ty.rem_euclid(REGION);
            if ly == 0 && lx == 24 && hash(tx, ty) % 2 == 0 {
                self.try_spawn_truck(tx, ty, false);
            } else if lx == 0 && ly == 24 && hash(tx, ty) % 2 == 1 {
                self.try_spawn_truck(tx, ty, true);
            } else if ly == 0 && lx == 8 && hash(tx, ty) % 3 == 0 {
                self.try_spawn_tank(tx, ty, false);
            } else if lx == 0 && ly == 8 && hash(tx, ty) % 3 == 1 {
                self.try_spawn_tank(tx, ty, true);
            }
        }
    }

    pub(crate) fn spawn_nearby(&mut self, pump: &mut dyn FnMut()) {
        let cx = tile_of(self.x);
        let cy = tile_of(self.y);
        let mut rows = 0u32;
        for ty in cy - 40..=cy + 40 {
            for tx in cx - 40..=cx + 40 {
                self.consider_tile(tx, ty);
            }
            rows += 1;
            if rows & 1 == 0 {
                pump();
            }
        }
    }

    pub(crate) fn retire_far(&mut self) {
        let cx = tile_of(self.x);
        let cy = tile_of(self.y);
        for soldier in &mut self.soldiers {
            if !soldier.alive {
                continue;
            }
            let tx = tile_of(soldier.x);
            let ty = tile_of(soldier.y);
            if soldier.officer || soldier.objective {
                continue;
            }
            if (tx - cx).abs() > 70 || (ty - cy).abs() > 70 {
                soldier.alive = false;
            }
        }
        for truck in &mut self.trucks {
            if !truck.alive {
                continue;
            }
            let tx = tile_of(truck.x);
            let ty = tile_of(truck.y);
            if truck.goal || truck.objective {
                continue;
            }
            if (tx - cx).abs() > 70 || (ty - cy).abs() > 70 {
                truck.alive = false;
            }
        }
        for tank in &mut self.tanks {
            if !tank.alive {
                continue;
            }
            let tx = tile_of(tank.x);
            let ty = tile_of(tank.y);
            if tank.objective {
                continue;
            }
            if (tx - cx).abs() > 70 || (ty - cy).abs() > 70 {
                tank.alive = false;
            }
        }
        for bunker in &mut self.bunkers {
            if !bunker.alive {
                continue;
            }
            let tx = tile_of(bunker.x);
            let ty = tile_of(bunker.y);
            if bunker.objective {
                continue;
            }
            if (tx - cx).abs() > 70 || (ty - cy).abs() > 70 {
                bunker.alive = false;
            }
        }
    }

    pub(crate) fn maintain_actors(&mut self) {
        self.retire_far();
        // One row a frame: the whole neighbourhood refreshes about once a second
        // without stalling the picture.
        let cx = tile_of(self.x);
        let cy = tile_of(self.y);
        let row = cy - 40 + self.spawn_row as i32;
        self.spawn_row = self.spawn_row.wrapping_add(1) % 81;
        for tx in cx - 40..=cx + 40 {
            self.consider_tile(tx, row);
        }
    }

    pub(crate) fn heading(&self) -> (i32, i32) {
        (self.yaw.sin().to_raw(), self.yaw.cos().to_raw())
    }

    pub(crate) fn horizon(&self) -> i32 {
        // Pitch is in turns (1.0 = full circle). A dive lifts the horizon.
        let pitch_px = (self.pitch * Num::new(220)).to_raw() >> 8;
        let alt_px = self.alt >> 8;
        // Under six the ground rises in the glass, so a low pass looks low
        // and there is still air to hold before the hit.
        let skim = (6 - alt_px).max(0) * 8;
        let level = 58 - pitch_px - skim;
        // Past a shallow nose-down the vanishing point leaves the screen, so a
        // dive shows the ground ahead and not the skyline or the far map.
        if self.pitch <= num!(0.05) {
            return level.clamp(12, 108);
        }
        let alt_px = (self.alt >> 8).max(1);
        let skip = (alt_px * FLOOR_ZOOM * 25 / 48).max(12);
        let buried = -skip;
        let start = 13; // 0.05 turn
        let span = 26; // the next 0.10 turn buries the horizon
        let mix = ((self.pitch.to_raw() - start) * 256 / span).clamp(0, 256);
        (level + (buried - level) * mix / 256).clamp(-140, 108)
    }

    pub(crate) fn fly(&mut self, input: &ButtonController, bg: &mut AffineBackground) {
        if self.bloom > 0 {
            self.bloom -= 1;
        }
        if self.recover > 0 {
            self.recover -= 1;
            if self.recover == 0 {
                self.relaunch();
            }
            self.update_booms();
            return;
        }
        if !self.fly_plane(input, bg) {
            return;
        }

        if matches!(task_of(self.level), Task::Fetch) {
            self.fetch_tick(input);
        } else if self.mode != Mode::Gate
            && self.mode != Mode::Fpv
            && self.mode != Mode::Guide
            && self.signal_loss() < 16
        {
            if input.is_just_pressed(Button::A) && !matches!(task_of(self.level), Task::Swarm) {
                self.arm = (self.arm + 1) % 5;
            }
            if self.fire_cd > 0 {
                self.fire_cd -= 1;
            }
            let arm = self.arm;
            let shoot = input.is_pressed(Button::B) && self.fire_cd == 0;
            let once = input.is_just_pressed(Button::B);
            if arm == ARM_GUN && shoot && self.try_gun() {
                self.fire_cd = 14;
            } else if arm == ARM_BOMB && shoot && self.bombs > 0 && self.try_shot(KIND_BOMB) {
                self.bombs -= 1;
                self.fire_cd = 16;
            } else if arm == ARM_HE && once && self.he > 0 && self.try_shot(KIND_HE) {
                self.he -= 1;
            } else if arm == ARM_MINE && once && self.mines > 0 && self.try_shot(KIND_MINE) {
                self.mines -= 1;
            } else if arm == ARM_VOG && once && self.vog > 0 && self.try_shot(KIND_VOG) {
                self.vog -= 1;
            }
        }
        if self.mode == Mode::Guide {
            self.guide_tick(input, bg);
        }

        self.update_shots(bg);
        self.update_trucks();
        self.update_tanks();
        self.check_convoy();
        self.check_cover();
        self.check_rush();
        self.update_soldiers();
        self.update_foes();
        self.check_base();
        self.enemy_fire();
        self.aa_tick();
        self.update_booms();
        self.wear();
    }

    /// B under the crosshair calls the salvo. The count runs, then the rounds land.
    pub(crate) fn guide_tick(&mut self, input: &ButtonController, bg: &mut AffineBackground) {
        if self.guide == 0
            && self.salvo == 0
            && self.signal_loss() < 16
            && !self.falling
            && input.is_just_pressed(Button::B)
        {
            let (sin_r, cos_r) = self.heading();
            let (nx, ny, _) = self.nose_ground(sin_r, cos_r);
            self.mark_x = nx;
            self.mark_y = ny;
            self.lay_ground_frame(nx, ny);
            self.guide = GUIDE_WAIT;
            self.sfx |= SFX_MARK;
        }
        if self.guide > 0 {
            self.guide -= 1;
            if self.guide == 0 {
                self.salvo = 6;
                self.salvo_cd = 0;
            }
        }
        if self.salvo > 0 {
            if self.salvo_cd > 0 {
                self.salvo_cd -= 1;
            } else {
                let n = (6 - self.salvo) as i32;
                let jitter_x = if n == 0 {
                    0
                } else {
                    ((hash(self.age, n) % 33) as i32 - 16) << 8
                };
                let jitter_y = if n == 0 {
                    0
                } else {
                    ((hash(self.age, n + 5) % 33) as i32 - 16) << 8
                };
                self.explode(bg, self.mark_x + jitter_x, self.mark_y + jitter_y, 22, 0, KIND_BOMB);
                self.salvo -= 1;
                self.salvo_cd = 8;
            }
        }
        if self.guide == 0 && self.salvo == 0 && self.signal_loss() < 16 && !self.falling {
            let (sin_r, cos_r) = self.heading();
            let (nx, ny, _) = self.nose_ground(sin_r, cos_r);
            self.lay_ground_frame(nx, ny);
        }
    }

    /// A square of lip tiles on the map, around the ground under the crosshair.
    /// It stays in the world when the glass moves. Camera brackets do not.
    pub(crate) fn lay_ground_frame(&mut self, x: i32, y: i32) {
        let cx = tile_of(x);
        let cy = tile_of(y);
        if self.guide_frame_n > 0 && self.guide_cx == cx && self.guide_cy == cy {
            return;
        }
        self.clear_ground_frame();
        self.guide_cx = cx;
        self.guide_cy = cy;
        let mut n = 0;
        for dy in -2i32..=2 {
            for dx in -2i32..=2 {
                if dx.abs() != 2 && dy.abs() != 2 {
                    continue;
                }
                let tx = cx + dx;
                let ty = cy + dy;
                if self.ground_at(tx, ty) == T_WATER {
                    continue;
                }
                let was = self.ground_at(tx, ty);
                self.guide_frame[n] = (tx, ty, was);
                n += 1;
                if was != T_LIP {
                    self.insert_mark(tx, ty, T_LIP, false);
                    self.blit(tx, ty);
                }
            }
        }
        self.guide_frame_n = n as u8;
    }

    pub(crate) fn clear_ground_frame(&mut self) {
        let n = self.guide_frame_n as usize;
        self.guide_frame_n = 0;
        self.guide_cx = i32::MIN;
        self.guide_cy = i32::MIN;
        for step in 0..n {
            let (tx, ty, was) = self.guide_frame[step];
            if self.ground_at(tx, ty) == T_LIP && was != T_LIP {
                self.insert_mark(tx, ty, was, false);
                self.blit(tx, ty);
            }
        }
    }

    /// The stick slides the quad. Looking down does not change that speed.
    /// Holding a direction speeds it up. A tap stays slow enough for a hatch.
    /// Forward glass: up and down are height, left and right turn.
    pub(crate) fn slide_motion(
        &mut self,
        sin_r: i32,
        cos_r: i32,
        pitch_in: Tri,
        yaw_in: Tri,
        fetch_fly: bool,
        turning: bool,
    ) {
        let slide_pitch = if turning { Tri::Zero } else { pitch_in };
        let slide_yaw = if turning { Tri::Zero } else { yaw_in };
        let creep = if fetch_fly {
            self.quad_creep(slide_pitch, slide_yaw)
        } else {
            48
        };
        let along = -tri_value(slide_pitch) * creep;
        let strafe = tri_value(slide_yaw) * creep;
        self.vx = (sin_r * along + cos_r * strafe) >> 8;
        self.vy = ((-cos_r) * along + sin_r * strafe) >> 8;
        self.x = wrap_coord(self.x + self.vx);
        self.y = wrap_coord(self.y + self.vy);
        if !turning {
            self.bank += (Num::new(0) - self.bank) / Num::new(6);
        }
        // The Fetch quad sits on a round. Only the forward glass climbs and descends.
        if fetch_fly && !self.hover {
            self.alt += -tri_value(pitch_in) * 48;
            if self.alt < 0 {
                self.alt = 0;
            }
        }
    }

    /// A dead battery is not a wing. The quad drops where it is.
    pub(crate) fn drop_motion(&mut self) {
        self.vx = (self.vx * 3) / 4;
        self.vy = (self.vy * 3) / 4;
        self.x = wrap_coord(self.x + self.vx);
        self.y = wrap_coord(self.y + self.vy);
        self.alt -= 96;
        if self.alt < 0 {
            self.alt = 0;
        }
    }

    /// Always flies forward. The stick has already tilted the nose.
    pub(crate) fn cruise_motion(&mut self, sin_r: i32, cos_r: i32) {
        let sp = self.pitch.sin().to_raw();
        let cp = self.pitch.cos().to_raw().max(0);
        // The spotter holds one height and loiters. Pitch only aims the glass.
        // Slower than a sortie, fast enough that the nose does not sit on a crow.
        let cruise = if self.mode == Mode::Guide {
            48
        } else if defend_card(self.level) {
            200
        } else {
            400
        };
        let horiz = if self.mode == Mode::Guide {
            cruise
        } else {
            (cruise * cp) >> 8
        };
        self.vx = (sin_r * horiz) >> 8;
        self.vy = -((cos_r * horiz) >> 8);
        self.x = wrap_coord(self.x + self.vx);
        self.y = wrap_coord(self.y + self.vy);
        // 110 subpixels per frame at a vertical nose. Shallower angles sink less.
        // The spotter's stick tilts the camera and does not climb or dive.
        if self.mode != Mode::Guide || self.falling {
            self.alt -= (110 * sp) >> 8;
        }
    }

    /// Always flies forward. The stick tilts the nose; height and speed follow
    /// that angle, and the nose stays where you leave it.
    /// Kamikaze spends one drone and returns to the pad instead of ending the mission.
    pub(crate) fn fly_plane(&mut self, input: &ButtonController, bg: &mut AffineBackground) -> bool {
        self.age = self.age.wrapping_add(1);
        // Past the radio the stick drops frames, then drops out.
        let loss = self.signal_loss();
        // The stick only flutters in the last part of the radio, and dies at the edge.
        let mute = loss >= 16
            || (loss >= 13 && (hash(self.age, 3) % 16) < (loss as u32 - 12));
        let pitch_in = if mute || self.falling {
            Tri::Zero
        } else {
            input.y_tri()
        };
        let yaw_in = if mute || self.falling {
            Tri::Zero
        } else {
            input.x_tri()
        };
        // A dead link or a fall drops the hover. Select on the quad switches it.
        let fetch = matches!(task_of(self.level), Task::Fetch);
        if self.mode != Mode::Quad || self.falling || loss >= 16 {
            self.hover = false;
        } else if input.is_just_pressed(Button::Select) {
            // Select swaps the glass. On the mine quad the forward glass also climbs.
            self.hover = !self.hover;
            self.sfx |= SFX_MARK;
        }

        // About a second from level to a steep dive. Releasing the stick does
        // not snap the nose back. Hover keeps the nose so leaving it is the same attitude.
        let fetch_fly = fetch && !self.falling && loss < 16;
        if !self.hover && !fetch_fly {
            self.pitch += Num::new(tri_value(pitch_in)) / Num::new(240);
        }
        if self.falling {
            self.pitch += Num::new(1) / Num::new(60);
        }
        if self.pitch > num!(0.20) {
            self.pitch = num!(0.20);
        }
        if self.pitch < num!(-0.12) {
            self.pitch = num!(-0.12);
        }
        // Overhead slides. The forward glass of the mine quad yaws like the others.
        let turning = fetch_fly && !self.hover;
        let yaw_step = if (self.hover || fetch_fly) && !turning {
            Num::new(0)
        } else {
            Num::new(tri_value(yaw_in)) / Num::new(160)
        };
        self.yaw += yaw_step;
        wrap_yaw(&mut self.yaw);
        // The airframe does not roll. The indicator leans into the turn and
        // settles back when the stick is released.
        if !self.hover && (!fetch_fly || turning) {
            let want = yaw_step * Num::new(14);
            self.bank += (want - self.bank) / Num::new(6);
            if self.bank > num!(0.06) {
                self.bank = num!(0.06);
            }
            if self.bank < num!(-0.06) {
                self.bank = num!(-0.06);
            }
        }

        let (sin_r, cos_r) = self.heading();
        if self.hover || fetch_fly {
            self.slide_motion(sin_r, cos_r, pitch_in, yaw_in, fetch_fly, turning);
        } else if fetch && (self.falling || loss >= 16) {
            self.drop_motion();
        } else {
            self.cruise_motion(sin_r, cos_r);
        }
        if self.alt > 40 << 8 {
            self.alt = 40 << 8;
        }
        if !self.cross_gate() {
            return false;
        }

        // Contact is the airframe itself, a few pixels out, so you can fly
        // up to a soldier before the charge goes.
        let touching = self.alt <= 4 << 8 && self.point_blank();
        // A live Fetch quad sits on a round. A dead battery still hits the ground.
        let dug_in = self.alt <= 0 && !(fetch && !self.falling && loss < 16);
        if touching || dug_in {
            if dug_in {
                self.alt = 0;
            }
            if self.mode == Mode::Fpv {
                self.explode(bg, self.x, self.y, 48, 0, KIND_BOMB);
            } else if touching {
                let _ = self.strike_at(bg, self.x, self.y);
            } else {
                self.add_boom(self.x, self.y, 0);
            }
            self.falling = false;
            self.vx = 0;
            self.vy = 0;
            // An empty battery ends the sortie. A crash from the radio still returns to the pad.
            if self.battery == 0 {
                self.fail(Why::Cell);
            } else {
                self.recover = 22;
            }
            return true;
        }
        true
    }

    /// True when the airframe is on top of a soldier or a truck.
    pub(crate) fn point_blank(&self) -> bool {
        let reach = 14 * 14;
        self.soldiers.iter().any(|soldier| {
            soldier.alive && dist2_px(self.x, self.y, soldier.x, soldier.y) <= reach
        }) || self.trucks.iter().any(|truck| {
            truck.alive && dist2_px(self.x, self.y, truck.x, truck.y) <= reach
        })
    }

    /// Where the nose meets the ground. Level flight looks far ahead; a dive
    /// pulls that point in toward the plane.
    pub(crate) fn nose(&self, sin_r: i32, cos_r: i32) -> (i32, i32) {
        let sp = self.pitch.sin().to_raw();
        let cp = self.pitch.cos().to_raw().max(32);
        let alt_px = self.alt >> 8;
        let lead = if sp > 12 {
            (alt_px * cp / sp).clamp(6, 180)
        } else {
            (alt_px * 4).clamp(24, 180)
        };
        (
            wrap_coord(self.x + sin_r * lead),
            wrap_coord(self.y - cos_r * lead),
        )
    }

    /// Whoever stands on the nose point is the kamikaze target.
    pub(crate) fn strike_at(&mut self, bg: &mut AffineBackground, hx: i32, hy: i32) -> bool {
        let reach_man = 28 * 28;
        let reach_truck = 32 * 32;
        let mut man: Option<usize> = None;
        let mut man_d = reach_man + 1;
        for (index, soldier) in self.soldiers.iter().enumerate() {
            if !soldier.alive {
                continue;
            }
            let distance = dist2_px(hx, hy, soldier.x, soldier.y);
            if distance <= reach_man && distance < man_d {
                man = Some(index);
                man_d = distance;
            }
        }
        if let Some(index) = man {
            let x = self.soldiers[index].x;
            let y = self.soldiers[index].y;
            let hx = self.soldiers[index].home_tx;
            let hy = self.soldiers[index].home_ty;
            let officer = self.soldiers[index].officer;
            let objective = self.soldiers[index].objective;
            self.soldiers[index].alive = false;
            self.mark_quiet(hx, hy);
            self.note_kill(
                bg,
                x,
                y,
                if officer { 200 } else { 50 },
                officer,
                false,
                false,
                objective,
            );
            return true;
        }
        let mut truck_hit: Option<(usize, i32, i32)> = None;
        for (index, truck) in self.trucks.iter().enumerate() {
            if truck.alive && dist2_px(hx, hy, truck.x, truck.y) <= reach_truck {
                truck_hit = Some((index, truck.x, truck.y));
                break;
            }
        }
        if let Some((index, x, y)) = truck_hit {
            let hx = self.trucks[index].home_tx;
            let hy = self.trucks[index].home_ty;
            let goal = self.trucks[index].goal;
            let objective = self.trucks[index].objective;
            self.trucks[index].alive = false;
            self.mark_quiet(hx, hy);
            self.note_kill(
                bg,
                x,
                y,
                if goal { 200 } else { 100 },
                false,
                !goal,
                goal,
                objective,
            );
            return true;
        }
        false
    }

    pub(crate) fn note_kill(
        &mut self,
        bg: &mut AffineBackground,
        x: i32,
        y: i32,
        points: i32,
        officer: bool,
        truck: bool,
        goal: bool,
        objective: bool,
    ) {
        self.add_boom(x, y, 0);
        self.score += points;
        self.claim(officer, truck, goal, objective);
        let tx = tile_of(x);
        let ty = tile_of(y);
        if self.ground_at(tx, ty) != T_WATER {
            self.paint(bg, tx, ty, T_CRATER);
        }
    }

    pub(crate) fn relaunch(&mut self) -> bool {
        if self.targets <= 0 {
            return true;
        }
        if self.mode == Mode::Fpv {
            self.drones -= 1;
            if self.drones <= 0 {
                self.again = true;
                return false;
            }
        }
        self.x = self.pad_x;
        self.y = self.pad_y;
        self.alt = self.pad_alt;
        self.vx = 0;
        self.vy = 0;
        self.pitch = num!(0);
        self.bank = num!(0);
        self.hull = 100;
        self.battery = 100;
        self.blackout = 0;
        self.falling = false;
        true
    }

    /// Pixels from the pad, cheap distance. The radio is a disc around the pad,
    /// so the generated plain can go on without a flight ever using the far part.
    pub(crate) fn signal_loss(&self) -> i32 {
        let dx = ((self.x - self.pad_x) >> 8).abs();
        let dy = ((self.y - self.pad_y) >> 8).abs();
        let dist = dx.max(dy) + dx.min(dy) / 2;
        // Clear well past the objectives. Snow, then a dead radio, much further out.
        let near = 64 * 8;
        let far = 160 * 8;
        if dist <= near {
            return 0;
        }
        ((dist - near) * 16 / (far - near)).min(16)
    }

    pub(crate) fn on_charger(&self) -> bool {
        if !matches!(task_of(self.level), Task::Fetch) || self.falling || (self.alt >> 8) > 1 {
            return false;
        }
        let tx = tile_of(self.x);
        let ty = tile_of(self.y);
        charge_spots(self.level)
            .iter()
            .any(|&(cx, cy)| (tx - cx).abs() <= 1 && (ty - cy).abs() <= 1)
    }

    pub(crate) fn wear(&mut self) {
        let span = 64 - heat(self.level);
        if self.on_charger() {
            if self.battery < 100 && self.age % 3 == 0 {
                self.battery += 1;
            }
            if self.battery < 100 && self.age % 24 == 0 {
                self.sfx |= SFX_MARK;
            }
        } else if self.age % span == 0 && self.battery > 0 {
            self.battery -= 1;
        }
        let loss = self.signal_loss();
        if !self.on_charger() && loss >= 14 && self.age % 40 == 0 && self.battery > 0 {
            self.battery -= 1;
        }
        if loss >= 16 {
            self.blackout += 1;
        } else if self.blackout > 0 {
            self.blackout -= 2;
            if self.blackout < 0 {
                self.blackout = 0;
            }
        }
        for boom in &self.booms {
            if !boom.alive || boom.frame != 1 {
                continue;
            }
            let distance = dist2_px(self.x, self.y, boom.x, boom.y);
            // The burst is on the ground. Cruise height is above it, so a
            // drop under a hover does not wreck the airframe. A low pass does.
            let dz = ((self.alt - boom.z) >> 8).abs();
            if dz > 8 {
                continue;
            }
            if distance < 20 * 20 {
                self.hull -= 40;
            } else if distance < 36 * 36 {
                self.hull -= 16;
            }
        }
        if self.hull < 0 {
            self.hull = 0;
        }
        if self.hull == 0 && self.recover == 0 {
            if self.mode == Mode::Fpv {
                self.recover = 22;
                self.vx = 0;
                self.vy = 0;
            } else {
                self.fail(Why::Hull);
            }
            return;
        }
        // A dead radio or a dead battery drops the nose. The airframe falls
        // until it hits. An empty battery ends the sortie there.
        let was_falling = self.falling;
        let loss = self.signal_loss();
        if self.recover == 0 && (loss >= 16 || self.battery == 0) {
            self.falling = true;
        }
        if self.falling && !was_falling {
            self.sfx |= SFX_FALL;
        }
        // The feed crackles as the radio thins out.
        if loss >= 6 {
            let gap = (20 - loss).clamp(4, 14) * 8;
            if self.age % gap == 0 {
                self.sfx |= SFX_LINK;
            }
        }
    }

    pub(crate) fn try_shot(&mut self, kind: u8) -> bool {
        // One release. The round leaves the airframe with the airframe's
        // speed. The glass does not move the spawn.
        let Some(slot) = self.shots.iter_mut().find(|shot| !shot.alive) else {
            return false;
        };
        *slot = Shot {
            x: self.x,
            y: self.y,
            z: self.alt,
            vx: self.vx,
            vy: self.vy,
            vz: -((110 * self.pitch.sin().to_raw()) >> 8),
            kind,
            life: 0,
            alive: true,
        };
        if kind == KIND_BOMB {
            self.sfx |= SFX_WHISTLE;
        }
        true
    }

    /// Where the nose meets the ground. Pitch and height decide it. The glass
    /// is drawn so this point sits on the crosshair, and the overhead picture
    /// draws the crosshair on this point.
    pub(crate) fn nose_ground(&self, sin_r: i32, cos_r: i32) -> (i32, i32, i32) {
        let view_cy = VIEW_Y + VIEW_H / 2;
        let drop = (view_cy - self.horizon()).max(4);
        let fwd = (self.alt as i64) * (FOCAL as i64) * (FLOOR_ZOOM as i64) / drop as i64;
        // Ninety tiles. That covers a group by the posts and the ground ahead of it.
        let lead = (fwd >> 8).clamp(8, 720) as i32;
        (
            wrap_coord(self.x + sin_r * lead),
            wrap_coord(self.y - cos_r * lead),
            0,
        )
    }

    /// The drone in the glass, if the crosshair is on its picture.
    /// One above the airframe is not drawn, so it cannot be the target either.
    pub(crate) fn foe_under_sight(&self) -> Option<usize> {
        let (sin_r, cos_r) = self.heading();
        let horizon = self.horizon();
        let alt_px = self.alt >> 8;
        let aim_y = VIEW_Y + VIEW_H / 2;
        let mut best: Option<(usize, i32)> = None;
        for (index, foe) in self.foes.iter().enumerate() {
            if !foe.alive || (foe.z >> 8) + 2 >= alt_px {
                continue;
            }
            let Some((sx, sy)) = project(
                self.x,
                self.y,
                self.alt,
                sin_r,
                cos_r,
                horizon,
                foe.x,
                foe.y,
                foe.z,
            ) else {
                continue;
            };
            let dx = sx - WIDTH / 2;
            let dy = sy - aim_y;
            let dist = dx * dx + dy * dy;
            // On the picture, not a wide halo around it.
            if dist > 6 * 6 {
                continue;
            }
            if best.is_none_or(|(_, near)| dist < near) {
                best = Some((index, dist));
            }
        }
        best.map(|(index, _)| index)
    }

    /// A burst along the nose to the ground. It hurts people and trucks, not the ground.
    /// A hostile quad in the crosshair is the target instead, so the round meets that picture.
    pub(crate) fn try_gun(&mut self) -> bool {
        let (sin_r, cos_r) = self.heading();
        let (nx, ny, nz) = if let Some(index) = self.foe_under_sight() {
            let foe = &self.foes[index];
            (foe.x, foe.y, foe.z)
        } else {
            self.nose_ground(sin_r, cos_r)
        };
        let span = (nx as i64 - self.x as i64)
            .abs()
            .max((ny as i64 - self.y as i64).abs())
            >> 8;
        let steps = (span / 6).clamp(4, 18) as i32;
        let Some(slot) = self.shots.iter_mut().find(|shot| !shot.alive) else {
            return false;
        };
        *slot = Shot {
            x: self.x,
            y: self.y,
            z: self.alt,
            vx: (nx - self.x) / steps,
            vy: (ny - self.y) / steps,
            vz: (nz - self.alt) / steps,
            kind: KIND_GUN,
            life: 0,
            alive: true,
        };
        self.sfx |= SFX_GUN;
        true
    }

    /// A round at the aircraft, not at the crosshair. `slope` is the steepest
    /// the barrel will lift (rise/run in eighths). Past that the shell keeps
    /// the low arc and passes under the machine.
    pub(crate) fn launch_at_player(&mut self, x: i32, y: i32, kind: u8, slope: i32) -> bool {
        let Some(slot) = self.shots.iter_mut().find(|shot| !shot.alive) else {
            return false;
        };
        // A few pixels of scatter. Flak throws a wider burst: a machine that
        // holds still would otherwise sit inside every shell, because the
        // round is aimed at where it is now and does not lead.
        let spread: u32 = if kind == KIND_FLAK { 41 } else { 9 };
        let half = (spread as i32) / 2;
        let scatter_x = (hash(self.age, x) % spread) as i32 - half;
        let scatter_y = (hash(self.age.wrapping_add(9), y) % spread) as i32 - half;
        let tx = self.x.wrapping_add(scatter_x << 8);
        let ty = self.y.wrapping_add(scatter_y << 8);
        let dx = delta(tx, x);
        let dy = delta(ty, y);
        let adx = (dx >> 8).abs();
        let ady = (dy >> 8).abs();
        let dist = adx.max(ady) + adx.min(ady) / 2;
        let muzzle = 1 << 8;
        let rise = self.alt - muzzle;
        // A flat barrel cannot crane over its own hull. The old ten-pixel
        // pin was smaller than the hatch, so hovering to aim ate the shells.
        // Flak still points up.
        let overhead = if slope < 8 { dist < TANK_NEAR } else { dist < 10 };
        let steps = if overhead {
            if slope >= 8 { 16 } else { 10 }
        } else {
            (dist / 3).clamp(8, 26)
        };
        let vx = dx / steps;
        let vy = dy / steps;
        let horiz = (vx.abs().max(vy.abs()) + vx.abs().min(vy.abs()) / 2).max(1);
        let mut vz = rise / steps + GRAVITY * (steps + 1) / 2;
        let cap = if overhead && slope < 8 {
            48
        } else if overhead {
            vz.max(48)
        } else {
            (horiz * slope / 8).max(48)
        };
        if vz > cap {
            vz = cap;
        }
        if vz < 0 {
            vz = 0;
        }
        *slot = Shot {
            x,
            y,
            z: muzzle,
            vx,
            vy,
            vz,
            kind,
            life: 0,
            alive: true,
        };
        true
    }

    /// Under `AA_HIDE` a battery sees the drone inside its circle. At that
    /// height and above, the drone is gone and the guns do not fire.
    pub(crate) fn aa_tick(&mut self) {
        let guns = aa_guns(task_of(self.level));
        if guns.is_empty() || self.recover > 0 {
            return;
        }
        if !guns_see(self.alt >> 8) {
            return;
        }
        if !aa_covers(guns, self.x, self.y) {
            return;
        }
        let heat = heat(self.level);
        let chip = 1 + heat / 10;
        // The circle burns while you stay in it. Twice as slow as the old
        // tick, so crossing it and stopping to aim are not the same death.
        if self.age % ((44 - heat) * 2) == 0 {
            self.hull -= chip;
            if self.hull < 0 {
                self.hull = 0;
            }
        }
        if self.age % (52 - heat) == 0 {
            let mut best = 0;
            let mut best_d = i32::MAX;
            for (index, gun) in guns.iter().enumerate() {
                let (gx, gy) = gun_at(gun);
                let distance = dist2_px(self.x, self.y, gx, gy);
                if distance < best_d {
                    best_d = distance;
                    best = index;
                }
            }
            let (gx, gy) = gun_at(&guns[best]);
            let _ = self.launch_at_player(gx, gy, KIND_FLAK, FLAK_SLOPE);
        }
    }

    /// Whether this gunner commits. Low cards miss often, and a high pass is
    /// harder to hit than a deck-level one. The last card fires in range.
    pub(crate) fn will_shoot(&self, salt: i32, keen: bool) -> bool {
        let heat = heat(self.level);
        let roll = (hash(self.age, salt) % 12) as i32;
        let need = if keen {
            6 + heat * 6 / 16
        } else {
            2 + heat * 5 / 16
        };
        roll < need
    }

    pub(crate) fn enemy_fire(&mut self) {
        if self.mode == Mode::Gate {
            return;
        }
        let alt_px = self.alt >> 8;
        // Same line as the batteries. Above it the drone is not there to shoot.
        if !guns_see(alt_px) {
            return;
        }
        let keen = alt_px <= 7;
        let pause = (16 - heat(self.level)) * 6;
        let cool = |base: i32| (base + pause).clamp(1, 255) as u8;
        for index in 0..MAX_SOLDIERS {
            if !self.soldiers[index].alive {
                continue;
            }
            if self.soldiers[index].cool > 0 {
                self.soldiers[index].cool -= 1;
                continue;
            }
            let sx = self.soldiers[index].x;
            let sy = self.soldiers[index].y;
            if dist2_px(self.x, self.y, sx, sy) > 40 * 40 {
                continue;
            }
            if !self.will_shoot(index as i32, keen) {
                self.soldiers[index].cool = cool(50);
                continue;
            }
            self.soldiers[index].cool = cool(200);
            let _ = self.launch_at_player(sx, sy, KIND_FLAK, FLAK_SLOPE);
        }
        for index in 0..MAX_TRUCKS {
            if !self.trucks[index].alive {
                continue;
            }
            if self.trucks[index].cool > 0 {
                self.trucks[index].cool -= 1;
                continue;
            }
            let tx = self.trucks[index].x;
            let ty = self.trucks[index].y;
            if dist2_px(self.x, self.y, tx, ty) > 48 * 48 {
                continue;
            }
            if !self.will_shoot(index as i32 + 19, keen) {
                self.trucks[index].cool = cool(40);
                continue;
            }
            self.trucks[index].cool = cool(170);
            let _ = self.launch_at_player(tx, ty, KIND_SHELL, TANK_SLOPE);
        }
        for index in 0..MAX_TANKS {
            if !self.tanks[index].alive {
                continue;
            }
            if self.tanks[index].cool > 0 {
                self.tanks[index].cool -= 1;
                continue;
            }
            let tx = self.tanks[index].x;
            let ty = self.tanks[index].y;
            if dist2_px(self.x, self.y, tx, ty) > 56 * 56 {
                continue;
            }
            if !self.will_shoot(index as i32 + 31, keen) {
                self.tanks[index].cool = cool(40);
                continue;
            }
            self.tanks[index].cool = cool(150);
            let _ = self.launch_at_player(tx, ty, KIND_SHELL, TANK_SLOPE);
        }
        for index in 0..MAX_BUNKERS {
            if !self.bunkers[index].alive {
                continue;
            }
            if self.bunkers[index].cool > 0 {
                self.bunkers[index].cool -= 1;
                continue;
            }
            let bx = self.bunkers[index].x;
            let by = self.bunkers[index].y;
            if dist2_px(self.x, self.y, bx, by) > 64 * 64 {
                continue;
            }
            if !self.will_shoot(index as i32 + 47, keen) {
                self.bunkers[index].cool = cool(36);
                continue;
            }
            self.bunkers[index].cool = cool(110);
            let _ = self.launch_at_player(bx, by, KIND_FLAK, FLAK_SLOPE);
        }
    }

    pub(crate) fn step_gun(&mut self, bg: &mut AffineBackground, index: usize) {
        self.shots[index].x = wrap_coord(self.shots[index].x + self.shots[index].vx);
        self.shots[index].y = wrap_coord(self.shots[index].y + self.shots[index].vy);
        self.shots[index].z += self.shots[index].vz;
        self.shots[index].life += 1;
        let x = self.shots[index].x;
        let y = self.shots[index].y;
        let z = self.shots[index].z;
        let life = self.shots[index].life;
        if z > 16 << 8 && life < 10 {
            return;
        }
        let mut bird = None;
        for (slot, foe) in self.foes.iter().enumerate() {
            if foe.alive
                && dist2_px(x, y, foe.x, foe.y) <= 8 * 8
                && ((z - foe.z) >> 8).abs() <= 5
            {
                bird = Some(slot);
                break;
            }
        }
        if let Some(slot) = bird {
            self.shots[index].alive = false;
            self.hurt_foe(slot);
            return;
        }
        let mut man = None;
        for (slot, soldier) in self.soldiers.iter().enumerate() {
            if soldier.alive && dist2_px(x, y, soldier.x, soldier.y) <= 8 * 8 {
                man = Some(slot);
                break;
            }
        }
        if let Some(slot) = man {
            self.shots[index].alive = false;
            self.hurt_soldier(bg, slot);
            return;
        }
        let mut rig = None;
        for (slot, truck) in self.trucks.iter().enumerate() {
            if truck.alive && dist2_px(x, y, truck.x, truck.y) <= 10 * 10 {
                rig = Some(slot);
                break;
            }
        }
        if let Some(slot) = rig {
            self.shots[index].alive = false;
            self.hurt_truck(bg, slot);
            return;
        }
        let mut armour = None;
        for (slot, tank) in self.tanks.iter().enumerate() {
            if tank.alive && dist2_px(x, y, tank.x, tank.y) <= 12 * 12 {
                armour = Some(slot);
                break;
            }
        }
        if let Some(slot) = armour {
            self.shots[index].alive = false;
            self.hurt_tank(bg, slot);
            return;
        }
        let mut nest = None;
        for (slot, bunker) in self.bunkers.iter().enumerate() {
            if bunker.alive && dist2_px(x, y, bunker.x, bunker.y) <= 12 * 12 {
                nest = Some(slot);
                break;
            }
        }
        if let Some(slot) = nest {
            self.shots[index].alive = false;
            self.hurt_bunker(bg, slot);
            return;
        }
        if z <= 1 << 8 || life > 22 {
            let tx = tile_of(x);
            let ty = tile_of(y);
            if self.hq_goal(tx, ty) {
                self.paint(bg, tx, ty, T_CRATER);
            }
            self.shots[index].alive = false;
        }
    }

    pub(crate) fn hurt_foe(&mut self, index: usize) {
        let x = self.foes[index].x;
        let y = self.foes[index].y;
        let z = self.foes[index].z;
        self.foes[index].alive = false;
        self.add_boom(x, y, z);
        self.score += 50;
        self.sfx |= SFX_BOOM;
        if matches!(task_of(self.level), Task::Swarm | Task::Cover) && self.targets > 0 {
            self.targets -= 1;
            self.sfx |= SFX_MARK;
        }
    }

    /// Holds one height. Chases under `AA_HIDE` and goes home above it.
    /// The bullet is the only thing that brings one down. The same height is a bump.
    pub(crate) fn update_foes(&mut self) {
        let see = guns_see(self.alt >> 8) && !self.falling;
        let px = self.x;
        let py = self.y;
        let pz = self.alt;
        let defend = defend_card(self.level);
        let cover = matches!(task_of(self.level), Task::Cover);
        let escort = if cover {
            self.trucks
                .iter()
                .find(|truck| truck.alive && truck.goal)
                .map(|truck| (truck.x, truck.y))
        } else {
            None
        };
        let base_x = self.pad_x;
        let base_y = self.pad_y;
        let mut bump = false;
        for foe in &mut self.foes {
            if !foe.alive {
                continue;
            }
            if foe.cool > 0 {
                foe.cool -= 1;
            }
            // A wing keeps its cruise. It does not park on the truck or the player.
            if foe.wing {
                let speed = if defend {
                    40
                } else if cover {
                    80
                } else {
                    64
                };
                if defend {
                    let dx = base_x.wrapping_sub(foe.x);
                    let step_x = dx.signum() * 20;
                    foe.x = foe.x.wrapping_add(if step_x.abs() > dx.abs() { dx } else { step_x });
                    foe.y = foe.y.wrapping_add(speed);
                } else {
                    if foe.heading == 0 {
                        foe.heading = -1;
                    }
                    let ty = tile_of(foe.y);
                    if ty <= 8 {
                        foe.heading = 1;
                    } else if ty >= 58 {
                        foe.heading = -1;
                    }
                    foe.y = foe.y.wrapping_add(i32::from(foe.heading) * speed);
                    if see && !cover {
                        let dx = px.wrapping_sub(foe.x);
                        foe.x = foe.x.wrapping_add(dx.signum() * 16);
                    }
                }
            } else {
                // Crows fly at the pad. Cover rides above the truck. The swarm flies at you.
                let (tx, ty) = if defend {
                    (base_x, base_y)
                } else if let Some((ex, ey)) = escort {
                    let side = if foe.home_x < ex { -(24 << 8) } else { 24 << 8 };
                    (ex.wrapping_add(side), ey.wrapping_sub(20 << 8))
                } else if see {
                    (px, py)
                } else {
                    (foe.home_x, foe.home_y)
                };
                let dx = tx.wrapping_sub(foe.x) as i64;
                let dy = ty.wrapping_sub(foe.y) as i64;
                let dist = isqrt64(dx * dx + dy * dy);
                let step = if defend {
                    56
                } else if cover {
                    40
                } else {
                    72
                };
                if dist > 256 {
                    foe.x = foe.x.wrapping_add((dx * step / dist) as i32);
                    foe.y = foe.y.wrapping_add((dy * step / dist) as i32);
                }
            }
            let dz = ((foe.z - pz) >> 8).abs();
            if dz <= 2 && foe.cool == 0 && dist2_px(foe.x, foe.y, px, py) <= 12 * 12 {
                foe.cool = 45;
                let ox = foe.x.wrapping_sub(px) as i64;
                let oy = foe.y.wrapping_sub(py) as i64;
                let push = isqrt64(ox * ox + oy * oy).max(1);
                foe.x = px.wrapping_add((ox * (18 << 8) / push) as i32);
                foe.y = py.wrapping_add((oy * (18 << 8) / push) as i32);
                if ox == 0 && oy == 0 {
                    foe.y = py.wrapping_sub(18 << 8);
                }
                bump = true;
            }
        }
        if bump {
            self.hull = (self.hull - 16).max(0);
            self.sfx |= SFX_BOOM;
        }
    }

    pub(crate) fn hurt_soldier(&mut self, bg: &mut AffineBackground, index: usize) {
        if self.soldiers[index].hp > 0 {
            self.soldiers[index].hp -= 1;
        }
        if self.soldiers[index].hp > 0 {
            return;
        }
        let x = self.soldiers[index].x;
        let y = self.soldiers[index].y;
        let hx = self.soldiers[index].home_tx;
        let hy = self.soldiers[index].home_ty;
        let officer = self.soldiers[index].officer;
        let objective = self.soldiers[index].objective;
        self.soldiers[index].alive = false;
        self.mark_quiet(hx, hy);
        self.note_kill(
            bg,
            x,
            y,
            if officer { 200 } else { 50 },
            officer,
            false,
            false,
            objective,
        );
    }

    pub(crate) fn hurt_truck(&mut self, bg: &mut AffineBackground, index: usize) {
        if self.trucks[index].hp > 0 {
            self.trucks[index].hp -= 1;
        }
        if self.trucks[index].hp > 0 {
            return;
        }
        let x = self.trucks[index].x;
        let y = self.trucks[index].y;
        let hx = self.trucks[index].home_tx;
        let hy = self.trucks[index].home_ty;
        let goal = self.trucks[index].goal;
        let objective = self.trucks[index].objective;
        self.trucks[index].alive = false;
        self.mark_quiet(hx, hy);
        self.note_kill(
            bg,
            x,
            y,
            if goal { 200 } else { 100 },
            false,
            !goal,
            goal,
            objective,
        );
    }

    pub(crate) fn hurt_tank(&mut self, bg: &mut AffineBackground, index: usize) {
        if self.tanks[index].hp > 0 {
            self.tanks[index].hp -= 1;
        }
        if self.tanks[index].hp > 0 {
            return;
        }
        let x = self.tanks[index].x;
        let y = self.tanks[index].y;
        let hx = self.tanks[index].home_tx;
        let hy = self.tanks[index].home_ty;
        let objective = self.tanks[index].objective;
        self.tanks[index].alive = false;
        self.mark_quiet(hx, hy);
        if objective {
            self.claim_armor();
        }
        self.note_kill(bg, x, y, 150, false, false, false, false);
    }

    pub(crate) fn hurt_bunker(&mut self, bg: &mut AffineBackground, index: usize) {
        if self.bunkers[index].hp > 0 {
            self.bunkers[index].hp -= 1;
        }
        if self.bunkers[index].hp > 0 {
            return;
        }
        let x = self.bunkers[index].x;
        let y = self.bunkers[index].y;
        let hx = self.bunkers[index].home_tx;
        let hy = self.bunkers[index].home_ty;
        let objective = self.bunkers[index].objective;
        self.bunkers[index].alive = false;
        self.mark_quiet(hx, hy);
        if objective {
            self.claim_pill();
        }
        self.note_kill(bg, x, y, 100, false, false, false, false);
    }

    pub(crate) fn step_flak(&mut self, index: usize) {
        let kind = self.shots[index].kind;
        self.shots[index].vz -= GRAVITY;
        self.shots[index].x = wrap_coord(self.shots[index].x + self.shots[index].vx);
        self.shots[index].y = wrap_coord(self.shots[index].y + self.shots[index].vy);
        self.shots[index].z += self.shots[index].vz;
        self.shots[index].life += 1;
        let x = self.shots[index].x;
        let y = self.shots[index].y;
        let z = self.shots[index].z;
        let life = self.shots[index].life;
        let near = dist2_px(x, y, self.x, self.y) <= 7 * 7;
        let level = (z - self.alt).abs() <= 4 << 8;
        if near && level {
            self.shots[index].alive = false;
            let heat = heat(self.level);
            let hit = if kind == KIND_SHELL {
                4 + heat / 4
            } else {
                2 + heat / 8
            };
            self.hull -= hit;
            if self.hull < 0 {
                self.hull = 0;
            }
            self.add_boom(self.x, self.y, z);
            return;
        }
        if life > 28 || z > 48 << 8 || z < 0 {
            self.shots[index].alive = false;
        }
    }

    pub(crate) fn update_shots(&mut self, bg: &mut AffineBackground) {
        for index in 0..MAX_SHOTS {
            if !self.shots[index].alive {
                continue;
            }
            let kind = self.shots[index].kind;
            if kind == KIND_GUN {
                self.step_gun(bg, index);
                continue;
            }
            if kind == KIND_FLAK || kind == KIND_SHELL {
                self.step_flak(index);
                continue;
            }
            self.shots[index].vz -= GRAVITY;
            self.shots[index].z += self.shots[index].vz;
            self.shots[index].x = wrap_coord(self.shots[index].x + self.shots[index].vx);
            self.shots[index].y = wrap_coord(self.shots[index].y + self.shots[index].vy);
            self.shots[index].life += 1;
            if self.shots[index].z > 0 && self.shots[index].life < 400 {
                continue;
            }
            let shot = self.shots[index];
            self.shots[index].alive = false;
            let radius = match shot.kind {
                KIND_HE => 22,
                KIND_VOG => 12,
                KIND_MINE => 10,
                _ => 14,
            };
            self.explode(bg, shot.x, shot.y, radius, 0, shot.kind);
        }
    }

    pub(crate) fn update_trucks(&mut self) {
        for index in 0..MAX_TRUCKS {
            if !self.trucks[index].alive {
                continue;
            }
            if self.trucks[index].objective && matches!(task_of(self.level), Task::Push) {
                self.trucks[index].y -= rush_step(self.level);
                continue;
            }
            // Faster than a road crawl, slow enough to climb, shoot, then dive.
            if self.trucks[index].goal && matches!(task_of(self.level), Task::Cover) {
                self.trucks[index].y -= 24;
                continue;
            }
            let step = ground_step(self.trucks[index].dir);
            if self.trucks[index].vertical {
                self.trucks[index].y += step;
            } else {
                self.trucks[index].x += step;
            }
            if self.trucks[index].goal {
                continue;
            }
            let probe = ground_step(self.trucks[index].dir).signum() * (10 << 8);
            let ahead_x = if self.trucks[index].vertical {
                self.trucks[index].x
            } else {
                self.trucks[index].x + probe
            };
            let ahead_y = if self.trucks[index].vertical {
                self.trucks[index].y + probe
            } else {
                self.trucks[index].y
            };
            if self.ground_at(tile_of(ahead_x), tile_of(ahead_y)) != T_ROAD {
                self.trucks[index].dir = -self.trucks[index].dir;
            }
        }
    }

    pub(crate) fn update_tanks(&mut self) {
        for index in 0..MAX_TANKS {
            if !self.tanks[index].alive {
                continue;
            }
            let step = ground_step(self.tanks[index].dir);
            if self.tanks[index].vertical {
                self.tanks[index].y += step;
            } else {
                self.tanks[index].x += step;
            }
            let probe = step.signum() * (10 << 8);
            let ahead_x = if self.tanks[index].vertical {
                self.tanks[index].x
            } else {
                self.tanks[index].x + probe
            };
            let ahead_y = if self.tanks[index].vertical {
                self.tanks[index].y + probe
            } else {
                self.tanks[index].y
            };
            if self.ground_at(tile_of(ahead_x), tile_of(ahead_y)) != T_ROAD {
                self.tanks[index].dir = -self.tanks[index].dir;
            }
        }
    }

    pub(crate) fn update_soldiers(&mut self) {
        for index in 0..MAX_SOLDIERS {
            if !self.soldiers[index].alive {
                continue;
            }
            if self.soldiers[index].objective && matches!(task_of(self.level), Task::Fpv | Task::Nest)
            {
                continue;
            }
            if self.soldiers[index].objective
                && matches!(task_of(self.level), Task::Rush | Task::Push)
            {
                self.soldiers[index].y -= rush_step(self.level);
                self.soldiers[index].step = self.soldiers[index].step.wrapping_add(1);
                continue;
            }
            let sign = if self.soldiers[index].dir < 0 { -1 } else { 1 };
            let ahead = self.soldiers[index].x + sign * (10 << 8);
            let tile = self.ground_at(tile_of(ahead), tile_of(self.soldiers[index].y));
            let sign = if tile == T_TRENCH || tile == T_LIP {
                sign
            } else {
                -sign
            };
            self.soldiers[index].dir = sign * 80;
            self.soldiers[index].x += ground_step(self.soldiers[index].dir);
            self.soldiers[index].step = self.soldiers[index].step.wrapping_add(1);
        }
    }

    pub(crate) fn update_booms(&mut self) {
        for boom in &mut self.booms {
            if !boom.alive {
                continue;
            }
            boom.frame += 1;
            if boom.frame > 40 {
                boom.alive = false;
            }
        }
    }

    pub(crate) fn explode(&mut self, bg: &mut AffineBackground, x: i32, y: i32, radius: i32, depth: i32, kind: u8) {
        self.add_boom(x, y, 0);
        self.damage_tiles(bg, x, y, radius);
        let reach = radius * radius;
        let mut graves = [(0i32, 0i32); MAX_SOLDIERS + MAX_TRUCKS];
        let mut graves_n = 0;
        let mut trucks_hit = 0;
        let mut goals_hit = 0;
        let mut obj_trucks = 0;
        let mut officers_hit = 0;
        let mut obj_men = 0;
        let mut men_hit = 0;
        for truck in &mut self.trucks {
            if kind == KIND_BOMB && truck.alive && dist2_px(truck.x, truck.y, x, y) <= reach {
                graves[graves_n] = (truck.home_tx, truck.home_ty);
                graves_n += 1;
                if truck.goal {
                    goals_hit += 1;
                } else if truck.objective {
                    obj_trucks += 1;
                } else {
                    trucks_hit += 1;
                }
                truck.alive = false;
            }
        }
        for soldier in &mut self.soldiers {
            if matches!(kind, KIND_VOG | KIND_BOMB | KIND_HE | KIND_MINE)
                && soldier.alive
                && dist2_px(soldier.x, soldier.y, x, y) <= reach
            {
                graves[graves_n] = (soldier.home_tx, soldier.home_ty);
                graves_n += 1;
                if soldier.officer {
                    officers_hit += 1;
                } else if soldier.objective {
                    obj_men += 1;
                } else {
                    men_hit += 1;
                }
                soldier.alive = false;
            }
        }
        self.score += (trucks_hit + goals_hit + obj_trucks) * 100
            + goals_hit * 100
            + (officers_hit + obj_men) * 50
            + officers_hit * 150
            + men_hit * 50;
        for _ in 0..trucks_hit {
            self.claim(false, true, false, false);
        }
        for _ in 0..obj_trucks {
            self.claim(false, true, false, true);
        }
        for _ in 0..goals_hit {
            self.claim(false, false, true, true);
        }
        for _ in 0..officers_hit {
            self.claim(true, false, false, true);
        }
        for _ in 0..obj_men {
            self.claim(false, false, false, true);
        }
        for _ in 0..men_hit {
            self.claim(false, false, false, false);
        }
        for step in 0..graves_n {
            self.mark_quiet(graves[step].0, graves[step].1);
        }
        let mut wrecks = [(0i32, 0i32); MAX_TANKS + MAX_BUNKERS];
        let mut wrecks_n = 0;
        let mut armor = 0;
        for tank in &mut self.tanks {
            if !tank.alive || kind == KIND_GUN {
                continue;
            }
            let dist = dist2_px(tank.x, tank.y, x, y);
            let hatch = dist <= HATCH * HATCH;
            let hits = hull_hits(kind);
            let near = hits > 0 && dist <= TANK_NEAR * TANK_NEAR;
            if !hatch && !near {
                continue;
            }
            if hatch {
                tank.hp = 0;
            } else {
                tank.hp -= HP_TANK / hits;
            }
            if tank.hp > 0 {
                continue;
            }
            wrecks[wrecks_n] = (tank.home_tx, tank.home_ty);
            wrecks_n += 1;
            if tank.objective {
                armor += 1;
            }
            tank.alive = false;
            self.score += 150;
        }
        let mut pills = 0;
        for bunker in &mut self.bunkers {
            if kind == KIND_HE && bunker.alive && dist2_px(bunker.x, bunker.y, x, y) <= reach {
                wrecks[wrecks_n] = (bunker.home_tx, bunker.home_ty);
                wrecks_n += 1;
                if bunker.objective {
                    pills += 1;
                }
                bunker.alive = false;
                self.score += 100;
            }
        }
        for _ in 0..armor {
            self.claim_armor();
        }
        for _ in 0..pills {
            self.claim_pill();
        }
        for step in 0..wrecks_n {
            self.mark_quiet(wrecks[step].0, wrecks[step].1);
        }
        if depth >= 4 {
            return;
        }
        let mut chain = [(0i32, 0i32); MAX_MINES];
        let mut count = 0;
        for mine in &mut self.mine_list {
            if mine.alive && dist2_px(mine.x, mine.y, x, y) <= reach {
                mine.alive = false;
                chain[count] = (mine.x, mine.y);
                count += 1;
            }
        }
        for step in 0..count {
            self.explode(bg, chain[step].0, chain[step].1, 24, depth + 1, KIND_BOMB);
        }
    }

    pub(crate) fn damage_tiles(&mut self, bg: &mut AffineBackground, x: i32, y: i32, radius: i32) {
        let cx = x >> 8;
        let cy = y >> 8;
        let reach = radius * radius;
        let x0 = (cx - radius).div_euclid(8);
        let x1 = (cx + radius).div_euclid(8);
        let y0 = (cy - radius).div_euclid(8);
        let y1 = (cy + radius).div_euclid(8);
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                let dx = tx * 8 + 4 - cx;
                let dy = ty * 8 + 4 - cy;
                if dx * dx + dy * dy > reach {
                    continue;
                }
                if self.ground_at(tx, ty) != T_WATER {
                    self.paint(bg, tx, ty, T_CRATER);
                }
            }
        }
    }

    pub(crate) fn paint(&mut self, bg: &mut AffineBackground, tx: i32, ty: i32, tile: u8) {
        let previous = self.ground_at(tx, ty);
        self.insert_mark(tx, ty, tile, false);
        let _ = bg;
        self.blit(tx, ty);
        if tile == T_CRATER && previous != T_CRATER && self.hq_goal(tx, ty) && self.targets > 0 {
            self.targets -= 1;
            self.score += 50;
            self.sfx |= SFX_MARK;
        }
    }

    /// The first cause sticks. A later hit in the same frame does not replace it.
    pub(crate) fn fail(&mut self, why: Why) {
        if self.crashed {
            return;
        }
        self.crashed = true;
        self.why = why;
    }

    /// The convoy truck reached the mark. The sortie is lost.
    pub(crate) fn check_convoy(&mut self) {
        if !matches!(task_of(self.level), Task::Convoy) || self.targets <= 0 {
            return;
        }
        let arrived = self.trucks.iter().any(|truck| {
            truck.alive && truck.goal && tile_of(truck.y) <= 4
        });
        if arrived {
            self.fail(Why::Truck);
        }
    }

    /// The escort truck reached the posts. The quads do not stop that.
    pub(crate) fn check_cover(&mut self) {
        if !matches!(task_of(self.level), Task::Cover) || self.targets <= 0 {
            return;
        }
        let arrived = self
            .trucks
            .iter()
            .any(|truck| truck.alive && truck.goal && tile_of(truck.y) <= 14);
        if arrived {
            self.fail(Why::Truck);
        }
    }

    /// The base line is north. A living mark that reaches it takes the place.
    pub(crate) fn check_rush(&mut self) {
        let task = task_of(self.level);
        if !matches!(task, Task::Rush | Task::Push) || self.targets <= 0 {
            return;
        }
        let men = self.soldiers.iter().any(|soldier| {
            soldier.alive && soldier.objective && tile_of(soldier.y) <= 14
        });
        let rigs = self.trucks.iter().any(|truck| {
            truck.alive && truck.objective && tile_of(truck.y) <= 14
        });
        if men || rigs {
            self.fail(Why::Line);
        }
    }

    /// The last card. A hostile quad that reaches the pad takes the base.
    pub(crate) fn check_base(&mut self) {
        if !defend_card(self.level) || self.targets <= 0 || self.crashed {
            return;
        }
        let reached = self.foes.iter().any(|foe| {
            foe.alive && dist2_px(foe.x, foe.y, self.pad_x, self.pad_y) <= 24 * 24
        });
        if reached {
            self.add_boom(self.pad_x, self.pad_y, 0);
            self.fail(Why::Base);
        }
    }

    /// Crossing the gate line either clears the card or sends the plane back.
    pub(crate) fn cross_gate(&mut self) -> bool {
        if self.mode != Mode::Gate || self.targets <= 0 {
            return true;
        }
        let gy = self.gate_y();
        let prev = self.y - self.vy;
        if !(prev > gy && self.y <= gy) {
            return true;
        }
        let dx = ((self.x - self.gate_x()) >> 8).abs();
        let alt = self.alt >> 8;
        if dx <= 16 && (8..=15).contains(&alt) {
            self.targets = 0;
            self.score += 200;
            self.sfx |= SFX_MARK;
            true
        } else {
            self.recover = 22;
            self.vx = 0;
            self.vy = 0;
            false
        }
    }

    pub(crate) fn add_boom(&mut self, x: i32, y: i32, z: i32) {
        let mut slot = 0;
        for (index, boom) in self.booms.iter().enumerate() {
            if !boom.alive {
                slot = index;
                break;
            }
        }
        self.booms[slot] = Boom {
            x: wrap_coord(x),
            y: wrap_coord(y),
            z,
            frame: 0,
            alive: true,
        };
        self.sfx |= SFX_BOOM;
    }
}

