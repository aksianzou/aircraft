//! Forest drone on the Game Boy Advance.
//!
//! The ground is a mode 7 floor. Every level flies forward: left and right yaw,
//! down dives toward the ground, up climbs away. The last level is kamikaze.

#![no_std]
#![no_main]

extern crate alloc;

use agb::{
    display::{
        AffineMatrix, HEIGHT, Palette16, Priority, Rgb, Rgb15, WIDTH, WinIn,
        object::{
            AffineMode, GraphicsMode, Object, ObjectAffine, PaletteVram, PaletteVramSingle, Sprite,
            SpriteVram,
        },
        tiled::{
            AffineBackground, AffineBackgroundId, AffineBackgroundSize,
            AffineBackgroundWrapBehaviour,
        },
    },
    fixnum::{Num, num, rect, vec2},
    interrupt::{self, Interrupt},
    include_aseprite, include_background_gfx, include_wav,
    input::{Button, ButtonController, Tri},
    sound::mixer::{ChannelId, Frequency, SoundChannel, SoundData},
};
use alloc::vec;
use alloc::vec::Vec;

include_background_gfx!(mod backgrounds, TERRAIN => 256 "gfx/terrain.png");

include_aseprite!(
    mod sprites,
    "gfx/reticle.png",
    "gfx/bomb.png",
    "gfx/he.png",
    "gfx/mine.png",
    "gfx/vog.png",
    16x16 "gfx/boom.png",
    32x32 "gfx/truck.png",
    32x32 "gfx/tank.png",
    32x32 "gfx/bunker.png",
    32x32 "gfx/soldier.png",
    8x8 "gfx/digits.png",
    "gfx/lbl_score.png",
    "gfx/lbl_bomb.png",
    "gfx/lbl_he.png",
    "gfx/lbl_mine.png",
    "gfx/lbl_vog.png",
    "gfx/lbl_alt.png",
    "gfx/lbl_drone.png",
    "gfx/title.png",
    "gfx/help_quad.png",
    "gfx/help_fpv.png",
    "gfx/lvl_trench.png",
    "gfx/lvl_officer.png",
    "gfx/lvl_night.png",
    "gfx/lvl_heat.png",
    "gfx/help_heat.png",
    "gfx/lvl_guide.png",
    "gfx/help_guide.png",
    "gfx/lvl_armor.png",
    "gfx/help_armor.png",
    "gfx/lvl_pill.png",
    "gfx/help_pill.png",
    "gfx/lvl_column.png",
    "gfx/help_column.png",
    "gfx/lvl_rush.png",
    "gfx/help_rush.png",
    "gfx/lvl_push.png",
    "gfx/help_push.png",
    "gfx/lvl_lane.png",
    "gfx/help_lane.png",
    "gfx/lvl_flare.png",
    "gfx/help_flare.png",
    "gfx/lvl_nest.png",
    "gfx/help_nest.png",
    "gfx/lvl_dart.png",
    "gfx/help_dart.png",
    "gfx/lvl_bend.png",
    "gfx/help_bend.png",
    "gfx/lvl_skirt.png",
    "gfx/help_skirt.png",
    "gfx/lvl_lift.png",
    "gfx/help_lift.png",
    "gfx/lvl_haul.png",
    "gfx/help_haul.png",
    "gfx/lvl_gate.png",
    "gfx/lvl_fpv.png",
    "gfx/lvl_convoy.png",
    "gfx/lvl_hq.png",
    "gfx/author.png",
    32x32 "gfx/hq.png",
    "gfx/help_gate.png",
    "gfx/help_convoy.png",
    "gfx/crash.png",
    "gfx/win.png",
    32x16 "gfx/cloud.png",
    32x16 "gfx/horizon.png",
    32x32 "gfx/panel.png",
    8x32 "gfx/post.png",
    32x8 "gfx/beam.png",
    32x32 "gfx/radar.png",
    32x16 "gfx/adi_bar.png",
    32x16 "gfx/adi_wings.png",
    8x8 "gfx/blip.png",
    8x8 "gfx/dot.png",
    8x8 "gfx/round.png",
    16x16 "gfx/wep.png",
    32x8 "gfx/hull.png",
    32x32 "gfx/crack.png",
    8x8 "gfx/static.png",
    8x8 "gfx/bead.png",
    8x8 "gfx/bracket.png",
    8x8 "gfx/north.png",
    "gfx/pause_resume.png",
    "gfx/pause_resume_on.png",
    "gfx/pause_exit.png",
    "gfx/pause_exit_on.png",
    64x64 "gfx/noise.png",
    64x64 "gfx/flash.png",
);

static MUSIC: SoundData = include_wav!("sfx/music.wav");
static GUN_SFX: SoundData = include_wav!("sfx/gun.wav");
static WHISTLE_SFX: SoundData = include_wav!("sfx/whistle.wav");
static BOOM_SFX: SoundData = include_wav!("sfx/boom.wav");
static FAR_SFX: SoundData = include_wav!("sfx/far.wav");
static RUMBLE_SFX: SoundData = include_wav!("sfx/rumble.wav");
static BIRD_SFX: SoundData = include_wav!("sfx/bird.wav");
static BIRD2_SFX: SoundData = include_wav!("sfx/bird2.wav");
static ENGINE_SFX: SoundData = include_wav!("sfx/engine.wav");
static VOICE_SFX: SoundData = include_wav!("sfx/voice.wav");
static MARK_SFX: SoundData = include_wav!("sfx/mark.wav");
static LINK_SFX: SoundData = include_wav!("sfx/static.wav");
static FALL_SFX: SoundData = include_wav!("sfx/fall.wav");
static CLEAR_SFX: SoundData = include_wav!("sfx/clear.wav");

const SFX_GUN: u8 = 1;
const SFX_LINK: u8 = 2;
const SFX_BOOM: u8 = 4;
const SFX_MARK: u8 = 8;
const SFX_FALL: u8 = 16;
const SFX_WHISTLE: u8 = 32;

/// Affine window kept around the plane. The world itself is generated, not stored.
const VIEW: i32 = 128;
/// Half-width of the tiles guaranteed to match the seed. A wider view would
/// wrap onto the wrong district, so the fog starts before that distance.
const SPAN: i32 = 60;
const REGION: i32 = 64;
const MAX_MARKS: usize = 128;
const FOCAL: i32 = 160;
/// Same factor the mode 7 floor uses, so a world point lands on its scanline.
/// Low enough that a moderate altitude still shows a field, not a ribbon.
const FLOOR_ZOOM: i32 = 5;
const GRAVITY: i32 = 2;
/// Mode 7 sits in this rectangle. The margins are the instrument frame, so the
/// far corners of the map stay off the picture.
const VIEW_X: i32 = 32;
const VIEW_Y: i32 = 16;
const VIEW_W: i32 = 176;
const VIEW_H: i32 = 144;
/// Radar radius, in world pixels. About three dozen tiles.
const RADAR_RANGE: i32 = 28 * 8;

const T_SKY: u8 = 0;
const T_FLOOR: u8 = 1;
const T_FLOOR2: u8 = 2;
const T_DIRT: u8 = 3;
const T_ROAD: u8 = 4;
const T_WATER: u8 = 5;
const T_TREE: u8 = 6;
const T_CRATER: u8 = 7;
/// Parked with the sheet. The mortar round is a sprite and bursts when it lands.
const T_MINE: u8 = 8;
const T_TRENCH: u8 = 9;
const T_LIP: u8 = 10;
const T_FOG: u8 = 11;
/// Extra grass, already parked in VRAM after the fog tile. They break up the flat field.
const T_GRASS_L: u8 = 12;
const T_GRASS_D: u8 = 13;
const T_TUFT: u8 = 14;
const T_BUSH: u8 = 15;
const T_PATCH: u8 = 16;

const LEVEL_COUNT: u8 = 22;
/// Guns own everything at this height and above. Under it, only their circles.
const AA_CEIL: i32 = 15;

const KIND_BOMB: u8 = 0;
const KIND_HE: u8 = 1;
/// Mortar round. A tank dies when any charge but a bullet lands in the hatch.
const KIND_MINE: u8 = 2;
const KIND_GUN: u8 = 3;
const KIND_FLAK: u8 = 4;
const KIND_SHELL: u8 = 5;
/// Underbarrel grenade. Infantry only.
const KIND_VOG: u8 = 6;

const ARM_GUN: u8 = 0;
const ARM_BOMB: u8 = 1;
const ARM_HE: u8 = 2;
const ARM_MINE: u8 = 3;
const ARM_VOG: u8 = 4;
/// World pixels from the turret centre. The hatch is the dark square in the top view.
const HATCH: i32 = 6;
/// Beside the hull. Wide enough that a normal bomb drop counts.
const TANK_NEAR: i32 = 28;
/// Bombs beside the hull that finish a tank. The hatch is still one shot.
const BOMB_HULL_HITS: i8 = 3;

/// How many blasts beside the hull finish a full tank. Zero leaves the hull alone.
fn hull_hits(kind: u8) -> i8 {
    match kind {
        KIND_BOMB => BOMB_HULL_HITS,
        KIND_MINE => 2,
        _ => 0,
    }
}

const HP_MAN: i8 = 8;
const HP_OFFICER: i8 = 12;
const HP_TRUCK: i8 = 22;
const HP_TANK: i8 = 36;
const HP_BUNKER: i8 = 28;

const MAX_SHOTS: usize = 12;
const MAX_MINES: usize = 8;
const MAX_BOOMS: usize = 6;
const MAX_TRUCKS: usize = 4;
const MAX_TANKS: usize = 3;
const MAX_BUNKERS: usize = 3;
const MAX_SOLDIERS: usize = 8;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
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
enum Mode {
    Quad,
    /// Fly the opening. Weapons are off; missing the gap returns you to the pad.
    Gate,
    Fpv,
    /// Spotter. The sight marks a point, then the salvo arrives.
    Guide,
}

