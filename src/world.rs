use agb::display::WIDTH;
use agb::fixnum::{Num, num};
use agb::input::Tri;
use alloc::vec::Vec;

mod chain;
mod combat;
mod field;
mod flight;

pub(crate) use chain::*;

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
/// A pine is solid only on a low pass. Cruise and every mission start sit above it.
pub(crate) const TREE_TOP: i32 = 6;
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
/// Sixteen texels from the zenith to the haze. The sky's lines point at them.
pub(crate) const T_HAZE: u8 = 21;

/// Every card, the stage cards included. A card's number is its place in the save.
pub(crate) const LEVEL_COUNT: u8 = 40;
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
/// A fuel dump going up. It hits like a bomb and burns more than it digs.
pub(crate) const KIND_FUEL: u8 = 7;

pub(crate) const ARM_GUN: u8 = 0;
pub(crate) const ARM_BOMB: u8 = 1;
pub(crate) const ARM_HE: u8 = 2;
pub(crate) const ARM_MINE: u8 = 3;
pub(crate) const ARM_VOG: u8 = 4;
/// World pixels from the hatch. The hatch is the dark square on the turret.
pub(crate) const HATCH: i32 = 6;
/// The turret sits back on the hull: its hatch is this many world pixels
/// behind the middle of the tank, where the sheets draw it.
pub(crate) const HATCH_BACK: i32 = 6;
/// Beside the hull. Wide enough that a normal bomb drop counts.
pub(crate) const TANK_NEAR: i32 = 28;
/// Bombs beside the hull that finish a tank. The hatch is still one shot.
pub(crate) const BOMB_HULL_HITS: i8 = 3;

/// What a kill counts as on the card: the officer, a truck, the truck the
/// card is about, and one of the marked targets.
#[derive(Clone, Copy, Default)]
pub(crate) struct Kill {
    pub(crate) officer: bool,
    pub(crate) truck: bool,
    pub(crate) goal: bool,
    pub(crate) objective: bool,
}

