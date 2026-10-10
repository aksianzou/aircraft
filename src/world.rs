use agb::display::WIDTH;
use agb::fixnum::{Num, num};
use agb::input::Tri;
use alloc::vec::Vec;

mod combat;
mod field;
mod flight;

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
/// Tiles out to the corner of the glass. Inside the loaded window, and short
/// of the 64-tile alias, so the far ground is still the real district.
pub(crate) const REACH: i32 = 56;
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
/// A trunk left after a fire. The tile is scorched mud, the sprite is the stick.
pub(crate) const T_STUMP: u8 = 17;

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
pub(crate) const MAX_BURNS: usize = 6;
pub(crate) const MAX_TRUCKS: usize = 4;
pub(crate) const MAX_TANKS: usize = 3;
pub(crate) const MAX_BUNKERS: usize = 3;
pub(crate) const MAX_SOLDIERS: usize = 8;
pub(crate) const MAX_FOES: usize = 8;
pub(crate) const MAX_HUSKS: usize = 8;
pub(crate) const MAX_SCRAP: usize = 12;
pub(crate) const SCRAP_TURRET: u8 = 0;
pub(crate) const SCRAP_WHEEL: u8 = 1;
pub(crate) const SCRAP_BIT: u8 = 2;
pub(crate) const HUSK_TRUCK: u8 = 0;
pub(crate) const HUSK_TANK: u8 = 1;
pub(crate) const HUSK_BUNKER: u8 = 2;
pub(crate) const HUSK_MAN: u8 = 3;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    /// Company ident, then the level card.
    Splash,
    Title,
    Fly,
    /// Held flight. Resume continues it. Exit returns to the level card.
    Pause,
    Crash,
    Win,
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
    /// Snow on the ground. Painted and matched, for a later card. No sortie wears it yet.
    Winter,
}

pub(crate) fn sensor_of(level: u8) -> Sensor {
    match task_of(level) {
        Task::Night | Task::Armor | Task::Flare => Sensor::Night,
        Task::Heat | Task::Pill | Task::Dart | Task::Skirt => Sensor::Heat,
        Task::Guide | Task::Column | Task::Rush | Task::Push | Task::Bend => Sensor::Ir,
        _ => {
            // Keeps the winter glass constructed until a later card returns it.
            let _ = Sensor::Winter;
            Sensor::Day
        }
    }
}