/// What ends the sortie. Each card is a different task and a different way to fly.
enum Task {
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
    /// Day strike. Stay under the gun ceiling and follow the open lane.
    Lane,
    /// Night sight, tanks, and the same low lane between the guns.
    Flare,
    /// Kamikaze along the lane. The men stand in the open gaps.
    Nest,
    /// Kamikaze through the heat sight. Trucks sit in the open gaps.
    Dart,
    /// Spotter locked under the ceiling, so the lane is the only road.
    Bend,
    /// Heat sight. The bunkers sit past the lane.
    Skirt,
    /// Small quad. It holds one charge, and only while looking straight down.
    Fetch,
}

/// What the glass shows. The side instruments stay in their own colours.
#[derive(Clone, Copy)]
enum Sensor {
    Day,
    Night,
    Heat,
    /// Active infrared illuminator: grey-green, with blooms on a blast.
    Ir,
}

fn sensor_of(level: u8) -> Sensor {
    match task_of(level) {
        Task::Night | Task::Armor | Task::Flare => Sensor::Night,
        Task::Heat | Task::Pill | Task::Dart | Task::Skirt => Sensor::Heat,
        Task::Guide | Task::Column | Task::Rush | Task::Push | Task::Bend => Sensor::Ir,
        _ => Sensor::Day,
    }
}

fn task_of(level: u8) -> Task {
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
        _ => Task::Guide,
    }
}

#[derive(Clone, Copy)]
struct Shot {
    x: i32,
    y: i32,
    z: i32,
    vx: i32,
    vy: i32,
    vz: i32,
    kind: u8,
    life: i16,
    alive: bool,
}

#[derive(Clone, Copy)]
struct Mine {
    x: i32,
    y: i32,
    arm: i16,
    alive: bool,
}

/// One mortar round lying in the grass. The quad can carry a single one.
#[derive(Clone, Copy)]
struct Cache {
    x: i32,
    y: i32,
    alive: bool,
}

#[derive(Clone, Copy)]
struct Boom {
    x: i32,
    y: i32,
    frame: i16,
    alive: bool,
}

#[derive(Clone, Copy)]
struct Truck {
    x: i32,
    y: i32,
    dir: i32,
    vertical: bool,
    home_tx: i32,
    home_ty: i32,
    /// This is the truck that has to be stopped.
    goal: bool,
    /// This truck is one of the mission marks.
    objective: bool,
    hp: i8,
    cool: u8,
    alive: bool,
}

#[derive(Clone, Copy)]
struct Tank {
    x: i32,
    y: i32,
    dir: i32,
    vertical: bool,
    home_tx: i32,
    home_ty: i32,
    objective: bool,
    hp: i8,
    cool: u8,
    alive: bool,
}

#[derive(Clone, Copy)]
struct Bunker {
    x: i32,
    y: i32,
    home_tx: i32,
    home_ty: i32,
    objective: bool,
    hp: i8,
    cool: u8,
    alive: bool,
}

#[derive(Clone, Copy)]
struct Soldier {
    x: i32,
    y: i32,
    dir: i32,
    step: u8,
    home_tx: i32,
    home_ty: i32,
    officer: bool,
    /// This soldier is one of the mission marks.
    objective: bool,
    hp: i8,
    cool: u8,
    alive: bool,
}

#[derive(Clone, Copy)]
struct Mark {
    tx: i32,
    ty: i32,
    tile: u8,
    quiet: bool,
    used: bool,
}

struct World {
    marks: Vec<Mark>,
    mark_len: usize,
    view_tx: i32,
    view_ty: i32,
    spawn_row: u8,
    level: u8,
    mode: Mode,
    x: i32,
    y: i32,
    alt: i32,
    pad_x: i32,
    pad_y: i32,
    pad_alt: i32,
    drones: i32,
    vx: i32,
    vy: i32,
    yaw: Num<i32, 8>,
    pitch: Num<i32, 8>,
    /// Display only. Follows the yaw stick so the attitude indicator banks.
    bank: Num<i32, 8>,
    bombs: i32,
    he: i32,
    mines: i32,
    vog: i32,
    /// How long the quad stick has been held, and which way. Speed grows with it.
    stick_hold: i16,
    stick_key: i8,
    /// What B fires. A steps to the next one.
    arm: u8,
    /// Airframe, 0..=100. A close blast spends it.
    hull: i32,
    /// Flight time, 0..=100. Empty drops the link.
    battery: i32,
    /// Frames spent past the radio. Turning back spends it down.
    blackout: i16,
    age: i32,
    score: i32,
    targets: i32,
    fire_cd: i16,
    recover: i16,
    crashed: bool,
    /// The last FPV airframe is gone. The same mission starts over.
    again: bool,
    /// The radio is gone. The nose drops until the airframe hits.
    falling: bool,
    /// Quad only. Holds height, looks straight down, and the stick slides.
    hover: bool,
    /// The one mortar round a small quad can carry. Fetch missions only.
    cargo: bool,
    caches: [Cache; 3],
    /// Frames until the marked salvo. Zero means the sight is free.
    guide: i16,
    salvo: i8,
    salvo_cd: i8,
    mark_x: i32,
    mark_y: i32,
    /// IR bloom after a blast. Counts down to the plain illuminator picture.
    bloom: i16,
    /// Gun, drop and blast cues for this frame.
    sfx: u8,
    shots: [Shot; MAX_SHOTS],
    mine_list: [Mine; MAX_MINES],
    booms: [Boom; MAX_BOOMS],
    trucks: [Truck; MAX_TRUCKS],
    tanks: [Tank; MAX_TANKS],
    bunkers: [Bunker; MAX_BUNKERS],
    soldiers: [Soldier; MAX_SOLDIERS],
}

/// mGBA keeps SRAM only when the ROM contains this tag.
#[used]
static SAVE_KIND: [u8; 12] = *b"SRAM_V113\0\0\0";

struct Progress {
    /// Highest mission index the arrows may select.
    unlocked: u8,
    /// Bit i is set once mission i has been cleared.
    cleared: u32,
    level: u8,
}

fn load_progress() -> Progress {
    let raw = sram_load();
    let ok = raw[0] == 0x41
        && raw[1] == 0x52
        && raw[2] == 0x43
        && raw[3] == 0x31
        && raw[7] == raw[4] ^ raw[5] ^ raw[6] ^ 0x5A;
    if !ok {
        return Progress {
            unlocked: 0,
            cleared: 0,
            level: 0,
        };
    }
    let mut cleared = raw[5] as u32;
    if raw[9] == raw[8] ^ 0xA5 {
        cleared |= (raw[8] as u32) << 8;
    }
    if raw[11] == raw[10] ^ 0x3C {
        cleared |= (raw[10] as u32) << 16;
    }
    if raw[13] == raw[12] ^ 0xC3 {
        cleared |= (raw[12] as u32) << 24;
    }
    let mut unlocked = raw[4].min(LEVEL_COUNT - 1);
    // A finished FPV card already earned the spotter mission added after it.
    if cleared & (1 << 7) != 0 {
        unlocked = unlocked.max(8).min(LEVEL_COUNT - 1);
    }
    // A finished spotter card already earned the three sorties added after it.
    if cleared & (1 << 8) != 0 {
        unlocked = unlocked.max(9).min(LEVEL_COUNT - 1);
    }
    if cleared & (1 << 11) != 0 {
        unlocked = unlocked.max(12).min(LEVEL_COUNT - 1);
    }
    Progress {
        unlocked,
        cleared,
        level: raw[6].min(unlocked),
    }
}

fn store_progress(progress: &Progress) {
    let level = progress.level.min(progress.unlocked);
    let lo = (progress.cleared & 0xFF) as u8;
    let hi = ((progress.cleared >> 8) & 0xFF) as u8;
    let mid = ((progress.cleared >> 16) & 0xFF) as u8;
    let top = ((progress.cleared >> 24) & 0xFF) as u8;
    let bytes = [
        0x41,
        0x52,
        0x43,
        0x31,
        progress.unlocked,
        lo,
        level,
        progress.unlocked ^ lo ^ level ^ 0x5A,
        hi,
        hi ^ 0xA5,
        mid,
        mid ^ 0x3C,
        top,
        top ^ 0xC3,
    ];
    for (index, byte) in bytes.into_iter().enumerate() {
        unsafe {
            (0x0E00_0000 as *mut u8)
                .add(index)
                .write_volatile(byte);
        }
    }
}

fn tone(mixer: &mut agb::sound::mixer::Mixer, data: SoundData) {
    let mut channel = SoundChannel::new_high_priority(data);
    channel.volume(num!(0.85));
    let _ = mixer.play_sound(channel);
}

/// Low priority, so a cue can never take the music's channel.
fn cue(mixer: &mut agb::sound::mixer::Mixer, data: SoundData, volume: Num<i16, 8>) {
    let mut channel = SoundChannel::new(data);
    channel.volume(volume);
    let _ = mixer.play_sound(channel);
}

fn play_cues(mixer: &mut agb::sound::mixer::Mixer, world: &mut World) {
    if world.sfx & SFX_GUN != 0 {
        cue(mixer, GUN_SFX, num!(0.8));
    }
    if world.sfx & SFX_WHISTLE != 0 {
        cue(mixer, WHISTLE_SFX, num!(0.4));
    }
    if world.sfx & SFX_MARK != 0 {
        cue(mixer, MARK_SFX, num!(0.7));
    }
    if world.sfx & SFX_LINK != 0 {
        cue(mixer, LINK_SFX, num!(0.5));
    }
    if world.sfx & SFX_FALL != 0 {
        cue(mixer, FALL_SFX, num!(0.75));
    }
    if world.sfx & SFX_BOOM != 0 {
        cue(mixer, BOOM_SFX, num!(0.75));
    }
    world.sfx = 0;
}

/// Menu and the opening cards keep the tune. Flight is the field instead.
fn music_bed(mixer: &mut agb::sound::mixer::Mixer, slot: &mut Option<ChannelId>, play: bool) {
    if play {
        let playing = match slot {
            Some(id) => mixer.channel(id).is_some(),
            None => false,
        };
        if !playing {
            let mut music = SoundChannel::new_high_priority(MUSIC);
            music.should_loop();
            music.volume(num!(0.5));
            *slot = mixer.play_sound(music);
        }
    } else if let Some(id) = slot.take() {
        if let Some(channel) = mixer.channel(&id) {
            channel.stop();
        }
    }
}