/// How many blasts beside the hull finish a full tank. Zero leaves the hull alone.
pub(crate) fn hull_hits(kind: u8) -> i8 {
    match kind {
        KIND_BOMB | KIND_FUEL => BOMB_HULL_HITS,
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
/// A blast lives on as smoke, and a cook-off is several, so a few more
/// than are ever on fire at once.
pub(crate) const MAX_BOOMS: usize = 16;
/// Fuel dumps on the field at once.
pub(crate) const MAX_DRUMS: usize = 6;
/// Blasts still to come: a cook-off's later bursts, and a dump catching.
pub(crate) const MAX_POPS: usize = 10;
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
#[derive(Clone, Copy)]
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
    /// Daylight over snow, mud, and sulfur. The trees are already burned. Men still hold the trenches.
    Pits,
    /// Green pines in clean snow. The men stand on one mud island.
    Grove,
    /// One truck on the snow road. Stop it before the north mark.
    Drift,
    /// Three tanks, each parked on its own mud island.
    Thaw,
    /// One officer in the pines.
    Hearth,
    /// Fly low over each mark in turn. What they find is the next stage.
    Scout,
    /// Knock out the card's guns. From above their height they cannot see the drop.
    Battery,
    /// A card of stages. Each stage is one of the tasks above.
    Chain,
}

/// What the glass shows. The side instruments stay in their own colours.
#[derive(Clone, Copy)]
pub(crate) enum Sensor {
    Day,
    Night,
    Heat,
    /// Active infrared illuminator: grey-green, with blooms on a blast.
    Ir,
    /// Daylight on snow. The ash pits burn the trees. The later cards keep the pines green.
    Winter,
}

pub(crate) fn sensor_of(level: u8) -> Sensor {
    if let Some(card) = chain(level) {
        return card.sensor;
    }
    match task_of(level) {
        Task::Night | Task::Armor | Task::Flare => Sensor::Night,
        Task::Heat | Task::Pill | Task::Dart | Task::Skirt => Sensor::Heat,
        Task::Guide | Task::Column | Task::Rush | Task::Push | Task::Bend => Sensor::Ir,
        Task::Pits | Task::Grove | Task::Drift | Task::Thaw | Task::Hearth => Sensor::Winter,
        _ => Sensor::Day,
    }
}

pub(crate) fn task_of(level: u8) -> Task {
    // One campaign, in order. The cards are not a menu.
    match level % LEVEL_COUNT {
        // The lesson is first. Everything that used to sit ahead of it
        // moves up one card. The rest of the list stays put.
        0 => Task::Gate,
        1 => Task::Officer,
        2 => Task::Trench,
        3 => Task::Hq,
        4 => Task::Convoy,
        5 => Task::Night,
        6 => Task::Heat,
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
        25 => Task::Pits,
        26 => Task::Grove,
        27 => Task::Drift,
        28 => Task::Thaw,
        29 => Task::Hearth,
        _ => Task::Chain,
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
    if let Some(card) = chain(level) {
        return card.budget;
    }
    let frames = match task_of(level) {
        Task::Officer | Task::Gate => 1800,
        Task::Trench | Task::Convoy | Task::Night | Task::Nest | Task::Dart => 2700,
        Task::Hq
        | Task::Heat
        | Task::Guide
        | Task::Fpv
        | Task::Lane
        | Task::Flare
        | Task::Bend
        | Task::Column
        | Task::Armor
        | Task::Pill
        | Task::Rush
        | Task::Skirt => 3600,
        Task::Push | Task::Swarm => 4200,
        Task::Fetch => 4800,
        Task::Cover | Task::Pits | Task::Drift => 4500,
        Task::Grove | Task::Thaw => 3600,
        Task::Hearth => 2700,
        Task::Scout | Task::Battery | Task::Chain => 3600,
    };
    let frames = frames + approach_frames(level);
    if haul_card(level) || defend_card(level) {
        frames + 1200
    } else {
        frames
    }
}

/// The time a pad moved back adds to the run: about six frames a tile for a
/// quad, forty for the spotter, and the FPV flies it once for every drone.
fn approach_frames(level: u8) -> i32 {
    match task_of(level) {
        Task::Trench | Task::Officer | Task::Hq | Task::Pill => 150,
        Task::Grove | Task::Hearth => 100,
        // The row loops at each end of its run, and that takes a while.
        Task::Night => 700,
        Task::Heat => 50,
        Task::Pits => 80,
        Task::Armor => 220,
        Task::Guide => 540,
        Task::Column => 780,
        Task::Nest => 520,
        _ => 0,
    }
}

/// Bombs, shells, mines, grenades. A bare card carries nothing.
pub(crate) fn rack(task: Task) -> (i32, i32, i32, i32) {
    match task {
        Task::Officer => (0, 0, 0, 4),
        Task::Trench => (4, 1, 0, 8),
        Task::Hq => (4, 4, 0, 0),
        Task::Convoy | Task::Cover => (3, 0, 0, 0),
        Task::Night | Task::Heat | Task::Lane => (6, 0, 0, 0),
        Task::Armor | Task::Flare => (6, 0, 4, 0),
        Task::Pill | Task::Skirt => (0, 6, 0, 0),
        Task::Pits => (4, 2, 0, 8),
        Task::Grove => (2, 1, 0, 6),
        Task::Drift => (4, 0, 0, 6),
        Task::Thaw => (6, 0, 4, 6),
        Task::Hearth => (2, 0, 0, 6),
        // Two bombs: the lesson, and one miss that still clears.
        Task::Gate => (2, 0, 0, 0),
        Task::Fpv
        | Task::Guide
        | Task::Column
        | Task::Rush
        | Task::Push
        | Task::Nest
        | Task::Dart
        | Task::Bend
        | Task::Fetch
        | Task::Swarm
        | Task::Scout
        | Task::Battery
        | Task::Chain => (0, 0, 0, 0),
    }
}

/// The round in hand when the card starts.
pub(crate) fn opening_arm(task: Task) -> u8 {
    match task {
        Task::Fpv
        | Task::Nest
        | Task::Dart
        | Task::Swarm
        | Task::Cover
        | Task::Grove
        | Task::Hearth => ARM_GUN,
        Task::Fetch | Task::Armor | Task::Flare | Task::Thaw => ARM_MINE,
        Task::Pill | Task::Skirt | Task::Hq => ARM_HE,
        Task::Pits | Task::Trench | Task::Officer | Task::Drift => ARM_VOG,
        _ => ARM_BOMB,
    }
}
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
    if let Some(card) = chain(world.level) {
        return match card.spare {
            Spare::Drops(limit) => world.spent <= limit,
            Spare::Drones(left) => world.drones >= left,
        };
    }
    let limit = match task_of(world.level) {
        // The lesson spends the one bomb it asks for.
        Task::Gate => return world.spent <= 1,
        Task::Fpv => return world.drones >= 3,
        Task::Nest | Task::Dart => return world.drones >= 2,
        Task::Swarm => return world.hull >= 50,
        Task::Officer | Task::Convoy | Task::Cover => 2,
        Task::Hearth => 4,
        Task::Guide
        | Task::Column
        | Task::Rush
        | Task::Bend
        | Task::Fetch
        | Task::Night
        | Task::Heat
        | Task::Lane
        | Task::Push => 4,
        Task::Drift
        | Task::Trench
        | Task::Pill
        | Task::Skirt
        | Task::Armor
        | Task::Flare
        | Task::Grove => 6,
        Task::Thaw | Task::Pits | Task::Hq => 8,
        Task::Scout | Task::Battery | Task::Chain => 6,
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
    /// Frames it still burns, with smoke climbing off it.
    pub(crate) burn: i16,
}

/// Fuel drums stacked on the ground. Any blast near them, or a burst of the
/// gun, sets them off, and their own blast reaches whatever stands close.
#[derive(Clone, Copy)]
pub(crate) struct Drum {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) home_tx: i32,
    pub(crate) home_ty: i32,
    pub(crate) alive: bool,
}

/// Frames a wreck burns: a truck or a tank killed from outside, a tank
/// whose rounds went up inside, a bunker.
pub(crate) const BURN_TRUCK: i16 = 420;
pub(crate) const BURN_COOK_OFF: i16 = 900;
pub(crate) const BURN_BUNKER: i16 = 240;

/// How far a dump's own blast reaches, in pixels.
pub(crate) const DRUM_BLAST: i32 = 26;

/// A blast waiting its turn. `hurts` is a dump going up, which damages
/// what is near; the rest are a cook-off's bursts, fire and noise only.
#[derive(Clone, Copy)]
pub(crate) struct Pop {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) wait: i16,
    pub(crate) radius: i16,
    pub(crate) hurts: bool,
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
    /// Frames into a U-turn, 0 while it drives. The rig loops out into the
    /// other lane and back instead of swapping its nose and tail.
    pub(crate) turn: u8,
    pub(crate) turn_len: u8,
    /// The side of the road the next U-turn swings to: 1 or -1.
    pub(crate) lane: i8,
    /// Where the U-turn began, in sub-pixels.
    pub(crate) pivot_x: i32,
    pub(crate) pivot_y: i32,
}