pub(crate) fn task_of(level: u8) -> Task {
    // One campaign, in order. The cards are not a menu.
    match level % LEVEL_COUNT {
        // One new idea at a time: a single man, then a line, a building, a
        // moving truck, two glasses, a height gate, the mark, then tanks,
        // bunkers, and only then the kamikaze.
        0 => Task::Officer,
        1 => Task::Trench,
        2 => Task::Hq,
        3 => Task::Convoy,
        4 => Task::Night,
        5 => Task::Heat,
        6 => Task::Gate,
        7 => Task::Guide,
        8 => Task::Column,
        9 => Task::Armor,
        10 => Task::Pill,
        11 => Task::Fpv,
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

/// Frames for the quick mark. A straight run fits. A search does not.
pub(crate) fn time_budget(level: u8) -> i32 {
    let frames = match task_of(level) {
        Task::Officer | Task::Gate => 1800,
        Task::Trench | Task::Convoy | Task::Night | Task::Nest | Task::Dart => 2700,
        Task::Hq | Task::Heat | Task::Guide | Task::Fpv | Task::Lane | Task::Flare
        | Task::Bend | Task::Column | Task::Armor | Task::Pill | Task::Rush | Task::Skirt => 3600,
        Task::Push | Task::Swarm => 4200,
        Task::Fetch => 4800,
        Task::Cover => 4500,
    };
    if haul_card(level) || defend_card(level) {
        frames + 1200
    } else {
        frames
    }
}

/// Bit 0 is time, bit 1 is battery still at 30, bit 2 is a spare load.
pub(crate) fn earned_marks(world: &World) -> u8 {
    let mut bits = 0u8;
    if world.age <= time_budget(world.level) {
        bits |= 1;
    }
    if world.battery >= 30 {
        bits |= 2;
    }
    if spare_clear(world) {
        bits |= 4;
    }
    bits
}

fn spare_clear(world: &World) -> bool {
    let limit = match task_of(world.level) {
        Task::Gate => return world.spent == 0,
        Task::Fpv => return world.drones >= 3,
        Task::Nest | Task::Dart => return world.drones >= 2,
        Task::Swarm => return world.hull >= 50,
        Task::Officer | Task::Convoy | Task::Cover => 2,
        Task::Guide | Task::Column | Task::Rush | Task::Bend | Task::Fetch | Task::Night
        | Task::Heat | Task::Lane | Task::Push => 3,
        Task::Trench | Task::Pill | Task::Skirt | Task::Armor | Task::Flare => 6,
        Task::Hq => 8,
    };
    world.spent <= limit
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

/// A tree or bush the blast set alight. It stands until `life` runs out.
#[derive(Clone, Copy)]
pub(crate) struct Burn {
    pub(crate) tx: i32,
    pub(crate) ty: i32,
    pub(crate) life: u8,
    pub(crate) alive: bool,
}

/// Burned, not scattered. The kill already counted. The shape stays put.
#[derive(Clone, Copy)]
pub(crate) struct Husk {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) dir: i32,
    pub(crate) vertical: bool,
    pub(crate) kind: u8,
    pub(crate) alive: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct Boom {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) z: i32,
    /// Kill radius in pixels. A wide hit draws a larger fire.
    pub(crate) radius: i16,
    pub(crate) frame: i16,
    /// White flash, then the black cloud, and the glass washes out.
    pub(crate) plume: bool,
    pub(crate) alive: bool,
}

/// A piece thrown off a vehicle. It rises and falls back onto the field.
#[derive(Clone, Copy)]
pub(crate) struct Scrap {
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
    /// Ground wash after a blast. Counts down to the plain picture.
    pub(crate) bloom: i16,
    /// Drops, marks, and a gate that came back to the pad.
    pub(crate) spent: i16,
    /// Next row of the menu map build. Negative means the window is ready.
    pub(crate) boot: i16,
    /// Gun, drop and blast cues for this frame.
    pub(crate) sfx: u8,
    pub(crate) shots: [Shot; MAX_SHOTS],
    pub(crate) mine_list: [Mine; MAX_MINES],
    pub(crate) booms: [Boom; MAX_BOOMS],
    pub(crate) burns: [Burn; MAX_BURNS],
    pub(crate) trucks: [Truck; MAX_TRUCKS],
    pub(crate) tanks: [Tank; MAX_TANKS],
    pub(crate) bunkers: [Bunker; MAX_BUNKERS],
    pub(crate) soldiers: [Soldier; MAX_SOLDIERS],
    pub(crate) foes: [Foe; MAX_FOES],
    /// Next wreck to replace once the field is full of them.
    pub(crate) husk_at: u8,
    pub(crate) husks: [Husk; MAX_HUSKS],
    pub(crate) scrap: [Scrap; MAX_SCRAP],
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
    // Woods share a 3×3 cell, so trees stand together. Often enough to read on every district.
    let wx = tx.div_euclid(3);
    let wy = ty.div_euclid(3);
    if hash(wx, wy.wrapping_add(17)) % 6 == 0 {
        return if n % 6 == 0 { T_BUSH } else { T_TREE };
    }
    // Mud yards between the copses. A 2×2 block, so the dirt does not speckle.
    let dx = tx.div_euclid(2);
    let dy = ty.div_euclid(2);
    if hash(dx, dy.wrapping_add(3)) % 4 == 0 {
        return T_DIRT;
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

pub(crate) fn dead_burn() -> Burn {
    Burn {
        tx: 0,
        ty: 0,
        life: 0,
        alive: false,
    }
}

pub(crate) fn dead_husk() -> Husk {
    Husk {
        x: 0,
        y: 0,
        dir: 0,
        vertical: false,
        kind: 0,
        alive: false,
    }
}

pub(crate) fn dead_boom() -> Boom {
    Boom {
        x: 0,
        y: 0,
        z: 0,
        radius: 0,
        frame: 0,
        plume: false,
        alive: false,
    }
}

pub(crate) fn dead_scrap() -> Scrap {
    Scrap {
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

pub(crate) fn dead_mark() -> Mark {
    Mark {
        tx: 0,
        ty: 0,
        tile: T_FLOOR,
        quiet: false,
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