/// Daytime birds, and the war going on somewhere else in the field.
fn ambience(mixer: &mut agb::sound::mixer::Mixer, world: &World, shot: bool, blew: bool) {
    let day = matches!(sensor_of(world.level), Sensor::Day);
    if day && world.age > 40 && hash(world.age, 4) % 190 == 0 {
        let bird = if hash(world.age, 2) & 1 == 0 {
            BIRD_SFX
        } else {
            BIRD2_SFX
        };
        cue(mixer, bird, num!(0.28));
    }
    let close = 72 * 72;
    let by_rig = world.trucks.iter().any(|truck| {
        truck.alive && dist2_px(world.x, world.y, truck.x, truck.y) <= close
    }) || world.tanks.iter().any(|tank| {
        tank.alive && dist2_px(world.x, world.y, tank.x, tank.y) <= close
    });
    if by_rig && hash(world.age, 21) % 75 == 0 {
        cue(mixer, ENGINE_SFX, num!(0.3));
    }
    let talk = 48 * 48;
    let by_men = world.soldiers.iter().any(|soldier| {
        soldier.alive && dist2_px(world.x, world.y, soldier.x, soldier.y) <= talk
    });
    if by_men && hash(world.age, 27) % 120 == 0 {
        cue(mixer, VOICE_SFX, num!(0.28));
    }
    let shot_every = if day { 150 } else { 80 };
    let boom_every = if day { 340 } else { 200 };
    if !shot && world.age > 40 && hash(world.age, 9) % shot_every == 0 {
        cue(mixer, FAR_SFX, num!(0.42));
    }
    if !blew && world.age > 40 && hash(world.age, 13) % boom_every == 0 {
        cue(mixer, RUMBLE_SFX, num!(0.4));
    }
}

fn sram_load() -> [u8; 14] {
    let mut raw = [0u8; 14];
    for (index, byte) in raw.iter_mut().enumerate() {
        *byte = unsafe { (0x0E00_0000 as *const u8).add(index).read_volatile() };
    }
    raw
}

#[agb::entry]
fn main(mut gba: agb::Gba) -> ! {
    core::hint::black_box(&SAVE_KIND);
    let mut gfx = gba.graphics.get();
    gfx.set_background_palettes(backgrounds::PALETTES);
    // Colour 0 is the backdrop and is unused by the ground tiles. The sky is
    // just this fill; clouds and ridges are sprites on top of it.
    gfx.set_background_palette_colour_256(0, Rgb::new(88, 144, 200).to_rgb15());

    // agb refuses a 16 KB map (its heap check is exclusive), so the loader is
    // 64×64. The real 128×128 window is written straight into the next 16 KB.
    let mut ground = AffineBackground::new(
        Priority::P1,
        AffineBackgroundSize::Background64x64,
        AffineBackgroundWrapBehaviour::Wrap,
    );
    clear_ground_map();
    install_floor();
    let mut progress = load_progress();
    let mut phase = Phase::Splash;
    let mut splash = 0u8;
    let mut splash_age = 0i32;
    let mut menu = 0u8;
    let mut input = ButtonController::new();
    let mut mixer = gba.mixer.mixer(Frequency::Hz10512);
    let mut music_id: Option<ChannelId> = None;
    let mut world = World::new(&mut ground, progress.level, &mut || {
        mixer.frame();
    });

    loop {
        input.update();

        if phase == Phase::Splash {
            let skip = input.is_just_pressed(Button::Start) || input.is_just_pressed(Button::A);
            splash_age += 1;
            if skip || splash_age > 160 {
                splash_age = 0;
                if splash == 0 {
                    splash = 1;
                } else {
                    phase = Phase::Title;
                }
            }
        } else if phase == Phase::Title {
            let picked = match input.just_pressed_x_tri() {
                Tri::Positive if world.level < progress.unlocked => Some(world.level + 1),
                Tri::Negative if world.level > 0 => Some(world.level - 1),
                _ => None,
            };
            if let Some(level) = picked {
                progress.level = level;
                store_progress(&progress);
                world = World::new(&mut ground, level, &mut || {
                    mixer.frame();
                });
            }
        }

        if phase == Phase::Pause {
            match input.just_pressed_y_tri() {
                Tri::Positive => menu = 1,
                Tri::Negative => menu = 0,
                Tri::Zero => {}
            }
            let choose =
                input.is_just_pressed(Button::A) || input.is_just_pressed(Button::Start);
            if choose {
                if menu == 0 {
                    phase = Phase::Fly;
                } else {
                    world = World::new(&mut ground, world.level, &mut || {
                        mixer.frame();
                    });
                    phase = Phase::Title;
                }
            }
        } else if phase != Phase::Splash && input.is_just_pressed(Button::Start) {
            match phase {
                Phase::Title => {
                    world = World::new(&mut ground, world.level, &mut || {
                        mixer.frame();
                    });
                    phase = Phase::Fly;
                }
                Phase::Crash => {
                    world = World::new(&mut ground, world.level, &mut || {
                        mixer.frame();
                    });
                    phase = Phase::Title;
                }
                Phase::Win | Phase::Done => {
                    let next = if world.level < progress.unlocked {
                        world.level + 1
                    } else {
                        world.level
                    };
                    progress.level = next;
                    store_progress(&progress);
                    world = World::new(&mut ground, next, &mut || {
                        mixer.frame();
                    });
                    phase = Phase::Title;
                }
                Phase::Fly => {
                    menu = 0;
                    phase = Phase::Pause;
                }
                Phase::Splash | Phase::Pause => {}
            }
        }

        match phase {
            Phase::Splash => {}
            Phase::Title => {
                world.yaw += num!(1) / Num::new(360);
                wrap_yaw(&mut world.yaw);
                world.update_trucks();
                world.update_tanks();
                world.update_soldiers();
            }
            Phase::Fly => {
                world.fly(&input, &mut ground);
                let restart = world.again;
                if world.targets <= 0 {
                    tone(&mut mixer, CLEAR_SFX);
                    progress.cleared |= 1u32 << world.level;
                    if world.level + 1 < LEVEL_COUNT {
                        progress.unlocked = progress.unlocked.max(world.level + 1);
                    }
                    store_progress(&progress);
                    phase = Phase::Win;
                } else if world.crashed {
                    phase = Phase::Crash;
                }
                let shot = world.sfx & SFX_GUN != 0;
                let blew = world.sfx & SFX_BOOM != 0;
                play_cues(&mut mixer, &mut world);
                if restart && phase == Phase::Fly {
                    let level = world.level;
                    world = World::new(&mut ground, level, &mut || {
                        mixer.frame();
                    });
                    phase = Phase::Title;
                } else if phase == Phase::Fly {
                    ambience(&mut mixer, &world, shot, blew);
                }
            }
            Phase::Pause | Phase::Crash | Phase::Win | Phase::Done => {}
        }

        if phase != Phase::Pause {
            world.maintain_actors();
        }

        // Ahead of the floor build. A second fill before vblank covers a long
        // frame, which otherwise replays a slice of the music.
        music_bed(
            &mut mixer,
            &mut music_id,
            !matches!(phase, Phase::Fly | Phase::Pause),
        );
        mixer.frame();

        if phase == Phase::Splash {
            gfx.set_background_palette_colour_256(0, Rgb::new(8, 8, 16).to_rgb15());
            let mut frame = gfx.frame();
            if splash == 0 {
                show(&mut frame, sprites::TITLE.sprite(0), 88, 64);
            } else {
                show(&mut frame, sprites::AUTHOR.sprite(0), 88, 64);
            }
            mixer.frame();
            frame.commit();
            continue;
        }

        let looking_down = world.hover && matches!(phase, Phase::Fly | Phase::Pause);
        set_nadir(if looking_down {
            nadir_scale(world.alt)
        } else {
            0
        });
        let horizon = if looking_down {
            -160
        } else {
            world.horizon()
        };
        let (sin_r, cos_r) = world.heading();
        build_floor(
            ring_space(world.x),
            ring_space(world.y),
            world.alt.max(256),
            world.yaw,
            horizon,
        );

        let sky_h = sky_bottom(horizon, world.alt);
        let sensor = sensor_of(world.level);
        let playing = matches!(phase, Phase::Fly | Phase::Pause);
        let blown = playing && world.recover > 0;
        paint_sensor(sensor);
        gfx.set_background_palette_colour_256(
            0,
            if blown {
                Rgb::new(32, 32, 32)
            } else {
                match sensor {
                    Sensor::Night => Rgb::new(1, 8, 2),
                    Sensor::Heat => Rgb::new(2, 1, 4),
                    Sensor::Ir => Rgb::new(5, 5, 5),
                    Sensor::Day => Rgb::new(88, 144, 200),
                }
            }
            .to_rgb15(),
        );

        let mut frame = gfx.frame();
        let ground_id = ground.show(&mut frame);
        let cockpit = playing;
        mask_view(&mut frame, ground_id, sky_h, cockpit, false);

        // First object submitted is the front-most. Instruments go in before
        // the snow, so the operator's panel stays up while the glass is noise.
        if blown {
            if phase == Phase::Pause {
                draw_pause(&mut frame, menu);
            }
            draw_hud(&mut frame, &world);
            draw_cockpit(&mut frame, &world, sin_r, cos_r);
            draw_noise(&mut frame, world.age);
            mixer.frame();
            frame.commit();
            arm_ground();
            arm_haze(world.bloom as i32);
            publish_floor();
            world.sync_ground(&mut || {
                mixer.frame();
            });
            continue;
        }
        match phase {
            Phase::Title => {
                show(&mut frame, sprites::TITLE.sprite(0), 88, 4);
                let card = match task_of(world.level) {
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
                    Task::Fetch if world.level % LEVEL_COUNT == 21 => sprites::LVL_HAUL.sprite(0),
                    Task::Fetch => sprites::LVL_LIFT.sprite(0),
                };
                show(&mut frame, card, 88, 40);
                draw_digits(&mut frame, world.level as i32 + 1, 2, 64, 52);
                if progress.cleared & (1u32 << world.level) != 0 {
                    show(&mut frame, sprites::DOT.sprite(1), 84, 52);
                }
                let help = match task_of(world.level) {
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
                    Task::Fetch if world.level % LEVEL_COUNT == 21 => sprites::HELP_HAUL.sprite(0),
                    Task::Fetch => sprites::HELP_LIFT.sprite(0),
                    Task::Armor => sprites::HELP_ARMOR.sprite(0),
                    Task::Pill => sprites::HELP_PILL.sprite(0),
                    _ => sprites::HELP_QUAD.sprite(0),
                };
                show(&mut frame, help, 88, 108);
            }
            Phase::Crash => show(&mut frame, sprites::CRASH.sprite(0), 88, 64),
            Phase::Win | Phase::Done => {
                show(&mut frame, sprites::WIN.sprite(0), 88, 56);
                draw_digits(&mut frame, world.score, 4, 104, 96);
            }
            Phase::Fly | Phase::Pause => {
                if phase == Phase::Pause {
                    draw_pause(&mut frame, menu);
                }
                draw_hud(&mut frame, &world);
                draw_cockpit(&mut frame, &world, sin_r, cos_r);
                draw_hurt(&mut frame, &world);
            }
            Phase::Splash => {}
        }
        draw_world(
            &mut frame,
            &world,
            sin_r,
            cos_r,
            horizon,
            cockpit,
            sensor,
        );
        draw_horizon(
            &mut frame,
            &world,
            sky_h,
            cockpit,
            !matches!(sensor, Sensor::Day),
        );

        mixer.frame();
        frame.commit();
        // Still in vblank: retarget BG2, publish this frame's scanlines, then
        // write new map columns before the picture starts.
        arm_ground();
        arm_haze(world.bloom as i32);
        publish_floor();
        world.sync_ground(&mut || {
            mixer.frame();
        });
    }
}