/// Radius of a truck's U-turn: half a lane.
pub(crate) const TURN_RADIUS: i32 = 6 << 8;

impl Truck {
    /// Along the road, the way the nose points: x, y in 1/256.
    fn road(&self) -> (i32, i32) {
        let along = self.dir.signum() * 256;
        if self.vertical {
            (0, along)
        } else {
            (along, 0)
        }
    }

    /// Across the road, toward the side the U-turn swings out to.
    fn across(&self) -> (i32, i32) {
        let side = i32::from(self.lane) * 256;
        if self.vertical { (side, 0) } else { (0, side) }
    }

    /// The loop as (out, ahead, heading out, heading ahead), each in 1/256
    /// of a radius or of a unit: a quarter turn out, then three quarters
    /// back the other way, which ends two radii on facing back down the lane.
    fn bend(&self) -> (i32, i32, i32, i32) {
        let len = i32::from(self.turn_len).max(1);
        let t = i32::from(self.turn);
        let quarter = len / 4;
        if t <= quarter {
            let a = Num::<i32, 8>::new(t) / Num::new(quarter.max(1) * 4);
            let (cos, sin) = (a.cos().to_raw(), a.sin().to_raw());
            (256 - cos, sin, sin, cos)
        } else {
            let b = Num::<i32, 8>::new(t - quarter) / Num::new((len - quarter).max(1) * 4 / 3);
            let (cos, sin) = (b.cos().to_raw(), b.sin().to_raw());
            (256 + sin, 512 - cos, cos, sin)
        }
    }

    /// Where the nose points, in 1/256: along the road, or round the loop.
    pub(crate) fn facing(&self) -> (i32, i32) {
        let (dx, dy) = self.road();
        if self.turn == 0 {
            return (dx, dy);
        }
        let (nx, ny) = self.across();
        let (_, _, out, ahead) = self.bend();
        ((dx * ahead + nx * out) >> 8, (dy * ahead + ny * out) >> 8)
    }

    /// Start a U-turn from where it stands, at `speed` sub-pixels a frame.
    pub(crate) fn start_turn(&mut self, speed: i32) {
        self.pivot_x = self.x;
        self.pivot_y = self.y;
        // A whole circle's length at the truck's own pace.
        self.turn_len = (TURN_RADIUS * 1608 / 256 / speed.abs().max(1)).clamp(16, 250) as u8;
        self.turn = 1;
    }

    /// One frame round the loop. At the end it faces back down its lane.
    pub(crate) fn step_turn(&mut self) {
        let (dx, dy) = self.road();
        let (nx, ny) = self.across();
        let (out, ahead, _, _) = self.bend();
        let r = TURN_RADIUS >> 8;
        self.x = self.pivot_x + ((dx * ahead + nx * out) >> 8) * r;
        self.y = self.pivot_y + ((dy * ahead + ny * out) >> 8) * r;
        if self.turn >= self.turn_len {
            self.dir = -self.dir;
            self.lane = -self.lane;
            self.turn = 0;
        } else {
            self.turn += 1;
        }
    }
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

impl Tank {
    /// The way the barrel points, one step along x or y. A tank that does
    /// not drive still faces one way on the ground, not toward the camera.
    pub(crate) fn facing(&self) -> (i32, i32) {
        let along = if self.dir < 0 { -1 } else { 1 };
        if self.vertical {
            (0, along)
        } else {
            (along, 0)
        }
    }