/// Flight uses a smaller window so the map edges stay hidden. Menus keep the
/// full frame. Win0 is the ground, Win1 is the sky inside that window.
fn mask_view(
    frame: &mut agb::display::GraphicsFrame,
    ground_id: AffineBackgroundId,
    sky_h: i32,
    cockpit: bool,
    night: bool,
) {
    if !cockpit {
        {
            let sky = frame.windows().win_in(WinIn::Win0).enable_objects();
            if night {
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
        if night {
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
        if night {
            ground.enable_blending();
        }
        ground.set_pos(rect(vec2(VIEW_X, sky_bot), vec2(VIEW_W, ground_span)));
    }
    let outside = frame.windows().win_out().enable_objects();
    if night {
        outside.enable_blending();
    }
}

fn clear_ground_map() {
    unsafe {
        let map = 0x0600_C000 as *mut u8;
        for cell in 0..(128 * 128) {
            map.add(cell).write_volatile(T_FLOOR + 1);
        }
    }
}

/// BG2 control: priority 1, screenblock 24, wrap, size 128×128.
fn arm_ground() {
    unsafe {
        (0x0400_000C as *mut u16).write_volatile(0xF801);
    }
}

/// Horizon sprites and clouds blend into the sky and the ground, so the
/// skyline edge is a haze instead of a hard cut. HUD sprites stay opaque.
/// A positive bloom whites out the glass and then lets it back.
fn arm_haze(bloom: i32) {
    unsafe {
        if bloom > 0 {
            // Brighten BG2 and the backdrop only. Instruments are sprites.
            (0x0400_0050 as *mut u16).write_volatile(0x00A4);
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
fn draw_horizon(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    sky_h: i32,
    cockpit: bool,
    night: bool,
) {
    if night {
        return;
    }
    let pan = world.yaw.to_raw();
    let alt_px = (world.alt >> 8).max(1);
    draw_clouds(frame, pan / 3, sky_h, alt_px, cockpit);
    draw_mist(frame, pan / 2, sky_h, alt_px, cockpit);
    draw_skyline(frame, pan, sky_h, cockpit);
}

fn draw_skyline(frame: &mut agb::display::GraphicsFrame, scroll: i32, sky_h: i32, cockpit: bool) {
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

fn draw_clouds(
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
fn draw_mist(
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

fn wrap_coord(value: i32) -> i32 {
    value
}

fn wrap_yaw(yaw: &mut Num<i32, 8>) {
    if *yaw >= num!(1) {
        *yaw -= num!(1);
    }
    if *yaw < num!(0) {
        *yaw += num!(1);
    }
}

fn delta(a: i32, b: i32) -> i32 {
    a.wrapping_sub(b)
}

fn tile_of(sub: i32) -> i32 {
    (sub >> 8).div_euclid(8)
}

fn tri_value(tri: Tri) -> i32 {
    match tri {
        Tri::Negative => -1,
        Tri::Zero => 0,
        Tri::Positive => 1,
    }
}

fn hash(x: i32, y: i32) -> u32 {
    let mut n = x
        .wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263));
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    n as u32
}

fn start_tile(level: u8) -> (i32, i32) {
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
    }
}

struct AaGun {
    tx: i32,
    ty: i32,
    radius: i32,
}

/// Three gates. The open side shifts right, then back left.
const LANE_GUNS: [AaGun; 6] = [
    AaGun { tx: -4, ty: 78, radius: 104 },
    AaGun { tx: 48, ty: 78, radius: 104 },
    AaGun { tx: 6, ty: 56, radius: 120 },
    AaGun { tx: 54, ty: 56, radius: 104 },
    AaGun { tx: -6, ty: 34, radius: 104 },
    AaGun { tx: 42, ty: 34, radius: 104 },
];

fn aa_guns(task: Task) -> &'static [AaGun] {
    match task {
        Task::Lane | Task::Flare | Task::Nest | Task::Dart | Task::Bend | Task::Skirt => {
            &LANE_GUNS
        }
        _ => &[],
    }
}

fn gun_at(gun: &AaGun) -> (i32, i32) {
    ((gun.tx * 8 + 4) << 8, (gun.ty * 8 + 4) << 8)
}

fn aa_covers(guns: &[AaGun], x: i32, y: i32) -> bool {
    guns.iter().any(|gun| {
        let (gx, gy) = gun_at(gun);
        dist2_px(x, y, gx, gy) <= gun.radius * gun.radius
    })
}

/// Radar dial point back into the world. Ahead on the dial is negative `ry`.
fn lane_world(cam_x: i32, cam_y: i32, sin_r: i32, cos_r: i32, rx: i32, ry: i32) -> (i32, i32) {
    let fwd = -ry * RADAR_RANGE / 12;
    let right = rx * RADAR_RANGE / 12;
    let dx = (fwd * sin_r + right * cos_r) >> 8;
    let dy = (right * sin_r - fwd * cos_r) >> 8;
    (cam_x + (dx << 8), cam_y + (dy << 8))
}

/// One battlefield everywhere. Roads tie the districts together. Trenches
/// and a few copses sit in the grass. Water is a pond, not a scatter of puddles.
/// The skyline is not this map.
fn terrain(level: u8, tx: i32, ty: i32) -> u8 {
    let (pad_x, pad_y) = start_tile(level);
    if (tx - pad_x).abs() <= 3 && (ty - pad_y).abs() <= 3 {
        return T_FLOOR;
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
fn pond(tx: i32, ty: i32) -> bool {
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

fn dist2_px(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    let dx = delta(ax, bx) >> 8;
    let dy = delta(ay, by) >> 8;
    dx * dx + dy * dy
}

fn dead_shot() -> Shot {
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

fn dead_mine() -> Mine {
    Mine {
        x: 0,
        y: 0,
        arm: 0,
        alive: false,
    }
}

fn dead_cache() -> Cache {
    Cache {
        x: 0,
        y: 0,
        alive: false,
    }
}

/// Rounds to pick up. The later card keeps them farther from the pad.
fn cache_spots(level: u8) -> [(i32, i32); 3] {
    if level % LEVEL_COUNT == 21 {
        [(14, 78), (34, 76), (24, 70)]
    } else {
        [(18, 82), (30, 82), (24, 76)]
    }
}

/// Where those rounds have to land. The trucks stay put.
fn mortar_spots(level: u8) -> [(i32, i32); 3] {
    if level % LEVEL_COUNT == 21 {
        [(10, 40), (24, 34), (40, 42)]
    } else {
        [(16, 52), (26, 46), (36, 50)]
    }
}

fn dead_boom() -> Boom {
    Boom {
        x: 0,
        y: 0,
        frame: 0,
        alive: false,
    }
}

fn dead_mark() -> Mark {
    Mark {
        tx: 0,
        ty: 0,
        tile: T_FLOOR,
        quiet: false,
        used: false,
    }
}

fn dead_truck() -> Truck {
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

fn dead_tank() -> Tank {
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

fn dead_bunker() -> Bunker {
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

fn dead_soldier() -> Soldier {
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

/// Facing stays in `dir`. The step is a crawl so a mark cannot leave the radio.
fn ground_step(dir: i32) -> i32 {
    if dir > 0 {
        18
    } else if dir < 0 {
        -18
    } else {
        0
    }
}

impl World {
    fn new(bg: &mut AffineBackground, level: u8, pump: &mut dyn FnMut()) -> Self {
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
            drones: if mode == Mode::Fpv { 4 } else { 0 },
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
            arm: if mode == Mode::Fpv {
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
            },
            fire_cd: 0,
            recover: 0,
            crashed: false,
            again: false,
            falling: false,
            hover: fetch,
            cargo: false,
            caches: [dead_cache(); 3],
            guide: 0,
            salvo: 0,
            salvo_cd: 0,
            mark_x: 0,
            mark_y: 0,
            bloom: 0,
            sfx: 0,
            shots: [dead_shot(); MAX_SHOTS],
            mine_list: [dead_mine(); MAX_MINES],
            booms: [dead_boom(); MAX_BOOMS],
            trucks: [dead_truck(); MAX_TRUCKS],
            tanks: [dead_tank(); MAX_TANKS],
            bunkers: [dead_bunker(); MAX_BUNKERS],
            soldiers: [dead_soldier(); MAX_SOLDIERS],
        };
        world.park(bg);
        world.refill(pump);
        world.seat_objective();
        if matches!(task, Task::Convoy) {
            world.paint(bg, 0, 4, T_FLOOR);
            world.paint(bg, 0, 3, T_FLOOR);
        }
        world.spawn_nearby(pump);
        world
    }

    fn gate_x(&self) -> i32 {
        (24 * 8 + 4) << 8
    }

    fn gate_y(&self) -> i32 {
        (6 * 8 + 4) << 8
    }

    fn seat_objective(&mut self) {
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

    fn hq_goal(&self, tx: i32, ty: i32) -> bool {
        matches!(task_of(self.level), Task::Hq) && (22..=24).contains(&tx) && (16..=17).contains(&ty)
    }

    /// A kill counts only when it is this card's objective.
    fn claim(&mut self, officer: bool, truck: bool, goal: bool, objective: bool) {
        let counts = match task_of(self.level) {
            Task::Officer => officer,
            Task::Trench => objective && !truck,
            Task::Night | Task::Heat | Task::Guide | Task::Column | Task::Lane | Task::Dart
            | Task::Bend => truck && objective,
            Task::Convoy => goal,
            Task::Fpv => objective,
            Task::Rush => objective && !truck,
            Task::Nest => objective && !truck,
            Task::Push => objective,
            Task::Gate | Task::Hq | Task::Armor | Task::Pill | Task::Flare | Task::Skirt
            | Task::Fetch => false,
        };
        if counts && self.targets > 0 {
            self.targets -= 1;
            self.sfx |= SFX_MARK;
        }
    }

    fn claim_armor(&mut self) {
        if matches!(task_of(self.level), Task::Armor | Task::Flare | Task::Fetch)
            && self.targets > 0
        {
            self.targets -= 1;
            self.sfx |= SFX_MARK;
        }
    }

    fn claim_pill(&mut self) {
        if matches!(task_of(self.level), Task::Pill | Task::Skirt) && self.targets > 0 {
            self.targets -= 1;
            self.sfx |= SFX_MARK;
        }
    }

    /// Hold time, not the glass, sets how fast the quad slides.
    fn quad_creep(&mut self, pitch_in: Tri, yaw_in: Tri) -> i32 {
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
    fn fetch_tick(&mut self, input: &ButtonController) {
        self.arm = ARM_MINE;
        self.mines = if self.cargo { 1 } else { 0 };
        if !self.hover || self.signal_loss() >= 16 || !input.is_just_pressed(Button::B) {
            return;
        }
        if self.cargo {
            if self.try_shot(KIND_MINE) {
                self.cargo = false;
            }
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

    fn ground_at(&self, tx: i32, ty: i32) -> u8 {
        for mark in &self.marks[..self.mark_len] {
            if mark.tx == tx && mark.ty == ty {
                return mark.tile;
            }
        }
        terrain(self.level, tx, ty)
    }

    fn is_quiet(&self, tx: i32, ty: i32) -> bool {
        self.marks[..self.mark_len]
            .iter()
            .any(|mark| mark.quiet && mark.tx == tx && mark.ty == ty)
    }

    fn evict_mark(&self) -> usize {
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

    fn insert_mark(&mut self, tx: i32, ty: i32, tile: u8, quiet: bool) {
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

    fn mark_quiet(&mut self, tx: i32, ty: i32) {
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

    fn blit(&self, tx: i32, ty: i32) {
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
    fn park(&mut self, bg: &mut AffineBackground) {
        let last = (T_FOG as u16 + 7).max(T_MINE as u16);
        for tile in (T_SKY as u16)..=last {
            bg.set_tile((tile as i32, 0), &backgrounds::TERRAIN.tiles, tile);
        }
    }

    /// The window is thousands of tiles. `pump` keeps the music buffer full
    /// while that write runs, so a mission change does not replay a slice.
    fn refill(&mut self, pump: &mut dyn FnMut()) {
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
    fn sync_ground(&mut self, pump: &mut dyn FnMut()) {
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

    fn occupied(&self, tx: i32, ty: i32) -> bool {
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

    fn try_spawn_soldier(&mut self, tx: i32, ty: i32) {
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

    fn try_spawn_truck(&mut self, tx: i32, ty: i32, vertical: bool) {
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

    fn try_spawn_tank(&mut self, tx: i32, ty: i32, vertical: bool) {
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

    fn try_spawn_bunker(&mut self, tx: i32, ty: i32) {
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

    fn consider_tile(&mut self, tx: i32, ty: i32) {
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

    fn spawn_nearby(&mut self, pump: &mut dyn FnMut()) {
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

    fn retire_far(&mut self) {
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

    fn maintain_actors(&mut self) {
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

    fn heading(&self) -> (i32, i32) {
        (self.yaw.sin().to_raw(), self.yaw.cos().to_raw())
    }

    fn horizon(&self) -> i32 {
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

    fn fly(&mut self, input: &ButtonController, bg: &mut AffineBackground) {
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
            if input.is_just_pressed(Button::A) {
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
        self.check_rush();
        self.update_soldiers();
        self.enemy_fire();
        self.aa_tick();
        self.update_booms();
        self.wear();
    }

    /// B under the crosshair calls the salvo. The count runs, then the rounds land.
    fn guide_tick(&mut self, input: &ButtonController, bg: &mut AffineBackground) {
        if self.guide == 0
            && self.salvo == 0
            && self.signal_loss() < 16
            && !self.falling
            && input.is_just_pressed(Button::B)
        {
            let (sin_r, cos_r) = self.heading();
            let (nx, ny, _) = self.sight_aim(sin_r, cos_r);
            self.mark_x = nx;
            self.mark_y = ny;
            self.guide = 240;
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
    }

    /// Always flies forward. The stick tilts the nose; height and speed follow
    /// that angle, and the nose stays where you leave it.
    /// Kamikaze spends one drone and returns to the pad instead of ending the mission.
    fn fly_plane(&mut self, input: &ButtonController, bg: &mut AffineBackground) -> bool {
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
        } else if fetch && input.is_just_pressed(Button::Select) {
            // The quad keeps the same slide. Select only swaps the glass.
            self.hover = !self.hover;
            self.sfx |= SFX_MARK;
        } else if !fetch && input.is_just_pressed(Button::Select) {
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
        let yaw_step = if self.hover || fetch_fly {
            Num::new(0)
        } else {
            Num::new(tri_value(yaw_in)) / Num::new(160)
        };
        self.yaw += yaw_step;
        wrap_yaw(&mut self.yaw);
        // The airframe does not roll. The indicator leans into the turn and
        // settles back when the stick is released.
        if !self.hover && !fetch_fly {
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
            // The stick slides the quad. Looking down does not change that speed.
            // Holding a direction speeds it up. A tap stays slow enough for a hatch.
            let creep = if fetch_fly {
                self.quad_creep(pitch_in, yaw_in)
            } else {
                48
            };
            let along = -tri_value(pitch_in) * creep;
            let strafe = tri_value(yaw_in) * creep;
            self.vx = (sin_r * along + cos_r * strafe) >> 8;
            self.vy = ((-cos_r) * along + sin_r * strafe) >> 8;
            self.x = wrap_coord(self.x + self.vx);
            self.y = wrap_coord(self.y + self.vy);
            self.bank += (Num::new(0) - self.bank) / Num::new(6);
        } else {
        let sp = self.pitch.sin().to_raw();
        let cp = self.pitch.cos().to_raw().max(0);
        // The spotter holds one height and loiters. Pitch only aims the glass.
        let cruise = if self.mode == Mode::Guide { 80 } else { 400 };
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
        if self.alt > 40 << 8 {
            self.alt = 40 << 8;
        }
        if !self.cross_gate() {
            return false;
        }

        // Contact is the airframe itself, a few pixels out, so you can fly
        // up to a soldier before the charge goes.
        let touching = self.alt <= 4 << 8 && self.point_blank();
        let dug_in = self.alt <= 0;
        if touching || dug_in {
            if dug_in {
                self.alt = 0;
            }
            if self.mode == Mode::Fpv {
                self.explode(bg, self.x, self.y, 48, 0, KIND_BOMB);
            } else if touching {
                let _ = self.strike_at(bg, self.x, self.y);
            } else {
                self.add_boom(self.x, self.y);
            }
            self.falling = false;
            self.recover = 22;
            self.vx = 0;
            self.vy = 0;
            return true;
        }
        true
    }

    /// True when the airframe is on top of a soldier or a truck.
    fn point_blank(&self) -> bool {
        let reach = 14 * 14;
        self.soldiers.iter().any(|soldier| {
            soldier.alive && dist2_px(self.x, self.y, soldier.x, soldier.y) <= reach
        }) || self.trucks.iter().any(|truck| {
            truck.alive && dist2_px(self.x, self.y, truck.x, truck.y) <= reach
        })
    }

    /// Where the nose meets the ground. Level flight looks far ahead; a dive
    /// pulls that point in toward the plane.
    fn nose(&self, sin_r: i32, cos_r: i32) -> (i32, i32) {
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
    fn strike_at(&mut self, bg: &mut AffineBackground, hx: i32, hy: i32) -> bool {
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

    fn note_kill(
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
        self.add_boom(x, y);
        self.score += points;
        self.claim(officer, truck, goal, objective);
        let tx = tile_of(x);
        let ty = tile_of(y);
        if self.ground_at(tx, ty) != T_WATER {
            self.paint(bg, tx, ty, T_CRATER);
        }
    }

    fn relaunch(&mut self) -> bool {
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
    fn signal_loss(&self) -> i32 {
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

    fn wear(&mut self) {
        if self.age % 48 == 0 && self.battery > 0 {
            self.battery -= 1;
        }
        let loss = self.signal_loss();
        if loss >= 14 && self.age % 40 == 0 && self.battery > 0 {
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
                self.crashed = true;
            }
            return;
        }
        // A dead radio or a dead battery drops the nose. The airframe falls
        // until it hits, instead of vanishing back to the pad.
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

    fn try_shot(&mut self, kind: u8) -> bool {
        let (sin_r, cos_r) = self.heading();
        // Hover drops straight through the crosshair. Forward flight still leads.
        let (x, y, vx, vy) = if self.hover {
            (self.x, self.y, 0, 0)
        } else {
            let lead = if self.mode == Mode::Quad { 36 } else { 12 };
            (
                wrap_coord(self.x + sin_r * lead),
                wrap_coord(self.y - cos_r * lead),
                self.vx,
                self.vy,
            )
        };
        let z = self.alt;
        let Some(slot) = self.shots.iter_mut().find(|shot| !shot.alive) else {
            return false;
        };
        *slot = Shot {
            x,
            y,
            z,
            vx,
            vy,
            vz: if self.hover {
                0
            } else {
                -((110 * self.pitch.sin().to_raw()) >> 8)
            },
            kind,
            life: 0,
            alive: true,
        };
        if kind == KIND_BOMB {
            self.sfx |= SFX_WHISTLE;
        }
        true
    }

    /// Ground under the fixed crosshair. The tracer stays on that plus.
    fn sight_aim(&self, sin_r: i32, cos_r: i32) -> (i32, i32, i32) {
        if self.hover {
            return (self.x, self.y, 0);
        }
        let view_cy = VIEW_Y + VIEW_H / 2;
        let drop = view_cy - self.horizon();
        if drop < 4 {
            let lead = 160;
            return (
                wrap_coord(self.x + sin_r * lead),
                wrap_coord(self.y - cos_r * lead),
                self.alt,
            );
        }
        let fwd = (self.alt as i64) * (FOCAL as i64) * (FLOOR_ZOOM as i64) / drop as i64;
        let lead = (fwd >> 8).clamp(8, 200) as i32;
        (
            wrap_coord(self.x + sin_r * lead),
            wrap_coord(self.y - cos_r * lead),
            0,
        )
    }

    /// A burst to the point under the crosshair. It hurts people and trucks, not the ground.
    fn try_gun(&mut self) -> bool {
        let (sin_r, cos_r) = self.heading();
        let (nx, ny, nz) = self.sight_aim(sin_r, cos_r);
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

    fn launch_at_player(&mut self, x: i32, y: i32, kind: u8) -> bool {
        let Some(slot) = self.shots.iter_mut().find(|shot| !shot.alive) else {
            return false;
        };
        // Wide of the drone on purpose: a small high target is hard to hit.
        let scatter_x = (hash(self.age, x) % 57) as i32 - 28;
        let scatter_y = (hash(self.age.wrapping_add(9), y) % 57) as i32 - 28;
        let z_err = ((hash(self.age, x ^ y) % 11) as i32 - 5) << 8;
        let tx = self.x + (scatter_x << 8);
        let ty = self.y + (scatter_y << 8);
        let steps = 22;
        *slot = Shot {
            x,
            y,
            z: 1 << 8,
            vx: (tx - x) / steps,
            vy: (ty - y) / steps,
            vz: (self.alt + z_err - (1 << 8)) / steps,
            kind,
            life: 0,
            alive: true,
        };
        true
    }

    /// Under the ceiling the circles bite. Above it the guns see the whole route.
    fn aa_tick(&mut self) {
        let guns = aa_guns(task_of(self.level));
        if guns.is_empty() || self.recover > 0 {
            return;
        }
        let low = (self.alt >> 8) < AA_CEIL;
        let inside = aa_covers(guns, self.x, self.y);
        if low && !inside {
            return;
        }
        if self.age % 24 == 0 {
            self.hull -= 3;
            if self.hull < 0 {
                self.hull = 0;
            }
        }
        if self.age % 36 == 0 {
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
            let _ = self.launch_at_player(gx, gy, KIND_FLAK);
        }
    }

    fn enemy_fire(&mut self) {
        if self.mode == Mode::Gate {
            return;
        }
        let alt_px = self.alt >> 8;
        // High enough and they never pick the drone out.
        if alt_px > 16 {
            return;
        }
        let keen = alt_px <= 7;
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
            if !keen && hash(self.age, index as i32) % 3 != 0 {
                self.soldiers[index].cool = 50;
                continue;
            }
            self.soldiers[index].cool = 200;
            let _ = self.launch_at_player(sx, sy, KIND_FLAK);
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
            if !keen && hash(self.age, index as i32 + 19) % 3 != 0 {
                self.trucks[index].cool = 40;
                continue;
            }
            self.trucks[index].cool = 170;
            let _ = self.launch_at_player(tx, ty, KIND_SHELL);
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
            if !keen && hash(self.age, index as i32 + 31) % 3 != 0 {
                self.tanks[index].cool = 40;
                continue;
            }
            self.tanks[index].cool = 150;
            let _ = self.launch_at_player(tx, ty, KIND_SHELL);
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
            if !keen && hash(self.age, index as i32 + 47) % 2 != 0 {
                self.bunkers[index].cool = 24;
                continue;
            }
            self.bunkers[index].cool = 70;
            let _ = self.launch_at_player(bx, by, KIND_FLAK);
        }
    }

    fn step_gun(&mut self, bg: &mut AffineBackground, index: usize) {
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

    fn hurt_soldier(&mut self, bg: &mut AffineBackground, index: usize) {
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

    fn hurt_truck(&mut self, bg: &mut AffineBackground, index: usize) {
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

    fn hurt_tank(&mut self, bg: &mut AffineBackground, index: usize) {
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

    fn hurt_bunker(&mut self, bg: &mut AffineBackground, index: usize) {
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

    fn step_flak(&mut self, index: usize) {
        let kind = self.shots[index].kind;
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
            self.hull -= if kind == KIND_SHELL { 8 } else { 4 };
            if self.hull < 0 {
                self.hull = 0;
            }
            self.add_boom(self.x, self.y);
            return;
        }
        if life > 28 || z > 48 << 8 || z < 0 {
            self.shots[index].alive = false;
        }
    }

    fn update_shots(&mut self, bg: &mut AffineBackground) {
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

    fn update_trucks(&mut self) {
        for index in 0..MAX_TRUCKS {
            if !self.trucks[index].alive {
                continue;
            }
            if self.trucks[index].objective && matches!(task_of(self.level), Task::Push) {
                self.trucks[index].y -= 48;
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

    fn update_tanks(&mut self) {
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

    fn update_soldiers(&mut self) {
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
                self.soldiers[index].y -= 48;
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

    fn update_booms(&mut self) {
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

    fn explode(&mut self, bg: &mut AffineBackground, x: i32, y: i32, radius: i32, depth: i32, kind: u8) {
        self.add_boom(x, y);
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

    fn damage_tiles(&mut self, bg: &mut AffineBackground, x: i32, y: i32, radius: i32) {
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

    fn paint(&mut self, bg: &mut AffineBackground, tx: i32, ty: i32, tile: u8) {
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

    /// The convoy truck reached the mark. The sortie is lost.
    fn check_convoy(&mut self) {
        if !matches!(task_of(self.level), Task::Convoy) || self.targets <= 0 {
            return;
        }
        let arrived = self.trucks.iter().any(|truck| {
            truck.alive && truck.goal && tile_of(truck.y) <= 4
        });
        if arrived {
            self.crashed = true;
        }
    }

    /// The base line is north. A living mark that reaches it takes the place.
    fn check_rush(&mut self) {
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
            self.crashed = true;
        }
    }

    /// Crossing the gate line either clears the card or sends the plane back.
    fn cross_gate(&mut self) -> bool {
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

    fn add_boom(&mut self, x: i32, y: i32) {
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
            frame: 0,
            alive: true,
        };
        self.sfx |= SFX_BOOM;
    }

}

/// Sky covers every line that would read past the streamed window.
/// A scanline's far corner is about `lambda * 25` tiles out. Past that the
/// 128-tile ring shows some other district, and the longer the flight the
/// more of those cells are roads, water and craters.
fn sky_bottom(horizon: i32, alt: i32) -> i32 {
    let alt_px = (alt >> 8).max(1);
    // 48 tiles is inside the 60-tile window and short of the 64-tile alias.
    let skip = (alt_px * FLOOR_ZOOM * 25 / 48).max(12);
    (horizon + skip).clamp(horizon + 8, HEIGHT).max(0)
}

/// The displayed map is 128 tiles, 1024 pixels. The hardware wraps that, but a
/// negative reference does not wrap: it shows the sky colour as one scanline.
/// Two extra rings keep every visible line positive without moving the tile.
fn ring_space(sub: i32) -> i32 {
    const RING: i32 = VIEW * 8 * 256;
    sub.rem_euclid(RING) + RING * 2
}

struct Nadir(core::cell::UnsafeCell<i32>);
unsafe impl Sync for Nadir {}
static NADIR: Nadir = Nadir(core::cell::UnsafeCell::new(0));

fn set_nadir(scale: i32) {
    unsafe { *NADIR.0.get() = scale }
}

fn nadir_on() -> i32 {
    unsafe { *NADIR.0.get() }
}

/// World pixels per screen pixel while the glass looks straight down.
fn nadir_scale(alt: i32) -> i32 {
    ((alt >> 8).max(8) / 8).clamp(1, 4)
}

/// Flat glass. Scanline 88, pixel 120 is the ground under the airframe.
fn floor_nadir(
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

fn floor_line(
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
struct FloorLine {
    a: i16,
    b: i16,
    c: i16,
    d: i16,
    x: i32,
    y: i32,
}

const FLAT_LINE: FloorLine = FloorLine {
    a: 0x100,
    b: 0,
    c: 0,
    d: 0,
    x: 0,
    y: 0,
};

struct FloorBuf {
    front: core::cell::UnsafeCell<usize>,
    lines: core::cell::UnsafeCell<[[FloorLine; 162]; 2]>,
}

unsafe impl Sync for FloorBuf {}

static FLOOR: FloorBuf = FloorBuf {
    front: core::cell::UnsafeCell::new(0),
    lines: core::cell::UnsafeCell::new([[FLAT_LINE; 162]; 2]),
};

fn write_floor_line(line: &FloorLine) {
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
const FLOOR_DMA: u32 = 4
    | (0b11 << 21)
    | (1 << 25)
    | (1 << 26)
    | (0b10 << 28)
    | (1 << 31);

/// The sound mixer takes an interrupt every frame. An HBlank interrupt then
/// misses lines and the ground sticks. DMA0 copies the line in hardware.
fn install_floor() {
    let stop = unsafe {
        interrupt::add_interrupt_handler(Interrupt::VBlank, |_| {
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

fn arm_floor_dma() {
    unsafe {
        let front = *FLOOR.front.get();
        let lines = &(*FLOOR.lines.get())[front];
        write_floor_line(&lines[0]);
        (0x0400_00B0 as *mut u32).write_volatile(lines.as_ptr().add(1) as u32);
        (0x0400_00B4 as *mut u32).write_volatile(0x0400_0020);
        (0x0400_00B8 as *mut u32).write_volatile(FLOOR_DMA);
    }
}

fn publish_floor() {
    unsafe {
        let front = FLOOR.front.get();
        *front ^= 1;
        write_floor_line(&(*FLOOR.lines.get())[*front][0]);
    }
}

fn build_floor(cam_x: i32, cam_y: i32, alt: i32, yaw: Num<i32, 8>, horizon: i32) {
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

fn narrow(value: Num<i32, 8>) -> Num<i16, 8> {
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

fn project(
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
        let sy = 88 - (fwd >> 8) / scale;
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
    if !(-32..WIDTH + 32).contains(&sx) || !(-32..HEIGHT + 32).contains(&sy) {
        return None;
    }
    Some((sx, sy))
}

fn sprite_pixels(drop: i32, alt: i32, world_h: i32, min_px: i32, max_px: i32) -> i32 {
    let scale = nadir_on();
    if scale > 0 {
        return (world_h * 2 / scale).clamp(min_px, max_px);
    }
    let alt_px = (alt >> 8).max(1);
    (world_h * drop / (alt_px * FLOOR_ZOOM)).clamp(min_px, max_px)
}

/// Eight headings around the object, then the same eight from a steeper dive.
fn heading_octant(fwd: i32, right: i32) -> usize {
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

fn projection(pitch: Num<i32, 8>, fx: i32, fy: i32, sin_r: i32, cos_r: i32) -> usize {
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

fn actor_matrix(inv: Num<i32, 8>) -> AffineMatrix<Num<i32, 8>> {
    AffineMatrix::from_scale(vec2(inv, inv))
}

/// White wash on the hit. The 64 art is blown up to the 128 double-size box.
fn show_glare(frame: &mut agb::display::GraphicsFrame, sx: i32, sy: i32) {
    let inv = Num::<i32, 8>::new(1) / Num::new(2);
    ObjectAffine::new(
        SpriteVram::from(sprites::FLASH.sprite(0)),
        actor_matrix(inv).into(),
        AffineMode::AffineDouble,
    )
    .set_pos((sx - 64, sy - 64))
    .set_priority(Priority::P0)
    .show(frame);
}

fn show_soldier(
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
) {
    let drop = (sy - horizon).max(1);
    let pixels = sprite_pixels(drop, alt, 16, 12, 28);
    let inv = Num::<i32, 8>::new(32) / Num::new(pixels);
    let (fx, fy) = if soldier.dir == 0 {
        (0, -1)
    } else {
        (soldier.dir.signum(), 0)
    };
    let frame_index = projection(pitch, fx, fy, sin_r, cos_r);
    let matrix = actor_matrix(inv);
    let bob = ((soldier.step / 8) & 1) as i32;
    let x = sx - 16;
    let y = sy - pixels / 2 - 16 - bob;
    if x < -32 || y < -32 || x > WIDTH || y > HEIGHT {
        return;
    }
    let mut object = ObjectAffine::new(
        SpriteVram::from(sprites::SOLDIER.sprite(frame_index)),
        matrix.into(),
        AffineMode::Affine,
    );
    if let Some(palette) = body_palette(sensor) {
        object.set_palette(palette);
    }
    object
        .set_pos((x, y))
        .set_priority(Priority::P0)
        .show(frame);
}

fn show_truck(
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
) {
    let drop = (sy - horizon).max(1);
    let pixels = sprite_pixels(drop, alt, 20, 16, 32);
    let inv = Num::<i32, 8>::new(32) / Num::new(pixels);
    let (fx, fy) = if truck.vertical {
        (0, truck.dir.signum())
    } else {
        (truck.dir.signum(), 0)
    };
    // Side views stay wheels-down. Straight down uses the plan frames.
    let _ = pitch;
    let fwd = fx * sin_r - fy * cos_r;
    let right = fx * cos_r + fy * sin_r;
    let oct = heading_octant(fwd, right);
    let top = nadir_on() > 0;
    let frame_index = if top { 8 + oct } else { oct };
    let matrix = actor_matrix(inv);
    // Side views sit on their wheels. The plan view is centred on the ground point.
    let x = sx - 16;
    let y = if top { sy - 16 } else { sy - pixels / 2 - 16 };
    if x < -32 || y < -32 || x > WIDTH || y > HEIGHT {
        return;
    }
    let mut object = ObjectAffine::new(
        SpriteVram::from(sprites::TRUCK.sprite(frame_index)),
        matrix.into(),
        AffineMode::Affine,
    );
    if let Some(palette) = body_palette(sensor) {
        object.set_palette(palette);
    }
    object
        .set_pos((x, y))
        .set_priority(Priority::P0)
        .show(frame);
}

fn show_tank(
    frame: &mut agb::display::GraphicsFrame,
    tank: &Tank,
    sx: i32,
    sy: i32,
    horizon: i32,
    alt: i32,
    sin_r: i32,
    cos_r: i32,
    sensor: Sensor,
) {
    let drop = (sy - horizon).max(1);
    let pixels = sprite_pixels(drop, alt, 22, 18, 34);
    let inv = Num::<i32, 8>::new(32) / Num::new(pixels);
    let (fx, fy) = if tank.vertical {
        (0, tank.dir.signum())
    } else {
        (tank.dir.signum(), 0)
    };
    let fwd = fx * sin_r - fy * cos_r;
    let right = fx * cos_r + fy * sin_r;
    let oct = heading_octant(fwd, right);
    let top = nadir_on() > 0;
    let frame_index = if top { 8 + oct } else { oct };
    let matrix = actor_matrix(inv);
    let x = sx - 16;
    let y = if top { sy - 16 } else { sy - pixels / 2 - 16 };
    if x < -32 || y < -32 || x > WIDTH || y > HEIGHT {
        return;
    }
    let mut object = ObjectAffine::new(
        SpriteVram::from(sprites::TANK.sprite(frame_index)),
        matrix.into(),
        AffineMode::Affine,
    );
    if let Some(palette) = body_palette(sensor) {
        object.set_palette(palette);
    }
    object
        .set_pos((x, y))
        .set_priority(Priority::P0)
        .show(frame);
}

fn show_bunker(
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
) {
    let drop = (sy - horizon).max(1);
    let pixels = sprite_pixels(drop, alt, 18, 14, 30);
    let inv = Num::<i32, 8>::new(32) / Num::new(pixels);
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
    let top = nadir_on() > 0;
    let frame_index = if top { 8 + oct } else { oct };
    let matrix = actor_matrix(inv);
    let x = sx - 16;
    let y = if top { sy - 16 } else { sy - pixels / 2 - 16 };
    if x < -32 || y < -32 || x > WIDTH || y > HEIGHT {
        return;
    }
    let mut object = ObjectAffine::new(
        SpriteVram::from(sprites::BUNKER.sprite(frame_index)),
        matrix.into(),
        AffineMode::Affine,
    );
    if let Some(palette) = body_palette(sensor) {
        object.set_palette(palette);
    }
    object
        .set_pos((x, y))
        .set_priority(Priority::P0)
        .show(frame);
}

/// Three red pips. They shrink as the target is hit.
fn show_life(
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
    for index in 0..n {
        show(frame, sprites::DOT.sprite(0), sx - 8 + index * 4, sy - 26);
    }
}

fn draw_pause(frame: &mut agb::display::GraphicsFrame, menu: u8) {
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

fn show(frame: &mut agb::display::GraphicsFrame, sprite: &'static Sprite, x: i32, y: i32) {
    if x < -64 || y < -32 || x > WIDTH || y > HEIGHT {
        return;
    }
    Object::new(SpriteVram::from(sprite))
        .set_pos((x, y))
        .set_priority(Priority::P0)
        .show(frame);
}

fn show_haze(
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

fn draw_digits(
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

fn in_view(sx: i32, sy: i32, cockpit: bool) -> bool {
    !cockpit || (sx >= VIEW_X && sx < VIEW_X + VIEW_W && sy >= VIEW_Y && sy < VIEW_Y + VIEW_H)
}

/// Fixed crosshair, attitude indicator on the left, radar on the right, and
/// the dark columns that hide the edges of the floor.
fn draw_cockpit(
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
fn draw_sight(frame: &mut agb::display::GraphicsFrame, world: &World) {
    let view_cx = VIEW_X + VIEW_W / 2;
    let view_cy = VIEW_Y + VIEW_H / 2;
    if world.mode == Mode::Fpv {
        draw_fpv_rig(frame, world, view_cx, view_cy);
    } else {
        show(frame, sprites::RETICLE.sprite(0), view_cx - 8, view_cy - 8);
        if world.mode == Mode::Guide {
            draw_guide_glass(frame, world, view_cx, view_cy);
        }
    }
}

fn show_flip(
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

fn draw_guide_glass(
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

    let horizon = world.horizon();
    let floor = sky_bottom(horizon, world.alt);
    let mut best: Option<(i32, i32)> = None;
    let mut best_d = 40 * 40;
    let mut consider = |wx: i32, wy: i32| {
        let Some((sx, sy)) = project(
            world.x, world.y, world.alt, sin_r, cos_r, horizon, wx, wy, 0,
        ) else {
            return;
        };
        // Same test as the vehicle sprite. A far mark still projects along
        // the look direction, but the box waits until the body is on the glass.
        if !in_view(sx, sy, true) || sy < floor {
            return;
        }
        let ddx = sx - view_cx;
        let ddy = sy - view_cy;
        let d = ddx * ddx + ddy * ddy;
        if d < best_d {
            best_d = d;
            best = Some((sx, sy));
        }
    };
    for truck in &world.trucks {
        if truck.alive {
            consider(truck.x, truck.y);
        }
    }
    for tank in &world.tanks {
        if tank.alive {
            consider(tank.x, tank.y);
        }
    }
    for soldier in &world.soldiers {
        if soldier.alive && soldier.objective {
            consider(soldier.x, soldier.y);
        }
    }
    if let Some((sx, sy)) = best {
        let s = 12;
        show_flip(frame, mark, sx - s, sy - s, false, false);
        show_flip(frame, mark, sx + s - 8, sy - s, true, false);
        show_flip(frame, mark, sx - s, sy + s - 8, false, true);
        show_flip(frame, mark, sx + s - 8, sy + s - 8, true, true);
    }
}

/// Two thin leads from the lower corners to the mine on the crosshair.
fn draw_fpv_rig(
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

fn draw_wire(
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

fn boom_sheet(frame: i16) -> usize {
    if frame < 7 {
        0
    } else if frame < 14 {
        1
    } else {
        2
    }
}

fn show_fit(
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
fn draw_gauges(frame: &mut agb::display::GraphicsFrame, world: &World) {
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
fn draw_hurt(frame: &mut agb::display::GraphicsFrame, world: &World) {
    if world.hull > 40 {
        return;
    }
    show(frame, sprites::CRACK.sprite(0), VIEW_X + 28, VIEW_Y + 24);
    if world.hull <= 20 {
        show(frame, sprites::CRACK.sprite(1), VIEW_X + 96, VIEW_Y + 64);
    }
}

fn draw_noise(frame: &mut agb::display::GraphicsFrame, age: i32) {
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

/// The field through the glass. Temperature is not the tile's brightness:
/// grass stays cold, and only bodies are painted hot.
fn paint_sensor(sensor: Sensor) {
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
                };
                ram.add(pal * 16 + index).write_volatile(out);
            }
        }
    }
}

fn ir_ground(colour: u16) -> u16 {
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

fn cold_ground(colour: u16) -> u16 {
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

fn nvg_ground(colour: u16) -> u16 {
    let (r, g, b) = channels(colour);
    let y = luma(colour);
    let mut lit = (9 + y * 2 / 3).clamp(8, 26);
    if b > r + 2 && b > g {
        lit = (lit - 5).max(6);
    } else if r > g && r > b {
        lit = (lit + 2).min(26);
    }
    let green = lit as u16;
    (green / 7) | (green << 5) | ((green / 6) << 10)
}

struct BodyPals {
    heat: PaletteVram,
    night: PaletteVram,
    ir: PaletteVram,
}

struct BodyHold(core::cell::UnsafeCell<Option<BodyPals>>);
unsafe impl Sync for BodyHold {}
static BODY_PALS: BodyHold = BodyHold(core::cell::UnsafeCell::new(None));

fn body_palette(sensor: Sensor) -> Option<PaletteVram> {
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
        Sensor::Day => None,
    }
}

fn bake_body(sensor: Sensor) -> Palette16 {
    let mut colours = [Rgb15(0); 16];
    unsafe {
        let src = 0x0500_0200 as *const u16;
        for index in 1..16 {
            let colour = src.add(index).read_volatile();
            colours[index] = Rgb15(match sensor {
                Sensor::Heat => heat_body(colour),
                Sensor::Night => nvg_body(colour),
                Sensor::Ir => ir_body(colour),
                Sensor::Day => colour,
            });
        }
    }
    Palette16::new(colours)
}

fn heat_body(colour: u16) -> u16 {
    let y = luma(colour);
    if y < 5 {
        8 | (1 << 5)
    } else if y < 12 {
        26 | (8 << 5) | (1 << 10)
    } else if y < 20 {
        31 | (18 << 5) | (3 << 10)
    } else {
        31 | (31 << 5) | (26 << 10)
    }
}

fn ir_body(colour: u16) -> u16 {
    let y = luma(colour);
    let v = (22 + y / 3).clamp(20, 31) as u16;
    v | (v << 5) | (v << 10)
}

fn nvg_body(colour: u16) -> u16 {
    let y = luma(colour);
    let green = (18 + y / 2).clamp(16, 31) as u16;
    (green / 6) | (green << 5) | ((green / 5) << 10)
}

fn channels(colour: u16) -> (i32, i32, i32) {
    let r = (colour & 31) as i32;
    let g = ((colour >> 5) & 31) as i32;
    let b = ((colour >> 10) & 31) as i32;
    (r, g, b)
}

fn luma(colour: u16) -> i32 {
    let (r, g, b) = channels(colour);
    (r * 3 + g * 6 + b) / 10
}

fn draw_static(frame: &mut agb::display::GraphicsFrame, world: &World) {
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

/// Yellow dots are the air the guns do not own, and only while under the ceiling.
fn draw_aa_lanes(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    sin_r: i32,
    cos_r: i32,
    radar_x: i32,
    inst_y: i32,
) {
    let guns = aa_guns(task_of(world.level));
    if guns.is_empty() || (world.alt >> 8) >= AA_CEIL {
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
fn draw_radar_goals(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    sin_r: i32,
    cos_r: i32,
    radar_x: i32,
    inst_y: i32,
) {
    let mut pts = [(0i32, 0i32); 3];
    let n = goal_points(world, &mut pts);
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

fn goal_points(world: &World, pts: &mut [(i32, i32); 3]) -> usize {
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
        Task::Convoy => world
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
                let mut n = 0;
                for cache in &world.caches {
                    if cache.alive && n < 3 {
                        pts[n] = (cache.x, cache.y);
                        n += 1;
                    }
                }
                n
            }
        }
    }
}

/// Nearest living mission soldiers, or mission trucks.
fn nearest_marked(
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
fn nearest(world: &World, pts: &mut [(i32, i32); 3], trucks_only: bool) -> usize {
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
fn show_dot(
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
fn radar_blip(dx: i32, dy: i32, sin_r: i32, cos_r: i32) -> (i32, i32) {
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

fn isqrt64(n: i64) -> i64 {
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

fn draw_hud(frame: &mut agb::display::GraphicsFrame, world: &World) {
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

fn draw_world(
    frame: &mut agb::display::GraphicsFrame,
    world: &World,
    sin_r: i32,
    cos_r: i32,
    horizon: i32,
    cockpit: bool,
    sensor: Sensor,
) {
    let floor = sky_bottom(horizon, world.alt);
    for boom in &world.booms {
        if !boom.alive {
            continue;
        }
        let Some((sx, sy)) = project(
            world.x, world.y, world.alt, sin_r, cos_r, horizon, boom.x, boom.y, 0,
        ) else {
            continue;
        };
        if !in_view(sx, sy, cockpit) {
            continue;
        }
        let frame_index = boom_sheet(boom.frame);
        let dist2 = dist2_px(world.x, world.y, boom.x, boom.y);
        // The FPV crash is drawn on the wire frame, in front of the glass.
        if world.mode == Mode::Fpv && dist2 < 48 * 48 {
            continue;
        }
        // Night and the grey picture wash out white at the hit. The fire
        // sprite is yellow, so it stays off those sensors. The far ground
        // line must not hide the wash: that is where a distant mark sits.
        if matches!(sensor, Sensor::Night | Sensor::Ir) {
            if boom.frame < 40 {
                show_glare(frame, sx, sy);
            }
        } else if sy >= floor {
            if boom.frame < 10 {
                show(frame, sprites::FLASH.sprite(0), sx - 32, sy - 32);
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
    for soldier in &world.soldiers {
        if !soldier.alive {
            continue;
        }
        let Some((sx, sy)) = project(
            world.x, world.y, world.alt, sin_r, cos_r, horizon, soldier.x, soldier.y, 0,
        ) else {
            continue;
        };
        if !in_view(sx, sy, cockpit) || sy < floor {
            continue;
        }
        if soldier.officer {
            show(frame, sprites::BLIP.sprite(1), sx - 4, sy - 22);
        }
        let man_max = if soldier.officer { HP_OFFICER } else { HP_MAN };
        show_life(frame, sx, sy, soldier.hp, man_max);
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
        if !in_view(sx, sy, cockpit) || sy < floor {
            continue;
        }
        show_life(frame, sx, sy, truck.hp, HP_TRUCK);
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
        if !in_view(sx, sy, cockpit) || sy < floor {
            continue;
        }
        show_life(frame, sx, sy, tank.hp, HP_TANK);
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
        if !in_view(sx, sy, cockpit) || sy < floor {
            continue;
        }
        show_life(frame, sx, sy, bunker.hp, HP_BUNKER);
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
        );
    }
    draw_gate(frame, world, sin_r, cos_r, horizon, cockpit);
    draw_sites(frame, world, sin_r, cos_r, horizon, cockpit);
    draw_aa_guns(frame, world, sin_r, cos_r, horizon, cockpit, sensor);
}

/// Two dark posts and a lintel. The opening is the mission.
fn draw_gate(
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
fn draw_sites(
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
        show(frame, sprites::HQ.sprite(0), sx - 16, sy - 32);
    }
}

/// The batteries themselves. Their circles are the holes in the radar lane.
fn draw_aa_guns(
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
        if cockpit && (!in_view(sx, sy, true) || sy < floor) {
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
        );
    }
}