    /// The middle of the hatch, in sub-pixels.
    pub(crate) fn hatch(&self) -> (i32, i32) {
        let (fx, fy) = self.facing();
        (
            self.x - fx * (HATCH_BACK << 8),
            self.y - fy * (HATCH_BACK << 8),
        )
    }
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

/// What the spawner and the reaper ask of a unit on the ground. The four
/// kinds keep their own fields; this is the part they share.
pub(crate) trait Unit {
    fn alive(&self) -> bool;
    /// The tile it was seated on.
    fn home(&self) -> (i32, i32);
    /// Where it is now, in sub-pixels.
    fn at(&self) -> (i32, i32);
    /// The card needs it, so it stays however far the aircraft goes.
    fn kept(&self) -> bool;
    fn retire(&mut self);
}

macro_rules! unit {
    ($kind:ty, |$it:ident| $kept:expr) => {
        impl Unit for $kind {
            fn alive(&self) -> bool {
                self.alive
            }
            fn home(&self) -> (i32, i32) {
                (self.home_tx, self.home_ty)
            }
            fn at(&self) -> (i32, i32) {
                (self.x, self.y)
            }
            fn kept(&self) -> bool {
                let $it = self;
                $kept
            }
            fn retire(&mut self) {
                self.alive = false;
            }
        }
    };
}

unit!(Soldier, |man| man.officer || man.objective);
unit!(Truck, |truck| truck.goal || truck.objective);
unit!(Tank, |tank| tank.objective);
unit!(Bunker, |bunker| bunker.objective);
unit!(Drum, |_drum| false);

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
    /// First card only. 0 hover over the mark, 1 drop, 2 fly the gap.
    pub(crate) lesson: u8,
    /// The stage in hand on a stage card. Zero on every other card.
    pub(crate) stage: u8,
    /// Scout marks already flown over, one bit each.
    pub(crate) seen: u8,
    /// Batteries knocked out, one bit per gun of the card.
    pub(crate) guns_down: u8,
    /// Frames the stage's line stays under the glass.
    pub(crate) cue: i16,
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
    pub(crate) drums: [Drum; MAX_DRUMS],
    pub(crate) pops: [Pop; MAX_POPS],
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

/// The middle of a tile, in sub-pixels. The way back from `tile_of`.
pub(crate) const fn tile_mid(tile: i32) -> i32 {
    (tile * 8 + 4) << 8
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
    // The pad sits a run south of the work, so a sortie opens with the
    // targets ahead and not under the nose. Pads stay off the trench rows
    // (20 and 36 of each 64) and the roads. The cards that need them close
    // keep them close: the lesson, the chase, the spotter's clocks, the mine
    // hauls, the pad under attack, the escort, and the snow cards laid out
    // around their pad. `approach_frames` pays for each extra run.
    if let Some(card) = chain(level) {
        return card.pad;
    }
    match task_of(level) {
        Task::Trench | Task::Officer | Task::Hq | Task::Pill => (24, 58),
        Task::Night => (24, 60),
        Task::Heat => (24, 30),
        Task::Convoy => (8, 48),
        // Off the posts and short of them, so the first card has to turn.
        Task::Gate => (14, 84),
        Task::Fpv => (24, 92),
        Task::Guide => (24, 56),
        Task::Column | Task::Armor => (24, 108),
        Task::Fetch => (24, 90),
        Task::Swarm if defend_card(level) => (24, 90),
        Task::Swarm => (24, 90),
        Task::Rush => (24, 68),
        Task::Push => (24, 80),
        // South of the first guns, so the lane itself is the flight.
        Task::Lane | Task::Flare | Task::Dart | Task::Bend | Task::Skirt => (22, 88),
        // The first nest is in the lane's first gap: the pad stays well short.
        Task::Nest => (22, 108),
        Task::Cover => (24, 64),
        Task::Pits => (24, 60),
        Task::Grove => (24, 70),
        Task::Drift => (14, 62),
        Task::Thaw => (24, 68),
        Task::Hearth => (24, 66),
        Task::Scout | Task::Battery | Task::Chain => (24, 90),
    }
}

/// The snow cards after the ash pits. Pines stay green.
pub(crate) fn winter_card(level: u8) -> bool {
    if let Some(card) = chain(level) {
        return card.winter;
    }
    matches!(
        task_of(level),
        Task::Grove | Task::Drift | Task::Thaw | Task::Hearth
    )
}

pub(crate) struct AaGun {
    pub(crate) tx: i32,
    pub(crate) ty: i32,
    pub(crate) radius: i32,
}

/// The pine post. The pad sits just outside. Climb before the circle, or the watch wakes.
pub(crate) const HEARTH_WATCH: [AaGun; 1] = [AaGun {
    tx: 24,
    ty: 30,
    radius: 112,
}];

/// Three gates. The open side shifts right, then back left.
pub(crate) const LANE_GUNS: [AaGun; 6] = [
    AaGun {
        tx: -4,
        ty: 78,
        radius: 104,
    },
    AaGun {
        tx: 48,
        ty: 78,
        radius: 104,
    },
    AaGun {
        tx: 6,
        ty: 56,
        radius: 120,
    },
    AaGun {
        tx: 54,
        ty: 56,
        radius: 104,
    },
    AaGun {
        tx: -6,
        ty: 34,
        radius: 104,
    },
    AaGun {
        tx: 42,
        ty: 34,
        radius: 104,
    },
];

/// The card's batteries. A stage card can knock them out; `World::gun_up` says which still stand.
pub(crate) fn aa_guns(level: u8) -> &'static [AaGun] {
    if let Some(card) = chain(level) {
        return card.guns;
    }
    match task_of(level) {
        Task::Lane | Task::Flare | Task::Nest | Task::Dart | Task::Bend | Task::Skirt => &LANE_GUNS,
        Task::Hearth => &HEARTH_WATCH,
        _ => &[],
    }
}

pub(crate) fn gun_at(gun: &AaGun) -> (i32, i32) {
    (tile_mid(gun.tx), tile_mid(gun.ty))
}

/// The drone is in sight under `AA_HIDE`. At that height every gun loses it.
pub(crate) fn guns_see(alt_px: i32) -> bool {
    alt_px < AA_HIDE
}

/// Radar dial point back into the world. Ahead on the dial is negative `ry`.
pub(crate) fn lane_world(
    cam_x: i32,
    cam_y: i32,
    sin_r: i32,
    cos_r: i32,
    rx: i32,
    ry: i32,
) -> (i32, i32) {
    let fwd = -ry * RADAR_RANGE / 12;
    let right = rx * RADAR_RANGE / 12;
    let dx = (fwd * sin_r + right * cos_r) >> 8;
    let dy = (right * sin_r - fwd * cos_r) >> 8;
    (cam_x + (dx << 8), cam_y + (dy << 8))
}

/// One battlefield everywhere. Roads tie the districts together. Trenches
/// and a few copses sit in the grass. Water is a pond, not a scatter of puddles.
/// The skyline is not this map. The ash pits dress that field after it is chosen.
pub(crate) fn terrain(level: u8, tx: i32, ty: i32) -> u8 {
    let tile = field_tile(level, tx, ty);
    if matches!(task_of(level), Task::Pits) {
        return pits_tile(level, tx, ty, tile);
    }
    if winter_card(level) {
        return winter_tile(level, tx, ty, tile);
    }
    tile
}

/// Snow broken by mud, sulfur crust, burned trunks, and extra trench lines.
fn pits_tile(level: u8, tx: i32, ty: i32, tile: u8) -> u8 {
    let (pad_x, pad_y) = start_tile(level);
    if (tx - pad_x).abs() <= 3 && (ty - pad_y).abs() <= 3 {
        return tile;
    }
    let lx = tx.rem_euclid(REGION);
    let ly = ty.rem_euclid(REGION);
    let gap = lx % 14 == 7;
    if !gap && lx > 2 && matches!(ly, 14 | 28 | 44) {
        return T_LIP;
    }
    if !gap && lx > 2 && matches!(ly, 15 | 29 | 45) {
        return T_TRENCH;
    }
    // Past the trenches the field is winter: white snow, a few burned trunks.
    if !(6..=44).contains(&tx) || !(12..=40).contains(&ty) {
        return snow_field(tx, ty, tile);
    }
    match tile {
        T_TREE | T_BUSH => T_STUMP,
        T_FLOOR | T_FLOOR2 | T_GRASS_L | T_GRASS_D | T_TUFT | T_FOG | T_PATCH => {
            // Another copse of burned trunks, standing together.
            let wx = tx.div_euclid(3);
            let wy = ty.div_euclid(3);
            if hash(wx, wy.wrapping_add(53)).is_multiple_of(3) {
                return T_STUMP;
            }
            // A patch of clean snow. The glass turns this pale tile white.
            let sx = tx.div_euclid(4);
            let sy = ty.div_euclid(4);
            if hash(sx, sy.wrapping_add(29)).is_multiple_of(3) {
                return T_SKY;
            }
            let n = hash(tx, ty.wrapping_add(91));
            if n.is_multiple_of(8) {
                T_DIRT
            } else if n.is_multiple_of(13) {
                T_CRATER
            } else {
                tile
            }
        }
        _ => tile,
    }
}

/// Clean snow, green pines, and a few mud islands. The ash pits do not use this.
fn winter_tile(level: u8, tx: i32, ty: i32, tile: u8) -> u8 {
    let (pad_x, pad_y) = start_tile(level);
    if (tx - pad_x).abs() <= 3 && (ty - pad_y).abs() <= 3 {
        return tile;
    }
    if tile == T_ROAD {
        return T_ROAD;
    }
    if mud_here(level, tx, ty) {
        return T_DIRT;
    }
    if burned_ring(level, tx, ty) {
        return T_STUMP;
    }
    if tile == T_TREE || tile == T_BUSH {
        return tile;
    }
    let wx = tx.div_euclid(3);
    let wy = ty.div_euclid(3);
    if hash(wx, wy.wrapping_add(71)).is_multiple_of(4) {
        return T_TREE;
    }
    if hash(tx, ty.wrapping_add(19)).is_multiple_of(6)
        && !matches!(tile, T_TRENCH | T_LIP | T_WATER)
    {
        return tile;
    }
    T_SKY
}

/// Centres and radii of the brown islands on a snow card.
fn mud_islands(level: u8) -> &'static [(i32, i32, i32)] {
    if let Some(card) = chain(level) {
        return card.mud;
    }
    match task_of(level) {
        Task::Grove => &[(22, 32, 6), (40, 28, 3)],
        Task::Drift => &[(12, 34, 4), (22, 20, 3)],
        Task::Thaw => &[(12, 42, 4), (28, 36, 4), (40, 44, 4), (24, 110, 4)],
        Task::Hearth => &[(24, 30, 3), (10, 36, 4), (38, 24, 3)],
        _ => &[],
    }
}

fn mud_here(level: u8, tx: i32, ty: i32) -> bool {
    mud_islands(level).iter().any(|&(cx, cy, r)| {
        let dx = tx - cx;
        let dy = ty - cy;
        dx * dx + dy * dy <= r * r
    })
}

/// Scorched trunks around the snow-wood fight. Farther out the pines stay green.
fn burned_ring(level: u8, tx: i32, ty: i32) -> bool {
    if !matches!(task_of(level), Task::Grove) {
        return false;
    }
    let dx = tx - 22;
    let dy = ty - 32;
    let dist = dx * dx + dy * dy;
    (49..=225).contains(&dist) && !hash(tx, ty.wrapping_add(4)).is_multiple_of(4)
}

/// White snow outside the fight, with burned trunks still standing in it.
fn snow_field(tx: i32, ty: i32, tile: u8) -> u8 {
    match tile {
        T_ROAD | T_WATER => tile,
        T_TREE | T_BUSH => T_STUMP,
        _ => {
            let wx = tx.div_euclid(3);
            let wy = ty.div_euclid(3);
            if hash(wx, wy.wrapping_add(53)).is_multiple_of(4) {
                T_STUMP
            } else if hash(tx, ty.wrapping_add(11)).is_multiple_of(14) {
                T_DIRT
            } else {
                T_SKY
            }
        }
    }
}

fn field_tile(level: u8, tx: i32, ty: i32) -> u8 {
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
    if hash(wx, wy.wrapping_add(17)).is_multiple_of(6) {
        return if n.is_multiple_of(6) { T_BUSH } else { T_TREE };
    }
    // Mud yards between the copses. A 2×2 block, so the dirt does not speckle.
    let dx = tx.div_euclid(2);
    let dy = ty.div_euclid(2);
    if hash(dx, dy.wrapping_add(3)).is_multiple_of(4) {
        return T_DIRT;
    }
    if n.is_multiple_of(29) {
        return T_TUFT;
    }
    if n.is_multiple_of(53) {
        return T_PATCH;
    }
    // One tone for a 4×4 patch. Neighbouring tiles stay in the same grass.
    let tone = hash(
        tx.div_euclid(4),
        ty.div_euclid(4).wrapping_add(level as i32 * 13),
    );
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
    if !hash(cx, cy.wrapping_add(41)).is_multiple_of(5) {
        return false;
    }
    let ox = 6 + (hash(cx, cy) % 8) as i32;
    let oy = 6 + (hash(cx, cy.wrapping_add(2)) % 8) as i32;
    let mid_x = cx * span + ox;
    let mid_y = cy * span + oy;
    let rx = mid_x.rem_euclid(REGION);
    let ry = mid_y.rem_euclid(REGION);
    if !(8..=REGION - 8).contains(&rx) || !(8..=REGION - 8).contains(&ry) {
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
    !(dist + 3 >= radius * radius && hash(tx, ty).is_multiple_of(3))
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
        // One belt just south of the pad. Three trips, and one miss, fit the clock.
        [
            (20, 96),
            (24, 96),
            (28, 96),
            (20, 100),
            (24, 100),
            (28, 100),
        ]
    }
}

/// Where the mine quad can sit and fill the battery. Clear of the tanks.
pub(crate) fn charge_spots(level: u8) -> &'static [(i32, i32)] {
    if let Some(card) = chain(level) {
        card.charge
    } else if haul_card(level) {
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
        burn: 0,
    }
}

pub(crate) fn dead_drum() -> Drum {
    Drum {
        x: 0,
        y: 0,
        home_tx: 0,
        home_ty: 0,
        alive: false,
    }
}

pub(crate) fn dead_pop() -> Pop {
    Pop {
        x: 0,
        y: 0,
        wait: 0,
        radius: 0,
        hurts: false,
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
        turn: 0,
        turn_len: 0,
        lane: 1,
        pivot_x: 0,
        pivot_y: 0,
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

/// 0 on the first card, 16 on card 22. Cards after that stay at 16. A stage
/// card is as hot as the card it follows.
pub(crate) fn heat(level: u8) -> i32 {
    (pace(level) as i32).min(21) * 16 / 21
}

/// How fast a rush or a push walks north. 24 on the first card, 40 on the
/// last. A flat 48 reached the line before a bad mark could be corrected.
pub(crate) fn rush_step(level: u8) -> i32 {
    24 + heat(level)
}

/// Facing stays in `dir`. Men and tanks keep this crawl.
pub(crate) fn ground_step(dir: i32) -> i32 {
    if dir > 0 {
        18
    } else if dir < 0 {
        -18
    } else {
        0
    }
}

/// Subpixels per frame for a truck that is actually driving.
/// A bomb from the usual pass falls for about a second, and the blast is 14
/// pixels. 72 subpixels carries the truck just past that circle, so a round
/// aimed at where it is now lands behind it. A spotter salvo waits four
/// seconds, so that pace stays at 22: still behind the mark, still in the glass.
pub(crate) fn drive_step(guide: bool, dir: i32) -> i32 {
    let pace = if guide { 22 } else { 72 };
    if dir > 0 {
        pace
    } else if dir < 0 {
        -pace
    } else {
        0
    }
}
